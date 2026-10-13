//! Application rules and use cases. No UI types or native dialogs.
pub mod documents;
pub mod password;
pub mod pdf;
pub mod qr;
pub mod regex;
pub use documents::*;
pub use password::{password, passwords, PasswordOptions};
pub use qr::qr_png;
