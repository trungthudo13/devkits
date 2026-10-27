//! Shared colors from egui's default visuals; window decorations belong to the OS.
use eframe::egui::{self, Color32};

#[derive(Clone, Copy)]
pub struct Role {
    pub base: Color32,
    pub on_base: Color32,
    pub text: Color32,
    pub bg: Color32,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub primary: Role,
    pub success: Role,
    pub warning: Role,
    pub danger: Role,
    pub info: Role,
    pub surface: Color32,
    pub surface_raised: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub border: Color32,
}

pub fn palette(dark: bool) -> Palette {
    colors_from_visuals(&if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    })
}

fn colors_from_visuals(v: &egui::Visuals) -> Palette {
    let normal = Role {
        base: v.widgets.inactive.bg_fill,
        on_base: v.text_color(),
        text: v.text_color(),
        bg: v.faint_bg_color,
    };
    let primary = Role {
        base: v.selection.bg_fill,
        on_base: v.selection.stroke.color,
        text: v.hyperlink_color,
        bg: v.selection.bg_fill,
    };
    Palette {
        primary,
        success: normal,
        warning: Role {
            text: v.warn_fg_color,
            ..normal
        },
        danger: Role {
            text: v.error_fg_color,
            ..normal
        },
        info: primary,
        surface: v.panel_fill,
        surface_raised: v.window_fill,
        text: v.text_color(),
        muted: v.weak_text_color(),
        border: v.widgets.noninteractive.bg_stroke.color,
    }
}

pub fn colors(ui: &egui::Ui) -> Palette {
    colors_from_visuals(ui.visuals())
}

pub fn notice(ui: &mut egui::Ui, role: Role, text: &str) {
    if !text.is_empty() {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.colored_label(role.text, text);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_uses_default_visuals_in_both_modes() {
        for dark in [false, true] {
            let v = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let p = palette(dark);
            assert_eq!(p.surface, v.panel_fill);
            assert_eq!(p.text, v.text_color());
            assert_eq!(p.primary.bg, v.selection.bg_fill);
            assert_eq!(p.danger.text, v.error_fg_color);
        }
    }
}
