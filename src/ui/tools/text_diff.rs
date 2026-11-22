use super::{crypto_job, *};
use egui::{FontId, Rect, Sense, Vec2};
use self_devkits::business::text_diff::{self, DiffLine, DiffResult};

const ROW_HEIGHT: f32 = 21.;
const NOTE: &str = "\\ No newline at end of file";
const COMPARE_EXAMPLE: &str = "Project: DevKits\nStatus: ready\nOwner: Linh\n\nFeature: JSON Editor\nFeature: Text Diff\nNew line: Tiếng Việt\nEnd\nExtra note";
const ORIGINAL_EXAMPLE: &str = "Project: DevKits\nStatus: draft\nOwner: Linh\nRemove this old line\n\nFeature: JSON Editor\nFeature: Text tools\nEnd\n";

#[derive(Clone, Copy)]
enum ViewRow {
    Line(usize),
    NewlineNote(usize),
}

#[derive(Default)]
pub(in crate::ui) struct TextDiff {
    compare: String,
    original: String,
    result: Option<DiffResult>,
    view_rows: Vec<ViewRow>,
    comparison: bool,
    job: crypto_job::Job<DiffResult>,
    message: String,
    horizontal: [f32; 2],
    widths: Option<[f32; 2]>,
    revision: u64,
}

#[derive(Clone, Copy)]
struct DiffColors {
    added: Color32,
    added_bg: Color32,
    deleted: Color32,
    deleted_bg: Color32,
}

fn diff_colors(dark: bool) -> DiffColors {
    if dark {
        DiffColors {
            added: Color32::from_rgb(119, 224, 147),
            added_bg: Color32::from_rgb(26, 60, 38),
            deleted: Color32::from_rgb(255, 135, 135),
            deleted_bg: Color32::from_rgb(70, 32, 36),
        }
    } else {
        DiffColors {
            added: Color32::from_rgb(18, 115, 50),
            added_bg: Color32::from_rgb(232, 248, 235),
            deleted: Color32::from_rgb(180, 34, 34),
            deleted_bg: Color32::from_rgb(255, 235, 235),
        }
    }
}

// Keep control characters visible while Copy always retains the original source.
fn display_line(text: &str) -> String {
    let mut output = String::new();
    let mut column = 0;
    for ch in text.chars() {
        if ch == '\t' {
            let spaces = 4 - column % 4;
            output.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else if ch == '\r' {
            output.push_str("\\r");
            column += 2;
        } else if ch.is_control() {
            let visible = format!("\\u{{{:x}}}", ch as u32);
            column += visible.len();
            output.push_str(&visible);
        } else {
            output.push(ch);
            column += 1;
        }
    }
    output
}

impl TextDiff {
    fn invalidate(&mut self) {
        self.result = None;
        self.view_rows.clear();
        self.comparison = false;
        self.message.clear();
        self.widths = None;
        self.horizontal = [0.; 2];
        self.revision = self.revision.wrapping_add(1);
    }

    fn replace(&mut self, side: usize, value: String) {
        if self.job.busy() {
            return;
        }
        if value.len() > text_diff::MAX_TEXT {
            self.message = "Each text must be at most 2 MiB. The current text was kept.".into();
            return;
        }
        if side == 0 {
            self.compare = value;
        } else {
            self.original = value;
        }
        self.invalidate();
    }

    fn example(&mut self) {
        if self.job.busy() {
            return;
        }
        self.compare = COMPARE_EXAMPLE.into();
        self.original = ORIGINAL_EXAMPLE.into();
        self.invalidate();
    }

    fn clear(&mut self) {
        if self.job.busy() {
            return;
        }
        self.compare.clear();
        self.original.clear();
        self.invalidate();
    }

    fn start(&mut self) {
        if self.job.busy() {
            return;
        }
        if self.compare.len() > text_diff::MAX_TEXT || self.original.len() > text_diff::MAX_TEXT {
            self.message = "Each text must be at most 2 MiB.".into();
            return;
        }
        self.invalidate();
        let compare = self.compare.clone();
        let original = self.original.clone();
        self.job
            .start(move || text_diff::compare(&compare, &original));
    }

    fn accept(&mut self, result: Result<DiffResult, String>) {
        match result {
            Ok(result) => {
                self.view_rows.clear();
                for (index, row) in result.rows.iter().enumerate() {
                    self.view_rows.push(ViewRow::Line(index));
                    if row.compare.as_ref().is_some_and(|line| !line.has_newline)
                        || row.original.as_ref().is_some_and(|line| !line.has_newline)
                    {
                        self.view_rows.push(ViewRow::NewlineNote(index));
                    }
                }
                self.result = Some(result);
                self.comparison = true;
                self.widths = None;
                self.horizontal = [0.; 2];
                self.message.clear();
            }
            Err(error) => {
                self.result = None;
                self.comparison = false;
                self.message = error;
            }
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.job.poll(ctx) {
            self.accept(result);
        }
    }

    pub(in crate::ui) fn toolbar(&mut self, ui: &mut egui::Ui) {
        self.poll(ui.ctx());
        let busy = self.job.busy();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!busy, |ui| {
                if ui.button("Compare").clicked() {
                    self.start();
                }
                if ui.selectable_label(!self.comparison, "Edit").clicked() {
                    self.comparison = false;
                }
                if ui.button("Example").clicked() {
                    self.example();
                    self.start();
                }
                if ui.button("Clear").clicked() {
                    self.clear();
                }
            });
            if busy || self.job.busy() {
                ui.spinner();
                ui.label("Comparing…");
            }
        });
    }

    fn pane_header(&mut self, ui: &mut egui::Ui, side: usize) {
        ui.strong(if side == 0 {
            "Compare — Left"
        } else {
            "Original — Right"
        });
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.job.busy(), |ui| {
                if ui.button("Open…").clicked() {
                    if let Some(path) = desktop::open_source("txt") {
                        match business::documents::read_source(&path) {
                            Ok(text) => self.replace(side, text),
                            Err(error) => self.message = error,
                        }
                    }
                }
                if ui.button("Paste").clicked() {
                    match desktop::clipboard_text() {
                        Ok(text) => self.replace(side, text),
                        Err(error) => self.message = format!("Cannot read clipboard: {error}"),
                    }
                }
            });
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(if side == 0 {
                    self.compare.clone()
                } else {
                    self.original.clone()
                });
            }
        });
    }

    pub(in crate::ui) fn ui(&mut self, ui: &mut egui::Ui) {
        self.poll(ui.ctx());
        let colors = diff_colors(ui.visuals().dark_mode);
        if self.comparison {
            if let Some(result) = &self.result {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(colors.added, format!("+ {} added", result.added));
                    ui.colored_label(colors.deleted, format!("− {} removed", result.deleted));
                    ui.label(if result.added == 0 && result.deleted == 0 {
                        "No differences".into()
                    } else {
                        format!("{} unchanged", result.unchanged)
                    });
                });
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(colors.added, "+ Added on left");
                ui.colored_label(colors.deleted, "− Removed on right");
                ui.label("Open or paste two texts, then Compare.");
            });
        }
        if !self.message.is_empty() {
            design::notice(ui, design::colors(ui).danger, &self.message);
        }
        ui.columns(2, |columns| {
            for (side, column) in columns.iter_mut().enumerate() {
                column.push_id(("text-diff-header", side), |ui| self.pane_header(ui, side));
            }
        });
        ui.add_space(4.);
        if self.comparison && self.result.is_some() {
            self.comparison_ui(ui);
        } else {
            self.edit_ui(ui);
        }
    }

    fn edit_ui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let height = ui.available_height().max(60.);
        ui.add_enabled_ui(!self.job.busy(), |ui| {
            ui.columns(2, |columns| {
                for (side, column) in columns.iter_mut().enumerate() {
                    let width = column.available_width();
                    let text = if side == 0 {
                        &mut self.compare
                    } else {
                        &mut self.original
                    };
                    egui::ScrollArea::both()
                        .id_salt(("text-diff-edit", side))
                        .auto_shrink([false, false])
                        .max_height(height)
                        .show(column, |ui| {
                            let mut layouter = |ui: &egui::Ui, text: &str, _: f32| {
                                ui.fonts(|fonts| {
                                    fonts.layout_job(egui::text::LayoutJob::simple(
                                        text.into(),
                                        FontId::monospace(14.),
                                        design::colors(ui).text,
                                        f32::INFINITY,
                                    ))
                                })
                            };
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(text)
                                        .id_salt(("text-diff-source", side))
                                        .code_editor()
                                        .desired_width(width)
                                        .min_size(egui::vec2(width, height))
                                        .hint_text(if side == 0 {
                                            "Compared text"
                                        } else {
                                            "Original text"
                                        })
                                        .layouter(&mut layouter),
                                )
                                .changed();
                        });
                }
            });
        });
        if changed {
            self.invalidate();
        }
    }

    fn comparison_ui(&mut self, ui: &mut egui::Ui) {
        let result = self.result.as_ref().unwrap();
        let font = FontId::monospace(14.);
        let space_width = ui.fonts(|fonts| fonts.glyph_width(&font, ' '));
        let digits = result.rows.len().to_string().len().max(3);
        let gutter = (digits + 3) as f32 * space_width;
        let widths = *self.widths.get_or_insert_with(|| {
            let mut widths = [0_f32; 2];
            ui.fonts(|fonts| {
                for row in &result.rows {
                    for (side, line) in [row.compare.as_ref(), row.original.as_ref()]
                        .into_iter()
                        .enumerate()
                    {
                        if let Some(line) = line {
                            let text = display_line(&line.text);
                            let width: f32 =
                                text.chars().map(|ch| fonts.glyph_width(&font, ch)).sum();
                            widths[side] = widths[side].max(width + 16.);
                            if !line.has_newline {
                                widths[side] = widths[side].max(
                                    fonts
                                        .layout_no_wrap(
                                            NOTE.into(),
                                            FontId::monospace(12.),
                                            Color32::WHITE,
                                        )
                                        .size()
                                        .x
                                        + 16.,
                                );
                            }
                        }
                    }
                }
            });
            widths
        });
        let bar_height = ui.spacing().scroll.allocated_width().max(16.) + 4.;
        let height = (ui.available_height() - bar_height - 4.).max(60.);
        let canvas_rect = Rect::from_min_size(
            ui.next_widget_position(),
            egui::vec2(ui.available_width(), height),
        );
        let pane_width = (canvas_rect.width() - ui.spacing().item_spacing.x) / 2.;
        // A shared vertical viewport guarantees both panes always show the same rows.
        // Horizontal wheel input and the two bottom scrollbars remain independent.
        ui.input_mut(|input| {
            if let Some(pointer) = input
                .pointer
                .hover_pos()
                .filter(|pos| canvas_rect.contains(*pos))
            {
                let side = usize::from(pointer.x >= canvas_rect.center().x);
                let max = (widths[side] + gutter - pane_width).max(0.);
                self.horizontal[side] =
                    (self.horizontal[side] - input.smooth_scroll_delta.x).clamp(0., max);
                input.smooth_scroll_delta.x = 0.;
            }
        });
        let viewport = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                ui.style_mut().always_scroll_the_only_direction = false;
                egui::ScrollArea::vertical()
                    .id_salt(("text-diff-rows", self.revision))
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show_rows(ui, ROW_HEIGHT, self.view_rows.len(), |ui, range| {
                        ui.columns(2, |columns| {
                            for (side, column) in columns.iter_mut().enumerate() {
                                for index in range.clone() {
                                    let view = self.view_rows[index];
                                    let (row, note) = match view {
                                        ViewRow::Line(row) => (&result.rows[row], false),
                                        ViewRow::NewlineNote(row) => (&result.rows[row], true),
                                    };
                                    let line = if side == 0 {
                                        row.compare.as_ref()
                                    } else {
                                        row.original.as_ref()
                                    };
                                    paint_cell(
                                        column,
                                        line,
                                        row.changed,
                                        note,
                                        side,
                                        gutter,
                                        self.horizontal[side],
                                    );
                                }
                            }
                        });
                    })
            })
            .inner;
        ui.add_space(4.);
        ui.allocate_ui_with_layout(
            egui::vec2(viewport.inner_rect.width(), bar_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.columns(2, |columns| {
                    for (side, column) in columns.iter_mut().enumerate() {
                        let output = egui::ScrollArea::horizontal()
                            .id_salt(("text-diff-horizontal", self.revision, side))
                            .horizontal_scroll_offset(self.horizontal[side])
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
                            .auto_shrink([false, false])
                            .max_height(bar_height)
                            .show(column, |ui| {
                                ui.allocate_space(egui::vec2(widths[side] + gutter, 1.));
                            });
                        if self.horizontal[side] != output.state.offset.x {
                            self.horizontal[side] = output.state.offset.x;
                            column.ctx().request_repaint();
                        }
                    }
                })
            },
        );
    }

    #[cfg(debug_assertions)]
    pub(in crate::ui) fn preview(&mut self, edit: bool) {
        self.example();
        if std::env::var_os("DEVKITS_TEXT_DIFF_LONG").is_some() {
            let line = format!("Long line: {}\n", "Tiếng Việt · text diff · ".repeat(15));
            self.compare.insert_str(0, &line);
            self.original.insert_str(0, &line);
        }
        if !edit {
            self.accept(text_diff::compare(&self.compare, &self.original));
        }
    }
}

fn paint_cell(
    ui: &mut egui::Ui,
    line: Option<&DiffLine>,
    changed: bool,
    note: bool,
    side: usize,
    gutter: f32,
    horizontal: f32,
) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let colors = design::colors(ui);
    let diff = diff_colors(ui.visuals().dark_mode);
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    let (color, background) = if line.is_some() && changed {
        if side == 0 {
            (diff.added, diff.added_bg)
        } else {
            (diff.deleted, diff.deleted_bg)
        }
    } else {
        (
            colors.text,
            if line.is_some() {
                colors.surface
            } else {
                colors.surface_raised
            },
        )
    };
    painter.rect_filled(rect, 0., background);
    painter.vline(
        rect.right() - 1.,
        rect.y_range(),
        egui::Stroke::new(1_f32, colors.border),
    );
    let Some(line) = line else {
        return;
    };
    let code_rect = Rect::from_min_max(egui::pos2(rect.left() + gutter, rect.top()), rect.max);
    let code = painter.with_clip_rect(painter.clip_rect().intersect(code_rect));
    if note {
        if !line.has_newline {
            code.text(
                egui::pos2(code_rect.left() - horizontal, rect.top() + 3.),
                egui::Align2::LEFT_TOP,
                NOTE,
                FontId::monospace(12.),
                color,
            );
        }
    } else {
        painter.text(
            egui::pos2(rect.left() + gutter - 24., rect.top() + 1.),
            egui::Align2::RIGHT_TOP,
            line.number.to_string(),
            FontId::monospace(14.),
            colors.muted,
        );
        if changed {
            painter.text(
                egui::pos2(rect.left() + gutter - 18., rect.top() + 1.),
                egui::Align2::LEFT_TOP,
                if side == 0 { "+" } else { "−" },
                FontId::monospace(14.),
                color,
            );
        }
        code.text(
            egui::pos2(code_rect.left() - horizontal, rect.top() + 1.),
            egui::Align2::LEFT_TOP,
            display_line(&line.text),
            FontId::monospace(14.),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &egui::Context,
        app: &mut DevKits,
        events: Vec<egui::Event>,
        width: f32,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 580.),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.show_shell(ctx),
        )
    }

    fn click(
        ctx: &egui::Context,
        app: &mut DevKits,
        point: egui::Pos2,
        width: f32,
    ) -> egui::FullOutput {
        let _ = frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            width,
        );
        frame(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            width,
        )
    }

    fn label_positions(output: &egui::FullOutput, label: &str) -> Vec<egui::Pos2> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.pos + text.galley.size() / 2.)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn render_places_green_additions_left_and_red_removals_right_in_both_themes() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            for width in [850., 1000.] {
                let ctx = egui::Context::default();
                ctx.set_theme(theme);
                let mut app = DevKits {
                    tool: Tool::TextDiff,
                    ..Default::default()
                };
                app.text_diff.compare = "shared\nnew\n".into();
                app.text_diff.original = "shared\nold\n".into();
                app.text_diff.accept(text_diff::compare(
                    &app.text_diff.compare,
                    &app.text_diff.original,
                ));
                let output = frame(&ctx, &mut app, vec![], width);
                let colors = diff_colors(theme == egui::Theme::Dark);
                let mut positions = Vec::new();
                for (label, color, background) in [
                    ("new", colors.added, colors.added_bg),
                    ("old", colors.deleted, colors.deleted_bg),
                ] {
                    let shape = output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(shape.galley.job.sections[0].format.color, color);
                    positions.push(shape.pos);
                    assert!(output.shapes.iter().any(|paint| matches!(
                        &paint.shape, egui::Shape::Rect(rect) if rect.fill == background && rect.rect.contains(shape.pos)
                    )));
                }
                assert!(positions[0].x < positions[1].x);
                assert_eq!(positions[0].y, positions[1].y);
                assert!(
                    label_positions(&output, "Compare — Left")[0].x
                        < label_positions(&output, "Original — Right")[0].x
                );
                assert!(
                    label_positions(&output, "+").len() == 1
                        && label_positions(&output, "−").len() == 1
                );
            }
        }
    }

    #[test]
    fn import_limit_edit_clear_and_busy_state_preserve_sources() {
        let mut diff = TextDiff::default();
        diff.example();
        let sources = (diff.compare.clone(), diff.original.clone());
        diff.accept(text_diff::compare(&diff.compare, &diff.original));
        diff.replace(0, "x".repeat(text_diff::MAX_TEXT + 1));
        assert_eq!((&diff.compare, &diff.original), (&sources.0, &sources.1));
        assert!(diff.result.is_some());
        assert!(diff.message.contains("2 MiB"));
        diff.replace(0, "edited".into());
        assert!(diff.result.is_none() && !diff.comparison && diff.message.is_empty());

        let (sender, receiver) = std::sync::mpsc::channel();
        diff.job.start(move || {
            receiver.recv().unwrap();
            Ok(DiffResult::default())
        });
        diff.replace(0, "forbidden".into());
        diff.clear();
        diff.example();
        assert_eq!(diff.compare, "edited");
        assert_eq!(diff.original, sources.1);
        sender.send(()).unwrap();
        let ctx = egui::Context::default();
        for _ in 0..1000 {
            diff.poll(&ctx);
            if !diff.job.busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!diff.job.busy());
        diff.clear();
        assert!(diff.compare.is_empty() && diff.original.is_empty() && diff.result.is_none());
    }

    #[test]
    fn copy_keeps_exact_crlf_source_and_edit_button_returns_to_two_inputs() {
        let ctx = egui::Context::default();
        let mut app = DevKits {
            tool: Tool::TextDiff,
            ..Default::default()
        };
        app.text_diff.compare = "left\r\n".into();
        app.text_diff.original = "right\r\n".into();
        app.text_diff.accept(text_diff::compare(
            &app.text_diff.compare,
            &app.text_diff.original,
        ));
        let output = frame(&ctx, &mut app, vec![], 850.);
        let copies = label_positions(&output, "Copy");
        assert_eq!(copies.len(), 2);
        let copied = click(&ctx, &mut app, copies[0], 850.);
        assert!(copied
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(command,
                egui::OutputCommand::CopyText(text) if text == "left\r\n"
            )));
        let edit = label_positions(&copied, "Edit")[0];
        let _ = click(&ctx, &mut app, edit, 850.);
        assert!(!app.text_diff.comparison);
        assert_eq!(app.text_diff.compare, "left\r\n");
        assert_eq!(app.text_diff.original, "right\r\n");
        app.tool = Tool::Json;
        let _ = frame(&ctx, &mut app, vec![], 850.);
        app.tool = Tool::TextDiff;
        let _ = frame(&ctx, &mut app, vec![], 850.);
        assert_eq!(app.text_diff.compare, "left\r\n");
    }

    #[test]
    fn newline_notes_control_characters_and_shared_virtual_rows_are_visible() {
        assert_eq!(display_line("a\r\tb"), "a\\r b");
        let ctx = egui::Context::default();
        let mut app = DevKits {
            tool: Tool::TextDiff,
            ..Default::default()
        };
        app.text_diff.compare = "a\rb".into();
        app.text_diff.original = "ab\n".into();
        app.text_diff.accept(text_diff::compare(
            &app.text_diff.compare,
            &app.text_diff.original,
        ));
        let output = frame(&ctx, &mut app, vec![], 850.);
        assert_eq!(label_positions(&output, NOTE).len(), 1);
        assert_eq!(label_positions(&output, "a\\rb").len(), 1);

        let rows: String = (0..1000).map(|n| format!("row {n}\n")).collect();
        app.text_diff.compare = rows.clone();
        app.text_diff.original = rows;
        app.text_diff.invalidate();
        app.text_diff.accept(text_diff::compare(
            &app.text_diff.compare,
            &app.text_diff.original,
        ));
        let output = frame(&ctx, &mut app, vec![], 850.);
        assert_eq!(label_positions(&output, "row 0").len(), 2);
        assert!(label_positions(&output, "row 999").is_empty());
        let hover = label_positions(&output, "row 0")[0];
        let _ = frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(hover),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0., -300.),
                    modifiers: Default::default(),
                },
            ],
            850.,
        );
        let output = frame(&ctx, &mut app, vec![], 850.);
        assert!(label_positions(&output, "row 0").is_empty());
        let mut checked = 0;
        for n in 1..1000 {
            let positions = label_positions(&output, &format!("row {n}"));
            if !positions.is_empty() {
                assert_eq!(positions.len(), 2);
                assert_eq!(positions[0].y, positions[1].y);
                checked += 1;
            }
        }
        assert!(checked > 0 && checked < 40);
    }

    #[test]
    fn long_lines_clip_to_their_side_and_horizontal_scrolling_is_independent() {
        let ctx = egui::Context::default();
        let mut app = DevKits {
            tool: Tool::TextDiff,
            ..Default::default()
        };
        let long = format!("Long {}", "text ".repeat(60));
        app.text_diff.compare = format!("{long}\n");
        app.text_diff.original = app.text_diff.compare.clone();
        app.text_diff.accept(text_diff::compare(
            &app.text_diff.compare,
            &app.text_diff.original,
        ));
        let output = frame(&ctx, &mut app, vec![], 850.);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == long => {
                    Some((text.pos, shape.clip_rect))
                }
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2);
        assert!(texts[0].1.right() < texts[1].1.left());
        let pointer = texts[0].0 + egui::vec2(5., 5.);
        let _ = frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(pointer),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(-150., 0.),
                    modifiers: Default::default(),
                },
            ],
            850.,
        );
        assert!(app.text_diff.horizontal[0] > 0.);
        assert_eq!(app.text_diff.horizontal[1], 0.);
        let output = frame(&ctx, &mut app, vec![], 850.);
        let positions: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == long => Some(text.pos),
                _ => None,
            })
            .collect();
        assert!(positions[0].x < texts[0].0.x);
        assert_eq!(positions[1].x, texts[1].0.x);
        assert_eq!(positions[0].y, positions[1].y);
    }
}
