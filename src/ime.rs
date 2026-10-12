//! Work around commit-only Linux IMEs in egui 0.31.
//! egui-winit ignores Linux Ime::Enabled. Fcitx5/Unikey may send a Commit
//! without Preedit, leaving TextEdit's composition anchor at an old cursor.
//! Upstream report: https://github.com/emilk/egui/issues/7906
use eframe::egui::{Event, ImeEvent};

#[derive(Default)]
pub struct LinuxIme {
    composing: bool,
}
impl LinuxIme {
    pub fn normalize(&mut self, events: &mut Vec<Event>) {
        let mut normalized = Vec::with_capacity(events.len());
        for event in std::mem::take(events) {
            match &event {
                Event::Ime(ImeEvent::Enabled | ImeEvent::Preedit(_)) => self.composing = true,
                Event::Ime(ImeEvent::Disabled) => self.composing = false,
                Event::Ime(ImeEvent::Commit(_)) => {
                    if !self.composing {
                        // Anchor the new composition at the actual selection. Preserve
                        // Commit (instead of converting to Text) so selection replacement works.
                        normalized.push(Event::Ime(ImeEvent::Enabled));
                    }
                    self.composing = false;
                }
                _ => {}
            }
            normalized.push(event);
        }
        *events = normalized;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{
        self,
        text::{CCursor, CCursorRange},
    };

    fn frame(ctx: &egui::Context, text: &mut String, events: Vec<Event>) {
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::TextEdit::multiline(text)
                        .id(egui::Id::new("ime-test"))
                        .show(ui);
                });
            },
        );
    }
    fn setup(text: &mut String, start: usize, end: usize) -> egui::Context {
        let ctx = egui::Context::default();
        frame(&ctx, text, vec![]);
        let id = egui::Id::new("ime-test");
        ctx.memory_mut(|m| m.request_focus(id));
        let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(start),
            CCursor::new(end),
        )));
        state.store(&ctx, id);
        ctx
    }
    #[test]
    fn reproduces_upstream_commit_only_bug() {
        let mut text = "abc ".to_owned();
        let ctx = setup(&mut text, 4, 4);
        frame(
            &ctx,
            &mut text,
            vec![Event::Ime(ImeEvent::Commit("tiếng Việt".into()))],
        );
        assert_eq!(
            text, "abc ",
            "Baseline demonstrates egui 0.31 dropping this commit"
        );
    }
    #[test]
    fn commit_only_at_end_middle_and_selection() {
        for (initial, start, end, expected) in [
            ("abc ", 4, 4, "abc tiếng Việt"),
            ("a b", 2, 2, "a tiếng Việtb"),
            ("abc", 1, 2, "atiếng Việtc"),
            ("", 0, 0, "tiếng Việt"),
            ("Chào ", 5, 5, "Chào tiếng Việt"),
        ] {
            let mut text = initial.to_owned();
            let ctx = setup(&mut text, start, end);
            let mut bridge = LinuxIme::default();
            let mut events = vec![Event::Ime(ImeEvent::Commit("tiếng Việt".into()))];
            bridge.normalize(&mut events);
            frame(&ctx, &mut text, events);
            assert_eq!(text, expected);
            let mut events = vec![Event::Ime(ImeEvent::Commit("!".into()))];
            bridge.normalize(&mut events);
            frame(&ctx, &mut text, events);
            assert!(text.contains("tiếng Việt!"));
        }
    }
    #[test]
    fn preedit_is_replaced_without_duplication() {
        let mut text = "abc ".to_owned();
        let ctx = setup(&mut text, 4, 4);
        let mut bridge = LinuxIme::default();
        for mut events in [
            vec![
                Event::Ime(ImeEvent::Enabled),
                Event::Ime(ImeEvent::Preedit("tie".into())),
            ],
            vec![Event::Ime(ImeEvent::Preedit("tiếng".into()))],
            vec![Event::Ime(ImeEvent::Commit("tiếng".into()))],
        ] {
            bridge.normalize(&mut events);
            frame(&ctx, &mut text, events);
        }
        assert_eq!(text, "abc tiếng");
    }
}
