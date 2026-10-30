use super::*;
impl DevKits {
    pub(super) fn body_header(&mut self, ui: &mut egui::Ui) {
        let panel = egui::TopBottomPanel::top("body-header").show_separator_line(false);
        let panel = if matches!(
            self.tool,
            Tool::Json
                | Tool::Xml
                | Tool::Sql
                | Tool::Regex
                | Tool::Cryptographic
                | Tool::Keygen
                | Tool::Base64
        ) {
            panel.min_height(0.).default_height(44.)
        } else {
            panel.exact_height(66.)
        };
        panel
            .frame(
                egui::Frame::new()
                    .fill(design::colors(ui).surface)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show_inside(ui, |ui| {
                // Keep controls on a common row; wrap whole controls, never their labels.
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.spacing_mut().button_padding = egui::vec2(9., 4.);
                ui.spacing_mut().interact_size.y =
                    ui.text_style_height(&egui::TextStyle::Button).max(21.) + 8.;
                if self.tool == Tool::Json {
                    self.json_toolbar(ui);
                } else if self.tool == Tool::Xml {
                    self.xml_toolbar(ui);
                } else if self.tool == Tool::Sql {
                    self.sql_toolbar(ui);
                } else if self.tool == Tool::Regex {
                    self.regex.toolbar(ui);
                } else if self.tool == Tool::Cryptographic {
                    self.cryptographic.toolbar(ui);
                } else if self.tool == Tool::Keygen {
                    self.keygen.toolbar(ui);
                } else if self.tool == Tool::Base64 {
                    self.base64.toolbar(ui);
                } else {
                    ui.heading(self.tool.title());
                    ui.colored_label(design::colors(ui).muted, self.tool.description());
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbars_keep_whole_labels_on_aligned_rows() {
        for tool in [
            Tool::Json,
            Tool::Xml,
            Tool::Sql,
            Tool::Regex,
            Tool::Cryptographic,
            Tool::Keygen,
            Tool::Base64,
        ] {
            for width in [600., 780., 1100.] {
                let ctx = egui::Context::default();
                bootstrap::configure_typography(&ctx);
                let mut app = DevKits {
                    tool,
                    ..Default::default()
                };
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 400.),
                    )),
                    ..Default::default()
                };
                let output = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.body_header(ui));
                });
                let mut rows: Vec<f32> = Vec::new();
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape {
                        assert_eq!(
                            text.galley.rows.len(),
                            1,
                            "{}: {:?}",
                            tool.title(),
                            text.galley.job.text
                        );
                        let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
                        assert!(
                            shape.clip_rect.expand(1.).contains_rect(bounds),
                            "{} at {width}: clipped {:?}",
                            tool.title(),
                            text.galley.job.text
                        );
                        let center = bounds.center().y;
                        if !rows.iter().any(|y| (center - y).abs() <= 1.) {
                            rows.push(center);
                        }
                    }
                }
                assert!(!rows.is_empty());
                if width >= 780. {
                    assert_eq!(rows.len(), 1, "{} at {width}: {rows:?}", tool.title());
                } else {
                    assert!(rows.len() <= 2, "{} at {width}: {rows:?}", tool.title());
                }
            }
        }
    }
}
