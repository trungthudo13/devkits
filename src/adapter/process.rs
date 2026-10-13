use super::files::limited_text;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
type PdfResult<T> = Result<T, String>;
pub(crate) fn run(
    tool: &str,
    args: &[&std::ffi::OsStr],
    work: &Path,
    cancel: &AtomicBool,
) -> PdfResult<PathBuf> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled.".into());
    }
    let stdout_path = work.join("stdout.log");
    let stderr_path = work.join("stderr.log");
    #[cfg(target_os = "macos")]
    let resolved = {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut locations: Vec<_> = std::env::split_paths(&path).collect();
        locations.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ]);
        locations
            .into_iter()
            .map(|p| p.join(tool))
            .find(|p| p.is_file())
            .unwrap_or_else(|| tool.into())
    };
    #[cfg(not(target_os = "macos"))]
    let resolved = tool;
    let mut command = Command::new(resolved);
    command
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout_path).map_err(|e| e.to_string())?)
        .stderr(fs::File::create(&stderr_path).map_err(|e| e.to_string())?);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound && tool == "tesseract" {
            "Tesseract is not installed. Ubuntu: sudo apt install tesseract-ocr tesseract-ocr-vie; macOS: brew install tesseract tesseract-lang; Windows: install Tesseract and add it to PATH.".into()
        } else if e.kind() == std::io::ErrorKind::NotFound {
            format!("Missing {tool}. Install Poppler: Ubuntu: sudo apt install poppler-utils; macOS: brew install poppler; Windows: add Poppler's bin folder to PATH.")
        } else { format!("Cannot start {tool}: {e}") }
    })?;
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancel.load(Ordering::Relaxed) {
                "Cancelled."
            } else {
                "PDF operation timed out after 120 seconds."
            }
            .into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(stdout_path);
                }
                let detail = limited_text(&stderr_path, 32 * 1024).unwrap_or_default();
                return Err(format!("{tool} failed: {}", detail.trim()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    }
}
