//! Presentation state and composition; tool behavior lives in business services.
use eframe::egui::{self, Color32};
use egui_phosphor::regular as icons;
use self_devkits::adapter::desktop;
#[cfg(target_os = "linux")]
use self_devkits::adapter::ime;
use self_devkits::business::{self, Action, Language, PasswordOptions, SqlDialect};
mod app;
mod bootstrap;
mod components;
mod design;
mod navigation;
#[cfg(debug_assertions)]
mod preview;
mod shell;
mod tools;
pub use bootstrap::run;
use navigation::Tool;
use tools::{pdf_editor, regex_editor};
#[derive(Default)]
struct Document {
    input: String,
    output: String,
    message: String,
    error: bool,
}
struct DevKits {
    tool: Tool,
    #[cfg(target_os = "linux")]
    ime: ime::LinuxIme,
    search: String,
    regex: regex_editor::RegexEditor,
    pdf: pdf_editor::PdfEditor,
    json_simple: bool,
    json_collapsed: std::collections::BTreeSet<usize>,
    sidebar_open: bool,
    pinned: bool,
    docs: [Document; 3],
    indent: usize,
    dialect: SqlDialect,
    sql_uppercase: bool,
    sql_alias_align: bool,
    dark: bool,
    password_options: PasswordOptions,
    password_count: usize,
    secrets: Vec<String>,
    reveal: bool,
    qr_input: String,
    qr_level: usize,
    qr_texture: Option<egui::TextureHandle>,
    qr_image: Option<image::GrayImage>,
    notice: String,
    notice_error: bool,
    #[cfg(debug_assertions)]
    capture_requested: bool,
}
impl Default for DevKits {
    fn default() -> Self {
        Self {
            tool: Tool::Json,
            #[cfg(target_os = "linux")]
            ime: ime::LinuxIme::default(),
            search: String::new(),
            regex: regex_editor::RegexEditor::default(),
            pdf: pdf_editor::PdfEditor::default(),
            json_simple: false,
            json_collapsed: std::collections::BTreeSet::new(),
            sidebar_open: true,
            pinned: false,
            docs: std::array::from_fn(|_| Document::default()),
            indent: 2,
            dialect: SqlDialect::MySql,
            sql_uppercase: true,
            sql_alias_align: true,
            dark: false,
            password_options: PasswordOptions::default(),
            password_count: 1,
            secrets: Vec::new(),
            reveal: false,
            qr_input: String::new(),
            qr_level: 1,
            qr_texture: None,
            qr_image: None,
            notice: String::new(),
            notice_error: false,
            #[cfg(debug_assertions)]
            capture_requested: false,
        }
    }
}
fn theme(ctx: &egui::Context, dark: bool) {
    ctx.set_visuals(design::visuals(dark));
    ctx.style_mut(|s| {
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
        s.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(crate::ui::components::code_editor::FONT_SIZE),
        );
        s.spacing.item_spacing = egui::vec2(10., 10.);
        s.spacing.button_padding = egui::vec2(12., 6.);
    });
}
impl DevKits {
    fn open(&mut self) {
        let lang = self.tool.lang().unwrap();
        if let Some(path) = desktop::open_source(lang.extension()) {
            let result = business::documents::read_source(&path);
            let doc = &mut self.docs[self.tool.index()];
            match result {
                Ok(text) => {
                    doc.input = text;
                    doc.output.clear();
                    doc.message.clear();
                }
                Err(e) => {
                    doc.error = true;
                    doc.message = e;
                }
            }
        }
    }
}
