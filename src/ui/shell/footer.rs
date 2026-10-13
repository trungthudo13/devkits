use super::*;
impl DevKits {
    pub(super) fn body_footer(&self, ui: &mut egui::Ui) {
        let radius = window_chrome::corner_radius(ui.ctx());
        egui::TopBottomPanel::bottom("body-footer")
            .exact_height(30.)
            .frame(
                egui::Frame::new()
                    .fill(design::colors(ui).surface_raised)
                    .corner_radius(egui::CornerRadius {
                        sw: if self.sidebar_open { 0 } else { radius },
                        se: radius,
                        ..Default::default()
                    })
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(design::colors(ui).muted, "Local processing · No uploads");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(
                            design::colors(ui).muted,
                            concat!("v", env!("CARGO_PKG_VERSION")),
                        );
                        if self.tool.lang().is_some() {
                            ui.colored_label(
                                design::colors(ui).muted,
                                "Ctrl/Cmd + Enter to format",
                            );
                        }
                    });
                });
            });
    }
}
