use eframe::egui::{self, text::LayoutJob, FontId, Stroke};

pub const FONT_SIZE: f32 = 14.;
pub const LINE_HEIGHT: f32 = 20.;

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
                            let mut job = layout(text, ui.visuals().dark_mode);
                            // Empty highlighters have no sections: specify the same font
                            // and row height as populated code instead of egui's default.
                            if text.is_empty() {
                                job.append(
                                    "",
                                    0.,
                                    egui::TextFormat {
                                        font_id: FontId::monospace(
                                            crate::ui::components::code_editor::FONT_SIZE,
                                        ),
                                        line_height: Some(
                                            crate::ui::components::code_editor::LINE_HEIGHT,
                                        ),
                                        ..Default::default()
                                    },
                                );
                            }
                            ui.ctx().data_mut(|d| d.insert_temp(id, (key, job.clone())));
                            job
                        }
                    };
                    ui.fonts(|f| f.layout_job(job))
                };
                // Reserve paint order before TextEdit; decoration geometry is known
                // only afterwards, but must remain beneath its text and caret.
                let decoration_slot = ui.painter().add(egui::Shape::Noop);
                let mut decorations = Vec::new();
                let editor_id = ui.make_persistent_id((language, "source"));
                let original_events = if language == "json"
                    && ui.memory(|memory| memory.has_focus(editor_id))
                {
                    egui::TextEdit::load_state(ui.ctx(), editor_id)
                        .and_then(|state| state.cursor.char_range())
                        .map(|cursor| {
                            ui.input_mut(|input| {
                                let events = input.events.clone();
                                input.events = super::json_typing::events(text, cursor, &events);
                                events
                            })
                        })
                } else {
                    None
                };
                let output = egui::TextEdit::multiline(text)
                    .id(editor_id)
                    .font(FontId::monospace(
                        crate::ui::components::code_editor::FONT_SIZE,
                    ))
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
                if let Some(events) = original_events {
                    ui.input_mut(|input| input.events = events);
                }
                let number_color = crate::ui::design::colors(ui).muted;
                for (index, row) in output.galley.rows.iter().enumerate() {
                    let y = output.galley_pos.y + row.rect.top();
                    if y + LINE_HEIGHT < ui.clip_rect().top() || y > ui.clip_rect().bottom() {
                        continue;
                    }
                    ui.painter().text(
                        egui::pos2(gutter.right() - 16., y + 1.),
                        egui::Align2::RIGHT_TOP,
                        (index + 1).to_string(),
                        FontId::monospace(crate::ui::components::code_editor::FONT_SIZE),
                        number_color,
                    );
                }
                for &(start, _) in ranges {
                    if let Some(row) = output.galley.rows.get(start) {
                        let y = output.galley_pos.y + row.rect.top();
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(gutter.right() - 15., y),
                            egui::vec2(14., LINE_HEIGHT),
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
                let guide = crate::ui::design::colors(ui).border;
                let character_width =
                    ui.fonts(|f| f.glyph_width(&FontId::monospace(FONT_SIZE), ' '));
                for (index, line) in text.lines().enumerate() {
                    let Some(row) = output.galley.rows.get(index) else {
                        break;
                    };
                    let y = output.galley_pos.y + row.rect.top();
                    if !ui.clip_rect().y_range().contains(y) {
                        continue;
                    }
                    for column in (0..line.chars().take_while(|c| *c == ' ').count()).step_by(2) {
                        let x = output.galley_pos.x + column as f32 * character_width;
                        decorations.push(egui::Shape::line_segment(
                            [egui::pos2(x, y), egui::pos2(x, y + LINE_HEIGHT)],
                            Stroke::new(1_f32, guide),
                        ));
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
                                egui::vec2(width - 48., LINE_HEIGHT),
                            );
                            decorations.push(egui::Shape::rect_stroke(
                                rect,
                                0.,
                                Stroke::new(1_f32, guide),
                                egui::StrokeKind::Inside,
                            ));
                        }
                    }
                }
                ui.painter()
                    .set(decoration_slot, egui::Shape::Vec(decorations));
                output.response
            })
            .inner
        })
        .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    fn type_at(
        source: &str,
        cursor: usize,
        events: Vec<egui::Event>,
        language: &str,
    ) -> (String, usize) {
        type_over_selection(source, cursor, cursor, events, language)
    }

    fn type_over_selection(
        source: &str,
        start: usize,
        end: usize,
        events: Vec<egui::Event>,
        language: &str,
    ) -> (String, usize) {
        let ctx = egui::Context::default();
        let mut text = source.to_owned();
        let mut editor_id = None;
        let render = |ctx: &egui::Context, text: &mut String| {
            egui::CentralPanel::default()
                .show(ctx, |ui| {
                    code_editor(ui, text, language, |text, _| {
                        LayoutJob::simple(
                            text.into(),
                            FontId::monospace(FONT_SIZE),
                            egui::Color32::WHITE,
                            f32::INFINITY,
                        )
                    })
                    .id
                })
                .inner
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            editor_id = Some(render(ctx, &mut text))
        });
        let id = editor_id.unwrap();
        ctx.memory_mut(|m| m.request_focus(id));
        let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(start),
                egui::text::CCursor::new(end),
            )));
        state.store(&ctx, id);
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                render(ctx, &mut text);
            },
        );
        let cursor = egui::TextEdit::load_state(&ctx, id)
            .unwrap()
            .cursor
            .char_range()
            .unwrap();
        assert_eq!(cursor.primary.index, cursor.secondary.index);
        (text, cursor.primary.index)
    }

    #[test]
    fn quote_wraps_selection_including_partial_words_and_unicode() {
        for events in [
            vec![egui::Event::Text("\"".into())],
            vec![
                egui::Event::Ime(egui::ImeEvent::Enabled),
                egui::Event::Ime(egui::ImeEvent::Commit("\"".into())),
            ],
            vec![
                egui::Event::Ime(egui::ImeEvent::Enabled),
                egui::Event::Ime(egui::ImeEvent::Preedit("".into())),
                egui::Event::Ime(egui::ImeEvent::Commit("\"".into())),
            ],
        ] {
            for (source, start, end, expected, cursor) in [
                ("ABC", 0, 2, "\"AB\"C", 4),
                ("ABC", 2, 0, "\"AB\"C", 4),
                ("xTiếng Việty", 1, 11, "x\"Tiếng Việt\"y", 13),
                ("ABC", 0, 3, "\"ABC\"", 5),
            ] {
                assert_eq!(
                    type_over_selection(source, start, end, events.clone(), "json"),
                    (expected.into(), cursor)
                );
            }
        }
        // Pasting a quote still replaces the selection as a normal paste operation.
        assert_eq!(
            type_over_selection("ABC", 0, 2, vec![egui::Event::Paste("\"".into())], "json"),
            ("\"C".into(), 1)
        );
    }

    #[test]
    fn json_typing_pairs_delimiters_and_keeps_cursor_inside() {
        for (typed, expected) in [("{", "{}"), ("\"", "\"\"")] {
            assert_eq!(
                type_at("", 0, vec![egui::Event::Text(typed.into())], "json"),
                (expected.into(), 1)
            );
        }
        assert_eq!(
            type_at(
                "{}",
                1,
                vec![
                    egui::Event::Text("\"".into()),
                    egui::Event::Text("tên".into()),
                    egui::Event::Text("\"".into()),
                    egui::Event::Text("}".into())
                ],
                "json"
            ),
            ("{\"tên\"}".into(), 7)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn json_typing_pairs_linux_ime_commits() {
        for (source, cursor, typed, expected, end) in [
            ("", 0, "{", "{}", 1),
            ("{}", 1, "\"", "{\"\"}", 2),
            ("{\"\"}", 2, "\"", "{\"\"}", 3),
            ("{}", 1, "}", "{}", 2),
            ("\"\"", 1, "tiếng Việt", "\"tiếng Việt\"", 11),
        ] {
            let mut events = vec![egui::Event::Ime(egui::ImeEvent::Commit(typed.into()))];
            self_devkits::adapter::ime::LinuxIme::default().normalize(&mut events);
            assert_eq!(
                type_at(source, cursor, events, "json"),
                (expected.into(), end)
            );
        }
    }

    #[test]
    fn json_typing_handles_preedit_and_commit_in_the_same_frame() {
        for preedit in ["", "{"] {
            let events = vec![
                egui::Event::Ime(egui::ImeEvent::Enabled),
                egui::Event::Ime(egui::ImeEvent::Preedit(preedit.into())),
                egui::Event::Ime(egui::ImeEvent::Commit("{".into())),
            ];
            assert_eq!(type_at("", 0, events, "json"), ("{}".into(), 1));
        }
    }

    #[test]
    fn json_typing_preserves_strings_paste_and_other_languages() {
        for (source, cursor, input, expected, end) in [
            ("\"xin chào \"", 10, "{", "\"xin chào {\"", 11),
            ("\"a\\\"", 3, "\"", "\"a\\\"\"", 4),
            ("{}", 1, "}", "{}", 2),
        ] {
            assert_eq!(
                type_at(
                    source,
                    cursor,
                    vec![egui::Event::Text(input.into())],
                    "json"
                ),
                (expected.into(), end)
            );
        }
        assert_eq!(
            type_at("", 0, vec![egui::Event::Paste("{".into())], "json"),
            ("{".into(), 1)
        );
        assert_eq!(
            type_at("", 0, vec![egui::Event::Text("{".into())], "sql"),
            ("{".into(), 1)
        );
    }

    #[test]
    fn empty_editor_decorations_are_beneath_the_caret() {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let mut last = None;
        for _ in 0..3 {
            last = Some(ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = code_editor(ui, &mut text, "json", |_, _| LayoutJob::default());
                    response.request_focus();
                });
            }));
        }
        let shapes = last.unwrap().shapes;
        let decoration_index = shapes.iter().position(|s| matches!(
            &s.shape, egui::Shape::Vec(v) if v.iter().any(|s| matches!(s, egui::Shape::Rect(_)))
        )).expect("active row decoration");
        let caret_index = shapes
            .iter()
            .rposition(|s| {
                matches!(
                    &s.shape, egui::Shape::LineSegment { points, .. }
                        if points[0].x == points[1].x && points[0].y != points[1].y
                )
            })
            .expect("visible vertical caret");
        assert!(
            decoration_index < caret_index,
            "caret must paint over row decoration"
        );
        assert!(text.is_empty(), "placeholder is not document content");
    }

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
                                    FontId::monospace(
                                        crate::ui::components::code_editor::FONT_SIZE,
                                    ),
                                    egui::Color32::WHITE,
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
