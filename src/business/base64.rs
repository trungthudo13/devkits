//! Base64 text conversion and random encryption-key material.
use ::base64::{
    engine::general_purpose::{STANDARD, URL_SAFE},
    Engine,
};
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const MAX_TEXT: usize = 2 * 1024 * 1024;
fn engine(url_safe: bool) -> &'static ::base64::engine::GeneralPurpose {
    if url_safe {
        &URL_SAFE
    } else {
        &STANDARD
    }
}

pub fn encode(input: &str, url_safe: bool) -> Result<String, String> {
    if input.len() > MAX_TEXT {
        return Err("Text exceeds 2 MiB.".into());
    }
    Ok(engine(url_safe).encode(input.as_bytes()))
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Decoded {
    pub value: String,
    pub byte_count: usize,
    pub is_utf8: bool,
}
pub fn decode(input: &str, url_safe: bool) -> Result<Decoded, String> {
    if input.len() > MAX_TEXT * 2 {
        return Err("Base64 input exceeds 4 MiB.".into());
    }
    let clean: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut normalized = Zeroizing::new(clean);
    // Accept common unpadded encodings, while retaining strict alphabet/trailing-bit checks.
    if !normalized.contains('=') {
        let padding = (4 - normalized.len() % 4) % 4;
        normalized.extend(std::iter::repeat_n('=', padding));
    }
    let bytes = Zeroizing::new(
        engine(url_safe)
            .decode(normalized.as_bytes())
            .map_err(|_| "Invalid Base64 for the selected alphabet.")?,
    );
    if bytes.len() > MAX_TEXT {
        return Err("Decoded data exceeds 2 MiB.".into());
    }
    let (value, is_utf8) = match std::str::from_utf8(&bytes) {
        Ok(text) => (text.to_owned(), true),
        Err(_) => (
            bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            false,
        ),
    };
    Ok(Decoded {
        value,
        byte_count: bytes.len(),
        is_utf8,
    })
}

pub fn encryption_key(byte_count: usize) -> Result<String, String> {
    if ![16, 24, 32, 64].contains(&byte_count) {
        return Err("Choose 16, 24, 32 or 64 random bytes.".into());
    }
    let mut bytes = Zeroizing::new(vec![0u8; byte_count]);
    crate::adapter::random::secure_rng()
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "Operating-system randomness is unavailable.")?;
    Ok(STANDARD.encode(bytes.as_slice()))
}
