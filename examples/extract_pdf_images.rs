//! Offline batch counterpart to the PDF Extraction screen.
use self_devkits::pdf_tools::{export_images, import, ImageMode};
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "Usage: cargo run --example extract_pdf_images -- INPUT.pdf OUTPUT_FOLDER".into(),
        );
    }
    let cancel = AtomicBool::new(false);
    let document = import(&PathBuf::from(&args[0]), &cancel)?;
    let output = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let (folder, count) = export_images(
        &document,
        &output,
        ImageMode::Embedded,
        2400,
        &cancel,
        |page, total| eprintln!("Page {page}/{total}"),
    )?;
    println!("{count} images: {}", folder.display());
    Ok(())
}
