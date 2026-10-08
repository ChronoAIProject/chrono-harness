//! Physical artifact effects shared by lifecycle and explicit output retention owners.
use std::{
    fs,
    path::{Path, PathBuf},
};
/// Publish an identity already durably enrolled at an exact staging slot.
/// Both supported kernel operations refuse to replace an occupied destination.
pub fn publish_entry(staged: &Path, destination: &Path) -> Result<(), String> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source = CString::new(staged.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    let target = CString::new(destination.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(source.as_ptr(), target.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("identity publication is unsupported on this platform".into());
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}
/// Identity of an exact entry; links are identities of the link, never its target.
pub fn identity(path: &Path) -> Result<String, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!(
            "{}:{}:{}",
            m.dev(),
            m.ino(),
            if m.is_dir() {
                "directory"
            } else if m.is_file() {
                "file"
            } else if m.file_type().is_symlink() {
                "link"
            } else {
                "special"
            }
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        Err("artifact identity is unsupported on this platform".into())
    }
}
/// Parent resolution excludes links. A final internal link is unlinked without traversal.
pub fn entry_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    crate::relative_path(relative)?;
    let path = Path::new(relative);
    let parent = path.parent().ok_or("artifact parent")?;
    let physical = if parent.as_os_str().is_empty() {
        root.to_owned()
    } else {
        crate::no_symlink_parents(root, parent.to_str().ok_or("artifact path UTF8")?)?
    };
    Ok(physical.join(path.file_name().ok_or("artifact name")?))
}
/// Exactly one unlink/rmdir effect. The owner supplies live policy/reference/exclusion checks.
pub fn dispose_entry(
    root: &Path,
    relative: &str,
    expected: &str,
    mut before_effect: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let path = entry_path(root, relative)?;
    let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !m.is_dir() && !m.is_file() && !m.file_type().is_symlink() {
        return Err("artifact contains a special file; preserve it".into());
    }
    before_effect()?;
    if identity(&path)? != expected {
        return Err("artifact identity changed before effect".into());
    }
    if m.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|e| e.to_string())
}
/// Legacy all-at-once contract; output retention uses dispose_entry for resumable bounds.
pub fn dispose_directory(path: &Path) -> Result<(), String> {
    fs::remove_dir_all(path).map_err(|e| e.to_string())
}

/// Reuse the Git fact owner to protect both committed and staged source. A
/// directory policy is never permission to remove a newly staged descendant.
/// Standalone fixture hosts without a Git owner have no source/index to query.
pub fn source_safe(root: &Path, relative: &str) -> Result<(), String> {
    crate::relative_path(relative)?;
    match fs::symlink_metadata(root.join(".git")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(_) => (),
    }
    let reader = if root.join(".chrono-harness/config.json").exists() {
        crate::facts::Reader::for_config(root, ".chrono-harness/config.json")?
    } else {
        crate::facts::Reader::legacy()
    };
    let index = reader.git(
        root,
        &["--literal-pathspecs", "ls-files", "-z", "--", relative],
    )?;
    let tree = reader.git(
        root,
        &[
            "--literal-pathspecs",
            "ls-tree",
            "-rz",
            "HEAD",
            "--",
            relative,
        ],
    )?;
    if !index.is_empty() || !tree.is_empty() {
        return Err(format!(
            "retained output contains committed/index source: {relative}"
        ));
    }
    Ok(())
}
