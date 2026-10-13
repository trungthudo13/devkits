use super::*;
impl DevKits {
    pub(in crate::ui) fn password_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Create passwords using your operating system's secure random source.");
        ui.add_space(12.);
        ui.add(egui::Slider::new(&mut self.password_options.length, 8..=128).text("Length"));
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.password_options.lower, "Lowercase a–z");
            ui.checkbox(&mut self.password_options.upper, "Uppercase A–Z");
            ui.checkbox(&mut self.password_options.digits, "Digits 0–9");
            ui.checkbox(&mut self.password_options.symbols, "Symbols");
        });
        ui.checkbox(
            &mut self.password_options.exclude_ambiguous,
            "Exclude ambiguous characters (I, l, 1, O, 0, o)",
        );
        ui.add_space(12.);
        ui.horizontal(|ui| {
            ui.label("Quantity");
            ui.add(egui::DragValue::new(&mut self.password_count).range(1..=1000));
        });
        if ui
            .add(design::button(
                "Generate passwords",
                design::colors(ui).primary,
            ))
            .clicked()
        {
            match business::passwords(&self.password_options, self.password_count) {
                Ok(p) => {
                    self.notice_error = false;
                    self.secrets = p;
                    self.notice = format!(
                        "{} password(s) generated. Nothing is saved to disk.",
                        self.secrets.len()
                    );
                }
                Err(e) => {
                    self.secrets.clear();
                    self.notice_error = true;
                    self.notice = e;
                }
            }
        }
        ui.add_space(12.);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.reveal, "Show passwords");
            if ui
                .add_enabled(!self.secrets.is_empty(), egui::Button::new("Copy all"))
                .clicked()
            {
                ui.ctx().copy_text(self.secrets.join("\n"));
                self.notice_error = false;
                self.notice = "Copied all passwords, one per line.".into();
            }
            if ui.button("Clear").clicked() {
                self.secrets.clear();
                self.notice.clear();
            }
        });
        design::notice(
            ui,
            if self.notice_error {
                design::colors(ui).danger
            } else {
                design::colors(ui).success
            },
            &self.notice,
        );
        ui.add_space(8.);
        egui::ScrollArea::vertical()
            .id_salt("password_results")
            .show(ui, |ui| {
                let field_width = (ui.available_width() - 120.).max(100.);
                for (index, secret) in self.secrets.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_sized([40., 24.], egui::Label::new(format!("{}.", index + 1)));
                        let mut value = secret.as_str();
                        ui.add_sized(
                            [field_width, 24.],
                            egui::TextEdit::singleline(&mut value)
                                .password(!self.reveal)
                                .font(egui::TextStyle::Monospace),
                        );
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(secret.clone());
                            self.notice_error = false;
                            self.notice = format!("Copied password {}.", index + 1);
                        }
                    });
                }
            });
    }
}
