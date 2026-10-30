use super::{crypto_job, *};
use self_devkits::business::base64;
use zeroize::Zeroize;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Encode,
    Decode,
    Key,
}
pub(in crate::ui) struct Base64 {
    mode: Mode,
    input: String,
    output: String,
    message: String,
    error: bool,
    url_safe: bool,
    bytes: usize,
    reveal: bool,
}
impl Default for Base64 {
    fn default() -> Self {
        Self {
            mode: Mode::Key,
            input: String::new(),
            output: String::new(),
            message: String::new(),
            error: false,
            url_safe: false,
            bytes: 32,
            reveal: false,
        }
    }
}
impl Drop for Base64 {
    fn drop(&mut self) {
        self.input.zeroize();
        self.output.zeroize();
    }
}
impl Base64 {
    fn reset_output(&mut self) {
        self.output.zeroize();
        self.message.clear();
        self.reveal = false;
    }
    pub(in crate::ui) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let previous = self.mode;
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.mode, Mode::Encode, "Encode");
            ui.selectable_value(&mut self.mode, Mode::Decode, "Decode");
            ui.selectable_value(&mut self.mode, Mode::Key, "Generate key");
        });
        if previous != self.mode {
            self.reset_output();
        }
    }
    pub(in crate::ui) fn ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("base64_scroll").show(ui, |ui| {
            if self.mode == Mode::Key {
                ui.label("ENCRYPTION_KEY_BASE64");
                ui.label("Generate random key bytes from the operating system, then encode as standard Base64.");
                let previous = self.bytes;
                ui.horizontal_wrapped(|ui| {
                    ui.label("Key length");
                    for count in [16, 24, 32, 64] { ui.selectable_value(&mut self.bytes, count, format!("{count} bytes")); }
                });
                if previous != self.bytes { self.reset_output(); }
                ui.label("Default: 32 bytes (256 bits), suitable for AES-256. Match the size required by your application.");
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    if ui.button("Generate random key").clicked() {
                        self.reset_output();
                        match base64::encryption_key(self.bytes) {
                            Ok(key) => { self.output = key; self.error = false; self.message = format!("Generated {} random bytes → {} Base64 characters.", self.bytes, self.output.len()); }
                            Err(error) => { self.message = error; self.error = true; }
                        }
                    }
                    if ui.button("Clear").clicked() { self.reset_output(); }
                });
                if !self.output.is_empty() {
                    ui.add_space(8.);
                    ui.checkbox(&mut self.reveal, "Show key");
                    let mut key = self.output.as_str();
                    ui.add(egui::TextEdit::singleline(&mut key).password(!self.reveal).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Copy value").clicked() { ui.ctx().copy_text(self.output.clone()); }
                        if ui.button("Copy ENCRYPTION_KEY_BASE64=…").clicked() { ui.ctx().copy_text(format!("ENCRYPTION_KEY_BASE64={}", self.output)); }
                    });
                }
            } else {
                ui.label("Base64 encodes data; it does not encrypt it.");
                if ui.checkbox(&mut self.url_safe, "URL-safe alphabet (- and _)").changed() { self.reset_output(); }
                ui.label(if self.mode == Mode::Encode { "Input text (UTF-8)" } else { "Base64 input" });
                if ui.add(egui::TextEdit::multiline(&mut self.input).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(6)).changed() { self.reset_output(); }
                ui.horizontal(|ui| {
                    if ui.button(if self.mode == Mode::Encode { "Encode text" } else { "Decode Base64" }).clicked() {
                        self.reset_output();
                        let result = if self.mode == Mode::Encode {
                            base64::encode(&self.input, self.url_safe).map(|output| (output, "Encoded UTF-8 text.".into()))
                        } else {
                            base64::decode(&self.input, self.url_safe).map(|mut decoded| {
                                let message = format!("{} decoded bytes · {}", decoded.byte_count, if decoded.is_utf8 { "UTF-8 text" } else { "binary data shown as hexadecimal" });
                                (std::mem::take(&mut decoded.value), message)
                            })
                        };
                        match result {
                            Ok((output, message)) => { self.output = output; self.message = message; self.error = false; }
                            Err(error) => { self.message = error; self.error = true; }
                        }
                    }
                    if ui.button("Clear").clicked() { self.input.zeroize(); self.reset_output(); }
                });
                ui.add_space(8.);
                crypto_job::output(ui, "Result", &self.output, 6);
            }
            design::notice(ui, if self.error { design::colors(ui).danger } else { design::colors(ui).success }, &self.message);
        });
    }
}
