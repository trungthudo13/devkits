//! PDF use cases: import, extraction, review, naming, cropping and OCR orchestration.
mod captions;
mod regions;
use crate::adapter::{
    files as fs, files::limited_text, images::decode_image, pdf as backend, workspace::Workspace,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
pub type PdfResult<T> = Result<T, String>;
const MAX_PDF: u64 = 100 * 1024 * 1024;
const MAX_TEXT: u64 = 16 * 1024 * 1024;
#[derive(Debug)]
pub struct PdfDocument {
    workspace: Workspace,
    pub name: String,
    pub pages: usize,
    pub bytes: u64,
}
impl PdfDocument {
    fn source(&self) -> PathBuf {
        self.workspace.0.join("source.pdf")
    }
}

pub fn import(path: &Path, cancel: &AtomicBool) -> PdfResult<Arc<PdfDocument>> {
    let workspace = Workspace::temporary("self-devkits-pdf")?;
    let source = workspace.0.join("source.pdf");
    let bytes = fs::snapshot(path, &source, MAX_PDF)?;
    let pages = backend::page_count(&source, &workspace.0, cancel)?;
    if !(1..=2000).contains(&pages) {
        return Err("Supported PDF size: 1–2,000 pages.".into());
    }
    Ok(Arc::new(PdfDocument {
        workspace,
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        pages,
        bytes,
    }))
}

pub fn read_text(doc: &PdfDocument, cancel: &AtomicBool) -> PdfResult<String> {
    let output = doc.workspace.0.join("text.txt");
    backend::text(&doc.source(), &output, &doc.workspace.0, cancel)?;
    // Poppler inserts form-feed page separators; render them as blank lines
    // instead of an unsupported control-character glyph in the text viewer.
    limited_text(&output, MAX_TEXT).map(|text| text.replace('\u{000c}', "\n\n"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageMode {
    Pages,
    Embedded,
}

pub fn export_images(
    doc: &PdfDocument,
    parent: &Path,
    mode: ImageMode,
    pixels: u32,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> PdfResult<(PathBuf, usize)> {
    if !(800..=4000).contains(&pixels) {
        return Err("Image size must be 800–4,000 pixels.".into());
    }
    let mut destination = Workspace::new(
        parent,
        if mode == ImageMode::Pages {
            "pdf-pages"
        } else {
            "pdf-images"
        },
    )?;
    let mut entries = Vec::new();
    let mut caption_warning = None;
    let layout_workspace = Workspace::temporary("self-devkits-pdf-layout")?;
    let captions = if mode == ImageMode::Embedded {
        let xml = layout_workspace.0.join("layout.xml");
        let result = backend::layout(&doc.source(), &xml, &layout_workspace.0, cancel)
            .and_then(|_| limited_text(&xml, 32 * 1024 * 1024))
            .and_then(|xml| captions::captions(&xml));
        match result {
            Ok(captions) => captions,
            Err(e) if cancel.load(Ordering::Relaxed) => return Err(e),
            Err(e) => {
                caption_warning = Some(e);
                Default::default()
            }
        }
    } else {
        Default::default()
    };
    for page in 1..=doc.pages {
        let prefix = destination.0.join(format!("page-{page:04}"));
        let source = doc.source();
        if mode == ImageMode::Pages {
            backend::render(&source, page, pixels, &prefix, &doc.workspace.0, cancel)?;
        } else {
            backend::extract(&source, page, &prefix, &doc.workspace.0, cancel)?;
        }
        if mode == ImageMode::Embedded {
            let rows = backend::image_list(&source, page, &doc.workspace.0, cancel)?;
            let names = captions.get(&page);
            let matched = names.is_some_and(|n| {
                n.len() == rows.iter().filter(|(_, kind)| kind == "image").count()
            });
            let mut image_index = 0;
            for (index, kind) in rows {
                let original = destination.0.join(format!("page-{page:04}-{index:03}.png"));
                if !original.is_file() {
                    continue;
                }
                let caption = if kind == "image" {
                    let name = if matched {
                        names.and_then(|n| n.get(image_index)).cloned().flatten()
                    } else {
                        None
                    };
                    image_index += 1;
                    name
                } else {
                    None
                };
                let stem = caption
                    .as_deref()
                    .map(captions::slug)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("page_{page:04}_{kind}_{:03}", index + 1));
                let mut filename = format!("{stem}.png");
                let mut suffix = 2;
                while destination.0.join(&filename).exists() {
                    filename = format!("{stem}_{suffix}.png");
                    suffix += 1;
                }
                fs::rename(&original, destination.0.join(&filename)).map_err(|e| e.to_string())?;
                entries.push(serde_json::json!({"file": filename, "page": page, "type": kind,
                    "caption": caption, "naming": if caption.is_some() { "nearby_caption" } else { "numbered_fallback" }}));
            }
        }
        progress(page, doc.pages);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled.".into());
    }
    let count = fs::read_dir(&destination.0)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .iter()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
        .count();
    if count == 0 {
        return Err(
            "No embedded images found. Use Pages to export the rendered pages instead.".into(),
        );
    }
    if mode == ImageMode::Embedded {
        let manifest = serde_json::json!({ "source": doc.name, "warning": caption_warning,
            "note": "Names inferred from nearby text. Ambiguous captions and masks use numbered names. Embedded collages remain a single image; no AI recognition or cropping is performed.", "images": entries });
        fs::write(
            destination.0.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    // Successful output belongs to the user; failures/cancellation remove only our new folder.
    let path = std::mem::take(&mut destination.0);
    Ok((path, count))
}

#[cfg(test)]
mod tests;

/// Images staged for review; dropped automatically when the review is replaced.
#[derive(Debug)]
pub struct PreparedImages {
    _workspace: Workspace,
    pub items: Vec<ReviewedImage>,
    pub warning: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReviewedImage {
    pub source: PathBuf,
    pub page: usize,
    pub name: String,
    pub caption: Option<String>,
    /// Pixel coordinates in the staged source image: x, y, width, height.
    pub crop: Option<[u32; 4]>,
}

pub fn prepare_images(
    doc: &PdfDocument,
    cancel: &AtomicBool,
    progress: impl FnMut(usize, usize),
) -> PdfResult<PreparedImages> {
    let workspace = Workspace::temporary("self-devkits-review")?;
    let (folder, _) = export_images(
        doc,
        &workspace.0,
        ImageMode::Embedded,
        2400,
        cancel,
        progress,
    )?;
    let manifest: serde_json::Value =
        serde_json::from_str(&limited_text(&folder.join("manifest.json"), MAX_TEXT)?)
            .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for entry in manifest["images"]
        .as_array()
        .ok_or("Missing image manifest")?
    {
        let file = entry["file"].as_str().ok_or("Invalid image filename")?;
        let source = folder.join(file);
        items.push(ReviewedImage {
            source,
            page: entry["page"].as_u64().unwrap_or(1) as usize,
            name: file.strip_suffix(".png").unwrap_or(file).into(),
            caption: entry["caption"].as_str().map(str::to_owned),
            crop: None,
        });
    }
    if items.is_empty() {
        return Err(
            "No images available for review. Use Page regions for scanned or vector PDFs.".into(),
        );
    }
    Ok(PreparedImages {
        _workspace: workspace,
        items,
        warning: manifest["warning"].as_str().map(str::to_owned),
    })
}

fn crop_image(
    image: image::DynamicImage,
    crop: Option<[u32; 4]>,
) -> PdfResult<image::DynamicImage> {
    if let Some([x, y, w, h]) = crop {
        if w == 0
            || h == 0
            || x.checked_add(w).is_none_or(|n| n > image.width())
            || y.checked_add(h).is_none_or(|n| n > image.height())
        {
            return Err("The selected region is outside the source image.".into());
        }
        Ok(image.crop_imm(x, y, w, h))
    } else {
        Ok(image)
    }
}

pub fn image_preview(item: &ReviewedImage) -> PdfResult<image::RgbaImage> {
    Ok(crop_image(decode_image(&item.source)?, item.crop)?
        .thumbnail(1200, 1200)
        .to_rgba8())
}

#[derive(Debug)]
pub struct PageImage {
    pub source: PathBuf,
    pub page: usize,
    pub width: u32,
    pub height: u32,
}

pub fn render_page(
    doc: &PdfDocument,
    page: usize,
    pixels: u32,
    cancel: &AtomicBool,
) -> PdfResult<PageImage> {
    if page == 0 || page > doc.pages || !(800..=4000).contains(&pixels) {
        return Err("Invalid page number or image size.".into());
    }
    let prefix = doc.workspace.0.join(format!("preview-{page}-{pixels}"));
    let source = prefix.with_extension("png");
    if !source.exists() {
        let result = backend::render(
            &doc.source(),
            page,
            pixels,
            &prefix,
            &doc.workspace.0,
            cancel,
        );
        if let Err(e) = result {
            let _ = fs::remove_file(&source);
            return Err(e);
        }
    }
    let (width, height) = crate::adapter::images::dimensions(&source)?;
    Ok(PageImage {
        source,
        page,
        width,
        height,
    })
}

/// Propose crops on a rendered page; callers must let the user review them.
pub fn suggest_regions(page: &PageImage, cancel: &AtomicBool) -> PdfResult<Vec<[u32; 4]>> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled.".into());
    }
    regions::detect(&decode_image(&page.source)?.to_rgb8(), cancel)
}

pub fn export_reviewed(
    items: &[ReviewedImage],
    parent: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> PdfResult<(PathBuf, usize)> {
    if items.is_empty() {
        return Err("Select at least one image to export.".into());
    }
    let mut destination = Workspace::new(parent, "pdf-selected")?;
    let mut manifest = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled.".into());
        }
        let stem = captions::slug(&item.name);
        if stem.is_empty() {
            return Err(format!("Image {} needs a filename.", index + 1));
        }
        let mut file = format!("{stem}.png");
        let mut suffix = 2;
        while destination.0.join(&file).exists() {
            file = format!("{stem}_{suffix}.png");
            suffix += 1;
        }
        let target = destination.0.join(&file);
        if item.crop.is_some() {
            crate::adapter::images::save_png(
                &crop_image(decode_image(&item.source)?, item.crop)?,
                &target,
            )?;
        } else {
            fs::copy(&item.source, &target).map_err(|e| e.to_string())?;
        }
        manifest.push(serde_json::json!({"file":file, "page":item.page, "caption":item.caption, "crop_pixels":item.crop, "name_from_review":true}));
        progress(index + 1, items.len());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled.".into());
    }
    fs::write(
        destination.0.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok((std::mem::take(&mut destination.0), items.len()))
}

pub fn ocr_languages(cancel: &AtomicBool) -> PdfResult<Vec<String>> {
    let workspace = Workspace::temporary("self-devkits-ocr")?;
    let languages = backend::languages(&workspace.0, cancel)?;
    if languages.is_empty() {
        return Err(
            "No OCR language data found. Install tesseract-ocr-eng and/or tesseract-ocr-vie."
                .into(),
        );
    }
    Ok(languages)
}

pub fn ocr_text(
    doc: &PdfDocument,
    first: usize,
    last: usize,
    language: &str,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> PdfResult<String> {
    if first == 0 || first > last || last > doc.pages {
        return Err("Invalid OCR page range.".into());
    }
    let installed = ocr_languages(cancel)?;
    if language
        .split('+')
        .any(|lang| !installed.iter().any(|l| l == lang))
    {
        return Err(format!(
            "OCR language unavailable. Installed: {}",
            installed.join(", ")
        ));
    }
    let mut result = String::new();
    let workspace = Workspace::temporary("self-devkits-ocr-pages")?;
    for page in first..=last {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled.".into());
        }
        // Reuse a single temporary raster so OCR of long documents does not
        // accumulate every rendered page in the document's preview cache.
        let raster_prefix = workspace.0.join("page");
        backend::render(
            &doc.source(),
            page,
            3200,
            &raster_prefix,
            &workspace.0,
            cancel,
        )?;
        let raster = raster_prefix.with_extension("png");
        let output = workspace.0.join("recognized");
        backend::ocr(&raster, &output, language, &workspace.0, cancel)?;
        let text = limited_text(&output.with_extension("txt"), MAX_TEXT)?;
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(&format!("--- Page {page} ---\n{text}"));
        if result.len() as u64 > MAX_TEXT {
            return Err("OCR text exceeds 16 MiB. Choose a smaller page range.".into());
        }
        progress(page - first + 1, last - first + 1);
    }
    Ok(result)
}
