use super::*;
use egui::{text::LayoutJob, FontId, RichText, TextFormat};
use self_devkits::regex_tools::{self, Mode, ResultData};

pub struct RegexEditor {
    pub mode: Mode,
    pattern: String,
    input: String,
    flags: String,
    delimiter: Option<char>,
    replacement: String,
    extraction: String,
    separator: String,
    result: Result<ResultData, String>,
    previous: Option<u64>,
    notice: String,
}
impl Default for RegexEditor {
    fn default() -> Self {
        Self {
            mode: Mode::Match,
            pattern: r"\x20".into(),
            input: "match all spaces".into(),
            flags: "gu".into(),
            delimiter: Some('/'),
            replacement: String::new(),
            extraction: "$0".into(),
            separator: "\n".into(),
            result: Ok(ResultData::default()),
            previous: None,
            notice: String::new(),
        }
    }
}
impl RegexEditor {
    #[cfg(debug_assertions)]
    pub fn preview_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::Extraction {
            self.pattern = r"(?P<word>\w+)".into();
            self.extraction = "${word}".into();
        }
    }
    fn refresh(&mut self) {
        let hash = egui::util::hash((
            &self.pattern,
            &self.input,
            &self.flags,
            self.mode as u8,
            &self.replacement,
            &self.extraction,
            &self.separator,
        ));
        if self.previous == Some(hash) {
            return;
        }
        self.previous = Some(hash);
        self.result = regex_tools::evaluate(
            &self.pattern,
            &self.input,
            &self.flags,
            self.mode,
            if self.mode == Mode::Extraction {
                &self.extraction
            } else {
                &self.replacement
            },
            &self.separator,
        );
    }
    fn highlighted(text: &str, matches: &ResultData, dark: bool) -> LayoutJob {
        let mut job = LayoutJob::default();
        let mut previous = 0;
        let style = TextFormat {
            font_id: FontId::monospace(crate::code_editor::FONT_SIZE),
            color: if dark {
                Color32::from_gray(225)
            } else {
                Color32::from_gray(35)
            },
            ..Default::default()
        };
        for m in &matches.matches {
            let (start, end) = m.bytes;
            if start < previous || end > text.len() {
                continue;
            }
            job.append(&text[previous..start], 0., style.clone());
            job.append(
                &text[start..end],
                0.,
                TextFormat {
                    background: if dark {
                        Color32::from_rgb(35, 78, 115)
                    } else {
                        Color32::from_rgb(190, 220, 255)
                    },
                    ..style.clone()
                },
            );
            previous = end;
        }
        job.append(&text[previous..], 0., style);
        job
    }
    fn inspect(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Explanation").color(accent(ui)));
        ui.separator();
        ui.label("Rust regex · UTF-8").on_hover_text(
            "Rust regex syntax. Look-around and backreferences in patterns are not supported.",
        );
        ui.weak(
            "Offsets below count Unicode characters, starting at 0; end positions are exclusive.",
        );
        for (flag, label) in [
            ('g', "Find all matches"),
            ('i', "Ignore case"),
            ('m', "Multiline anchors"),
            ('s', "Dot matches newline"),
            ('x', "Ignore pattern whitespace"),
            ('U', "Swap greedy / lazy"),
            ('u', "Unicode character classes"),
        ] {
            if self.flags.contains(flag) {
                ui.label(format!("{flag}  {label}"));
            }
        }
        ui.add_space(8.);
        ui.label(RichText::new("Match Information").color(accent(ui)));
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("regex-match-info")
            .max_height((ui.available_height() - 155.).max(100.))
            .show(ui, |ui| match &self.result {
                Err(e) => {
                    ui.colored_label(Color32::from_rgb(205, 65, 65), e);
                }
                Ok(data) if data.matches.is_empty() => {
                    ui.weak("No matches.");
                }
                Ok(data) => {
                    for (i, m) in data.matches.iter().enumerate() {
                        egui::CollapsingHeader::new(format!(
                            "Match {}   {}–{}",
                            i + 1,
                            m.characters.0,
                            m.characters.1
                        ))
                        .id_salt(("regex-match", i))
                        .default_open(i == 0)
                        .show(ui, |ui| {
                            for group in &m.captures {
                                ui.label(RichText::new(format!("Group {}", group.name)).strong());
                                if let Some(value) = &group.text {
                                    ui.label(RichText::new(format!("{value:?}")).monospace());
                                    if let Some((a, b)) = group.range {
                                        ui.weak(format!("{a}–{b}"));
                                    }
                                } else {
                                    ui.weak("Not participating");
                                }
                            }
                        });
                    }
                }
            });
        ui.add_space(12.);
        ui.label(RichText::new("Quick Reference").color(accent(ui)));
        ui.separator();
        for (token, help) in [
            (r"\d / \w / \s", "Digit / word / whitespace"),
            ("[abc] / [^abc]", "Character class / negation"),
            ("^ / $", "Start / end anchor"),
            ("* + ? {n,m}", "Repetition"),
            ("(?P<name>…)", "Named capture"),
            ("$0 / $1 / ${name}", "Template capture reference"),
            ("$$", "Literal $ in templates"),
        ] {
            ui.horizontal_wrapped(|ui| {
                ui.monospace(token);
                ui.weak(help);
            });
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(8., 8.);
        ui.horizontal_wrapped(|ui| {
            for mode in [Mode::Match, Mode::Substitution, Mode::Extraction] {
                ui.selectable_value(&mut self.mode, mode, format!("{mode:?}"));
            }
            if ui.button("Example").clicked() {
                self.pattern = r"\x20".into();
                self.input = "match all spaces".into();
                self.flags = "gu".into();
                self.replacement.clear();
                self.extraction = "$0".into();
            }
            ui.menu_button("Delimiters", |ui| {
                for (delimiter, label) in [
                    (Some('/'), "/ … /"),
                    (Some('#'), "# … #"),
                    (Some('~'), "~ … ~"),
                    (Some('|'), "| … |"),
                    (None, "None"),
                ] {
                    ui.selectable_value(&mut self.delimiter, delimiter, label);
                }
            });
            ui.menu_button("Regex Flags", |ui| {
                for (c, label) in [
                    ('g', "g · Global"),
                    ('i', "i · Ignore case"),
                    ('m', "m · Multiline"),
                    ('s', "s · Dotall"),
                    ('x', "x · Extended"),
                    ('U', "U · Ungreedy"),
                    ('u', "u · Unicode"),
                ] {
                    let mut checked = self.flags.contains(c);
                    if ui.checkbox(&mut checked, label).changed() {
                        self.flags.retain(|f| f != c);
                        if checked {
                            self.flags.push(c);
                        }
                    }
                }
            });
        });
        self.refresh();
        let width = ui.available_width();
        let height = ui.available_height();
        let side = (width * 0.34).clamp(205., 310.);
        let main = (width - side - 18.).max(200.);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(egui::vec2(main,height),egui::Layout::top_down(egui::Align::Min),|ui| {
                ui.set_width(main);
                ui.horizontal(|ui| {ui.label(RichText::new("Regular Expression").color(accent(ui)));if let Ok(data)=&self.result {ui.weak(format!("{} matches",data.matches.len()));}});
                egui::Frame::new().fill(ui.visuals().extreme_bg_color).corner_radius(5.).inner_margin(8.).show(ui,|ui| {
                    ui.horizontal(|ui| {
                        if let Some(d)=self.delimiter {ui.monospace(d.to_string());}
                        let extra=if self.delimiter.is_some() {95.} else {72.};
                        ui.add(egui::TextEdit::singleline(&mut self.pattern).font(egui::TextStyle::Monospace).desired_width((main-extra-20.).max(80.)).hint_text("Regular expression").frame(false));
                        if let Some(d)=self.delimiter {ui.monospace(d.to_string());}
                        ui.label(RichText::new(&self.flags).color(accent(ui))).on_hover_text("Use Regex Flags to toggle modifiers");
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Copy expression").clicked() {ui.ctx().copy_text(regex_tools::expression(&self.pattern,self.delimiter,&self.flags));self.notice="Expression copied.".into();}
                    if ui.small_button("Paste expression").clicked() {
                        match arboard::Clipboard::new().and_then(|mut c|c.get_text()) {
                            Ok(text)=>match regex_tools::parse_expression(&text,self.delimiter) {Ok((pattern,flags))=>{self.pattern=pattern;if self.delimiter.is_some() {self.flags=flags;}self.notice.clear();},Err(e)=>self.notice=e},
                            Err(e)=>self.notice=e.to_string(),
                        }
                    }
                });
                if !self.notice.is_empty() {ui.weak(&self.notice);}
                if self.result.is_err() {ui.colored_label(Color32::from_rgb(205,65,65),"Cannot evaluate expression. See details at right.");}
                ui.add_space(4.);ui.label(RichText::new("Input").strong());
                let input_height=if self.mode==Mode::Match {(ui.available_height()-20.).max(120.)} else {(ui.available_height()*0.46).max(100.)};
                let data=self.result.as_ref().ok().cloned().unwrap_or_default();
                let mut layouter=|ui:&egui::Ui,text:&str,width:f32| {
                    let mut job=if text==self.input {Self::highlighted(text,&data,ui.visuals().dark_mode)} else {Self::highlighted(text,&ResultData::default(),ui.visuals().dark_mode)};
                    job.wrap.max_width=width;ui.fonts(|f|f.layout_job(job))
                };
                // The source used for highlighting is copied so TextEdit can mutate independently.
                let mut input=self.input.clone();
                egui::Frame::new().fill(if ui.visuals().dark_mode {Color32::from_gray(24)} else {Color32::WHITE}).stroke(egui::Stroke::new(1_f32,ui.visuals().widgets.noninteractive.bg_stroke.color)).corner_radius(5.).inner_margin(10.).show(ui,|ui| {
                    egui::ScrollArea::vertical().id_salt("regex-input").max_height(input_height).min_scrolled_height(input_height).show(ui,|ui| {ui.add_sized([ui.available_width(),input_height],egui::TextEdit::multiline(&mut input).frame(false).desired_width(f32::INFINITY).layouter(&mut layouter));});
                });
                self.input=input;
                if self.mode!=Mode::Match {
                    ui.add_space(6.);ui.horizontal(|ui| {ui.label(RichText::new(format!("{:?}",self.mode)).color(accent(ui)));if ui.small_button("Copy result").clicked() {if let Ok(result)=&self.result {ui.ctx().copy_text(result.output.clone());}}});
                    let template=if self.mode==Mode::Extraction {&mut self.extraction} else {&mut self.replacement};
                    ui.add(egui::TextEdit::singleline(template).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).hint_text("Replacement; empty removes matches")).on_hover_text("$0 whole match · $1 capture · ${name} named capture · $$ literal dollar");
                    if self.mode==Mode::Extraction {
                        egui::ComboBox::from_id_salt("regex-separator").selected_text(match self.separator.as_str() {"\n"=>"Newline","\t"=>"Tab",", "=>"Comma",_=>"None"}).show_ui(ui,|ui| {for (label,value) in [("Newline","\n"),("Tab","\t"),("Comma",", "),("None","")] {ui.selectable_value(&mut self.separator,value.into(),label);}});
                    }
                    let mut output=self.result.as_ref().map(|r|r.output.as_str()).unwrap_or("");
                    let height=(ui.available_height()-10.).max(65.);
                    egui::ScrollArea::vertical().id_salt("regex-output").max_height(height).show(ui,|ui| {ui.add_sized([ui.available_width(),height],egui::TextEdit::multiline(&mut output).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));});
                }
            });
            ui.separator();
            ui.allocate_ui_with_layout(egui::vec2(side,height),egui::Layout::top_down(egui::Align::Min),|ui| {ui.set_width(side);egui::ScrollArea::vertical().id_salt("regex-inspector").max_height(height).show(ui,|ui|self.inspect(ui));});
        });
        let before = self.previous;
        self.refresh();
        if before != self.previous {
            ui.ctx().request_repaint();
        }
    }
}
fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(125, 175, 255)
    } else {
        Color32::from_rgb(30, 95, 175)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_and_modes_recompute_without_stale_results() {
        let mut editor = RegexEditor::default();
        editor.refresh();
        assert_eq!(editor.result.as_ref().unwrap().matches.len(), 2);
        editor.mode = Mode::Substitution;
        editor.refresh();
        assert_eq!(editor.result.as_ref().unwrap().output, "matchallspaces");
        editor.mode = Mode::Extraction;
        editor.pattern = r"(?P<word>\w+)".into();
        editor.extraction = "${word}".into();
        editor.refresh();
        assert_eq!(editor.result.as_ref().unwrap().output, "match\nall\nspaces");
        editor.pattern = "[".into();
        editor.refresh();
        assert!(editor.result.is_err());
        editor.pattern = "NOT_FOUND".into();
        editor.refresh();
        assert!(editor.result.as_ref().unwrap().output.is_empty());
        assert!(editor.result.as_ref().unwrap().matches.is_empty());
    }
}
