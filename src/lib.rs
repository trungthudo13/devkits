//! Public application API. Layers are explicit; legacy exports remain compatible.
pub mod adapter;
pub mod business;
pub use business::{documents::*, password::*, qr::qr_png};
pub use business::{pdf as pdf_tools, regex as regex_tools};
