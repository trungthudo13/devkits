//! Shared app shell: native controls, navigation, body header, tool content, footer.
use super::*;
mod footer;
mod header;
mod navbar;
pub(super) mod window_chrome;

impl DevKits {
    pub(super) fn show_shell(&mut self, ctx: &egui::Context) {
        window_chrome::window_frame(ctx, self.dark, |ui| {
            self.window_title_bar(ui);
            self.navbar(ui);
            self.body_footer(ui);
            self.body_header(ui);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(design::colors(ui).surface)
                        .inner_margin(16.),
                )
                .show_inside(ui, |ui| {
                    ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
                    match self.tool {
                        Tool::Json => self.json_ui(ui),
                        Tool::Xml => self.xml_ui(ui),
                        Tool::Sql => self.sql_ui(ui),
                        Tool::Regex => self.regex.ui(ui),
                        Tool::Password => self.password_ui(ui),
                        Tool::Qr => self.qr_ui(ui),
                        Tool::Pdf => self.pdf.ui(ui),
                    }
                });
        });
        window_chrome::resize_edges(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_keeps_header_above_the_shared_footer() {
        let mut expected = None;
        for sidebar_open in [true, false] {
            for tool in Tool::ALL {
                let ctx = egui::Context::default();
                theme(&ctx, false);
                let mut app = DevKits {
                    tool,
                    sidebar_open,
                    ..Default::default()
                };
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(850., 580.),
                    )),
                    ..Default::default()
                };
                let output = ctx.run(input, |ctx| app.show_shell(ctx));
                let position = |label: &str| {
                    output
                        .shapes
                        .iter()
                        .find_map(|shape| {
                            if let egui::Shape::Text(text) = &shape.shape {
                                if text.galley.job.text == label {
                                    return Some(text.pos.y);
                                }
                            }
                            None
                        })
                        .unwrap_or_else(|| panic!("Missing {label} in {}", tool.title()))
                };
                let header_label = if tool == Tool::Json {
                    format!("{} Format", icons::BRACKETS_CURLY)
                } else if tool == Tool::Sql {
                    format!("{} Beautify", icons::BRACKETS_CURLY)
                } else if tool == Tool::Regex {
                    "Match".to_owned()
                } else {
                    tool.title().to_owned()
                };
                let positions = (
                    position(&header_label),
                    position("Local processing · No uploads"),
                );
                assert!(positions.0 < positions.1 && positions.1 < 560.);
                if let Some(expected) = expected {
                    assert_eq!(positions.1, expected);
                } else {
                    expected = Some(positions.1);
                }
            }
        }
    }
}
