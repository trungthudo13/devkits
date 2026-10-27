//! Load OS fonts once at startup, leaving bundled fonts as a fallback.
use std::path::{Path, PathBuf};

pub struct SystemFont {
    pub bytes: Vec<u8>,
    pub index: u32,
}

fn read_font(path: &Path, index: u32) -> Option<SystemFont> {
    let bytes = std::fs::read(path).ok()?;
    // Validate with the same parser as the renderer before registering the font.
    ab_glyph::FontRef::try_from_slice_and_index(&bytes, index).ok()?;
    Some(SystemFont { bytes, index })
}

pub fn load(monospace: bool) -> Option<SystemFont> {
    let (path, index) = location(monospace)?;
    read_font(&path, index)
}

#[cfg(target_os = "macos")]
fn location(monospace: bool) -> Option<(PathBuf, u32)> {
    // shortcut: standard OS font files, use CoreText if per-user font selection is needed.
    let names = if monospace {
        ["SFNSMono.ttf", "Menlo.ttc"]
    } else {
        ["SFNS.ttf", "Helvetica.ttc"]
    };
    names
        .into_iter()
        .map(|name| Path::new("/System/Library/Fonts").join(name))
        .find(|path| path.is_file())
        .map(|path| (path, 0))
}

#[cfg(target_os = "windows")]
fn location(monospace: bool) -> Option<(PathBuf, u32)> {
    // shortcut: standard OS font files, use DirectWrite if custom UI font settings are needed.
    let root = std::env::var_os("WINDIR")?;
    Some((
        PathBuf::from(root).join("Fonts").join(if monospace {
            "consola.ttf"
        } else {
            "segoeui.ttf"
        }),
        0,
    ))
}

#[cfg(target_os = "linux")]
fn location(monospace: bool) -> Option<(PathBuf, u32)> {
    let output = std::process::Command::new("fc-match")
        .args([
            "-f",
            "%{file}\n%{index}",
            if monospace { "monospace" } else { "sans-serif" },
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_match(&output.stdout)
}

#[cfg(any(target_os = "linux", test))]
fn parse_match(output: &[u8]) -> Option<(PathBuf, u32)> {
    let text = std::str::from_utf8(output).ok()?;
    let (path, index) = text.trim_end().rsplit_once('\n')?;
    if path.is_empty() {
        return None;
    }
    // Fontconfig stores the collection face in the low 16 bits.
    Some((PathBuf::from(path), index.parse::<u32>().ok()? & 0xffff))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn location(_: bool) -> Option<(PathBuf, u32)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fontconfig_preserves_paths_and_collection_index() {
        assert_eq!(
            parse_match(b"/fonts/My Font.ttc\n2"),
            Some(("/fonts/My Font.ttc".into(), 2))
        );
        assert_eq!(
            parse_match(b"/fonts/Variable.ttf\n65538"),
            Some(("/fonts/Variable.ttf".into(), 2))
        );
        for invalid in [b"".as_slice(), b"/font.ttf", b"\n0", b"/font.ttf\ninvalid"] {
            assert!(parse_match(invalid).is_none());
        }
    }

    #[test]
    fn unreadable_or_invalid_fonts_fall_back() {
        assert!(read_font(Path::new("missing-system-font.ttf"), 0).is_none());
        assert!(read_font(Path::new("Cargo.toml"), 0).is_none());
        let bundled = Path::new("assets/fonts/noto-sans-mono/NotoSansMono-Regular.ttf");
        assert!(read_font(bundled, 0).is_some());
        assert!(read_font(bundled, 999).is_none());
    }
}
