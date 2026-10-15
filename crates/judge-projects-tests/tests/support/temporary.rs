//! Offline fixture source keeps its original checkout and declared external inputs.
//! This helper grants no generated-output disposal or lifecycle completion.
use std::{cell::RefCell, path::{Path, PathBuf}};

pub struct FixtureDirectory {
    directory: RefCell<Option<tempfile::TempDir>>,
    allocation: PathBuf,
    root: PathBuf,
}
impl FixtureDirectory {
    pub fn path(&self) -> &Path {
        &self.root
    }
    pub fn temporary_path(&self) -> &Path {
        &self.allocation
    }
    pub fn make_checkout_unavailable(&self) -> std::io::Result<PathBuf> {
        let retained = self.allocation.join("retained-offline-checkout");
        if retained.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "offline source already retained"));
        }
        // End automatic TempDir removal before moving any unique source. Failed
        // relocation also leaves the original allocation available for diagnosis.
        if let Some(directory) = self.directory.borrow_mut().take() {
            directory.keep();
        }
        std::fs::rename(&self.root, &retained)?;
        Ok(retained)
    }
    pub fn close(self) -> std::io::Result<()> {
        self.make_checkout_unavailable().map(|_| ())
    }
}
pub fn fixture_directory(prefix: &str) -> FixtureDirectory {
    let directory = tempfile::Builder::new().prefix(prefix).tempdir().unwrap();
    let allocation = std::fs::canonicalize(directory.path()).unwrap();
    let root = allocation.join("fixture");
    std::fs::create_dir(&root).unwrap();
    FixtureDirectory { directory: RefCell::new(Some(directory)), allocation, root }
}
