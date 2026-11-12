//! Stable, identity-bound kernel capabilities for an opted-in enrollment.
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct Lease {
    file: File,
    path: PathBuf,
    id: String,
    created: bool,
    #[cfg(unix)]
    _scope: crate::process_fds::Scope,
}

/// Metadata exclusion on the addressed directory itself. Removing an empty
/// registration does not leave a separate lock file or authorize a detached
/// waiter to publish into a replacement directory.
pub struct DirectoryLease {
    file: File,
    path: PathBuf,
    id: String,
    local: bool,
    owner_pid: u32,
    #[cfg(unix)]
    _scope: Option<crate::process_fds::Scope>,
}
impl DirectoryLease {
    pub fn acquire(
        path: &Path,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        Self::acquire_for(path, exclusive, wait, false)
    }
    /// Synchronous metadata operations have no descendant owner. Explicitly
    /// release their lock on return, including when an unrelated fork still
    /// holds an incidental copy of this open description before exec.
    pub fn acquire_local(
        path: &Path,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        Self::acquire_for(path, exclusive, wait, true)
    }
    fn acquire_for(
        path: &Path,
        exclusive: bool,
        wait: Option<Duration>,
        local: bool,
    ) -> Result<Option<Self>, String> {
        #[cfg(not(unix))]
        {
            let _ = (path, exclusive, wait);
            Err("kernel ownership is unsupported on this platform".into())
        }
        #[cfg(unix)]
        {
            use std::os::{
                fd::{AsFd, AsRawFd},
                unix::fs::{MetadataExt, OpenOptionsExt},
            };
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path)
                .map_err(|e| format!("directory ownership {}: {e}", path.display()))?;
            let m = file.metadata().map_err(|e| e.to_string())?;
            let id = format!("{}:{}", m.dev(), m.ino());
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
                    Some(bound) if began.elapsed() >= bound => {
                        return Err(format!(
                            "addressed directory remains busy at {}; ownership was not expired",
                            path.display()
                        ));
                    }
                    Some(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
            let scope = if local {
                None
            } else {
                Some(crate::process_fds::Scope::new(&[file.as_fd()])?)
            };
            let result = Self {
                file,
                path: path.into(),
                id,
                local,
                owner_pid: std::process::id(),
                _scope: scope,
            };
            result.stable()?;
            Ok(Some(result))
        }
    }
    pub fn stable(&self) -> Result<(), String> {
        let m = fs::symlink_metadata(&self.path).map_err(|e| e.to_string())?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err("held metadata directory changed".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if format!("{}:{}", m.dev(), m.ino()) != self.id {
                return Err("held metadata directory identity changed".into());
            }
        }
        self.file.metadata().map_err(|e| e.to_string())?;
        Ok(())
    }
}
impl Drop for DirectoryLease {
    fn drop(&mut self) {
        #[cfg(unix)]
        if crate::verified_status::release_local_directory(
            self.local,
            self.owner_pid,
            std::process::id(),
        ) {
            use std::os::fd::AsRawFd;
            // A forked copy must never unlock its parent's live operation.
            unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}
pub fn identity(path: &Path) -> Result<String, String> {
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
    pub fn acquire(
        path: &Path,
        expected: Option<&str>,
        create: bool,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        Self::acquire_observed(path, expected, create, exclusive, wait, || {})
    }

    pub fn acquire_observed(
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
                    Some(bound) if began.elapsed() >= bound => {
                        return Err(format!(
                            "automatic cleanup admission remains busy at {}; kernel ownership was not expired",
                            path.display()
                        ));
                    }
                    Some(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
            if identity(path)? != id {
                return Err("ownership lease replaced during acquisition".into());
            }
            let scope = crate::process_fds::Scope::new(&[file.as_fd()])?;
            Ok(Some(Self {
                file,
                path: path.into(),
                id,
                created,
                _scope: scope,
            }))
        }
    }
    pub fn capability(&self) -> Result<File, String> {
        self.stable()?;
        self.file.try_clone().map_err(|e| e.to_string())
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn newly_created(&self) -> bool {
        self.created
    }
    pub fn stable(&self) -> Result<(), String> {
        let _ = self.file.metadata().map_err(|e| e.to_string())?;
        if identity(&self.path)? != self.id {
            return Err("held ownership lease identity changed".into());
        }
        Ok(())
    }
}
// Transferred leases release ownership at final close. Never LOCK_UN those:
// descendants retain protection after the original owner dies. Local directory
// operations above explicitly have no descendant owner.
