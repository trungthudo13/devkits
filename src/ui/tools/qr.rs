use super::*;
impl DevKits {
    pub(in crate::ui) fn qr_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Text or URL");
        let changed = ui
            .add(
                egui::TextEdit::multiline(&mut self.qr_input)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("https://example.com"),
            )
            .changed();
        let mut level_changed = false;
        ui.horizontal(|ui| {
            ui.label("Error correction");
            egui::ComboBox::from_id_salt("qr_level")
                .selected_text(
                    ["Low · 7%", "Medium · 15%", "Quartile · 25%", "High · 30%"][self.qr_level],
                )
                .show_ui(ui, |ui| {
                    for (i, label) in ["Low · 7%", "Medium · 15%", "Quartile · 25%", "High · 30%"]
                        .into_iter()
                        .enumerate()
                    {
                        level_changed |=
                            ui.selectable_value(&mut self.qr_level, i, label).changed();
                    }
                });
        });
        if changed || level_changed {
            self.qr_texture = None;
            self.qr_image = None;
            self.notice.clear();
        }
        ui.horizontal(|ui| {
            if ui
                .add(design::button("Generate QR", design::colors(ui).primary))
                .clicked()
            {
                match business::qr_png(
                    &self.qr_input,
                    [
                        qrcode::EcLevel::L,
                        qrcode::EcLevel::M,
                        qrcode::EcLevel::Q,
                        qrcode::EcLevel::H,
                    ][self.qr_level],
                ) {
                    Ok(img) => {
                        let rgba = image::DynamicImage::ImageLuma8(img.clone()).into_rgba8();
                        let color = egui::ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_raw(),
                        );
                        self.qr_texture = Some(ui.ctx().load_texture(
                            "qr",
                            color,
                            egui::TextureOptions::NEAREST,
                        ));
                        self.qr_image = Some(img);
                        self.notice_error = false;
                        self.notice = "QR code ready.".into();
                    }
                    Err(e) => {
                        self.qr_texture = None;
                        self.qr_image = None;
                        self.notice_error = true;
                        self.notice = e;
                    }
                }
            }
            if ui
                .add_enabled(self.qr_image.is_some(), egui::Button::new("Save PNG…"))
                .clicked()
            {
                if let Some(path) = desktop::save_file("qrcode.png", "png") {
                    self.notice =
                        match business::qr::save_png(self.qr_image.as_ref().unwrap(), &path) {
                            Ok(()) => {
                                self.notice_error = false;
                                "QR image saved.".into()
                            }
                            Err(e) => {
                                self.notice_error = true;
                                e.to_string()
                            }
                        };
                }
            }
        });
        if let Some(texture) = &self.qr_texture {
            let size = ui.available_height().min(ui.available_width()).min(320.) - 45.;
            ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(size.max(80.), size.max(80.))),
            );
        }
        design::notice(
            ui,
            if self.notice_error {
                design::colors(ui).danger
            } else {
                design::colors(ui).success
            },
            &self.notice,
        );
    }
}
