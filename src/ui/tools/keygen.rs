use super::crypto_job::{self, Job};
use super::*;
use self_devkits::business::keygen::{self, KeyKind, KeyPair};

pub(in crate::ui) struct Keygen {
    kind: KeyKind,
    keys: Option<KeyPair>,
    reveal: bool,
    error: String,
    job: Job<KeyPair>,
}
impl Default for Keygen {
    fn default() -> Self {
        Self {
            kind: KeyKind::Rsa(3072),
            keys: None,
            reveal: false,
            error: String::new(),
            job: Job::default(),
        }
    }
}
impl Keygen {
    pub(in crate::ui) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(!self.job.busy(), |ui| {
            let previous = self.kind;
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(matches!(self.kind, KeyKind::Rsa(_)), "RSA")
                    .clicked()
                    && !matches!(self.kind, KeyKind::Rsa(_))
                {
                    self.kind = KeyKind::Rsa(3072);
                }
                ui.selectable_value(&mut self.kind, KeyKind::Age, "age (X25519)");
                ui.selectable_value(&mut self.kind, KeyKind::Aes256, "AES-256");
            });
            if previous != self.kind {
                self.keys = None;
                self.error.clear();
                self.reveal = false;
            }
        });
    }

    pub(in crate::ui) fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = self.job.poll(ui.ctx()) {
            match result {
                Ok(keys) => {
                    self.keys = Some(keys);
                    self.error.clear();
                }
                Err(error) => {
                    self.keys = None;
                    self.error = error;
                }
            }
        }
        egui::ScrollArea::vertical().id_salt("keygen_scroll").show(ui, |ui| {
            ui.add_enabled_ui(!self.job.busy(), |ui| {
                let previous = self.kind;
                if let KeyKind::Rsa(ref mut bits) = self.kind {
                    ui.horizontal(|ui| {
                        ui.label("Key size");
                        for size in [2048, 3072, 4096] { ui.selectable_value(bits, size, format!("{size} bits")); }
                    });
                }
                if previous != self.kind { self.keys = None; self.error.clear(); self.reveal = false; }
                ui.label(match self.kind {
                    KeyKind::Rsa(_) => "Public: SPKI PEM · Private: unencrypted PKCS#8 PEM",
                    KeyKind::Age => "Public recipient: age1… · Private identity: AGE-SECRET-KEY-1…",
                    KeyKind::Aes256 => "32 random bytes, Base64 encoded · use in Cryptographic → Symmetric",
                });
                ui.label("Generated locally. Copy keys to keep them; they are not saved automatically.");
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    if ui.button("Generate keys").clicked() {
                        self.keys = None; self.error.clear(); self.reveal = false;
                        let kind = self.kind;
                        self.job.start(move || keygen::generate(kind));
                        ui.ctx().request_repaint();
                    }
                    if ui.button("Clear").clicked() { self.keys = None; self.error.clear(); self.reveal = false; }
                });
            });
            if self.job.busy() { ui.horizontal(|ui| { ui.spinner(); ui.label("Generating keys locally…"); }); }
            design::notice(ui, design::colors(ui).danger, &self.error);
            if let Some(keys) = &self.keys {
                ui.add_space(12.);
                if !keys.public.is_empty() {
                    crypto_job::output(ui, "Public key / recipient", &keys.public, 5);
                    ui.add_space(12.);
                }
                ui.horizontal(|ui| {
                    ui.label("Private / secret key");
                    ui.checkbox(&mut self.reveal, "Show");
                    if ui.button("Copy private key").clicked() { ui.ctx().copy_text(keys.private.clone()); }
                });
                if self.reveal {
                    let mut text = keys.private.as_str();
                    ui.add(egui::TextEdit::multiline(&mut text).font(egui::TextStyle::Monospace).desired_rows(8).desired_width(f32::INFINITY));
                } else { ui.label("••••••••••••••••••••••••"); }
            }
        });
    }
}
