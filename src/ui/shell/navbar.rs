use super::*;
impl DevKits {
    pub(super) fn navbar(&mut self, ui: &mut egui::Ui) {
        if !self.sidebar_open {
            return;
        }
        let radius = window_chrome::corner_radius(ui.ctx());
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(204.)
            .frame(
                egui::Frame::new()
                    .fill(design::colors(ui).surface_raised)
                    .corner_radius(egui::CornerRadius {
                        sw: radius,
                        ..Default::default()
                    })
                    .inner_margin(12.),
            )
            .show_inside(ui, |ui| {
                ui.add_space(8.);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(format!("{} Search tools…", icons::MAGNIFYING_GLASS))
                        .margin(egui::Margin::symmetric(8, 8))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(16.);
                ui.spacing_mut().item_spacing.y = 4.;
                let query = self.search.to_lowercase();
                let mut count = 0;
                for tool in Tool::ALL {
                    if !tool.title().to_lowercase().contains(&query) {
                        continue;
                    }
                    count += 1;
                    let response = ui
                        .allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 30.),
                            egui::Layout::left_to_right(egui::Align::Center)
                                .with_main_align(egui::Align::Min)
                                .with_main_justify(true),
                            |ui| {
                                ui.selectable_label(
                                    self.tool == tool,
                                    format!("{}  {}", tool.icon(), tool.title()),
                                )
                            },
                        )
                        .inner;
                    if response.clicked() {
                        self.tool = tool;
                        self.notice.clear();
                    }
                }
                if count == 0 {
                    ui.colored_label(design::colors(ui).muted, "No matching tools.");
                }
            });
    }
}
