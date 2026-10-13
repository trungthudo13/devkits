use std::path::Path;
type PdfResult<T> = Result<T, String>;
pub fn decode_image(path: &Path) -> PdfResult<image::DynamicImage> {
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

pub fn dimensions(path: &Path) -> PdfResult<(u32, u32)> {
    image::image_dimensions(path).map_err(|e| e.to_string())
}
pub fn save_png(image: &image::DynamicImage, path: &Path) -> PdfResult<()> {
    image
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}
pub fn save_gray_png(image: &image::GrayImage, path: &Path) -> PdfResult<()> {
    image
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}
