#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, Color32};
use egui_phosphor::regular as icons;
mod code_editor;
#[cfg(any(target_os = "linux", test))]
mod ime;
mod json_editor;
mod pdf_editor;
mod regex_editor;
mod sql_editor;
mod window_chrome;
mod xml_editor;
use self_devkits::{Action, Language, PasswordOptions, SqlDialect};

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Json,
    Xml,
    Sql,
    Password,
    Qr,
    Regex,
    Pdf,
}
impl Tool {
    const ALL: [Self; 7] = [
        Self::Json,
        Self::Sql,
        Self::Xml,
        Self::Regex,
        Self::Password,
        Self::Qr,
        Self::Pdf,
    ];
    fn title(self) -> &'static str {
        match self {
            Self::Json => "JSON Editor",
            Self::Xml => "XML Editor",
            Self::Sql => "SQL Editor",
            Self::Password => "Random Password",
            Self::Qr => "QR Generator",
            Self::Regex => "Regex Editor",
            Self::Pdf => "PDF Extraction",
        }
    }
    fn lang(self) -> Option<Language> {
        match self {
            Self::Json => Some(Language::Json),
            Self::Xml => Some(Language::Xml),
            Self::Sql => Some(Language::Sql),
            _ => None,
        }
    }
    fn index(self) -> usize {
        match self {
            Self::Json => 0,
            Self::Xml => 1,
            Self::Sql => 2,
            _ => 0,
        }
    }
}
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
            #[cfg(debug_assertions)]
            capture_requested: false,
        }
    }
}
fn theme(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    if !dark {
        visuals.panel_fill = Color32::WHITE;
        visuals.window_fill = Color32::from_rgb(247, 247, 248);
        visuals.extreme_bg_color = Color32::from_rgb(239, 238, 244);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(225, 224, 233);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(214, 214, 229);
        visuals.selection.bg_fill = Color32::from_rgb(222, 225, 252);
        visuals.selection.stroke.color = Color32::from_rgb(57, 76, 198);
    }
    ctx.set_visuals(visuals);
    ctx.style_mut(|s| {
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
        s.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(crate::code_editor::FONT_SIZE),
        );
        s.spacing.item_spacing = egui::vec2(10., 10.);
        s.spacing.button_padding = egui::vec2(12., 6.);
    });
}
impl DevKits {
    fn open(&mut self) {
        let lang = self.tool.lang().unwrap();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Source", &[lang.extension(), "txt"])
            .pick_file()
        {
            let result = (|| -> Result<String, String> {
                use std::io::Read;
                let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                file.take(self_devkits::MAX_INPUT as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > self_devkits::MAX_INPUT {
                    return Err("File exceeds the 2 MiB limit.".into());
                }
                String::from_utf8(bytes).map_err(|e| e.to_string())
            })();
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
    fn password_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Create passwords using your operating system's secure random source.");
        ui.add_space(12.);
        ui.add(egui::Slider::new(&mut self.password_options.length, 8..=128).text("Length"));
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.password_options.lower, "Lowercase a–z");
            ui.checkbox(&mut self.password_options.upper, "Uppercase A–Z");
            ui.checkbox(&mut self.password_options.digits, "Digits 0–9");
            ui.checkbox(&mut self.password_options.symbols, "Symbols");
        });
        ui.checkbox(
            &mut self.password_options.exclude_ambiguous,
            "Exclude ambiguous characters (I, l, 1, O, 0, o)",
        );
        ui.add_space(12.);
        ui.horizontal(|ui| {
            ui.label("Quantity");
            ui.add(egui::DragValue::new(&mut self.password_count).range(1..=1000));
        });
        if ui.button("Generate passwords").clicked() {
            match self_devkits::passwords(&self.password_options, self.password_count) {
                Ok(p) => {
                    self.secrets = p;
                    self.notice = format!(
                        "{} password(s) generated. Nothing is saved to disk.",
                        self.secrets.len()
                    );
                }
                Err(e) => {
                    self.secrets.clear();
                    self.notice = e;
                }
            }
        }
        ui.add_space(12.);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.reveal, "Show passwords");
            if ui
                .add_enabled(!self.secrets.is_empty(), egui::Button::new("Copy all"))
                .clicked()
            {
                ui.ctx().copy_text(self.secrets.join("\n"));
                self.notice = "Copied all passwords, one per line.".into();
            }
            if ui.button("Clear").clicked() {
                self.secrets.clear();
                self.notice.clear();
            }
        });
        ui.label(&self.notice);
        ui.add_space(8.);
        egui::ScrollArea::vertical()
            .id_salt("password_results")
            .show(ui, |ui| {
                let field_width = (ui.available_width() - 120.).max(100.);
                for (index, secret) in self.secrets.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_sized([40., 24.], egui::Label::new(format!("{}.", index + 1)));
                        let mut value = secret.as_str();
                        ui.add_sized(
                            [field_width, 24.],
                            egui::TextEdit::singleline(&mut value)
                                .password(!self.reveal)
                                .font(egui::TextStyle::Monospace),
                        );
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(secret.clone());
                            self.notice = format!("Copied password {}.", index + 1);
                        }
                    });
                }
            });
    }
    fn qr_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Text or URL");
        let changed = ui
            .add(
                egui::TextEdit::multiline(&mut self.qr_input)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("https://example.com"),
            )
            .changed();
        let mut level_changed = false;
        ui.horizontal(|ui| {
            ui.label("Error correction");
            egui::ComboBox::from_id_salt("qr_level")
                .selected_text(
                    ["Low · 7%", "Medium · 15%", "Quartile · 25%", "High · 30%"][self.qr_level],
                )
                .show_ui(ui, |ui| {
                    for (i, label) in ["Low · 7%", "Medium · 15%", "Quartile · 25%", "High · 30%"]
                        .into_iter()
                        .enumerate()
                    {
                        level_changed |=
                            ui.selectable_value(&mut self.qr_level, i, label).changed();
                    }
                });
        });
        if changed || level_changed {
            self.qr_texture = None;
            self.qr_image = None;
            self.notice.clear();
        }
        ui.horizontal(|ui| {
            if ui.button("Generate QR").clicked() {
                match self_devkits::qr_png(
                    &self.qr_input,
                    [
                        qrcode::EcLevel::L,
                        qrcode::EcLevel::M,
                        qrcode::EcLevel::Q,
                        qrcode::EcLevel::H,
                    ][self.qr_level],
                ) {
                    Ok(img) => {
                        let rgba = image::DynamicImage::ImageLuma8(img.clone()).into_rgba8();
                        let color = egui::ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_raw(),
                        );
                        self.qr_texture = Some(ui.ctx().load_texture(
                            "qr",
                            color,
                            egui::TextureOptions::NEAREST,
                        ));
                        self.qr_image = Some(img);
                        self.notice = "QR code ready.".into();
                    }
                    Err(e) => {
                        self.qr_texture = None;
                        self.qr_image = None;
                        self.notice = e;
                    }
                }
            }
            if ui
                .add_enabled(self.qr_image.is_some(), egui::Button::new("Save PNG…"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PNG image", &["png"])
                    .set_file_name("qrcode.png")
                    .save_file()
                {
                    self.notice = match self
                        .qr_image
                        .as_ref()
                        .unwrap()
                        .save_with_format(path, image::ImageFormat::Png)
                    {
                        Ok(()) => "QR image saved.".into(),
                        Err(e) => e.to_string(),
                    };
                }
            }
        });
        if let Some(texture) = &self.qr_texture {
            let size = ui.available_height().min(ui.available_width()).min(320.) - 45.;
            ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(size.max(80.), size.max(80.))),
            );
        }
        ui.label(&self.notice);
    }
}
impl eframe::App for DevKits {
    #[cfg(target_os = "linux")]
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        self.ime.normalize(&mut input.events);
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // Opt-in native framebuffer capture for reproducible visual QA (debug builds only).
        #[cfg(debug_assertions)]
        if let Ok(path) = std::env::var("DEVKITS_CAPTURE") {
            if !self.capture_requested {
                self.docs[0].input = json_editor::EXAMPLE.into();
                self.json_action(Action::Format);
                if std::env::var_os("DEVKITS_CAPTURE_FOLDED").is_some() {
                    self.json_collapsed.extend([1, 6, 23]);
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("regex") {
                    self.tool = Tool::Regex;
                    self.regex
                        .preview_mode(match std::env::var("DEVKITS_REGEX_MODE").as_deref() {
                            Ok("substitution") => self_devkits::regex_tools::Mode::Substitution,
                            Ok("extraction") => self_devkits::regex_tools::Mode::Extraction,
                            _ => self_devkits::regex_tools::Mode::Match,
                        });
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("xml") {
                    self.tool = Tool::Xml;
                    self.docs[1].input = xml_editor::EXAMPLE.into();
                    self.xml_action(Action::Format);
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("sql") {
                    self.tool = Tool::Sql;
                    self.docs[2].input = sql_editor::EXAMPLE.into();
                    self.sql_action(Action::Format);
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("password") {
                    self.tool = Tool::Password;
                    self.password_count = 5;
                    self.secrets =
                        self_devkits::passwords(&self.password_options, self.password_count)
                            .unwrap();
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("qr") {
                    self.tool = Tool::Qr;
                    self.qr_input = "https://example.com".into();
                    let img = self_devkits::qr_png(&self.qr_input, qrcode::EcLevel::M).unwrap();
                    let rgba = image::DynamicImage::ImageLuma8(img.clone()).into_rgba8();
                    self.qr_texture = Some(ctx.load_texture(
                        "qr",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_raw(),
                        ),
                        egui::TextureOptions::NEAREST,
                    ));
                    self.qr_image = Some(img);
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("pdf") {
                    self.tool = Tool::Pdf;
                    if let Some(path) = std::env::var_os("DEVKITS_CAPTURE_PDF") {
                        self.pdf.preview_file(std::path::Path::new(&path), ctx);
                    }
                }
                if std::env::var("DEVKITS_CAPTURE_DARK").is_ok() {
                    self.dark = true;
                    theme(ctx, true);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.capture_requested = true;
            }
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    image::save_buffer(
                        &path,
                        &pixels,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    )
                    .expect("save QA screenshot");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        self.window_title_bar(ctx);
        if self.sidebar_open {
            egui::SidePanel::left("sidebar")
                .resizable(false)
                .exact_width(218.)
                .frame(
                    egui::Frame::new()
                        .fill(if self.dark {
                            Color32::from_rgb(29, 30, 35)
                        } else {
                            Color32::from_rgb(247, 247, 248)
                        })
                        .inner_margin(8.),
                )
                .show(ctx, |ui| {
                    ui.add_space(8.);

                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text(format!("{}   Search…", icons::MAGNIFYING_GLASS))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(16.);
                    ui.spacing_mut().item_spacing.y = 3.;
                    if !self.dark {
                        ui.visuals_mut().selection.bg_fill = Color32::from_gray(225);
                        ui.visuals_mut().selection.stroke.color = Color32::from_gray(35);
                    }
                    for tool in Tool::ALL {
                        if !tool
                            .title()
                            .to_lowercase()
                            .contains(&self.search.to_lowercase())
                        {
                            continue;
                        }
                        let icon = match tool {
                            Tool::Json => icons::BRACKETS_CURLY,
                            Tool::Sql => icons::DATABASE,
                            Tool::Xml => icons::CODE,
                            Tool::Password => icons::KEY,
                            Tool::Qr => icons::QR_CODE,
                            Tool::Regex => icons::ASTERISK,
                            Tool::Pdf => icons::FILE_PDF,
                        };
                        let response = ui
                            .allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Layout::left_to_right(egui::Align::Center)
                                    .with_main_align(egui::Align::Min)
                                    .with_main_justify(true),
                                |ui| {
                                    ui.add(egui::SelectableLabel::new(
                                        self.tool == tool,
                                        format!("{icon}  {}", tool.title()),
                                    ))
                                },
                            )
                            .inner;
                        if response.clicked() {
                            self.tool = tool;
                            self.notice.clear();
                        }
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        ui.weak("v0.1.0 · Rust native");
                        ui.weak("Local tools. Your data stays here.");
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(if matches!(self.tool, Tool::Json | Tool::Sql | Tool::Xml) {
                        egui::Margin::symmetric(16, 8)
                    } else {
                        egui::Margin::same(16)
                    }),
            )
            .show(ctx, |ui| match self.tool {
                Tool::Json => self.json_ui(ui),
                Tool::Xml => self.xml_ui(ui),
                Tool::Sql => self.sql_ui(ui),
                Tool::Password => self.password_ui(ui),
                Tool::Qr => self.qr_ui(ui),
                Tool::Regex => self.regex.ui(ui),
                Tool::Pdf => self.pdf.ui(ui),
            });
        window_chrome::resize_edges(ctx);
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "DevKits",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_app_id("devkits")
                .with_inner_size(
                    if cfg!(debug_assertions) && std::env::var_os("DEVKITS_CAPTURE_SMALL").is_some()
                    {
                        [850., 580.]
                    } else {
                        [1000., 700.]
                    },
                )
                .with_min_inner_size([850., 580.]),
            ..Default::default()
        },
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Noto Sans Mono".into(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/noto-sans-mono/NotoSansMono-Regular.ttf"
                ))
                .into(),
            );
            // Keep egui's fallback fonts for glyphs outside Noto Sans Mono's coverage.
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "Noto Sans Mono".into());
            // Keep UI typography, but cover Vietnamese characters missing from Ubuntu Light.
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("Noto Sans Mono".into());
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            theme(&cc.egui_ctx, false);
            Ok(Box::new(DevKits::default()))
        }),
    )
}
