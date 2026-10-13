use super::*;
use egui::{text::LayoutJob, FontId, TextFormat};
use sqlparser::{
    dialect::GenericDialect,
    keywords::Keyword,
    tokenizer::{Token, Tokenizer},
};

pub const EXAMPLE: &str = "SELECT\n  column1,\n  column2,\n  AVG(column3) AS avg_column,\n  COUNT(*) AS total_count\nFROM\n  table1\n  JOIN table2 ON table1.id = table2.id\nWHERE\n  column4 = 'value'\n  AND column5 > 100\nGROUP BY\n  column1\nHAVING\n  total_count > 10\nORDER BY\n  column2 DESC\nLIMIT\n  10;";

fn layout(text: &str, dark: bool) -> LayoutJob {
    let colors = design::palette(dark);
    let base = colors.text;
    let mut job = LayoutJob::default();
    let format = |color| TextFormat {
        font_id: FontId::monospace(crate::ui::components::code_editor::FONT_SIZE),
        color,
        line_height: Some(crate::ui::components::code_editor::LINE_HEIGHT),
        ..Default::default()
    };
    if text.len() > 128 * 1024 {
        job.append(text, 0., format(base));
        return job;
    }
    let tokens = Tokenizer::new(&GenericDialect {}, text)
        .with_unescape(false)
        .tokenize_with_location();
    if tokens.is_err() {
        job.append(text, 0., format(base));
        return job;
    }
    let mut offsets = vec![0];
    for (i, c) in text.char_indices() {
        if c == '\n' {
            offsets.push(i + 1);
        }
    }
    let byte = |line: u64, col: u64| {
        let offset = offsets
            .get(line.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(text.len());
        offset
            + text[offset..]
                .char_indices()
                .nth(col.saturating_sub(1) as usize)
                .map(|(i, _)| i)
                .unwrap_or(text.len() - offset)
    };
    let mut previous = 0;
    for token in tokens.unwrap() {
        let start = byte(token.span.start.line, token.span.start.column);
        let end = byte(token.span.end.line, token.span.end.column);
        if start < previous || end < start {
            continue;
        }
        job.append(&text[previous..start], 0., format(base));
        let color = match token.token {
            Token::Word(w) if w.keyword != Keyword::NoKeyword => colors.primary.text,
            Token::Number(..) => colors.warning.text,
            Token::SingleQuotedString(_)
            | Token::DollarQuotedString(_)
            | Token::EscapedStringLiteral(_) => colors.success.text,
            Token::Whitespace(_) => colors.muted,
            _ => base,
        };
        job.append(&text[start..end], 0., format(color));
        previous = end;
    }
    job.append(&text[previous..], 0., format(base));
    job
}

impl DevKits {
    pub fn sql_action(&mut self, action: Action) {
        let doc = &mut self.docs[2];
        match business::process_with_sql_options(
            &doc.input,
            Language::Sql,
            action,
            self.indent,
            self.dialect,
            business::SqlFormatOptions {
                keyword_uppercase: self.sql_uppercase,
                alias_align: self.sql_alias_align,
            },
        ) {
            Ok(value) => {
                doc.error = false;
                if action == Action::Lint {
                    doc.message = value;
                } else {
                    doc.input = value;
                    doc.output.clear();
                    doc.message.clear();
                }
            }
            Err(error) => {
                doc.error = true;
                doc.message = error;
            }
        }
    }
    pub fn sql_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(6., 8.);
        ui.spacing_mut().button_padding = egui::vec2(9., 4.);
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.));
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.));

        let compact = ui.available_width() < 700.;
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
            |ui| {
                if ui
                    .add_sized(
                        [76., 22.],
                        design::button(
                            format!("{} Beautify", icons::BRACKETS_CURLY),
                            design::colors(ui).primary,
                        ),
                    )
                    .on_hover_text("Ctrl/Cmd + Enter")
                    .clicked()
                {
                    self.sql_action(Action::Format);
                }
                if ui
                    .add_sized(
                        [96., 22.],
                        egui::Button::new(format!("{} Compression", icons::ARROWS_IN)),
                    )
                    .clicked()
                {
                    self.sql_action(Action::Minify);
                }
                if ui
                    .add_sized(
                        [80., 22.],
                        egui::Button::new(format!("{} Clipboard", icons::CLIPBOARD)),
                    )
                    .on_hover_text("Paste SQL from the system clipboard")
                    .clicked()
                {
                    match desktop::clipboard_text() {
                        Ok(text) if text.len() <= business::MAX_INPUT => {
                            self.docs[2] = Document {
                                input: text,
                                ..Default::default()
                            };
                        }
                        Ok(_) => {
                            self.docs[2].error = true;
                            self.docs[2].message = "Clipboard exceeds the 2 MiB limit.".into();
                        }
                        Err(e) => {
                            self.docs[2].error = true;
                            self.docs[2].message = format!("Cannot read clipboard: {e}");
                        }
                    }
                }
                if ui
                    .add_sized(
                        [68., 22.],
                        egui::Button::new(format!("{} Example", icons::FLASK)),
                    )
                    .clicked()
                {
                    self.docs[2].input = EXAMPLE.into();
                    self.sql_action(Action::Format);
                }
                if compact {
                    ui.end_row();
                }
                ui.add_sized(
                    [50., 28.],
                    egui::Checkbox::new(&mut self.sql_alias_align, "Alias\nalign"),
                );
                ui.add_sized(
                    [86., 28.],
                    egui::Checkbox::new(&mut self.sql_uppercase, "Keyword\nuppercase"),
                );
                egui::ComboBox::from_id_salt("sql-dialect")
                    .width(78.)
                    .selected_text(match self.dialect {
                        SqlDialect::MySql => "mysql",
                        SqlDialect::PostgreSql => "postgresql",
                        SqlDialect::Sqlite => "sqlite",
                        SqlDialect::Generic => "generic",
                    })
                    .show_ui(ui, |ui| {
                        for (d, label) in [
                            (SqlDialect::MySql, "mysql"),
                            (SqlDialect::PostgreSql, "postgresql"),
                            (SqlDialect::Sqlite, "sqlite"),
                            (SqlDialect::Generic, "generic"),
                        ] {
                            ui.selectable_value(&mut self.dialect, d, label);
                        }
                    });
            },
        );
    }

    pub fn sql_ui(&mut self, ui: &mut egui::Ui) {
        if !self.docs[2].message.is_empty() {
            design::notice(
                ui,
                if self.docs[2].error {
                    design::colors(ui).danger
                } else {
                    design::colors(ui).success
                },
                &self.docs[2].message,
            );
        }
        let response = crate::ui::components::code_editor::code_editor(
            ui,
            &mut self.docs[2].input,
            "sql",
            layout,
        );
        if response.changed() {
            self.docs[2].message.clear();
        }
        response.context_menu(|ui| {
            if ui.button("Lint / Validate").clicked() {
                self.sql_action(Action::Lint);
                ui.close_menu();
            }
            if ui.button("Copy SQL").clicked() {
                ui.ctx().copy_text(self.docs[2].input.clone());
                ui.close_menu();
            }
            if ui.button("Open file…").clicked() {
                self.open();
                ui.close_menu();
            }
            if ui.button("Save SQL…").clicked() {
                if let Some(path) = desktop::save_file("query.sql", "sql") {
                    if let Err(e) = business::documents::save_source(&path, &self.docs[2].input) {
                        self.docs[2].error = true;
                        self.docs[2].message = e.to_string();
                    }
                }
                ui.close_menu();
            }
        });
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter)) {
            self.sql_action(Action::Format);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toolbar_actions_replace_source_and_preserve_it_on_error() {
        let mut app = DevKits {
            tool: Tool::Sql,
            ..Default::default()
        };
        app.docs[2].input = "select id from users".into();
        app.sql_action(Action::Format);
        assert!(app.docs[2].input.starts_with("SELECT\n"));
        app.sql_action(Action::Minify);
        assert!(!app.docs[2].input.contains('\n'));
        let original = app.docs[2].input.clone();
        app.sql_action(Action::Lint);
        assert_eq!(app.docs[2].input, original);
        assert!(app.docs[2].message.contains("Valid SQL"));
        app.docs[2].input = "select from".into();
        app.sql_action(Action::Format);
        assert_eq!(app.docs[2].input, "select from");
        assert!(app.docs[2].error);
    }
    #[test]
    fn syntax_layout_retains_unicode_comments_and_invalid_input() {
        for text in [
            "SELECT 'Xin chào', 123; -- chú thích\nSELECT 2;",
            "SELECT 'unfinished",
        ] {
            let job = layout(text, false);
            assert_eq!(job.text, text);
        }
    }
}
