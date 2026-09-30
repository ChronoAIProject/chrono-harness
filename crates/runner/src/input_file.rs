//! Shared regular-file/absence observation for registered external inputs.
use crate::file_identity;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

/// Observe a regular file or explicit absence. Other errors and links are not absence.
/// This is a point-in-time observation, not protection against transient concurrent writes.
pub fn observe_file(path: &Path) -> Result<Option<(String, u64)>, String> {
    let mut prefix = PathBuf::new();
    for component in path.components() {
        if !matches!(
            component,
            Component::RootDir | Component::Prefix(_) | Component::Normal(_)
        ) {
            return Err(format!(
                "E_INPUT_PATH: non-normal input path: {}",
                path.display()
            ));
        }
        prefix.push(component);
        match fs::symlink_metadata(&prefix) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("E_INPUT_PATH: symlink input: {}", prefix.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("E_INPUT_PATH: {}: {e}", prefix.display())),
        }
    }
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("E_INPUT_PATH: {}: {e}", path.display())),
        Ok(_) => file_identity(path).map(Some),
    }
}
