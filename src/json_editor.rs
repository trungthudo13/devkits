use super::*;
use egui::{text::LayoutJob, FontId, TextFormat};
use std::collections::BTreeSet;

pub const EXAMPLE: &str = r#"{"user":{"id":123,"name":"John Doe","email":"john.doe@example.com"},"products":[{"id":"p1","name":"Product A","price":19.99},{"id":"p2","name":"Product B","price":29.99},{"id":"p3","name":"Product C","price":39.99}],"order":{"orderId":"abc123","date":"2023-08-18","items":[{"productId":"p1","quantity":2},{"productId":"p3","quantity":1}]}}"#;

fn layout(text: &str, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let base = if dark {
        Color32::from_gray(220)
    } else {
        Color32::from_rgb(25, 25, 30)
    };
    let key = if dark {
        Color32::from_rgb(245, 145, 145)
    } else {
        Color32::from_rgb(175, 20, 20)
    };
    let string = if dark {
        Color32::from_rgb(135, 175, 255)
    } else {
        Color32::from_rgb(0, 65, 160)
    };
    let value = if dark {
        Color32::from_rgb(120, 210, 160)
    } else {
        Color32::from_rgb(0, 125, 76)
    };
    if text.len() > 128 * 1024 {
        job.append(
            text,
            0.,
            TextFormat {
                font_id: FontId::monospace(11.),
                line_height: Some(16.),
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
                font_id: FontId::monospace(11.),
                line_height: Some(16.),
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
        match self_devkits::process(
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
        if doc.input.len() > self_devkits::MAX_INPUT {
            doc.error = true;
            doc.message = "Input exceeds the 2 MiB limit.".into();
            return;
        }
        let result = if remove {
            serde_json::from_str::<String>(&doc.input)
        } else {
            serde_json::to_string(&doc.input)
        };
        match result {
            Ok(text) => {
                doc.input = text;
                doc.message.clear();
                doc.error = false;
                self.json_collapsed.clear();
            }
            Err(e) => {
                doc.error = true;
                doc.message = format!("Expected a JSON-quoted string: {e}");
            }
        }
    }
    pub fn json_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(8., 8.);
        ui.spacing_mut().button_padding = egui::vec2(9., 4.);
        for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
            ui.style_mut()
                .text_styles
                .insert(style, FontId::proportional(11.));
        }
        if !self.dark {
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(239, 238, 244);
            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
            ui.visuals_mut().widgets.inactive.corner_radius = egui::CornerRadius::same(1);
        }
        ui.horizontal_wrapped(|ui| {
            // Align the compact toolbar to the right at normal window widths.
            ui.add_space((ui.available_width() - 494.).max(0.));
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
            if ui
                .button(format!("{} Remove Escape", icons::ARROWS_OUT))
                .on_hover_text("Decode a JSON-quoted string")
                .clicked()
            {
                self.json_escape(true);
            }
            if ui
                .button(format!("{} Escape", icons::ARROWS_IN))
                .on_hover_text("Encode the editor text as a JSON string")
                .clicked()
            {
                self.json_escape(false);
            }
            if ui
                .button(format!("{} Clipboard", icons::CLIPBOARD))
                .on_hover_text("Paste from system clipboard")
                .clicked()
            {
                match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                    Ok(text) if text.len() <= self_devkits::MAX_INPUT => {
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
                .button(format!("{} Compress", icons::ARROWS_IN))
                .clicked()
            {
                self.json_action(Action::Minify);
            }
            if ui
                .button(format!("{} Format", icons::BRACKETS_CURLY))
                .on_hover_text("Ctrl/Cmd + Enter")
                .clicked()
            {
                self.json_action(Action::Format);
            }
        });
        if !self.docs[0].message.is_empty() {
            ui.colored_label(
                if self.docs[0].error {
                    Color32::from_rgb(205, 55, 55)
                } else {
                    ui.visuals().text_color()
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
            crate::code_editor::editor_with_folds(
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
                    ui.spacing_mut().interact_size.y = 16.;
                    for row in rows {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            let (number_rect, _) =
                                ui.allocate_exact_size(egui::vec2(32., 16.), egui::Sense::hover());
                            ui.painter().text(
                                egui::pos2(number_rect.right(), number_rect.top() + 1.),
                                egui::Align2::RIGHT_TOP,
                                (row + 1).to_string(),
                                FontId::monospace(11.),
                                if self.dark {
                                    Color32::from_rgb(135, 180, 220)
                                } else {
                                    Color32::from_rgb(20, 105, 140)
                                },
                            );
                            if let Some(&(_, end)) = ranges.iter().find(|r| r.0 == row) {
                                let folded = self.json_collapsed.contains(&row);
                                if ui
                                    .add_sized(
                                        [16., 16.],
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
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("document.json")
                    .save_file()
                {
                    if let Err(e) = std::fs::write(path, &self.docs[0].input) {
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
