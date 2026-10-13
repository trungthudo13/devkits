use std::{
    fs,
    path::{Path, PathBuf},
};
type PdfResult<T> = Result<T, String>;
#[derive(Debug)]
pub(crate) struct Workspace(pub(crate) PathBuf);
impl Workspace {
    pub(crate) fn temporary(prefix: &str) -> PdfResult<Self> {
        Self::new(&std::env::temp_dir(), prefix)
    }
    pub(crate) fn new(parent: &Path, prefix: &str) -> PdfResult<Self> {
        for _ in 0..10 {
            let path = parent.join(format!("{prefix}-{:016x}", rand::random::<u64>()));
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Could not create a unique output folder.".into())
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
