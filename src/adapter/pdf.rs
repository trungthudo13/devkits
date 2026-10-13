//! Typed Poppler/Tesseract gateway. CLI flags and output formats stay in this layer.
use super::{files::limited_text, process::run};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
type Result<T> = std::result::Result<T, String>;

pub(crate) fn page_count(source: &Path, work: &Path, cancel: &AtomicBool) -> Result<usize> {
    let output = run("pdfinfo", &[source.as_os_str()], work, cancel)?;
    limited_text(&output, 1024 * 1024)?
        .lines()
        .find_map(|line| line.strip_prefix("Pages:")?.trim().parse().ok())
        .ok_or_else(|| "Cannot determine PDF page count.".into())
}
pub(crate) fn text(
    source: &Path,
    output: &Path,
    work: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    run(
        "pdftotext",
        &[
            "-layout".as_ref(),
            "-enc".as_ref(),
            "UTF-8".as_ref(),
            source.as_os_str(),
            output.as_os_str(),
        ],
        work,
        cancel,
    )
}
pub(crate) fn layout(
    source: &Path,
    output: &Path,
    work: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    run(
        "pdftohtml",
        &[
            "-xml".as_ref(),
            "-hidden".as_ref(),
            source.as_os_str(),
            output.as_os_str(),
        ],
        work,
        cancel,
    )
}
pub(crate) fn render(
    source: &Path,
    page: usize,
    pixels: u32,
    prefix: &Path,
    work: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let (page, pixels) = (page.to_string(), pixels.to_string());
    run(
        "pdftoppm",
        &[
            "-f".as_ref(),
            page.as_ref(),
            "-l".as_ref(),
            page.as_ref(),
            "-singlefile".as_ref(),
            "-scale-to".as_ref(),
            pixels.as_ref(),
            "-png".as_ref(),
            source.as_os_str(),
            prefix.as_os_str(),
        ],
        work,
        cancel,
    )
}
pub(crate) fn extract(
    source: &Path,
    page: usize,
    prefix: &Path,
    work: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let page = page.to_string();
    run(
        "pdfimages",
        &[
            "-f".as_ref(),
            page.as_ref(),
            "-l".as_ref(),
            page.as_ref(),
            "-png".as_ref(),
            source.as_os_str(),
            prefix.as_os_str(),
        ],
        work,
        cancel,
    )
}
pub(crate) fn image_list(
    source: &Path,
    page: usize,
    work: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<(usize, String)>> {
    let number = page.to_string();
    let output = run(
        "pdfimages",
        &[
            "-f".as_ref(),
            number.as_ref(),
            "-l".as_ref(),
            number.as_ref(),
            "-list".as_ref(),
            source.as_os_str(),
        ],
        work,
        cancel,
    )?;
    Ok(limited_text(&output, 4 * 1024 * 1024)?
        .lines()
        .filter_map(|line| {
            let columns: Vec<_> = line.split_whitespace().collect();
            if columns.len() < 3 || columns[0].parse::<usize>().ok() != Some(page) {
                return None;
            }
            Some((columns[1].parse().ok()?, columns[2].to_owned()))
        })
        .collect())
}
pub(crate) fn languages(work: &Path, cancel: &AtomicBool) -> Result<Vec<String>> {
    let output = run("tesseract", &["--list-langs".as_ref()], work, cancel)?;
    Ok(limited_text(&output, 64 * 1024)?
        .lines()
        .filter(|l| !l.contains(' ') && !l.trim().is_empty())
        .map(str::to_owned)
        .collect())
}
pub(crate) fn ocr(
    source: &Path,
    output: &Path,
    language: &str,
    work: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    run(
        "tesseract",
        &[
            source.as_os_str(),
            output.as_os_str(),
            "-l".as_ref(),
            language.as_ref(),
            "--psm".as_ref(),
            "3".as_ref(),
        ],
        work,
        cancel,
    )
}
