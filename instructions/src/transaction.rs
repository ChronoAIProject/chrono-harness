use std::fs::{self, Permissions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::{Builder, TempPath};

pub(super) struct Snapshot {
    pub bytes: Vec<u8>,
    pub permissions: Permissions,
}

pub(super) struct Change {
    pub path: PathBuf,
    pub before: Option<Snapshot>,
    pub after: After,
    pub permissions: Option<Permissions>,
}

pub(super) enum After {
    Regular(Vec<u8>),
    RelativeAlias,
}

impl Change {
    pub fn new(path: PathBuf, before: Option<Snapshot>, after: Vec<u8>) -> Self {
        let permissions = before.as_ref().map(|s| s.permissions.clone());
        Self {
            path,
            before,
            after: After::Regular(after),
            permissions,
        }
    }

    pub fn alias(path: PathBuf, before: Option<Snapshot>) -> Self {
        Self {
            path,
            before,
            after: After::RelativeAlias,
            permissions: None,
        }
    }

    pub fn unchanged(&self) -> bool {
        matches!((&self.before, &self.after), (Some(before), After::Regular(after)) if before.bytes == *after)
    }
}

#[derive(Default)]
pub(super) struct Fault {
    pub after: Option<usize>,
    pub rollback: Option<usize>,
}

struct Staged {
    next: TempPath,
    original: Option<TempPath>,
}

fn stage(path: &Path, bytes: &[u8], permissions: Option<&Permissions>) -> Result<TempPath, String> {
    let mut temp = Builder::new()
        .prefix(".chrono-instructions-")
        .tempfile_in(path.parent().expect("destination has parent"))
        .map_err(|e| format!("stage {}: {e}", path.display()))?;
    temp.write_all(bytes)
        .and_then(|()| temp.flush())
        .and_then(|()| {
            if let Some(p) = permissions {
                temp.as_file().set_permissions(p.clone())
            } else {
                Ok(())
            }
        })
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| {
            format!(
                "stage {} (temporary {}): {e}",
                path.display(),
                temp.path().display()
            )
        })?;
    Ok(temp.into_temp_path())
}

fn stage_alias(path: &Path) -> Result<TempPath, String> {
    #[cfg(unix)]
    {
        let temp = Builder::new()
            .prefix(".chrono-instructions-")
            .tempfile_in(path.parent().expect("destination has parent"))
            .map_err(|e| format!("stage alias {}: {e}", path.display()))?
            .into_temp_path();
        fs::remove_file(&temp)
            .and_then(|()| std::os::unix::fs::symlink("CLAUDE.md", &temp))
            .map_err(|e| format!("stage alias {}: {e}", path.display()))?;
        // Never chmod or open the symlink: its target may already exist.
        Ok(temp)
    }
    #[cfg(not(unix))]
    Err(format!(
        "{}: relative alias publication requires Unix",
        path.display()
    ))
}

fn clean(staged: Vec<Staged>) -> Vec<String> {
    let mut errors = Vec::new();
    for item in staged {
        for temp in [Some(item.next), item.original].into_iter().flatten() {
            let name = temp.display().to_string();
            // A successful rename has already removed the temporary name.
            if let Err(e) = temp.close() {
                if e.kind() != std::io::ErrorKind::NotFound {
                    errors.push(format!("{name}: {e}"));
                }
            }
        }
    }
    errors
}

fn remove_directories(directories: &[PathBuf], errors: &mut Vec<String>) {
    for path in directories.iter().rev() {
        if let Err(e) = fs::remove_dir(path) {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
}

pub(super) fn publish(
    changes: Vec<Change>,
    directories: Vec<PathBuf>,
    fault: Fault,
) -> Result<Vec<PathBuf>, String> {
    if changes.is_empty() {
        return Ok(Vec::new());
    }
    let mut made = Vec::new();
    let mut staged = Vec::new();
    let preparation = (|| {
        for path in directories {
            fs::create_dir(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
            made.push(path);
        }
        for change in &changes {
            let next = match &change.after {
                After::Regular(bytes) => stage(&change.path, bytes, change.permissions.as_ref())?,
                After::RelativeAlias => stage_alias(&change.path)?,
            };
            // Retain next in the cleanup set even if staging its backup fails.
            staged.push(Staged {
                next,
                original: None,
            });
            if let Some(snapshot) = &change.before {
                let backup = stage(&change.path, &snapshot.bytes, Some(&snapshot.permissions))?;
                staged.last_mut().expect("just pushed").original = Some(backup);
            }
        }
        Ok::<(), String>(())
    })();
    if let Err(e) = preparation {
        let mut cleanup = clean(staged);
        remove_directories(&made, &mut cleanup);
        return Err(format!(
            "{e}; published=[]; unrestored=[]; cleanup={cleanup:?}"
        ));
    }
    for (index, change) in changes.iter().enumerate() {
        let result = if fault.after == Some(index) {
            Err(std::io::Error::other("injected publication failure"))
        } else {
            fs::rename(&staged[index].next, &change.path)
        };
        if let Err(e) = result {
            let mut unrestored = Vec::new();
            let mut recovery = Vec::new();
            for j in (0..index).rev() {
                let path = &changes[j].path;
                let result = if fault.rollback == Some(j) {
                    Err(std::io::Error::other("injected rollback failure"))
                } else if let Some(original) = &staged[j].original {
                    fs::rename(original, path)
                } else {
                    fs::remove_file(path)
                };
                if let Err(e) = result {
                    unrestored.push(path.clone());
                    recovery.push(format!("{}: {e}", path.display()));
                    if let Some(original) = staged[j].original.take() {
                        let backup_name = original.display().to_string();
                        match original.keep() {
                            Ok(_) => recovery.push(format!("original retained at {backup_name}")),
                            Err(e) => recovery.push(format!("retain {backup_name}: {e}")),
                        }
                    }
                }
            }
            let mut cleanup = clean(staged);
            remove_directories(&made, &mut cleanup);
            let published: Vec<_> = changes[..index].iter().map(|c| &c.path).collect();
            return Err(format!(
                "publish {}: {e}; published={published:?}; unrestored={unrestored:?}; recovery={recovery:?}; cleanup={cleanup:?}",
                change.path.display()
            ));
        }
    }
    let paths: Vec<_> = changes.into_iter().map(|c| c.path).collect();
    let cleanup = clean(staged);
    if !cleanup.is_empty() {
        return Err(format!(
            "publication complete but temporary cleanup failed; published={paths:?}; cleanup={cleanup:?}"
        ));
    }
    Ok(paths)
}
