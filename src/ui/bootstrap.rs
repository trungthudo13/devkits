use super::*;
pub fn run() -> eframe::Result {
    eframe::run_native(
        "DevKits",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
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
            cc.egui_ctx.set_fonts(font_definitions());
            cc.egui_ctx.set_theme(egui::ThemePreference::System);
            configure_typography(&cc.egui_ctx);
            Ok(Box::new(DevKits::default()))
        }),
    )
}

pub(super) fn configure_typography(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        for text_style in [
            egui::TextStyle::Body,
            egui::TextStyle::Button,
            egui::TextStyle::Monospace,
        ] {
            style.text_styles.get_mut(&text_style).unwrap().size = 14.;
        }
        style.spacing.interact_size.y = 21.;
    });
}

fn font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Noto Sans Mono".into(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/noto-sans-mono/NotoSansMono-Regular.ttf"
        ))
        .into(),
    );
    // Bundled fonts remain available for missing system fonts or glyphs.
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "Noto Sans Mono".into());
    // Preserve Vietnamese coverage in the proportional fallback chain.
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("Noto Sans Mono".into());
    for (family, monospace, name) in [
        (egui::FontFamily::Proportional, false, "System UI"),
        (egui::FontFamily::Monospace, true, "System Mono"),
    ] {
        if let Some(font) = self_devkits::adapter::fonts::load(monospace) {
            let mut data = egui::FontData::from_owned(font.bytes);
            data.index = font.index;
            fonts.font_data.insert(name.into(), data.into());
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.into());
        }
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_is_consistent_in_both_themes() {
        let ctx = egui::Context::default();
        configure_typography(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            let style = ctx.style();
            for text_style in [
                egui::TextStyle::Body,
                egui::TextStyle::Button,
                egui::TextStyle::Monospace,
            ] {
                assert_eq!(style.text_styles[&text_style].size, 14.);
            }
            assert_eq!(style.text_styles[&egui::TextStyle::Heading].size, 18.);
        }
    }

    #[test]
    fn system_fonts_render_ui_editor_unicode_and_icons() {
        let definitions = font_definitions();
        for (family, name) in [
            (egui::FontFamily::Proportional, "System UI"),
            (egui::FontFamily::Monospace, "System Mono"),
        ] {
            if definitions.font_data.contains_key(name) {
                assert_eq!(definitions.families[&family][0], name);
            }
        }
        let ctx = egui::Context::default();
        ctx.set_fonts(definitions);
        let _ = ctx.run(Default::default(), |ctx| {
            ctx.fonts(|fonts| {
                for font in [
                    egui::FontId::proportional(14.),
                    egui::FontId::monospace(14.),
                ] {
                    let galley = fonts.layout_no_wrap(
                        format!("Tiếng Việt — {{ \"name\": 123 }} {}", icons::SIDEBAR),
                        font,
                        Color32::WHITE,
                    );
                    assert!(galley.size().x > 0. && galley.size().y > 0.);
                }
            });
        });
    }
}
