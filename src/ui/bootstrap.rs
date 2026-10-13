use super::*;
pub fn run() -> eframe::Result {
    eframe::run_native(
        "DevKits",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_transparent(true)
                .with_app_id("devkits")
                .with_icon(components::brand::native_icon())
                .with_inner_size(
                    if cfg!(debug_assertions) && std::env::var_os("DEVKITS_CAPTURE_SMALL").is_some()
                    {
                        [850., 580.]
                    } else {
                        [1000., 700.]
                    },
                )
                .with_min_inner_size([850., 580.]),
            ..Default::default()
        },
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Noto Sans Mono".into(),
                egui::FontData::from_static(include_bytes!(
                    "../../assets/fonts/noto-sans-mono/NotoSansMono-Regular.ttf"
                ))
                .into(),
            );
            // Keep egui's fallback fonts for glyphs outside Noto Sans Mono's coverage.
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "Noto Sans Mono".into());
            // Keep UI typography, but cover Vietnamese characters missing from Ubuntu Light.
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("Noto Sans Mono".into());
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            theme(&cc.egui_ctx, false);
            Ok(Box::new(DevKits::default()))
        }),
    )
}
