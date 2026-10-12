# Self DevKits

Ứng dụng desktop **Rust + egui/eframe**, chạy trên Windows, Ubuntu/Linux và macOS. UI native được render bằng OpenGL, không dùng Electron hoặc WebView. Bố cục lấy cảm hứng từ ảnh tham chiếu: sidebar, thanh cấu hình, hai vùng nhập/kết quả và tông xám tím nhạt.

## Chức năng

- **JSON**: format (2/4/8 spaces), minify, kiểm tra cú pháp, syntax highlighting. Giữ nguyên thứ tự/khóa trùng, số lớn và cách escape chuỗi.
- **XML**: format, minify, kiểm tra well-formedness, highlighting. Giữ nguyên mixed content, `xml:space="preserve"`, comment, CDATA và khoảng trắng trong phần tử lá. Với mixed content hoặc `xml:space="preserve"`, format/minify trả về nguyên văn để tránh đổi dữ liệu. Không tải DTD hay entity bên ngoài.
- **SQL**: format, minify, kiểm tra cú pháp, highlighting; Generic, PostgreSQL, MySQL, SQLite. Minify bỏ comment và khoảng trắng dư, giữ khoảng cách giữa token để không ghép nhầm toán tử. Lint là kiểm tra cú pháp, không kiểm tra schema, không thực thi truy vấn.
- **Random Password**: tạo 1–1000 mật khẩu mỗi lần, sao chép từng mật khẩu hoặc toàn bộ (mỗi dòng một mật khẩu); 8–128 ký tự; chọn chữ thường, chữ hoa, số, ký hiệu; loại ký tự dễ nhầm; có ít nhất một ký tự thuộc mỗi nhóm đã chọn. Dùng nguồn ngẫu nhiên hệ điều hành (`OsRng`).
- **QR Generator**: UTF-8, bốn mức error correction, preview, xuất PNG với quiet zone.
- Tìm kiếm công cụ, light/dark theme, mở/lưu file UTF-8, copy, dùng output làm input, mẫu dữ liệu; `Ctrl+Enter` / `Cmd+Enter` để format.

Dữ liệu chỉ giữ trong bộ nhớ; app không gửi dữ liệu qua mạng, không tự lưu phiên làm việc. Copy đưa dữ liệu vào clipboard của hệ điều hành. Giới hạn xử lý tài liệu: 2 MiB. “Compress” ở đây là minify văn bản, không phải tạo ZIP/gzip.

## Chạy từ source

Cài Rust stable bằng [rustup](https://rustup.rs/). Trên Windows dùng toolchain MSVC và Visual Studio C++ Build Tools. Trên macOS cài Xcode Command Line Tools (`xcode-select --install`).

Ubuntu:

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

File dialog trên Linux sử dụng XDG Desktop Portal; cần portal backend phù hợp desktop, ví dụ `xdg-desktop-portal-gtk` trên GNOME.

```sh
cargo run --locked
```

## Build và kiểm thử

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Binary: `target/release/self-devkits` hoặc `target/release/self-devkits.exe`.

Workflow `.github/workflows/ci.yml` kiểm thử và build release trên cả ba hệ điều hành, sau đó đóng gói và upload thành GitHub Actions artifact (Linux `.tar.gz`, Windows `.zip`, macOS `.app` trong `.zip`). Cần push repository để workflow chạy. Các artifact chưa được code-sign/notarize; chưa có installer MSI/DMG.

## Cấu trúc

- `src/lib.rs`: xử lý tài liệu, mật khẩu, QR độc lập với GUI.
- `src/main.rs`: cửa sổ native, điều hướng, editor, file dialog và theme.
- `tests/core.rs`: round-trip dữ liệu, lỗi cú pháp, ràng buộc mật khẩu và giải mã QR.

Thư viện GUI: [eframe](https://docs.rs/eframe/0.31.1/eframe/). SQL lexer/parser: [sqlparser](https://docs.rs/sqlparser/0.53.0/sqlparser/). XML parsing: [quick-xml](https://docs.rs/quick-xml/0.37.5/quick_xml/) và roxmltree.

Đóng gói tại máy sau khi build: `python3 scripts/package.py`. Kết quả nằm trong `dist/`.

Ảnh giao diện native: [JSON](docs/screenshots/json.png), [Password](docs/screenshots/password.png), [QR](docs/screenshots/qr.png), [Dark theme](docs/screenshots/dark.png).

## SQL Editor

SQL Editor dùng một vùng code nền trắng như ảnh tham chiếu: số dòng, syntax highlighting, indent guides và đánh dấu dòng đang sửa. Toolbar gồm dialect, **Keyword uppercase**, **Alias align**, **Example**, **Clipboard** (dán từ clipboard), **Compression**, **Beautify**. Hai thao tác định dạng thay đổi trực tiếp nội dung editor; lỗi cú pháp giữ nguyên dữ liệu đang nhập.

- `Ctrl+Enter` / `Cmd+Enter`: Beautify.
- Nhấp chuột phải trong editor: lint, copy, mở/lưu SQL.
- Lint nằm trong menu chuột phải. Icon mặt trời: theme; ghim: always on top; sidebar: ẩn/hiện điều hướng.
- Keyword uppercase giữ nguyên tên hàm; Alias align căn các alias `AS` liên tiếp ở cấp ngoài cùng, không thay đổi chuỗi hoặc `CAST(... AS ...)`.
- Icon dùng thư viện Phosphor (`egui-phosphor`). [Ảnh SQL Editor](docs/screenshots/sql.png).

## XML Editor

XML Editor dùng một vùng code nền trắng theo cùng bố cục SQL Editor: số dòng, indent guides, đánh dấu dòng hiện tại và highlight thẻ/giá trị/comment. Toolbar có chọn indent, **Example**, **Clipboard**, **Validate**, **Compress**, **Format**. Format và Compress cập nhật trực tiếp editor; dữ liệu không hợp lệ được giữ nguyên và lỗi được hiển thị.

`Ctrl+Enter` / `Cmd+Enter` để format. Menu chuột phải cung cấp copy, mở/lưu XML, validate và clear. Mixed content và `xml:space="preserve"` vẫn được giữ nguyên khi định dạng/nén. [Ảnh XML Editor](docs/screenshots/xml.png).

## JSON Editor

JSON Editor nay có một vùng code theo ảnh mẫu: số dòng, đường thụt lề, highlight key/string/number và nút gấp/mở object hoặc array. Sidebar giữ padding 8px.

- **Format / Compress**: cập nhật trực tiếp editor; lỗi giữ nguyên input.
- **Escape**: chuyển toàn bộ nội dung thành chuỗi JSON có dấu nháy/escape. **Remove Escape**: giải mã chuỗi JSON đó; hai thao tác đảo ngược nhau.
- **Clipboard**: dán nội dung clipboard. **Simple**: ẩn nút gấp/mở để dùng editor đơn giản.
- Khi đang gấp, nhấn mũi tên để mở từng block hoặc nhấn code để mở tất cả và chỉnh sửa. Dữ liệu nguồn không bị thay thế bằng dấu `…`.
- Menu chuột phải: Example, Validate, Expand all, Copy, Open/Save, Clear. `Ctrl/Cmd+Enter`: Format.

[JSON Editor](docs/screenshots/json.png) · [Gấp block](docs/screenshots/json-folded.png)

## Regex Editor

Tool **Regex Editor** gồm ba chế độ và cập nhật kết quả khi sửa pattern/input:

- **Match**: highlight các match, số lượng, vị trí ký tự và capture groups (kể cả named groups).
- **Substitution**: thay bằng template; `$0`, `$1`, `${name}`, `$$` lần lượt là toàn bộ match, group, named group, dấu dollar. Replacement rỗng xóa match.
- **Extraction**: xuất template cho từng match, nối bằng newline/tab/comma hoặc không có separator.
- **Delimiters**: `/`, `#`, `~`, `|`, hoặc None. Nhập pattern không kèm delimiters trong ô; Copy/Paste expression hỗ trợ biểu thức đầy đủ như `/pattern/gi`.
- **Regex Flags**: `g` (tất cả match), `i` (ignore case), `m` (multiline), `s` (dotall), `x` (extended), `U` (swap greed), `u` (Unicode). Mặc định `gu`.

Engine là Rust `regex`, **không phải PCRE2**: không hỗ trợ look-around hay backreference trong pattern. Capture references trong replacement/extraction được hỗ trợ. Vị trí hiển thị tính bằng Unicode characters, bắt đầu từ 0, điểm cuối không nằm trong match. Giới hạn input 128 KiB, pattern 4 KiB, 1.000 matches, 64 groups và output 2 MiB; vượt giới hạn báo lỗi thay vì trả kết quả thiếu.

Menu “Editor indentation” đã được bỏ; chọn indent của XML vẫn nằm ngay trên toolbar XML.

[Match](docs/screenshots/regex-match.png) · [Substitution](docs/screenshots/regex-substitution.png) · [Extraction](docs/screenshots/regex-extraction.png)

## Fcitx5 trên Linux

Ứng dụng có lớp tương thích IME cho egui 0.31: xử lý cả trường hợp Fcitx5/Unikey gửi commit trực tiếp mà không có preedit. Lớp này áp dụng cho mọi ô nhập liệu trên Linux, bao gồm các editor và Regex, giữ nguyên chuỗi sự kiện preedit bình thường.

Trên X11, khởi chạy app trong session có `XMODIFIERS=@im=fcitx` và locale UTF-8. Không cần đổi cấu hình GTK/Qt của hệ thống để dùng bản sửa này. Sau khi cập nhật binary, đóng và mở lại app.

Kiểm thử hồi quy gửi sự kiện IME vào TextEdit thật: commit ở đầu/cuối/giữa văn bản, thay selection, nhiều commit liên tiếp và preedit→commit. Kiểm thử baseline tái hiện commit bị mất trong egui 0.31 khi thiếu sự kiện bắt đầu composition.
