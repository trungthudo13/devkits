# DevKits

An offline desktop toolbox for developers, built with **Rust and egui/eframe**. DevKits runs on Windows, Linux and macOS with a native OpenGL UI, a shared light/dark theme, and no Electron or WebView.

## Tools

| Tool | Features |
| --- | --- |
| JSON Editor | Format, minify, validate, syntax highlighting, folding, escape/unescape |
| XML Editor | Format, minify, validate well-formedness, syntax highlighting |
| SQL Editor | Format, minify, syntax checking, keyword capitalization, alias alignment |
| Regex Editor | Live matching, capture groups, substitution and extraction |
| Random Password | Generate 1–1,000 passwords with configurable character groups |
| QR Generator | Preview QR codes and export PNG files |
| PDF Extraction | Review/rename embedded images, crop page regions, extract text, optional offline OCR |

Editors support UTF-8 files, clipboard operations, sample data, line numbers and indentation guides. `Ctrl+Enter` / `Cmd+Enter` formats the current JSON/XML/SQL document. Right-click an editor for its file and validation actions.

The custom title bar provides minimize, maximize/restore, close, pin and theme controls. Drag its empty area to move the window; double-click to maximize/restore. Resize from the edges and corners. The embedded editor font is **Noto Sans Mono Regular, 14 pt**, with 20 pt code rows. UI labels use egui's Ubuntu Light with Noto Sans Mono as a Unicode fallback; icons use Phosphor.

## PDF workflow

Import a PDF, then choose a workspace:

### Images & filenames

1. Click **Find embedded images** to extract original raster images into temporary storage.
2. Select an entry to preview it. Edit its suggested filename and uncheck images you do not need.
3. Click **Export selected** and choose a destination folder.

Suggested names come from nearby captions when a unique match is available. Vietnamese accents are removed, words are joined with underscores, and duplicate filenames get numeric suffixes. For example, `Súp bí đỏ` becomes `sup_bi_do.png`.

Caption matching is a geometric heuristic, not image recognition. It can be wrong on unfamiliar layouts: review the preview and names before exporting. Missing or ambiguous captions use numbered names. Image masks may be separate entries. An embedded collage remains one image; use **Page regions** to split it manually. `manifest.json` records the exported names, source pages and crop coordinates where applicable.

### Page regions

1. Choose a page and rendering size, then click **Load page**.
2. Drag a rectangle around one photo or illustration.
3. Enter its filename and click **Add region**. Repeat for other regions/pages.
4. Open **Images & filenames** to review and export the collected regions.

This works with scanned pages, collages and vector graphics. Regions are rasterized at the selected page resolution; the rectangle shown in the preview is mapped to the full-resolution page. It does not automatically identify objects or split dishes. **Export all pages** remains available when complete page PNGs are wanted.

### Text & OCR

- **Read text layer** extracts existing PDF text.
- **Check OCR languages** lists installed Tesseract language packs.
- **Run OCR** recognizes a selected page range. Enter `eng`, `vie`, or `vie+eng` as appropriate for installed packs.
- Review and edit the result, then **Copy** or **Save TXT** as UTF-8.

OCR runs locally and requires Tesseract. Accuracy and reading order depend on scan quality and layout. Text recognition does not automatically label image regions.

PDF work runs in a background thread with progress and cancellation. Import supports files up to 100 MiB and 2,000 pages. Text output is limited to 16 MiB. Page rendering supports up to 4,000 pixels on the longest edge. Oversized images can exceed the bounded preview decoder; page rendering is an alternative. Password-protected PDFs must be unlocked before import.

### PDF/OCR dependencies

PDF operations use Poppler's `pdfinfo`, `pdftoppm`, `pdfimages`, `pdftotext`, and `pdftohtml`. OCR additionally uses Tesseract and language data.

Ubuntu/Debian:

```sh
sudo apt install poppler-utils tesseract-ocr tesseract-ocr-eng tesseract-ocr-vie
```

macOS with Homebrew:

```sh
brew install poppler tesseract tesseract-lang
```

On Windows, install Poppler and optionally Tesseract, and add their executable directories to `PATH`. Install the required OCR language packs as well. Restart DevKits after installing these dependencies. The Windows and macOS installers do not bundle Poppler or Tesseract. The Debian package depends on Poppler and recommends OCR packages.

A batch extraction example is also available (uses automatic caption suggestions without interactive review):

```sh
cargo run --locked --example extract_pdf_images -- "menu.pdf" "output-folder"
```

## Data handling and limits

- No document contents are uploaded. Editor sessions are not automatically saved.
- PDF imports and previews use private temporary folders. They are deleted on normal cleanup; a crash or forced shutdown can leave temporary files behind.
- Export creates a new destination subfolder instead of overwriting existing images. Cancellation or failure removes that operation's partial folder.
- Clipboard actions use the operating system clipboard.
- JSON/XML/SQL processing accepts up to 2 MiB of input. “Compress” means text minification, not ZIP/gzip.
- JSON formatting preserves duplicate keys, exact number text and string escapes.
- XML mixed content and `xml:space="preserve"` documents are returned unchanged during formatting/minification. XML validation does not validate schemas or load external entities.
- SQL supports Generic, MySQL, PostgreSQL and SQLite syntax. Rewrites are checked against the parsed syntax tree. No queries are executed and database schemas are not checked.
- Regex uses Rust `regex`, not PCRE2; pattern look-around and backreferences are unsupported. Replacement capture references are supported. Limits: 128 KiB input, 4 KiB pattern, 1,000 matches, 64 capture groups, 2 MiB output.
- Passwords use the operating system random source, support lengths of 8–128 characters and include at least one character from every selected group.

## Run from source

Install a stable Rust toolchain. Windows builds use MSVC and Visual Studio C++ Build Tools. macOS builds require Xcode Command Line Tools.

Ubuntu build dependencies:

```sh
sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

Linux file dialogs use XDG Desktop Portal; install a backend appropriate for your desktop, such as `xdg-desktop-portal-gtk`.

```sh
cargo run --locked
```

The Cargo package and source binary retain the internal name `self-devkits` for compatibility. The application and installers are branded **DevKits**; the Debian launcher is `devkits`.

Linux includes an egui 0.31 IME compatibility fix for commit-only input from Fcitx5/Unikey. For X11, use a UTF-8 locale and `XMODIFIERS=@im=fcitx` in the application session.

## Validate and build

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 -m unittest discover -s tests -p 'test_*.py'
cargo build --release --locked --bin self-devkits
```

PDF integration tests run when Poppler is installed. OCR integration tests require Tesseract with English language data. Tests that need unavailable external tools report a skip message. CI installs these dependencies on Linux; core tests run on every platform.

## Installers

After building the release binary, use Python 3.11 or newer:

```sh
python3 scripts/package.py
```

The script uses the version from `Cargo.toml` and produces an installer plus a SHA-256 checksum in `dist/`:

| Platform | Output | Packaging requirement |
| --- | --- | --- |
| Ubuntu/Debian | `DevKits-<version>-linux-amd64.deb` | `dpkg-deb`, `objdump` (binutils) |
| Windows x64 | `DevKits-<version>-windows-x64-setup.exe` | NSIS (`choco install nsis -y`) |
| macOS Intel | `DevKits-<version>-macos-x64.dmg` | Xcode tools, `codesign`, `hdiutil` |
| macOS Apple Silicon | `DevKits-<version>-macos-arm64.dmg` | Xcode tools, `codesign`, `hdiutil` |

The `.deb` installs an application launcher, icon, binary and licenses. Install it with `sudo apt install ./DevKits-<version>-linux-amd64.deb` so dependencies are resolved. Windows installs per user, adds Start Menu shortcuts and includes an uninstaller. On macOS, drag `DevKits.app` to Applications.

Windows installers are not Authenticode-signed. macOS bundles receive an ad-hoc signature but are not Developer ID-signed or notarized; operating-system trust prompts may appear. No signing credentials are required by the build workflow.

## GitHub Actions CI/CD

`.github/workflows/ci.yml` uses standard public **GitHub-hosted runners**, not self-hosted runners:

- `ubuntu-22.04`: Linux amd64 `.deb`.
- `windows-2022`: Windows x64 installer `.exe`.
- `macos-15`: Apple Silicon `.dmg`.
- `macos-15-intel`: Intel `.dmg`.

Pushes, pull requests and manual dispatches run formatting, Clippy, tests, release builds and installer packaging. Each job uploads its installer and checksum as a downloadable Actions artifact, retained for 30 days.

For a release:

1. Update `Cargo.toml` and `Cargo.lock` to the desired version.
2. Commit the changes and push a matching tag, such as `v0.1.0`.
3. After all platform builds succeed, the release job publishes the installers and checksums to GitHub Releases.

The tag must match the Cargo version. Build jobs have read-only repository permissions; only the tagged release job has `contents: write`. Windows release binaries use a statically linked C runtime. Linux CI builds on Ubuntu 22.04; package metadata also checks the actual binary's minimum glibc symbol version.

Runner labels and availability are documented in [GitHub's hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Source layout

- `src/main.rs`: application state, navigation and startup.
- `src/window_chrome.rs`: custom title bar and resize controls.
- `src/lib.rs`: document processing, passwords and QR generation.
- `src/*_editor.rs`: individual tool interfaces.
- `src/code_editor.rs`: shared code editor, caret and indentation rendering.
- `src/pdf_tools.rs`: PDF extraction, staging, crops, export and OCR.
- `src/pdf_captions.rs`: conservative caption matching and safe filenames.
- `src/regex_tools.rs`: regex processing.
- `src/ime.rs`: Linux IME compatibility layer.
- `tests/`: integration and packaging tests.
- `scripts/`: platform packaging and Windows installer definition.

## License

DevKits is MIT-licensed. Noto Sans Mono is distributed under the SIL Open Font License; its license and attribution are included in `assets/fonts/noto-sans-mono/` and the installers. Third-party dependencies retain their respective licenses.
