use std::process::{Command, Stdio};

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
    let root = Workspace::temporary("devkits-crop-test").unwrap();
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
    let root = Workspace::temporary("devkits-ocr-test").unwrap();
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
    let root = Workspace::temporary("devkits-pdf-test").unwrap();
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
    let (pages, count) = export_images(&doc, &root.0, ImageMode::Pages, 1200, &cancel, |n, _| {
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
    let page = render_page(&scan, 1, 1200, &cancel).unwrap();
    let regions = suggest_regions(&page, &cancel).unwrap();
    assert_eq!(regions.len(), 1, "{regions:?}");
    let crop = regions[0];
    assert!(crop[0] <= 120 && crop[1] <= 360);
    assert!(crop[0] + crop[2] >= 300 && crop[1] + crop[3] >= 480);
    let item = ReviewedImage {
        source: page.source,
        page: 1,
        name: "detected photo".into(),
        caption: None,
        crop: Some(crop),
    };
    let (exported, count) = export_reviewed(&[item], &root.0, &cancel, |_, _| {}).unwrap();
    assert_eq!(count, 1);
    let output = image::open(exported.join("detected_photo.png")).unwrap();
    assert_eq!((output.width(), output.height()), (crop[2], crop[3]));
    let snapshot = scan.workspace.0.clone();
    drop(scan);
    assert!(!snapshot.exists());
}
