//! Filesystem boundary, including bounded imports and UTF-8 validation.
pub use std::fs::{copy, read_dir, remove_file, rename, write};
use std::{fs, io::Read, path::Path};
type PdfResult<T> = Result<T, String>;
pub fn limited_text(path: &Path, limit: u64) -> PdfResult<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("File exceeds the {limit} byte limit."));
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

pub fn snapshot(source: &Path, destination: &Path, limit: u64) -> Result<u64, String> {
    let mut input = fs::File::open(source)
        .map_err(|e| e.to_string())?
        .take(limit + 1);
    let copied = std::io::copy(
        &mut input,
        &mut fs::File::create(destination).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if copied > limit {
        return Err(format!("File exceeds the {limit} byte limit."));
    }
    Ok(copied)
}
