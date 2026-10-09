//! Stable, identity-bound kernel capabilities for an opted-in enrollment.
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(crate) struct Lease {
    file: File,
    path: PathBuf,
    id: String,
    created: bool,
    #[cfg(unix)]
    _scope: chrono_harness::process_fds::Scope,
}
pub(crate) fn identity(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("ownership identity {}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("ownership lease must be a physical regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        Err("kernel ownership is unsupported on this platform".into())
    }
}
impl Lease {
    pub(crate) fn acquire(
        path: &Path,
        expected: Option<&str>,
        create: bool,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        Self::acquire_observed(path, expected, create, exclusive, wait, || {})
    }

    pub(crate) fn acquire_observed(
        path: &Path,
        expected: Option<&str>,
        create: bool,
        exclusive: bool,
        wait: Option<Duration>,
        after_create: impl FnOnce(),
    ) -> Result<Option<Self>, String> {
        #[cfg(not(unix))]
        {
            let _ = (path, expected, create, exclusive, wait, after_create);
            return Err("kernel ownership is unsupported on this platform".into());
        }
        #[cfg(unix)]
        {
            use std::os::{
                fd::{AsFd, AsRawFd},
                unix::fs::{MetadataExt, OpenOptionsExt},
            };
            let mut options = OpenOptions::new();
            options
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            let (file, created) = if create {
                match options.create_new(true).open(path) {
                    Ok(file) => (file, true),
                    Err(e)
                        if e.kind() == std::io::ErrorKind::AlreadyExists && expected.is_none() =>
                    {
                        (
                            options
                                .create_new(false)
                                .open(path)
                                .map_err(|e| e.to_string())?,
                            false,
                        )
                    }
                    Err(e) => return Err(e.to_string()),
                }
            } else {
                (
                    options
                        .open(path)
                        .map_err(|e| format!("ownership lease unavailable; preserve work: {e}"))?,
                    false,
                )
            };
            if created {
                after_create();
            }
            let metadata = file.metadata().map_err(|e| e.to_string())?;
            if !metadata.is_file() {
                return Err("ownership lease must be a regular file".into());
            }
            let id = format!("{}:{}", metadata.dev(), metadata.ino());
            if expected.is_some_and(|v| v != id) || identity(path)? != id {
                return Err("ownership lease identity changed; preserve work".into());
            }
            let began = Instant::now();
            loop {
                if unsafe {
                    libc::flock(
                        file.as_raw_fd(),
                        (if exclusive {
                            libc::LOCK_EX
                        } else {
                            libc::LOCK_SH
                        }) | libc::LOCK_NB,
                    )
                } == 0
                {
                    break;
                }
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(error.to_string());
                }
                match wait {
                    None => return Ok(None),
                    Some(bound) if began.elapsed() >= bound => return Err("automatic cleanup admission remains busy; kernel ownership was not expired".into()),
                    Some(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
            if identity(path)? != id {
                return Err("ownership lease replaced during acquisition".into());
            }
            let scope = chrono_harness::process_fds::Scope::new(&[file.as_fd()])?;
            Ok(Some(Self {
                file,
                path: path.into(),
                id,
                created,
                _scope: scope,
            }))
        }
    }
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    pub(crate) fn newly_created(&self) -> bool {
        self.created
    }
    pub(crate) fn stable(&self) -> Result<(), String> {
        let _ = self.file.metadata().map_err(|e| e.to_string())?;
        if identity(&self.path)? != self.id {
            return Err("held ownership lease identity changed".into());
        }
        Ok(())
    }
}
// Closing the final descriptor releases ownership. Never LOCK_UN: descendants
// share the same open description and must retain protection after this owner dies.
