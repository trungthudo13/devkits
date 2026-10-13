pub fn qr_png(data: &str, level: qrcode::EcLevel) -> Result<image::GrayImage, String> {
    if data.is_empty() {
        return Err("Enter text or a URL first.".into());
    }
    let code = qrcode::QrCode::with_error_correction_level(data.as_bytes(), level)
        .map_err(|e| e.to_string())?;
    Ok(code
        .render::<image::Luma<u8>>()
        .quiet_zone(true)
        .module_dimensions(8, 8)
        .build())
}

/// Export a generated QR image through the image persistence adapter.
pub fn save_png(image: &image::GrayImage, path: &std::path::Path) -> Result<(), String> {
    crate::adapter::images::save_gray_png(image, path)
}
