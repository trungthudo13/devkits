use super::tools::{json_editor, sql_editor, xml_editor};
use super::*;
impl DevKits {
    pub(super) fn capture_preview(&mut self, ctx: &egui::Context) {
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
                            Ok("substitution") => self_devkits::business::regex::Mode::Substitution,
                            Ok("extraction") => self_devkits::business::regex::Mode::Extraction,
                            _ => self_devkits::business::regex::Mode::Match,
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
                        business::passwords(&self.password_options, self.password_count).unwrap();
                }
                if std::env::var("DEVKITS_CAPTURE_TOOL").as_deref() == Ok("qr") {
                    self.tool = Tool::Qr;
                    self.qr_input = "https://example.com".into();
                    let img = business::qr_png(&self.qr_input, qrcode::EcLevel::M).unwrap();
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
    }
}
