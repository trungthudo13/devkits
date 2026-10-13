//! Native application icon generated from the canonical SVG.
use eframe::egui;
const ICON: &[u8] = include_bytes!("../../../assets/icons/devkits.png");
fn image() -> image::RgbaImage {
    image::load_from_memory_with_format(ICON, image::ImageFormat::Png)
        .expect("checked-in DevKits icon must be a valid PNG")
        .to_rgba8()
}
pub fn native_icon() -> egui::IconData {
    let image = image();
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}
