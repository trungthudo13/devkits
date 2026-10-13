//! Bootstrap 5.3 semantic colors, adapted to native egui widgets.
//! Source: https://getbootstrap.com/docs/5.3/customize/css-variables/
use eframe::egui::{self, Color32, Stroke};

const fn hex(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

#[derive(Clone, Copy)]
pub struct Role {
    pub base: Color32,
    pub on_base: Color32,
    pub text: Color32,
    pub bg: Color32,
    pub border: Color32,
}
impl Role {
    const fn new(base: u32, on_base: u32, text: u32, bg: u32, border: u32) -> Self {
        Self {
            base: hex(base),
            on_base: hex(on_base),
            text: hex(text),
            bg: hex(bg),
            border: hex(border),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub primary: Role,
    pub secondary: Role,
    pub success: Role,
    pub warning: Role,
    pub danger: Role,
    pub info: Role,
    pub surface: Color32,
    pub surface_raised: Color32,
    pub surface_hover: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub border: Color32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            primary: Role::new(0x0d6efd, 0xffffff, 0x6ea8fe, 0x031633, 0x084298),
            secondary: Role::new(0x6c757d, 0xffffff, 0xa7acb1, 0x161719, 0x41464b),
            success: Role::new(0x198754, 0xffffff, 0x75b798, 0x051b11, 0x0f5132),
            warning: Role::new(0xffc107, 0x000000, 0xffda6a, 0x332701, 0x997404),
            danger: Role::new(0xdc3545, 0xffffff, 0xea868f, 0x2c0b0e, 0x842029),
            info: Role::new(0x0dcaf0, 0x000000, 0x6edff6, 0x032830, 0x087990),
            surface: hex(0x212529),
            surface_raised: hex(0x2b3035),
            surface_hover: hex(0x343a40),
            text: hex(0xdee2e6),
            muted: hex(0xadb5bd),
            border: hex(0x495057),
        }
    } else {
        Palette {
            primary: Role::new(0x0d6efd, 0xffffff, 0x052c65, 0xcfe2ff, 0x9ec5fe),
            secondary: Role::new(0x6c757d, 0xffffff, 0x2b2f32, 0xe2e3e5, 0xc4c8cb),
            success: Role::new(0x198754, 0xffffff, 0x0a3622, 0xd1e7dd, 0xa3cfbb),
            warning: Role::new(0xffc107, 0x000000, 0x664d03, 0xfff3cd, 0xffe69c),
            danger: Role::new(0xdc3545, 0xffffff, 0x58151c, 0xf8d7da, 0xf1aeb5),
            info: Role::new(0x0dcaf0, 0x000000, 0x055160, 0xcff4fc, 0x9eeaf9),
            surface: hex(0xffffff),
            surface_raised: hex(0xf8f9fa),
            surface_hover: hex(0xe9ecef),
            text: hex(0x212529),
            muted: hex(0x495057),
            border: hex(0xdee2e6),
        }
    }
}

pub fn colors(ui: &egui::Ui) -> Palette {
    palette(ui.visuals().dark_mode)
}

pub fn visuals(dark: bool) -> egui::Visuals {
    let p = palette(dark);
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.surface;
    v.window_fill = p.surface_raised;
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.surface_raised;
    v.code_bg_color = p.surface_raised;
    v.hyperlink_color = if dark { p.primary.text } else { p.primary.base };
    v.warn_fg_color = p.warning.text;
    v.error_fg_color = p.danger.text;
    v.selection.bg_fill = p.primary.bg;
    v.selection.stroke = Stroke::new(1_f32, p.primary.text);
    v.window_stroke = Stroke::new(1_f32, p.border);
    for widget in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        widget.bg_fill = p.surface_raised;
        widget.weak_bg_fill = p.surface_raised;
        widget.bg_stroke = Stroke::new(1_f32, p.border);
        widget.fg_stroke = Stroke::new(1_f32, p.text);
        widget.corner_radius = egui::CornerRadius::same(5);
    }
    v.widgets.hovered.bg_fill = p.surface_hover;
    v.widgets.hovered.weak_bg_fill = p.surface_hover;
    v.widgets.hovered.bg_stroke.color = p.primary.text;
    v.widgets.active.bg_fill = p.primary.bg;
    v.widgets.active.weak_bg_fill = p.primary.bg;
    v.widgets.active.bg_stroke.color = p.primary.text;
    v.widgets.active.fg_stroke.color = p.primary.text;
    v.widgets.open.bg_fill = p.primary.bg;
    v.widgets.open.weak_bg_fill = p.primary.bg;
    v.widgets.open.fg_stroke.color = p.primary.text;
    v
}

/// Role-colored action; egui still supplies keyboard focus and hover outlines.
pub fn button(label: impl Into<egui::WidgetText>, role: Role) -> egui::Button<'static> {
    egui::Button::new(label.into().color(role.on_base)).fill(role.base)
}

pub fn notice(ui: &mut egui::Ui, role: Role, text: &str) {
    if text.is_empty() {
        return;
    }
    egui::Frame::new()
        .fill(role.bg)
        .stroke(Stroke::new(1_f32, role.border))
        .corner_radius(5)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.colored_label(role.text, text);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contrast(a: Color32, b: Color32) -> f64 {
        let luminance = |c: Color32| {
            let linear = |n: u8| {
                let n = n as f64 / 255.;
                if n <= 0.04045 {
                    n / 12.92
                } else {
                    ((n + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn semantic_text_and_actions_have_readable_contrast_in_both_modes() {
        for dark in [false, true] {
            let p = palette(dark);
            for role in [
                p.primary,
                p.secondary,
                p.success,
                p.warning,
                p.danger,
                p.info,
            ] {
                assert!(contrast(role.text, role.bg) >= 4.5);
                assert!(contrast(role.text, p.surface) >= 4.5);
                assert!(contrast(role.on_base, role.base) >= 4.5);
            }
            for bg in [p.surface, p.surface_raised, p.surface_hover] {
                assert!(contrast(p.text, bg) >= 4.5);
                assert!(contrast(p.muted, bg) >= 4.5);
            }
        }
    }
}
