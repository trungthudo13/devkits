use super::*;
use egui::{text::LayoutJob, FontId, Stroke, TextFormat};
use sqlparser::{
    dialect::GenericDialect,
    keywords::Keyword,
    tokenizer::{Token, Tokenizer},
};

pub const EXAMPLE: &str = "SELECT\n  column1,\n  column2,\n  AVG(column3) AS avg_column,\n  COUNT(*) AS total_count\nFROM\n  table1\n  JOIN table2 ON table1.id = table2.id\nWHERE\n  column4 = 'value'\n  AND column5 > 100\nGROUP BY\n  column1\nHAVING\n  total_count > 10\nORDER BY\n  column2 DESC\nLIMIT\n  10;";

fn layout(text: &str, dark: bool) -> LayoutJob {
    let base = if dark {
        Color32::from_gray(220)
    } else {
        Color32::from_rgb(20, 20, 25)
    };
    let mut job = LayoutJob::default();
    let format = |color| TextFormat {
        font_id: FontId::monospace(11.),
        color,
        line_height: Some(16.),
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
            Token::Word(w) if w.keyword != Keyword::NoKeyword => {
                if dark {
                    Color32::from_rgb(135, 165, 255)
                } else {
                    Color32::from_rgb(0, 35, 230)
                }
            }
            Token::Number(..) => {
                if dark {
                    Color32::from_rgb(120, 210, 160)
                } else {
                    Color32::from_rgb(0, 125, 76)
                }
            }
            Token::SingleQuotedString(_)
            | Token::DollarQuotedString(_)
            | Token::EscapedStringLiteral(_) => {
                if dark {
                    Color32::from_rgb(245, 145, 145)
                } else {
                    Color32::from_rgb(210, 35, 35)
                }
            }
            Token::Whitespace(_) => Color32::from_rgb(130, 135, 145),
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
        match self_devkits::process_with_sql_options(
            &doc.input,
            Language::Sql,
            action,
            self.indent,
            self.dialect,
            self_devkits::SqlFormatOptions {
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
    pub fn sql_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(8., 8.);
        ui.spacing_mut().button_padding = egui::vec2(9., 4.);
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(11.));
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(11.));
        if !self.dark {
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(239, 238, 244);
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
            ui.visuals_mut().widgets.inactive.corner_radius = egui::CornerRadius::same(1);
        }
        ui.horizontal_wrapped(|ui| {
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
            ui.add_sized(
                [102., 28.],
                egui::Checkbox::new(&mut self.sql_uppercase, "Keyword\nuppercase"),
            );
            ui.add_sized(
                [60., 28.],
                egui::Checkbox::new(&mut self.sql_alias_align, "Alias\nalign"),
            );
            if ui
                .add_sized(
                    [72., 22.],
                    egui::Button::new(format!("{} Example", icons::FLASK)),
                )
                .clicked()
            {
                self.docs[2].input = EXAMPLE.into();
                self.sql_action(Action::Format);
            }
            if ui
                .add_sized(
                    [82., 22.],
                    egui::Button::new(format!("{} Clipboard", icons::CLIPBOARD)),
                )
                .on_hover_text("Paste SQL from the system clipboard")
                .clicked()
            {
                match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                    Ok(text) if text.len() <= self_devkits::MAX_INPUT => {
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
                    [100., 22.],
                    egui::Button::new(format!("{} Compression", icons::ARROWS_IN)),
                )
                .clicked()
            {
                self.sql_action(Action::Minify);
            }
            if ui
                .add_sized(
                    [76., 22.],
                    egui::Button::new(format!("{} Beautify", icons::BRACKETS_CURLY)),
                )
                .on_hover_text("Ctrl/Cmd + Enter")
                .clicked()
            {
                self.sql_action(Action::Format);
            }
        });
        if !self.docs[2].message.is_empty() {
            ui.colored_label(
                if self.docs[2].error {
                    Color32::from_rgb(205, 55, 55)
                } else {
                    ui.visuals().text_color()
                },
                &self.docs[2].message,
            );
        }
        let response = crate::code_editor::code_editor(ui, &mut self.docs[2].input, "sql", layout);
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
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("query.sql")
                    .save_file()
                {
                    if let Err(e) = std::fs::write(path, &self.docs[2].input) {
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
