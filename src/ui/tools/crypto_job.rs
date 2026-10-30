//! A single bounded background operation per tool, polled without blocking UI.
use eframe::egui;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(super) struct Job<T> {
    receiver: Option<Receiver<Result<T, String>>>,
}
impl<T> Default for Job<T> {
    fn default() -> Self {
        Self { receiver: None }
    }
}
impl<T: Send + 'static> Job<T> {
    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }
    pub fn start(&mut self, work: impl FnOnce() -> Result<T, String> + Send + 'static) {
        if self.busy() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        if let Err(error) = std::thread::Builder::new()
            .name("devkits-crypto".into())
            .spawn(move || {
                let _ = sender.send(work());
            })
        {
            let (sender, receiver) = mpsc::channel();
            let _ = sender.send(Err(format!("Could not start background task: {error}")));
            self.receiver = Some(receiver);
        }
    }
    pub fn poll(&mut self, ctx: &egui::Context) -> Option<Result<T, String>> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                Some(result)
            }
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                Some(Err("Background operation failed. Please retry.".into()))
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                None
            }
        }
    }
}

pub(super) fn output(ui: &mut egui::Ui, label: &str, value: &str, rows: usize) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui
            .add_enabled(!value.is_empty(), egui::Button::new("Copy"))
            .clicked()
        {
            ui.ctx().copy_text(value.to_owned());
        }
    });
    let mut text = value;
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(rows),
    );
}
