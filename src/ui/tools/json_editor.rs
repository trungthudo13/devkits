use super::*;
use egui::{text::LayoutJob, FontId, TextFormat};
use std::collections::BTreeSet;

pub const EXAMPLE: &str = r#"{"user":{"id":123,"name":"John Doe","email":"john.doe@example.com"},"products":[{"id":"p1","name":"Product A","price":19.99},{"id":"p2","name":"Product B","price":29.99},{"id":"p3","name":"Product C","price":39.99}],"order":{"orderId":"abc123","date":"2023-08-18","items":[{"productId":"p1","quantity":2},{"productId":"p3","quantity":1}]}}"#;

fn layout(text: &str, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let colors = design::palette(dark);
    let base = colors.text;
    let key = colors.primary.text;
    let string = colors.success.text;
    let value = colors.warning.text;
    if text.len() > 128 * 1024 {
        job.append(
            text,
            0.,
            TextFormat {
                font_id: FontId::monospace(crate::ui::components::code_editor::FONT_SIZE),
                line_height: Some(crate::ui::components::code_editor::LINE_HEIGHT),
                color: base,
                ..Default::default()
            },
        );
        return job;
    }
    let mut i = 0;
    while i < text.len() {
        let start = i;
        let c = text[i..].chars().next().unwrap();
        let color = if c == '"' {
            i += 1;
            let mut escaped = false;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                i += c.len_utf8();
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    break;
                }
            }
            if text[i..].trim_start().starts_with(':') {
                key
            } else {
                string
            }
        } else if c.is_ascii_digit() || c == '-' || c.is_ascii_alphabetic() {
            i += c.len_utf8();
            while i < text.len() && text.as_bytes()[i].is_ascii_alphanumeric()
                || i < text.len() && matches!(text.as_bytes()[i], b'.' | b'+' | b'-')
            {
                i += 1;
            }
            value
        } else {
            i += c.len_utf8();
            if matches!(c, '{' | '}' | '[' | ']') {
                string
            } else {
                base
            }
        };
        job.append(
            &text[start..i],
            0.,
            TextFormat {
                font_id: FontId::monospace(crate::ui::components::code_editor::FONT_SIZE),
                line_height: Some(crate::ui::components::code_editor::LINE_HEIGHT),
                color,
                ..Default::default()
            },
        );
    }
    job
}

// Brackets inside strings never become fold markers. Row numbers refer to source.
fn fold_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut stack = Vec::new();
    let mut ranges = Vec::new();
    let mut row = 0;
    let mut quoted = false;
    let mut escaped = false;
    for c in text.chars() {
        if c == '\n' {
            row += 1;
        }
        if quoted {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '{' | '[' => stack.push((c, row)),
            '}' | ']' => {
                if let Some((open, start)) = stack.pop() {
                    if (open == '{' && c == '}' || open == '[' && c == ']') && row > start {
                        ranges.push((start, row));
                    }
                }
            }
            _ => {}
        }
    }
    ranges.sort_unstable();
    ranges.dedup_by_key(|r| r.0);
    ranges
}
fn visible_rows(
    count: usize,
    ranges: &[(usize, usize)],
    collapsed: &BTreeSet<usize>,
) -> Vec<usize> {
    let mut result = Vec::new();
    let mut row = 0;
    while row < count {
        result.push(row);
        row = if collapsed.contains(&row) {
            ranges
                .iter()
                .find(|r| r.0 == row)
                .map(|r| r.1 + 1)
                .unwrap_or(row + 1)
        } else {
            row + 1
        };
    }
    result
}

impl DevKits {
    pub fn json_action(&mut self, action: Action) {
        let doc = &mut self.docs[0];
        match business::process(
            &doc.input,
            Language::Json,
            action,
            self.indent,
            self.dialect,
        ) {
            Ok(result) => {
                doc.error = false;
                if action == Action::Lint {
                    doc.message = result;
                } else {
                    doc.input = result;
                    doc.output.clear();
                    doc.message.clear();
                    self.json_collapsed.clear();
                }
            }
            Err(e) => {
                doc.error = true;
                doc.message = e;
            }
        }
    }
    fn json_escape(&mut self, remove: bool) {
        let doc = &mut self.docs[0];
        let result = business::documents::json_escape(&doc.input, remove);
        match result {
            Ok(text) => {
                doc.input = text;
                doc.message.clear();
                doc.error = false;
                self.json_collapsed.clear();
            }
            Err(e) => {
                doc.error = true;
                doc.message = e;
            }
        }
    }
    pub fn json_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(8., 8.);
        ui.spacing_mut().button_padding = egui::vec2(9., 4.);
        for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
            ui.style_mut()
                .text_styles
                .insert(style, FontId::proportional(14.));
        }

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
            |ui| {
                if ui
                    .add(design::button(
                        format!("{} Format", icons::BRACKETS_CURLY),
                        design::colors(ui).primary,
                    ))
                    .on_hover_text("Ctrl/Cmd + Enter")
                    .clicked()
                {
                    self.json_action(Action::Format);
                }
                if ui
                    .button(format!("{} Compress", icons::ARROWS_IN))
                    .clicked()
                {
                    self.json_action(Action::Minify);
                }
                if ui
                    .button(format!("{} Clipboard", icons::CLIPBOARD))
                    .on_hover_text("Paste from system clipboard")
                    .clicked()
                {
                    match desktop::clipboard_text() {
                        Ok(text) if text.len() <= business::MAX_INPUT => {
                            self.docs[0] = Document {
                                input: text,
                                ..Default::default()
                            };
                            self.json_collapsed.clear();
                        }
                        Ok(_) => {
                            self.docs[0].error = true;
                            self.docs[0].message = "Clipboard exceeds the 2 MiB limit.".into();
                        }
                        Err(e) => {
                            self.docs[0].error = true;
                            self.docs[0].message = format!("Cannot read clipboard: {e}");
                        }
                    }
                }
                if ui
                    .button(format!("{} Escape", icons::ARROWS_IN))
                    .on_hover_text("Encode the editor text as a JSON string")
                    .clicked()
                {
                    self.json_escape(false);
                }
                if ui
                    .button(format!("{} Remove Escape", icons::ARROWS_OUT))
                    .on_hover_text("Decode a JSON-quoted string")
                    .clicked()
                {
                    self.json_escape(true);
                }
                if ui
                    .add(
                        egui::Button::new(format!("{} Simple", icons::BOOKMARK_SIMPLE))
                            .selected(self.json_simple),
                    )
                    .on_hover_text("Simple editor: hide fold controls")
                    .clicked()
                {
                    self.json_simple = !self.json_simple;
                    self.json_collapsed.clear();
                }
            },
        );
    }

    pub fn json_ui(&mut self, ui: &mut egui::Ui) {
        if !self.docs[0].message.is_empty() {
            design::notice(
                ui,
                if self.docs[0].error {
                    design::colors(ui).danger
                } else {
                    design::colors(ui).success
                },
                &self.docs[0].message,
            );
        }
        let ranges = if self.json_simple {
            Vec::new()
        } else {
            fold_ranges(&self.docs[0].input)
        };
        self.json_collapsed
            .retain(|start| ranges.iter().any(|r| r.0 == *start));
        let response = if self.json_collapsed.is_empty() {
            crate::ui::components::code_editor::editor_with_folds(
                ui,
                &mut self.docs[0].input,
                "json",
                layout,
                &ranges,
                &mut self.json_collapsed,
            )
        } else {
            let lines: Vec<_> = self.docs[0].input.split('\n').collect();
            let rows = visible_rows(lines.len(), &ranges, &self.json_collapsed);
            egui::ScrollArea::both()
                .id_salt("json-folded-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    ui.spacing_mut().button_padding = egui::Vec2::ZERO;
                    ui.spacing_mut().interact_size.y =
                        crate::ui::components::code_editor::LINE_HEIGHT;
                    for row in rows {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            let (number_rect, _) = ui.allocate_exact_size(
                                egui::vec2(32., crate::ui::components::code_editor::LINE_HEIGHT),
                                egui::Sense::hover(),
                            );
                            ui.painter().text(
                                egui::pos2(number_rect.right(), number_rect.top() + 1.),
                                egui::Align2::RIGHT_TOP,
                                (row + 1).to_string(),
                                FontId::monospace(crate::ui::components::code_editor::FONT_SIZE),
                                design::colors(ui).muted,
                            );
                            if let Some(&(_, end)) = ranges.iter().find(|r| r.0 == row) {
                                let folded = self.json_collapsed.contains(&row);
                                if ui
                                    .add_sized(
                                        [16., crate::ui::components::code_editor::LINE_HEIGHT],
                                        egui::Button::new(if folded {
                                            icons::CARET_RIGHT
                                        } else {
                                            icons::CARET_DOWN
                                        })
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    if folded {
                                        self.json_collapsed.remove(&row);
                                    } else {
                                        self.json_collapsed.insert(row);
                                    }
                                }
                                let line = if folded {
                                    format!("{} … {}", lines[row], lines[end].trim_start())
                                } else {
                                    lines[row].to_string()
                                };
                                if ui
                                    .add(
                                        egui::Label::new(layout(&line, self.dark))
                                            .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text("Click code to expand all blocks and edit")
                                    .clicked()
                                {
                                    self.json_collapsed.clear();
                                }
                            } else {
                                ui.add_space(16.);
                                if ui
                                    .add(
                                        egui::Label::new(layout(lines[row], self.dark))
                                            .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text("Click code to expand all blocks and edit")
                                    .clicked()
                                {
                                    self.json_collapsed.clear();
                                }
                            }
                        });
                    }
                    ui.allocate_response(ui.available_size(), egui::Sense::click())
                })
                .inner
        };
        if response.changed() {
            self.json_collapsed.clear();
            self.docs[0].message.clear();
        }
        response.context_menu(|ui| {
            if ui.button("Example").clicked() {
                self.docs[0].input = EXAMPLE.into();
                self.json_action(Action::Format);
                ui.close_menu();
            }
            if ui.button("Validate JSON").clicked() {
                self.json_action(Action::Lint);
                ui.close_menu();
            }
            if ui.button("Expand all").clicked() {
                self.json_collapsed.clear();
                ui.close_menu();
            }
            if ui.button("Copy JSON").clicked() {
                ui.ctx().copy_text(self.docs[0].input.clone());
                ui.close_menu();
            }
            if ui.button("Open file…").clicked() {
                self.open();
                self.json_collapsed.clear();
                ui.close_menu();
            }
            if ui.button("Save JSON…").clicked() {
                if let Some(path) = desktop::save_file("document.json", "json") {
                    if let Err(e) = business::documents::save_source(&path, &self.docs[0].input) {
                        self.docs[0].error = true;
                        self.docs[0].message = e.to_string();
                    }
                }
                ui.close_menu();
            }
            if ui.button("Clear").clicked() {
                self.docs[0] = Document::default();
                self.json_collapsed.clear();
                ui.close_menu();
            }
        });
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter)) {
            self.json_action(Action::Format);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_actions_and_escape_roundtrip() {
        let mut app = DevKits::default();
        app.docs[0].input = EXAMPLE.into();
        app.json_action(Action::Format);
        assert!(app.docs[0].input.contains("\n  \"user\""));
        app.json_escape(false);
        app.json_escape(true);
        app.json_action(Action::Minify);
        assert_eq!(app.docs[0].input, EXAMPLE);
        app.docs[0].input = "{bad}".into();
        app.json_action(Action::Format);
        assert!(app.docs[0].error);
        assert_eq!(app.docs[0].input, "{bad}");
        app.json_escape(true);
        assert!(app.docs[0].error);
        assert_eq!(app.docs[0].input, "{bad}");
    }
    #[test]
    fn folding_preserves_original_line_numbers_and_ignores_quoted_brackets() {
        let text = "{\n  \"a\": [\n    \"}\\\"[\",\n    {}\n  ],\n  \"b\": true\n}";
        let ranges = fold_ranges(text);
        assert_eq!(ranges, vec![(0, 6), (1, 4)]);
        assert_eq!(
            visible_rows(7, &ranges, &BTreeSet::from([1])),
            vec![0, 1, 5, 6]
        );
        assert_eq!(visible_rows(7, &ranges, &BTreeSet::from([0, 1])), vec![0]);
        assert_eq!(layout(text, false).text, text);
        assert_eq!(
            layout("{\"name\":\"Tiếng Việt", false).text,
            "{\"name\":\"Tiếng Việt"
        );
    }
}
