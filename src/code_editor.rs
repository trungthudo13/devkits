use eframe::egui::{self, text::LayoutJob, Color32, FontId, Stroke};

pub fn code_editor(
    ui: &mut egui::Ui,
    text: &mut String,
    language: &str,
    layout: fn(&str, bool) -> LayoutJob,
) -> egui::Response {
    editor_with_folds(
        ui,
        text,
        language,
        layout,
        &[],
        &mut std::collections::BTreeSet::new(),
    )
}

pub fn editor_with_folds(
    ui: &mut egui::Ui,
    text: &mut String,
    language: &str,
    layout: fn(&str, bool) -> LayoutJob,
    ranges: &[(usize, usize)],
    collapsed: &mut std::collections::BTreeSet<usize>,
) -> egui::Response {
    let height = ui.available_height();
    let width = ui.available_width();
    egui::ScrollArea::both()
        .id_salt((language, "code-scroll"))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                let (gutter, _) =
                    ui.allocate_exact_size(egui::vec2(48., height), egui::Sense::hover());
                let mut layouter = |ui: &egui::Ui, text: &str, _: f32| {
                    let key = egui::util::hash((text, ui.visuals().dark_mode));
                    let id = egui::Id::new((language, "highlight-cache"));
                    let cached = ui.ctx().data_mut(|d| d.get_temp::<(u64, LayoutJob)>(id));
                    let job = match cached {
                        Some((old, job)) if old == key => job,
                        _ => {
                            let job = layout(text, ui.visuals().dark_mode);
                            ui.ctx().data_mut(|d| d.insert_temp(id, (key, job.clone())));
                            job
                        }
                    };
                    ui.fonts(|f| f.layout_job(job))
                };
                let output = egui::TextEdit::multiline(text)
                    .id_salt((language, "source"))
                    .font(FontId::monospace(11.))
                    .code_editor()
                    .frame(false)
                    .margin(egui::Margin::ZERO)
                    .desired_width((width - 48.).max(100.))
                    .min_size(egui::vec2(width - 48., height))
                    .hint_text(format!(
                        "Paste {} here, or choose Example…",
                        language.to_uppercase()
                    ))
                    .layouter(&mut layouter)
                    .show(ui);
                let number_color = if ui.visuals().dark_mode {
                    Color32::from_rgb(135, 180, 220)
                } else {
                    Color32::from_rgb(20, 105, 140)
                };
                for (index, row) in output.galley.rows.iter().enumerate() {
                    let y = output.galley_pos.y + row.rect.top();
                    if y + 16. < ui.clip_rect().top() || y > ui.clip_rect().bottom() {
                        continue;
                    }
                    ui.painter().text(
                        egui::pos2(gutter.right() - 16., y + 1.),
                        egui::Align2::RIGHT_TOP,
                        (index + 1).to_string(),
                        FontId::monospace(11.),
                        number_color,
                    );
                }
                for &(start, _) in ranges {
                    if let Some(row) = output.galley.rows.get(start) {
                        let y = output.galley_pos.y + row.rect.top();
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(gutter.right() - 15., y),
                            egui::vec2(14., 16.),
                        );
                        if ui.clip_rect().intersects(rect)
                            && ui
                                .put(
                                    rect,
                                    egui::Button::new(egui_phosphor::regular::CARET_DOWN)
                                        .frame(false),
                                )
                                .on_hover_text("Collapse block")
                                .clicked()
                        {
                            collapsed.insert(start);
                        }
                    }
                }
                let guide = if ui.visuals().dark_mode {
                    Color32::from_gray(60)
                } else {
                    Color32::from_gray(232)
                };
                for (index, line) in text.lines().enumerate() {
                    let Some(row) = output.galley.rows.get(index) else {
                        break;
                    };
                    let y = output.galley_pos.y + row.rect.top();
                    if !ui.clip_rect().y_range().contains(y) {
                        continue;
                    }
                    for column in (0..line.chars().take_while(|c| *c == ' ').count()).step_by(2) {
                        let x = output.galley_pos.x + column as f32 * 6.6;
                        ui.painter().line_segment(
                            [egui::pos2(x, y), egui::pos2(x, y + 16.)],
                            Stroke::new(1_f32, guide),
                        );
                    }
                }
                if output.response.has_focus() {
                    if let Some(cursor) = output.cursor_range {
                        if let Some(row) = output.galley.rows.get(cursor.primary.rcursor.row) {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    output.galley_pos.x,
                                    output.galley_pos.y + row.rect.top(),
                                ),
                                egui::vec2(width - 48., 16.),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                0.,
                                Stroke::new(1_f32, guide),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                }
                output.response
            })
            .inner
        })
        .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clicking_fold_gutter_keeps_source_intact() {
        let ctx = egui::Context::default();
        let mut text = "{\n  \"a\": 1\n}".to_string();
        let mut collapsed = std::collections::BTreeSet::new();
        for events in [
            vec![],
            vec![
                egui::Event::PointerMoved(egui::pos2(48., 16.)),
                egui::Event::PointerButton {
                    pos: egui::pos2(48., 16.),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            vec![egui::Event::PointerButton {
                pos: egui::pos2(48., 16.),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600., 400.),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        editor_with_folds(
                            ui,
                            &mut text,
                            "test",
                            |text, _| {
                                LayoutJob::simple(
                                    text.into(),
                                    FontId::monospace(11.),
                                    Color32::WHITE,
                                    f32::INFINITY,
                                )
                            },
                            &[(0, 2)],
                            &mut collapsed,
                        );
                    });
                },
            );
        }
        assert!(collapsed.contains(&0));
        assert_eq!(text, "{\n  \"a\": 1\n}");
    }
}
