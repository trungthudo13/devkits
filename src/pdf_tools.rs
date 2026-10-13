//! Offline PDF operations through the Poppler command-line tools.
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub type PdfResult<T> = Result<T, String>;
const MAX_PDF: u64 = 100 * 1024 * 1024;
const MAX_TEXT: u64 = 16 * 1024 * 1024;

#[derive(Debug)]
struct Workspace(PathBuf);
impl Workspace {
    fn new(parent: &Path, prefix: &str) -> PdfResult<Self> {
        for _ in 0..10 {
            let path = parent.join(format!("{prefix}-{:016x}", rand::random::<u64>()));
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Could not create a unique output folder.".into())
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

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

fn limited_text(path: &Path, limit: u64) -> PdfResult<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Extracted text exceeds the 16 MiB display limit.".into());
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn run(
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

pub fn import(path: &Path, cancel: &AtomicBool) -> PdfResult<Arc<PdfDocument>> {
    let workspace = Workspace::new(&std::env::temp_dir(), "self-devkits-pdf")?;
    let mut input = fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_PDF + 1);
    let source = workspace.0.join("source.pdf");
    let bytes = std::io::copy(
        &mut input,
        &mut fs::File::create(&source).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if bytes > MAX_PDF {
        return Err("PDF exceeds the 100 MiB limit.".into());
    }
    let info = run("pdfinfo", &[source.as_os_str()], &workspace.0, cancel)?;
    let info = limited_text(&info, 1024 * 1024)?;
    let pages = info
        .lines()
        .find_map(|line| line.strip_prefix("Pages:")?.trim().parse::<usize>().ok())
        .ok_or("Cannot determine PDF page count.")?;
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
    run(
        "pdftotext",
        &[
            "-layout".as_ref(),
            "-enc".as_ref(),
            "UTF-8".as_ref(),
            doc.source().as_os_str(),
            output.as_os_str(),
        ],
        &doc.workspace.0,
        cancel,
    )?;
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
    let layout_workspace = Workspace::new(&std::env::temp_dir(), "self-devkits-pdf-layout")?;
    let captions = if mode == ImageMode::Embedded {
        let xml = layout_workspace.0.join("layout.xml");
        let result = run(
            "pdftohtml",
            &[
                "-xml".as_ref(),
                "-hidden".as_ref(),
                doc.source().as_os_str(),
                xml.as_os_str(),
            ],
            &layout_workspace.0,
            cancel,
        )
        .and_then(|_| limited_text(&xml, 32 * 1024 * 1024))
        .and_then(|xml| crate::pdf_captions::captions(&xml));
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
        let page_num = page.to_string();
        let prefix = destination.0.join(format!("page-{page:04}"));
        let source = doc.source();
        if mode == ImageMode::Pages {
            let pixels = pixels.to_string();
            run(
                "pdftoppm",
                &[
                    "-f".as_ref(),
                    page_num.as_ref(),
                    "-l".as_ref(),
                    page_num.as_ref(),
                    "-singlefile".as_ref(),
                    "-scale-to".as_ref(),
                    pixels.as_ref(),
                    "-png".as_ref(),
                    source.as_os_str(),
                    prefix.as_os_str(),
                ],
                &doc.workspace.0,
                cancel,
            )?;
        } else {
            run(
                "pdfimages",
                &[
                    "-f".as_ref(),
                    page_num.as_ref(),
                    "-l".as_ref(),
                    page_num.as_ref(),
                    "-png".as_ref(),
                    source.as_os_str(),
                    prefix.as_os_str(),
                ],
                &doc.workspace.0,
                cancel,
            )?;
        }
        if mode == ImageMode::Embedded {
            let listing = run(
                "pdfimages",
                &[
                    "-f".as_ref(),
                    page_num.as_ref(),
                    "-l".as_ref(),
                    page_num.as_ref(),
                    "-list".as_ref(),
                    source.as_os_str(),
                ],
                &doc.workspace.0,
                cancel,
            )?;
            let listing = limited_text(&listing, 4 * 1024 * 1024)?;
            let rows: Vec<_> = listing
                .lines()
                .filter_map(|line| {
                    let columns: Vec<_> = line.split_whitespace().collect();
                    if columns.len() < 3 || columns[0].parse::<usize>().ok() != Some(page) {
                        return None;
                    }
                    Some((columns[1].parse::<usize>().ok()?, columns[2].to_owned()))
                })
                .collect();
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
                    .map(crate::pdf_captions::slug)
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
mod tests {
    use super::*;

    // A real two-page PDF with a shared raster image and optional text layer.
    fn fixture(with_text: bool) -> Vec<u8> {
        let content = if with_text {
            b"BT /F1 14 Tf 20 80 Td (Hello PDF) Tj ET q 30 0 0 20 20 20 cm /Im1 Do Q".as_slice()
        } else {
            b"q 30 0 0 20 20 20 cm /Im1 Do Q".as_slice()
        };
        let stream = |header: &str, data: &[u8]| {
            let mut bytes = format!("<< {header} /Length {} >>\nstream\n", data.len()).into_bytes();
            bytes.extend(data);
            bytes.extend(b"\nendstream");
            bytes
        };
        let page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 7 0 R >> >> /Contents 6 0 R >>";
        let objects = [
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
            page.to_vec(), page.to_vec(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            stream("", content),
            stream("/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8", &[255,0,0,0,255,0,0,0,255,255,255,255]),
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend(format!("{} 0 obj\n", i + 1).as_bytes());
            pdf.extend(object);
            pdf.extend(b"\nendobj\n");
        }
        let xref = pdf.len();
        pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            pdf.extend(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }

    #[test]
    fn reviewed_crops_rename_without_overwriting_and_clean_up_on_error() {
        let root = Workspace::new(&std::env::temp_dir(), "devkits-crop-test").unwrap();
        let source = root.0.join("original.png");
        let mut image = image::RgbImage::new(4, 3);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = image::Rgb([x as u8 * 50, y as u8 * 60, 123]);
        }
        image.save(&source).unwrap();
        let cancel = AtomicBool::new(false);
        let item = ReviewedImage {
            source: source.clone(),
            page: 2,
            name: "Súp bí đỏ".into(),
            caption: None,
            crop: Some([1, 1, 2, 2]),
        };
        let (output, count) =
            export_reviewed(&[item.clone(), item.clone()], &root.0, &cancel, |_, _| {}).unwrap();
        assert_eq!(count, 2);
        assert!(output.join("sup_bi_do_2.png").exists());
        let crop = image::open(output.join("sup_bi_do.png")).unwrap().to_rgb8();
        assert_eq!(crop.dimensions(), (2, 2));
        assert_eq!(crop.get_pixel(0, 0), image.get_pixel(1, 1));
        assert_eq!(image::open(&source).unwrap().to_rgb8(), image);
        let before = fs::read_dir(&root.0).unwrap().count();
        let mut invalid = item.clone();
        invalid.crop = Some([3, 2, 2, 2]);
        assert!(export_reviewed(&[invalid], &root.0, &cancel, |_, _| {}).is_err());
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), before);
        assert!(
            export_reviewed(&[item.clone(), item], &root.0, &cancel, |_, _| cancel
                .store(true, Ordering::Relaxed))
            .is_err()
        );
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), before);
    }

    #[test]
    fn ocr_recognizes_rendered_pdf_and_rejects_unknown_language() {
        let cancel = AtomicBool::new(false);
        let Ok(languages) = ocr_languages(&cancel) else {
            eprintln!("Skipping OCR integration: Tesseract unavailable.");
            return;
        };
        if !languages.iter().any(|l| l == "eng")
            || Command::new("pdftoppm")
                .arg("-v")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_err()
        {
            eprintln!("Skipping OCR integration: English data or Poppler unavailable.");
            return;
        }
        let root = Workspace::new(&std::env::temp_dir(), "devkits-ocr-test").unwrap();
        let source = root.0.join("ocr.pdf");
        fs::write(&source, fixture(true)).unwrap();
        let doc = import(&source, &cancel).unwrap();
        let text = ocr_text(&doc, 1, 1, "eng", &cancel, |_, _| {}).unwrap();
        assert!(text.contains("Hello PDF"), "{text}");
        assert!(ocr_text(&doc, 1, 1, "nonexistent-language", &cancel, |_, _| {}).is_err());
        assert!(ocr_text(&doc, 2, 1, "eng", &cancel, |_, _| {}).is_err());
    }

    #[test]
    fn poppler_roundtrip_export_text_cancel_and_invalid_pdf() {
        if ["pdfinfo", "pdftotext", "pdftoppm", "pdfimages"]
            .iter()
            .any(|tool| {
                Command::new(tool)
                    .arg("-v")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
            })
        {
            eprintln!("Skipping PDF integration: install Poppler to run this test.");
            return;
        }
        let root = Workspace::new(&std::env::temp_dir(), "devkits-pdf-test").unwrap();
        let source = root.0.join("input with spaces.pdf");
        fs::write(&source, fixture(true)).unwrap();
        let cancel = AtomicBool::new(false);
        let doc = import(&source, &cancel).unwrap();
        assert_eq!(doc.pages, 2);
        // Operations use the imported snapshot even after the original changes.
        fs::write(&source, b"changed").unwrap();
        assert_eq!(
            read_text(&doc, &cancel)
                .unwrap()
                .matches("Hello PDF")
                .count(),
            2
        );
        let mut progress = Vec::new();
        let (pages, count) =
            export_images(&doc, &root.0, ImageMode::Pages, 1200, &cancel, |n, _| {
                progress.push(n)
            })
            .unwrap();
        assert_eq!((count, progress), (2, vec![1, 2]));
        let png = image::open(pages.join("page-0001.png")).unwrap();
        assert_eq!((png.width(), png.height()), (1200, 600));
        let (images, count) =
            export_images(&doc, &root.0, ImageMode::Embedded, 1200, &cancel, |_, _| {}).unwrap();
        assert_eq!(count, 2);
        let embedded = fs::read_dir(images)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "png"))
            .unwrap();
        let png = image::open(embedded).unwrap().to_rgb8();
        assert_eq!(png.dimensions(), (2, 2));
        assert_eq!(png.get_pixel(0, 0).0, [255, 0, 0]);
        let before = fs::read_dir(&root.0).unwrap().count();
        let error = export_images(&doc, &root.0, ImageMode::Pages, 1200, &cancel, |_, _| {
            cancel.store(true, Ordering::Relaxed)
        })
        .unwrap_err();
        assert_eq!(error, "Cancelled.");
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), before);
        cancel.store(false, Ordering::Relaxed);
        assert!(import(&source, &cancel).is_err());
        fs::write(&source, fixture(false)).unwrap();
        let scan = import(&source, &cancel).unwrap();
        assert!(read_text(&scan, &cancel).unwrap().trim().is_empty());
        let snapshot = scan.workspace.0.clone();
        drop(scan);
        assert!(!snapshot.exists());
    }
}

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
    let workspace = Workspace::new(&std::env::temp_dir(), "self-devkits-review")?;
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

fn decode_image(path: &Path) -> PdfResult<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path).map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|e| format!("Cannot preview image: {e}"))
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
        let number = page.to_string();
        let size = pixels.to_string();
        let result = run(
            "pdftoppm",
            &[
                "-f".as_ref(),
                number.as_ref(),
                "-l".as_ref(),
                number.as_ref(),
                "-singlefile".as_ref(),
                "-scale-to".as_ref(),
                size.as_ref(),
                "-png".as_ref(),
                doc.source().as_os_str(),
                prefix.as_os_str(),
            ],
            &doc.workspace.0,
            cancel,
        );
        if let Err(e) = result {
            let _ = fs::remove_file(&source);
            return Err(e);
        }
    }
    let (width, height) = image::image_dimensions(&source).map_err(|e| e.to_string())?;
    Ok(PageImage {
        source,
        page,
        width,
        height,
    })
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
        let stem = crate::pdf_captions::slug(&item.name);
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
            crop_image(decode_image(&item.source)?, item.crop)?
                .save_with_format(&target, image::ImageFormat::Png)
                .map_err(|e| e.to_string())?;
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
    let workspace = Workspace::new(&std::env::temp_dir(), "self-devkits-ocr")?;
    let stdout = run(
        "tesseract",
        &["--list-langs".as_ref()],
        &workspace.0,
        cancel,
    )?;
    let languages = limited_text(&stdout, 64 * 1024)?
        .lines()
        .filter(|l| !l.contains(' ') && !l.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
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
    let workspace = Workspace::new(&std::env::temp_dir(), "self-devkits-ocr-pages")?;
    for page in first..=last {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled.".into());
        }
        // Reuse a single temporary raster so OCR of long documents does not
        // accumulate every rendered page in the document's preview cache.
        let number = page.to_string();
        let raster_prefix = workspace.0.join("page");
        run(
            "pdftoppm",
            &[
                "-f".as_ref(),
                number.as_ref(),
                "-l".as_ref(),
                number.as_ref(),
                "-singlefile".as_ref(),
                "-scale-to".as_ref(),
                "3200".as_ref(),
                "-png".as_ref(),
                doc.source().as_os_str(),
                raster_prefix.as_os_str(),
            ],
            &workspace.0,
            cancel,
        )?;
        let raster = raster_prefix.with_extension("png");
        let output = workspace.0.join("recognized");
        run(
            "tesseract",
            &[
                raster.as_os_str(),
                output.as_os_str(),
                "-l".as_ref(),
                language.as_ref(),
                "--psm".as_ref(),
                "3".as_ref(),
            ],
            &workspace.0,
            cancel,
        )?;
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
