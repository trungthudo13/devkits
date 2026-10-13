use super::*;
use egui::{text::LayoutJob, FontId, Stroke, TextFormat};

pub const EXAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<store>
  <user id="123">
    <name>John Doe</name>
    <email>john.doe@example.com</email>
  </user>
  <products>
    <product id="p1" price="19.99">Product A</product>
    <product id="p2" price="29.99">Product B</product>
    <product id="p3" price="39.99">Product C</product>
  </products>
  <order id="abc123" date="2023-08-18">
    <item productId="p1" quantity="2" />
    <item productId="p3" quantity="1" />
  </order>
</store>"#;

// Highlight unfinished XML too: editing must not require a valid document.
fn layout(text: &str, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let base = if dark {
        Color32::from_gray(220)
    } else {
        Color32::from_rgb(20, 20, 25)
    };
    let tag = if dark {
        Color32::from_rgb(135, 165, 255)
    } else {
        Color32::from_rgb(0, 35, 230)
    };
    let value = if dark {
        Color32::from_rgb(245, 145, 145)
    } else {
        Color32::from_rgb(190, 35, 35)
    };
    let comment = if dark {
        Color32::from_rgb(145, 180, 150)
    } else {
        Color32::from_rgb(90, 130, 100)
    };
    let mut append = |s: &str, color| {
        job.append(
            s,
            0.,
            TextFormat {
                font_id: FontId::monospace(crate::code_editor::FONT_SIZE),
                line_height: Some(crate::code_editor::LINE_HEIGHT),
                color,
                ..Default::default()
            },
        )
    };
    if text.len() > 128 * 1024 {
        append(text, base);
        return job;
    }
    let mut i = 0;
    let mut inside_tag = false;
    while i < text.len() {
        let rest = &text[i..];
        let (length, color) = if rest.starts_with("<!--") {
            (
                rest.find("-->").map(|n| n + 3).unwrap_or(rest.len()),
                comment,
            )
        } else if rest.starts_with("<![CDATA[") {
            (rest.find("]]>").map(|n| n + 3).unwrap_or(rest.len()), base)
        } else if inside_tag && (rest.starts_with('"') || rest.starts_with('\'')) {
            let quote = rest.as_bytes()[0] as char;
            (
                rest[1..].find(quote).map(|n| n + 2).unwrap_or(rest.len()),
                value,
            )
        } else if rest.starts_with('<') {
            inside_tag = true;
            (1, tag)
        } else if rest.starts_with('>') && inside_tag {
            inside_tag = false;
            (1, tag)
        } else {
            let len = rest
                .char_indices()
                .skip(1)
                .find(|(_, c)| matches!(c, '<' | '>' | '\'' | '"'))
                .map(|(n, _)| n)
                .unwrap_or(rest.len());
            (len, if inside_tag { tag } else { base })
        };
        append(&text[i..i + length], color);
        i += length;
    }
    job
}

impl DevKits {
    pub fn xml_action(&mut self, action: Action) {
        let doc = &mut self.docs[1];
        match self_devkits::process(&doc.input, Language::Xml, action, self.indent, self.dialect) {
            Ok(result) => {
                doc.error = false;
                if action == Action::Lint {
                    doc.message = result;
                } else {
                    doc.input = result;
                    doc.output.clear();
                    doc.message.clear();
                }
            }
            Err(e) => {
                doc.error = true;
                doc.message = e;
            }
        }
    }
    pub fn xml_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(8., 8.);
        ui.spacing_mut().button_padding = egui::vec2(9., 4.);
        for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
            ui.style_mut()
                .text_styles
                .insert(style, FontId::proportional(11.));
        }
        if !self.dark {
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(239, 238, 244);
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
            ui.visuals_mut().widgets.inactive.corner_radius = egui::CornerRadius::same(1);
        }
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("xml-indent")
                .width(85.)
                .selected_text(format!("{} spaces", self.indent))
                .show_ui(ui, |ui| {
                    for n in [2, 4, 8] {
                        ui.selectable_value(&mut self.indent, n, format!("{n} spaces"));
                    }
                });
            if ui.button(format!("{} Example", icons::FLASK)).clicked() {
                self.docs[1].input = EXAMPLE.into();
                self.xml_action(Action::Format);
            }
            if ui
                .button(format!("{} Clipboard", icons::CLIPBOARD))
                .on_hover_text("Paste XML from the system clipboard")
                .clicked()
            {
                match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                    Ok(text) if text.len() <= self_devkits::MAX_INPUT => {
                        self.docs[1] = Document {
                            input: text,
                            ..Default::default()
                        }
                    }
                    Ok(_) => {
                        self.docs[1].error = true;
                        self.docs[1].message = "Clipboard exceeds the 2 MiB limit.".into();
                    }
                    Err(e) => {
                        self.docs[1].error = true;
                        self.docs[1].message = format!("Cannot read clipboard: {e}");
                    }
                }
            }
            if ui.button("Validate").clicked() {
                self.xml_action(Action::Lint);
            }
            if ui
                .button(format!("{} Compress", icons::ARROWS_IN))
                .clicked()
            {
                self.xml_action(Action::Minify);
            }
            if ui
                .button(format!("{} Format", icons::CODE))
                .on_hover_text("Ctrl/Cmd + Enter")
                .clicked()
            {
                self.xml_action(Action::Format);
            }
        });
        if !self.docs[1].message.is_empty() {
            ui.colored_label(
                if self.docs[1].error {
                    Color32::from_rgb(205, 55, 55)
                } else {
                    ui.visuals().text_color()
                },
                &self.docs[1].message,
            );
        }
        let response = crate::code_editor::code_editor(ui, &mut self.docs[1].input, "xml", layout);
        if response.changed() {
            self.docs[1].message.clear();
        }
        response.context_menu(|ui| {
            if ui.button("Validate XML").clicked() {
                self.xml_action(Action::Lint);
                ui.close_menu();
            }
            if ui.button("Copy XML").clicked() {
                ui.ctx().copy_text(self.docs[1].input.clone());
                ui.close_menu();
            }
            if ui.button("Open file…").clicked() {
                self.open();
                ui.close_menu();
            }
            if ui.button("Save XML…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("document.xml")
                    .save_file()
                {
                    if let Err(e) = std::fs::write(path, &self.docs[1].input) {
                        self.docs[1].error = true;
                        self.docs[1].message = e.to_string();
                    }
                }
                ui.close_menu();
            }
            if ui.button("Clear").clicked() {
                self.docs[1] = Document::default();
                ui.close_menu();
            }
        });
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter)) {
            self.xml_action(Action::Format);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_editor_actions_preserve_data_and_report_errors() {
        let mut app = DevKits {
            tool: Tool::Xml,
            ..Default::default()
        };
        app.docs[1].input = "<root><item id=\"1\">Xin chào</item></root>".into();
        app.xml_action(Action::Format);
        assert!(app.docs[1].input.contains("\n  <item"));
        app.xml_action(Action::Minify);
        assert_eq!(
            app.docs[1].input,
            "<root><item id=\"1\">Xin chào</item></root>"
        );
        let before = app.docs[1].input.clone();
        app.xml_action(Action::Lint);
        assert_eq!(app.docs[1].input, before);
        assert!(app.docs[1].message.contains("Valid XML"));
        for text in ["<root><broken></root>", "<p>Hello <b>world</b> !</p>"] {
            app.docs[1].input = text.into();
            app.xml_action(Action::Format);
            assert_eq!(app.docs[1].input, text);
        }
    }
    #[test]
    fn highlighting_keeps_exact_source_even_while_typing() {
        for text in [
            EXAMPLE,
            "<x title=\"a > b\">Tiếng Việt</x>",
            "<!-- <x> -->\n<x><![CDATA[<raw>]]></x>",
            "<unfinished attr='text",
            "",
            "text ' quote >",
        ] {
            let job = layout(text, false);
            assert_eq!(job.text, text);
        }
        assert!(layout(EXAMPLE, false)
            .sections
            .iter()
            .any(|s| s.format.color == Color32::from_rgb(190, 35, 35)));
    }
}
