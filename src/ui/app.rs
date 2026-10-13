use super::*;
impl eframe::App for DevKits {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.; 4]
    }
    #[cfg(target_os = "linux")]
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        self.ime.normalize(&mut input.events);
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        self.capture_preview(ctx);
        self.show_shell(ctx);
    }
}
