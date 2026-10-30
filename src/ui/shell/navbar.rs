use super::*;
impl DevKits {
    pub(super) fn navbar(&mut self, ui: &mut egui::Ui) {
        if !self.sidebar_open {
            return;
        }
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(204.)
            .frame(
                egui::Frame::new()
                    .fill(design::colors(ui).surface_raised)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                ui.add_space(8.);
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(4, 0))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text(format!("{} Search tools…", icons::MAGNIFYING_GLASS))
                                .margin(egui::Margin::symmetric(8, 8))
                                .desired_width(f32::INFINITY),
                        );
                    });
                ui.add_space(8.);
                let query = self.search.to_lowercase();
                let mut count = 0;
                for tool in Tool::ALL {
                    if !tool.title().to_lowercase().contains(&query) {
                        continue;
                    }
                    count += 1;
                    ui.add_space(1.);
                    let response = nav_item(ui, tool, self.tool == tool);
                    ui.add_space(1.);
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

// One focusable hit target; icon and label have independent font metrics.
fn nav_item(ui: &mut egui::Ui, tool: Tool, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            tool.title(),
        )
    });
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        let color = visuals.text_color();
        if selected || response.hovered() || response.highlighted() || response.has_focus() {
            let border = if selected {
                egui::Stroke::new(1., ui.visuals().selection.bg_fill)
            } else if response.has_focus() {
                visuals.bg_stroke
            } else {
                egui::Stroke::NONE
            };
            ui.painter().rect(
                rect,
                egui::CornerRadius::ZERO,
                visuals.weak_bg_fill,
                border,
                egui::StrokeKind::Inside,
            );
        }
        let font = egui::FontId::proportional(14.);
        let label = ui
            .painter()
            .layout_no_wrap(tool.title().into(), font.clone(), color);
        // Use a shared ascender/descender sample so every label keeps the same
        // baseline, while centering visible letterforms rather than font leading.
        let reference = ui
            .painter()
            .layout_no_wrap("Hg".into(), font.clone(), color);
        let icon = ui.painter().layout_no_wrap(tool.icon().into(), font, color);
        let icon_center = egui::pos2(rect.left() + 8. + 10., rect.center().y);
        let icon_pos = icon_center - icon.mesh_bounds.center().to_vec2();
        let label_pos = egui::pos2(
            rect.left() + 8. + 20. + 8.,
            rect.center().y - reference.mesh_bounds.center().y,
        );
        ui.painter().galley(icon_pos, icon, color);
        ui.painter().galley(label_pos, label, color);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_is_borderless_and_selection_has_no_expansion() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let ctx = egui::Context::default();
            bootstrap::configure_typography(&ctx);
            ctx.set_theme(theme);
            let mut app = DevKits {
                tool: Tool::Sql,
                ..Default::default()
            };
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(850., 580.),
                )),
                ..Default::default()
            };
            let output = ctx.run(input.clone(), |ctx| app.show_shell(ctx));
            let label_center = |output: &egui::FullOutput, tool: Tool| {
                let label = tool.title();
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.job.text == label => {
                            Some(text.pos + text.galley.size() / 2.)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            let hover = label_center(&output, Tool::Json);
            input.events.push(egui::Event::PointerMoved(hover));
            let output = ctx.run(input, |ctx| app.show_shell(ctx));
            for tool in [Tool::Json, Tool::Sql] {
                let center = label_center(&output, tool);
                let rect = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.rect.contains(center) && rect.rect.height() == 32. =>
                        {
                            Some(rect)
                        }
                        _ => None,
                    })
                    .expect("navigation background");
                assert_eq!(rect.corner_radius, egui::CornerRadius::ZERO);
                if tool == Tool::Sql {
                    assert_eq!(rect.stroke.width, 1.);
                    assert_eq!(rect.stroke.color, ctx.style().visuals.selection.bg_fill);
                    assert_eq!(rect.fill, rect.stroke.color);
                } else {
                    assert_eq!(rect.stroke, egui::Stroke::NONE);
                }
            }
        }
    }

    #[test]
    fn navigation_rows_have_one_point_margins_and_left_aligned_text() {
        let ctx = egui::Context::default();
        bootstrap::configure_typography(&ctx);
        let mut app = DevKits::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(850., 580.),
            )),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| app.show_shell(ctx));
        let mut previous = None;
        let mut left = None;
        for tool in Tool::ALL {
            let label = tool.title();
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                    _ => None,
                })
                .expect("navigation label");
            assert_eq!(text.galley.job.sections[0].format.font_id.size, 14.);
            if let Some(y) = previous {
                assert_eq!(text.pos.y - y, 32. + 1. + 1.);
            }
            if let Some(x) = left {
                assert_eq!(text.pos.x, x);
            } else {
                left = Some(text.pos.x);
            }
            previous = Some(text.pos.y);
        }
    }
}
