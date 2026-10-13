use super::*;
impl DevKits {
    pub(super) fn body_header(&mut self, ui: &mut egui::Ui) {
        let panel = egui::TopBottomPanel::top("body-header").show_separator_line(false);
        let panel = if matches!(self.tool, Tool::Json | Tool::Xml | Tool::Sql | Tool::Regex) {
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
                if self.tool == Tool::Json {
                    self.json_toolbar(ui);
                } else if self.tool == Tool::Xml {
                    self.xml_toolbar(ui);
                } else if self.tool == Tool::Sql {
                    self.sql_toolbar(ui);
                } else if self.tool == Tool::Regex {
                    self.regex.toolbar(ui);
                } else {
                    ui.heading(self.tool.title());
                    ui.colored_label(design::colors(ui).muted, self.tool.description());
                }
            });
    }
}
