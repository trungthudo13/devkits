use super::crypto_job::{self, Job};
use super::*;
use self_devkits::business::cryptographic::{
    self as crypto, ArgonOptions, HashAlgorithm, SopsFormat,
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Hash,
    Symmetric,
    Asymmetric,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Asymmetric {
    Rsa,
    Age,
    Sops,
}
pub(in crate::ui) struct Cryptographic {
    mode: Mode,
    algorithm: HashAlgorithm,
    asymmetric: Asymmetric,
    decrypt: bool,
    sops_format: SopsFormat,
    options: ArgonOptions,
    input: String,
    key: String,
    phc: String,
    output: String,
    message: String,
    error: bool,
    reveal: bool,
    job: Job<String>,
}
impl Default for Cryptographic {
    fn default() -> Self {
        Self {
            mode: Mode::Hash,
            algorithm: HashAlgorithm::Sha256,
            asymmetric: Asymmetric::Rsa,
            decrypt: false,
            sops_format: SopsFormat::Yaml,
            options: ArgonOptions::default(),
            input: String::new(),
            key: String::new(),
            phc: String::new(),
            output: String::new(),
            message: String::new(),
            error: false,
            reveal: false,
            job: Job::default(),
        }
    }
}
impl Drop for Cryptographic {
    fn drop(&mut self) {
        self.input.zeroize();
        self.key.zeroize();
        self.output.zeroize();
    }
}
impl Cryptographic {
    pub(in crate::ui) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(!self.job.busy(), |ui| {
            let previous = self.mode;
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.mode, Mode::Hash, "Hashing");
                ui.selectable_value(&mut self.mode, Mode::Symmetric, "Symmetric");
                ui.selectable_value(&mut self.mode, Mode::Asymmetric, "Asymmetric");
            });
            if previous != self.mode {
                self.output.zeroize();
                self.key.zeroize();
                self.message.clear();
                self.reveal = false;
            }
        });
    }

    pub(in crate::ui) fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = self.job.poll(ui.ctx()) {
            match result {
                Ok(output) => {
                    self.output = output;
                    self.error = false;
                    self.message = "Done.".into();
                }
                Err(error) => {
                    self.output.zeroize();
                    self.error = true;
                    self.message = error;
                }
            }
        }
        egui::ScrollArea::vertical().id_salt("cryptographic_scroll").show(ui, |ui| {
            ui.add_enabled_ui(!self.job.busy(), |ui| {
                let previous = (self.mode, self.algorithm, self.asymmetric, self.decrypt);
                match self.mode {
                    Mode::Hash => {
                        ui.horizontal_wrapped(|ui| {
                            for (algorithm, name) in [(HashAlgorithm::Md5, "MD5"), (HashAlgorithm::Sha256, "SHA-256"), (HashAlgorithm::Sha512, "SHA-512"), (HashAlgorithm::Argon2id, "Argon2id")] {
                                ui.selectable_value(&mut self.algorithm, algorithm, name);
                            }
                        });
                        if self.algorithm == HashAlgorithm::Md5 { ui.label("MD5 is for legacy checksums, not password storage or security."); }
                        if self.algorithm == HashAlgorithm::Argon2id {
                            ui.label("Password hashing · random salt · PHC output includes salt and parameters.");
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Memory (KiB)"); ui.add(egui::DragValue::new(&mut self.options.memory_kib).range(8..=262144));
                                ui.label("Iterations"); ui.add(egui::DragValue::new(&mut self.options.iterations).range(1..=10));
                                ui.label("Parallelism"); ui.add(egui::DragValue::new(&mut self.options.parallelism).range(1..=8));
                            });
                        } else { ui.label("Hash exact UTF-8 text, including whitespace and line endings."); }
                    }
                    Mode::Symmetric => { ui.label("AES-256-GCM · authenticated encryption · a fresh nonce for every message."); }
                    Mode::Asymmetric => {
                        ui.horizontal_wrapped(|ui| {
                            ui.selectable_value(&mut self.asymmetric, Asymmetric::Rsa, "RSA-OAEP-SHA256");
                            ui.selectable_value(&mut self.asymmetric, Asymmetric::Age, "age (raw)");
                            if ui.selectable_value(&mut self.asymmetric, Asymmetric::Sops, "SOPS + age").clicked() { self.decrypt = true; }
                        });
                        ui.label(match self.asymmetric {
                            Asymmetric::Rsa => "RSA: short messages only (190 UTF-8 bytes with a 2048-bit key).",
                            Asymmetric::Age => "Raw age armored messages. For SOPS-encrypted documents, select SOPS + age.",
                            Asymmetric::Sops => "Paste the whole document including sops metadata. Requires SOPS CLI 3.13+.",
                        });
                        if self.asymmetric == Asymmetric::Sops {
                            let previous_format = self.sops_format;
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Document format");
                                for (format, label) in [(SopsFormat::Yaml, "YAML"), (SopsFormat::Json, "JSON"), (SopsFormat::Dotenv, ".env"), (SopsFormat::Ini, "INI")] {
                                    ui.selectable_value(&mut self.sops_format, format, label);
                                }
                            });
                            if previous_format != self.sops_format { self.output.zeroize(); self.message.clear(); }
                        }
                    }
                }
                if self.mode != Mode::Hash {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.decrypt, false, "Encrypt");
                        ui.selectable_value(&mut self.decrypt, true, "Decrypt");
                    });
                }
                if previous != (self.mode, self.algorithm, self.asymmetric, self.decrypt) {
                    self.output.zeroize(); self.message.clear(); self.reveal = false;
                    if previous.0 != self.mode || previous.2 != self.asymmetric || (previous.3 != self.decrypt && self.mode == Mode::Asymmetric) { self.key.zeroize(); }
                }
                ui.add_space(8.);
                ui.label(if self.mode == Mode::Hash { "Input text / password" } else if self.decrypt { "Ciphertext" } else { "Plaintext (UTF-8)" });
                if self.mode == Mode::Hash && self.algorithm == HashAlgorithm::Argon2id {
                    ui.add(egui::TextEdit::singleline(&mut self.input).password(!self.reveal).desired_width(f32::INFINITY));
                    ui.checkbox(&mut self.reveal, "Show password");
                } else {
                    ui.add(egui::TextEdit::multiline(&mut self.input).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(5));
                }
                if self.mode != Mode::Hash {
                    ui.add_space(8.);
                    let label = match (self.mode, self.asymmetric, self.decrypt) {
                        (Mode::Symmetric, _, _) => "Secret key (32 bytes, Base64)",
                        (_, Asymmetric::Rsa, false) => "Public key (SPKI PEM)",
                        (_, Asymmetric::Rsa, true) => "Private key (PKCS#8 PEM, unencrypted)",
                        (_, Asymmetric::Age, false) => "Recipient (age1…)",
                        (_, Asymmetric::Age, true) => "Identity (AGE-SECRET-KEY-1…)",
                        (_, Asymmetric::Sops, false) => "age recipients (age1…), one per line",
                        (_, Asymmetric::Sops, true) => "age identities (AGE-SECRET-KEY-1…), or paste the contents of keys.txt",
                    };
                    ui.label(label);
                    let secret = self.mode == Mode::Symmetric || self.decrypt;
                    if secret { ui.checkbox(&mut self.reveal, "Show secret key"); }
                    if secret && !self.reveal {
                        ui.add(egui::TextEdit::singleline(&mut self.key).password(true).desired_width(f32::INFINITY));
                    } else {
                        ui.add(egui::TextEdit::multiline(&mut self.key).font(egui::TextStyle::Monospace).desired_rows(3).desired_width(f32::INFINITY));
                    }
                    if self.mode == Mode::Symmetric {
                        ui.horizontal(|ui| {
                            if ui.button("Generate AES key").clicked() {
                                match business::keygen::aes_key() {
                                    Ok(key) => { self.key.zeroize(); self.key = key; self.output.zeroize(); self.message.clear(); }
                                    Err(e) => { self.message = e; self.error = true; }
                                }
                            }
                            if ui.add_enabled(!self.key.is_empty(), egui::Button::new("Copy key")).clicked() { ui.ctx().copy_text(self.key.clone()); }
                        });
                        ui.label("Output: devkits:aes256gcm:v1: + Base64(nonce | ciphertext | tag).");
                    }
                }
                let mut verify = false;
                if self.mode == Mode::Hash && self.algorithm == HashAlgorithm::Argon2id {
                    ui.label("Existing PHC hash (for verification)");
                    ui.add(egui::TextEdit::singleline(&mut self.phc).desired_width(f32::INFINITY));
                }
                let mut run = false;
                ui.horizontal_wrapped(|ui| {
                    run = ui.button(if self.mode == Mode::Hash { "Hash" } else if self.decrypt { "Decrypt" } else { "Encrypt" }).clicked();
                    if self.mode == Mode::Hash && self.algorithm == HashAlgorithm::Argon2id {
                        verify = ui.add_enabled(!self.phc.is_empty(), egui::Button::new("Verify password")).clicked();
                    }
                    if ui.button("Clear").clicked() {
                        self.input.zeroize(); self.key.zeroize(); self.output.zeroize(); self.phc.clear(); self.message.clear(); self.reveal = false;
                    }
                });
                if run || verify {
                    self.output.zeroize(); self.message.clear();
                    let input = Zeroizing::new(self.input.clone());
                    let key = Zeroizing::new(self.key.clone());
                    let phc = self.phc.clone();
                    let sops_format = self.sops_format;
                    let (mode, algorithm, asymmetric, decrypt, options) = (self.mode, self.algorithm, self.asymmetric, self.decrypt, self.options);
                    self.job.start(move || {
                        if verify {
                            return crypto::verify_argon2id(&input, &phc).map(|matches| if matches { "Password matches.".into() } else { "Password does not match.".into() });
                        }
                        match (mode, asymmetric, decrypt) {
                            (Mode::Hash, _, _) => crypto::hash(algorithm, &input, options),
                            (Mode::Symmetric, _, false) => crypto::aes_encrypt(&input, &key),
                            (Mode::Symmetric, _, true) => crypto::aes_decrypt(&input, &key),
                            (Mode::Asymmetric, Asymmetric::Rsa, false) => crypto::rsa_encrypt(&input, &key),
                            (Mode::Asymmetric, Asymmetric::Rsa, true) => crypto::rsa_decrypt(&input, &key),
                            (Mode::Asymmetric, Asymmetric::Age, false) => crypto::age_encrypt(&input, &key),
                            (Mode::Asymmetric, Asymmetric::Age, true) => crypto::age_decrypt(&input, &key),
                            (Mode::Asymmetric, Asymmetric::Sops, decrypt) => crypto::sops_transform(&input, &key, sops_format, decrypt),
                        }
                    });
                    ui.ctx().request_repaint();
                }
            });
            if self.job.busy() { ui.horizontal(|ui| { ui.spinner(); ui.label("Processing locally…"); }); }
            design::notice(ui, if self.error { design::colors(ui).danger } else { design::colors(ui).success }, &self.message);
            ui.add_space(8.);
            crypto_job::output(ui, "Result", &self.output, 6);
            if self.mode != Mode::Hash && ui.add_enabled(!self.output.is_empty() && !self.job.busy(), egui::Button::new("Use result as input")).clicked() {
                self.input.zeroize();
                self.input = std::mem::take(&mut self.output);
                self.decrypt = !self.decrypt;
                if self.mode == Mode::Asymmetric { self.key.zeroize(); }
                self.message.clear();
                self.reveal = false;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    // Struct update syntax cannot move fields from a type with secret-clearing Drop.
    #![allow(clippy::field_reassign_with_default)]
    use super::*;

    fn frame(
        tool: &mut Cryptographic,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(650., 1100.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| tool.ui(ui));
            },
        )
    }
    fn click(tool: &mut Cryptographic, ctx: &egui::Context, label: &str) {
        let output = frame(tool, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .rev()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.pos + text.galley.size() / 2.)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing button: {label}"));
        for pressed in [true, false] {
            frame(
                tool,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    fn finish(tool: &mut Cryptographic, ctx: &egui::Context) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while tool.job.busy() {
            assert!(
                std::time::Instant::now() < deadline,
                "Background job did not finish"
            );
            frame(tool, ctx, vec![]);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!tool.error, "{}", tool.message);
    }
    #[test]
    fn hash_button_runs_background_work_and_clear_removes_results() {
        let ctx = egui::Context::default();
        let mut tool = Cryptographic::default();
        tool.input = "abc".into();
        click(&mut tool, &ctx, "Hash");
        finish(&mut tool, &ctx);
        assert_eq!(
            tool.output,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        click(&mut tool, &ctx, "Clear");
        assert!(tool.input.is_empty() && tool.output.is_empty());
    }
    #[test]
    fn symmetric_result_can_be_decrypted_without_losing_the_key() {
        let ctx = egui::Context::default();
        let mut tool = Cryptographic::default();
        tool.mode = Mode::Symmetric;
        tool.input = "Xin chào 🔐".into();
        tool.key = business::keygen::aes_key().unwrap();
        let key = tool.key.clone();
        click(&mut tool, &ctx, "Encrypt");
        finish(&mut tool, &ctx);
        assert!(tool.output.starts_with("devkits:aes256gcm:v1:"));
        click(&mut tool, &ctx, "Use result as input");
        assert!(tool.decrypt);
        assert_eq!(tool.key, key);
        click(&mut tool, &ctx, "Decrypt");
        finish(&mut tool, &ctx);
        assert_eq!(tool.output, "Xin chào 🔐");
    }
}
