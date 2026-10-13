//! Native picker and clipboard integration, with no domain rules.
use std::path::PathBuf;
pub fn open_source(extension: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Source", &[extension, "txt"])
        .pick_file()
}
pub fn open_pdf() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("PDF", &["pdf"])
        .pick_file()
}
pub fn folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}
pub fn save_file(name: &str, extension: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(extension, &[extension])
        .set_file_name(name)
        .save_file()
}
pub fn clipboard_text() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .map_err(|e| e.to_string())
}
