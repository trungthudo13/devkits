use super::*;

impl DevKits {
    pub(super) fn window_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("app-toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(icons::SIDEBAR)
                    .on_hover_text("Toggle sidebar")
                    .clicked()
                {
                    self.sidebar_open = !self.sidebar_open;
                }
                if ui.checkbox(&mut self.pinned, "Always on top").changed() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.pinned {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        }));
                }
                ui.separator();
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });
    }
}
