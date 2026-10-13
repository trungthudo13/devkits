use super::*;
use egui::{Align2, CursorIcon, FontId, Rect, ResizeDirection, Sense, Stroke, Vec2};

impl DevKits {
    pub fn window_title_bar(&mut self, ctx: &egui::Context) {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        egui::TopBottomPanel::top("window-title")
            .exact_height(44.)
            .frame(egui::Frame::new().fill(if self.dark {
                Color32::from_rgb(29, 30, 35)
            } else {
                Color32::from_rgb(247, 247, 250)
            }))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let ink = if focused {
                    ui.visuals().text_color()
                } else if self.dark {
                    Color32::from_gray(160)
                } else {
                    Color32::from_gray(100)
                };
                let button = |ui: &mut egui::Ui, x: f32, icon: &str, label: &str, close: bool| {
                    let r =
                        Rect::from_min_size(egui::pos2(x, rect.top() + 7.), egui::vec2(32., 30.));
                    let response = ui.interact(r, ui.id().with(label), Sense::click());
                    let hovered = response.hovered();
                    if hovered {
                        ui.painter().rect_filled(
                            r,
                            6.,
                            if close {
                                Color32::from_rgb(205, 57, 70)
                            } else {
                                ui.visuals().widgets.hovered.weak_bg_fill
                            },
                        );
                    }
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        icon,
                        FontId::proportional(16.),
                        if close && hovered {
                            Color32::WHITE
                        } else {
                            ink
                        },
                    );
                    response.on_hover_text(label).clicked()
                };
                if button(
                    ui,
                    rect.left() + 10.,
                    icons::SIDEBAR,
                    "Toggle sidebar",
                    false,
                ) {
                    self.sidebar_open = !self.sidebar_open;
                }
                let right = rect.right() - 10.;
                if button(ui, right - 32., icons::X, "Close", true) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if button(
                    ui,
                    right - 68.,
                    if maximized {
                        icons::COPY_SIMPLE
                    } else {
                        icons::SQUARE
                    },
                    if maximized { "Restore" } else { "Maximize" },
                    false,
                ) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if button(ui, right - 104., icons::MINUS, "Minimize", false) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                ui.painter().line_segment(
                    [
                        egui::pos2(right - 117., rect.top() + 14.),
                        egui::pos2(right - 117., rect.bottom() - 14.),
                    ],
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                if button(
                    ui,
                    right - 161.,
                    if self.dark { icons::MOON } else { icons::SUN },
                    "Toggle light / dark theme",
                    false,
                ) {
                    self.dark = !self.dark;
                    theme(ctx, self.dark);
                }
                if button(
                    ui,
                    right - 197.,
                    if self.pinned {
                        icons::PUSH_PIN_SLASH
                    } else {
                        icons::PUSH_PIN
                    },
                    if self.pinned {
                        "Unpin window"
                    } else {
                        "Always on top"
                    },
                    false,
                ) {
                    self.pinned = !self.pinned;
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.pinned {
                        egui::WindowLevel::AlwaysOnTop
                    } else {
                        egui::WindowLevel::Normal
                    }));
                }
                // Only the space between controls starts a native move operation.
                let drag_rect = Rect::from_min_max(
                    egui::pos2(rect.left() + 50., rect.top() + 5.),
                    egui::pos2(right - 207., rect.bottom()),
                );
                let drag = ui.interact(drag_rect, ui.id().with("drag"), Sense::click_and_drag());
                ui.painter().text(
                    egui::pos2(drag_rect.left() + 6., rect.center().y),
                    Align2::LEFT_CENTER,
                    "DevKits",
                    FontId::proportional(14.),
                    ink,
                );
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.center().y),
                    Align2::CENTER_CENTER,
                    self.tool.title(),
                    FontId::proportional(13.),
                    ink,
                );
                if drag.double_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                } else if drag.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom() - 0.5,
                    Stroke::new(
                        1_f32,
                        if self.dark {
                            Color32::from_rgb(47, 48, 56)
                        } else {
                            Color32::from_rgb(224, 225, 232)
                        },
                    ),
                );
            });
    }
}

/// Native resize operations keep compositor snapping and size constraints intact.
pub fn resize_edges(ctx: &egui::Context) {
    if ctx.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let r = ctx.screen_rect();
    let edge = 5.;
    let corner = 12.;
    use ResizeDirection::*;
    let regions = [
        (
            NorthWest,
            r.min,
            Vec2::splat(corner),
            CursorIcon::ResizeNwSe,
        ),
        (
            NorthEast,
            egui::pos2(r.right() - corner, r.top()),
            Vec2::splat(corner),
            CursorIcon::ResizeNeSw,
        ),
        (
            SouthWest,
            egui::pos2(r.left(), r.bottom() - corner),
            Vec2::splat(corner),
            CursorIcon::ResizeNeSw,
        ),
        (
            SouthEast,
            r.max - Vec2::splat(corner),
            Vec2::splat(corner),
            CursorIcon::ResizeNwSe,
        ),
        (
            North,
            r.min + egui::vec2(corner, 0.),
            egui::vec2(r.width() - 2. * corner, edge),
            CursorIcon::ResizeVertical,
        ),
        (
            South,
            egui::pos2(r.left() + corner, r.bottom() - edge),
            egui::vec2(r.width() - 2. * corner, edge),
            CursorIcon::ResizeVertical,
        ),
        (
            West,
            r.min + egui::vec2(0., corner),
            egui::vec2(edge, r.height() - 2. * corner),
            CursorIcon::ResizeHorizontal,
        ),
        (
            East,
            egui::pos2(r.right() - edge, r.top() + corner),
            egui::vec2(edge, r.height() - 2. * corner),
            CursorIcon::ResizeHorizontal,
        ),
    ];
    for (direction, pos, size, cursor) in regions {
        egui::Area::new(egui::Id::new(("window-resize", direction as u8)))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .movable(false)
            .constrain(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let (_, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
                let response = response.on_hover_cursor(cursor);
                if response.hovered() && ui.input(|i| i.pointer.primary_pressed()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &egui::Context,
        app: &mut DevKits,
        events: Vec<egui::Event>,
        maximized: bool,
    ) -> Vec<egui::ViewportCommand> {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000., 700.),
            )),
            events,
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .maximized = Some(maximized);
        let output = ctx.run(input, |ctx| {
            app.window_title_bar(ctx);
            egui::CentralPanel::default().show(ctx, |_| {});
            resize_edges(ctx);
        });
        output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .clone()
    }

    fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[test]
    fn window_buttons_dispatch_native_commands_without_starting_drag() {
        for (x, maximized, expected) in [
            (974., false, egui::ViewportCommand::Close),
            (938., false, egui::ViewportCommand::Maximized(true)),
            (938., true, egui::ViewportCommand::Maximized(false)),
            (902., false, egui::ViewportCommand::Minimized(true)),
        ] {
            let ctx = egui::Context::default();
            let mut app = DevKits::default();
            frame(&ctx, &mut app, vec![], maximized);
            frame(&ctx, &mut app, vec![], maximized);
            let pos = egui::pos2(x, 22.);
            let down = frame(&ctx, &mut app, pointer(pos, true), maximized);
            let up = frame(&ctx, &mut app, pointer(pos, false), maximized);
            assert!(up.contains(&expected), "{up:?}");
            assert!(!down.contains(&egui::ViewportCommand::StartDrag));
            assert!(!up.contains(&egui::ViewportCommand::StartDrag));
        }
    }

    #[test]
    fn resize_corner_only_dispatches_when_not_maximized() {
        for maximized in [false, true] {
            let ctx = egui::Context::default();
            let mut app = DevKits::default();
            frame(&ctx, &mut app, vec![], maximized);
            frame(&ctx, &mut app, vec![], maximized);
            let commands = frame(
                &ctx,
                &mut app,
                pointer(egui::pos2(998., 698.), true),
                maximized,
            );
            assert_eq!(
                commands.contains(&egui::ViewportCommand::BeginResize(
                    ResizeDirection::SouthEast
                )),
                !maximized,
                "{commands:?}"
            );
        }
    }
}
