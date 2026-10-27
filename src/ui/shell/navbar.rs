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
                    .inner_margin(egui::Margin::symmetric(12, 0)),
            )
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                ui.spacing_mut().button_padding.y = 4.;
                ui.add_space(8.);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(format!("{} Search tools…", icons::MAGNIFYING_GLASS))
                        .margin(egui::Margin::symmetric(8, 8))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.);
                let query = self.search.to_lowercase();
                let mut count = 0;
                for tool in Tool::ALL {
                    if !tool.title().to_lowercase().contains(&query) {
                        continue;
                    }
                    count += 1;
                    ui.add_space(1.);
                    let response = ui
                        .allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 29.),
                            egui::Layout::left_to_right(egui::Align::Center)
                                .with_main_align(egui::Align::Min)
                                .with_main_justify(true),
                            |ui| {
                                let selected = self.tool == tool;
                                let selection_color = ui.visuals().selection.bg_fill;
                                let widgets = &mut ui.visuals_mut().widgets;
                                for visual in [
                                    &mut widgets.inactive,
                                    &mut widgets.hovered,
                                    &mut widgets.active,
                                ] {
                                    visual.corner_radius = egui::CornerRadius::ZERO;
                                    visual.expansion = 0.;
                                    visual.bg_stroke.width = 1.;
                                    if selected {
                                        visual.bg_stroke.color = selection_color;
                                    }
                                }
                                ui.selectable_label(
                                    selected,
                                    egui::RichText::new(format!(
                                        "{}  {}",
                                        tool.icon(),
                                        tool.title()
                                    ))
                                    .size(14.)
                                    .line_height(Some(21.)),
                                )
                            },
                        )
                        .inner;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_and_selection_have_thin_borders_without_expansion() {
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
                let label = format!("{}  {}", tool.icon(), tool.title());
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
                            if rect.rect.contains(center) && rect.rect.height() == 29. =>
                        {
                            Some(rect)
                        }
                        _ => None,
                    })
                    .expect("navigation background");
                assert_eq!(rect.stroke.width, 1.);
                assert_eq!(rect.corner_radius, egui::CornerRadius::ZERO);
                if tool == Tool::Sql {
                    assert_eq!(rect.stroke.color, ctx.style().visuals.selection.bg_fill);
                    assert_eq!(rect.fill, rect.stroke.color);
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
            let label = format!("{}  {}", tool.icon(), tool.title());
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                    _ => None,
                })
                .expect("navigation label");
            assert_eq!(text.galley.job.sections[0].format.font_id.size, 14.);
            assert_eq!(text.galley.job.sections[0].format.line_height, Some(21.));
            if let Some(y) = previous {
                assert_eq!(text.pos.y - y, 21. + 4. + 4. + 1. + 1.);
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
