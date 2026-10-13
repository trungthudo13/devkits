//! Operating-system, filesystem and external-tool integrations. No business-layer imports.
pub mod desktop;
pub mod files;
pub mod images;
#[cfg(any(target_os = "linux", test))]
pub mod ime;
pub(crate) mod pdf;
pub(crate) mod process;
pub(crate) mod random;
pub(crate) mod workspace;
