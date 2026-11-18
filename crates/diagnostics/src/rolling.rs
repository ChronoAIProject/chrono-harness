//! Bounded, process-safe JSONL files. Only the explicitly selected log family is touched.
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn generation_deadline(born: u64, seconds: u64) -> (deadline: Option<u64>)
    ensures
        if seconds == 0 || born as int + seconds as int > u64::MAX {
            deadline.is_none()
        } else {
            deadline == Some((born + seconds) as u64)
        },
{
    if seconds == 0 || seconds > u64::MAX - born {
        None
    } else {
        Some(born + seconds)
    }
}

pub fn generation_expired(deadline: u64, observed: u64) -> (expired: bool)
    ensures expired == (observed >= deadline),
{
    observed >= deadline
}

}
#[cfg(not(verus_keep_ghost))]
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};

#[cfg(not(verus_keep_ghost))]
pub use crate::verified_rolling::MAX_FILES;

#[cfg(not(verus_keep_ghost))]
const GENERATION_SCHEMA: &str = "chrono-runtime-log-generation/v1";
#[cfg(not(verus_keep_ghost))]
const GENERATION_PREFIX: &[u8] = b"{\"schema\":\"chrono-runtime-log-generation/v1\"";
#[cfg(not(verus_keep_ghost))]
const HEADER_LIMIT: u64 = 128;

#[cfg(not(verus_keep_ghost))]
pub struct RollingFile {
    directory: PathBuf,
    max_bytes: u64,
    max_files: usize,
    keep_seconds: Option<u64>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Generation {
    schema: String,
    deadline: u64,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Default, Serialize)]
pub struct ExpiredLogs {
    pub removed: usize,
    pub legacy: usize,
    pub busy: bool,
    pub errors: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn unix_seconds() -> io::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|age| age.as_secs())
        .map_err(io::Error::other)
}

#[cfg(not(verus_keep_ghost))]
fn regular(path: &Path) -> io::Result<bool> {
    crate::verified_rolling::regular_member(
        fs::symlink_metadata(path).map(|metadata| metadata.is_file()),
        |error: &io::Error| error.kind() == io::ErrorKind::NotFound,
        || {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "log member must be a regular file",
            )
        },
    )
}

#[cfg(not(verus_keep_ghost))]
impl RollingFile {
    /// `directory` is exclusively adopted for this program-owned log family.
    /// `max_files` includes the current file. Existing excess members are retired on open.
    pub fn open(directory: &Path, max_bytes: u64, max_files: usize) -> io::Result<Self> {
        Self::open_with_window(directory, max_bytes, max_files, None)
    }

    pub(crate) fn open_with_window(
        directory: &Path,
        max_bytes: u64,
        max_files: usize,
        keep_seconds: Option<u64>,
    ) -> io::Result<Self> {
        if !crate::verified_rolling::valid_bounds(max_bytes, max_files) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log bounds must be positive; max_files must be at most 64",
            ));
        }
        if keep_seconds == Some(0) {
            return Err(invalid("log retention must be positive"));
        }
        Self::check_directory(directory, true)?;
        let writer = Self {
            directory: directory.to_owned(),
            max_bytes,
            max_files,
            keep_seconds,
        };
        let _lock = writer.lock()?;
        // Validate all reserved slots before deleting any, including a changed policy's excess.
        crate::verified_rolling::adopt_members(
            (),
            max_files,
            |(), index| ((), regular(&writer.path(index))),
            |(), index| ((), fs::remove_file(writer.path(index))),
        )
        .1?;
        Ok(writer)
    }

    // Walk explicit components only; never follow a directory alias during disposal.
    fn check_directory(directory: &Path, create: bool) -> io::Result<bool> {
        let mut checked = PathBuf::new();
        for component in directory.components() {
            checked.push(component);
            match fs::symlink_metadata(&checked) {
                Ok(metadata) if metadata.is_dir() => (),
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "log directory contains an alias or non-directory",
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if !create {
                        return Ok(false);
                    }
                    match fs::create_dir(&checked) {
                        Ok(()) => (),
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                            if !fs::symlink_metadata(&checked)?.is_dir() {
                                return Err(error);
                            }
                        }
                        Err(error) => return Err(error),
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }

    fn path(&self, index: usize) -> PathBuf {
        Self::member(&self.directory, index)
    }

    fn member(directory: &Path, index: usize) -> PathBuf {
        directory.join(if index == 0 {
            "runtime.jsonl".into()
        } else {
            format!("runtime.{index}.jsonl")
        })
    }

    fn lock(&self) -> io::Result<File> {
        let path = self.directory.join("runtime.lock");
        regular(&path)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)?;
        // A separate open description per write prevents one thread from releasing another's lock.
        // File::lock blocks in the OS and is released by close, including process termination.
        file.lock()?;
        Ok(file)
    }

    fn rotate(&self) -> io::Result<()> {
        crate::verified_rolling::rotate_members(
            (),
            self.max_files,
            |(), index| ((), regular(&self.path(index))),
            |(), index| ((), fs::remove_file(self.path(index))),
            |(), source, target| ((), fs::rename(self.path(source), self.path(target))),
        )
        .1
    }

    fn generation(file: &File) -> io::Result<Option<Generation>> {
        let mut header = Vec::new();
        BufReader::new(file)
            .take(HEADER_LIMIT)
            .read_until(b'\n', &mut header)?;
        // A former v1 log opens directly with a chrono-log/v1 record.
        if !header.starts_with(GENERATION_PREFIX) {
            return Ok(None);
        }
        if !header.ends_with(b"\n") {
            return Err(invalid("incomplete log generation header"));
        }
        serde_json::from_slice(&header)
            .map(Some)
            .map_err(io::Error::other)
    }

    fn clear_pending(directory: &Path) -> io::Result<()> {
        let path = directory.join("runtime.pending");
        if regular(&path)? {
            let mut bytes = Vec::new();
            File::open(&path)?
                .take(HEADER_LIMIT + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > HEADER_LIMIT
                || (!GENERATION_PREFIX.starts_with(&bytes) && !bytes.starts_with(GENERATION_PREFIX))
            {
                return Err(invalid("unknown file at the log generation staging path"));
            }
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn write_expiring(&self, bytes: &[u8], keep: u64) -> io::Result<usize> {
        let now = unix_seconds()?;
        let deadline = generation_deadline(now, keep)
            .ok_or_else(|| invalid("log deadline exceeds supported time range"))?;
        let generation = Generation {
            schema: GENERATION_SCHEMA.into(),
            deadline,
        };
        let mut header = serde_json::to_vec(&generation).map_err(io::Error::other)?;
        header.push(b'\n');
        let available = self.max_bytes.saturating_sub(header.len() as u64);
        if !crate::verified_rolling::record_fits(bytes.len() as u64, available) {
            return Err(invalid("log record and generation header exceed max_bytes"));
        }
        Self::clear_pending(&self.directory)?;
        let path = self.path(0);
        if regular(&path)? {
            let file = OpenOptions::new().read(true).append(true).open(&path)?;
            let generation = Self::generation(&file)?;
            if let Some(generation) = generation {
                if generation_expired(generation.deadline, now) {
                    fs::remove_file(&path)?;
                } else if !crate::verified_rolling::needs_rotation(
                    file.metadata()?.len(),
                    self.max_bytes,
                    bytes.len() as u64,
                ) {
                    (&file).write_all(bytes)?;
                    (&file).flush()?;
                    return Ok(bytes.len());
                } else {
                    self.rotate()?;
                }
            } else {
                self.rotate()?;
            }
        }
        // This exact staging name holds only the generation header. A failed
        // or interrupted first write leaves no unreported business row there.
        let pending = self.directory.join("runtime.pending");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(&header)?;
        file.sync_all()?;
        fs::rename(pending, path)?;
        file.write_all(bytes)?;
        file.flush()?;
        Ok(bytes.len())
    }

    pub(crate) fn expire(
        directory: &Path,
        observed: u64,
        keep_seconds: u64,
        max_bytes: u64,
    ) -> io::Result<ExpiredLogs> {
        let mut result = ExpiredLogs::default();
        if !Self::check_directory(directory, false)? {
            return Ok(result);
        }
        let lock_path = directory.join("runtime.lock");
        if !regular(&lock_path)? {
            return Err(invalid("log family has no adopted writer lock"));
        }
        let lock = OpenOptions::new().write(true).open(lock_path)?;
        match lock.try_lock() {
            Ok(()) => (),
            Err(fs::TryLockError::WouldBlock) => {
                result.busy = true;
                return Ok(result);
            }
            Err(fs::TryLockError::Error(error)) => return Err(error),
        }
        if let Err(error) = Self::clear_pending(directory) {
            result.errors.push(format!("runtime.pending: {error}"));
        }
        for index in 0..MAX_FILES {
            let path = Self::member(directory, index);
            if let Err(error) =
                Self::expire_member(&path, observed, keep_seconds, max_bytes, &mut result)
            {
                result.errors.push(format!("{}: {error}", path.display()));
            }
        }
        Ok(result)
    }

    fn expire_member(
        path: &Path,
        observed: u64,
        keep_seconds: u64,
        max_bytes: u64,
        result: &mut ExpiredLogs,
    ) -> io::Result<()> {
        if !regular(path)? {
            return Ok(());
        }
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let expired = match Self::generation(&file)? {
            Some(generation) => generation_expired(generation.deadline, observed),
            None => {
                let born = metadata
                    .created()?
                    .duration_since(UNIX_EPOCH)
                    .map_err(io::Error::other)?
                    .as_secs();
                let deadline = generation_deadline(born, keep_seconds)
                    .ok_or_else(|| invalid("legacy log deadline exceeds supported time range"))?;
                if !generation_expired(deadline, observed)
                    || !Self::complete_legacy_log(&file, metadata.len(), max_bytes)?
                {
                    result.legacy += 1;
                    return Ok(());
                }
                true
            }
        };
        if expired {
            // All adopted writers and maintenance hold the family lock. Recheck
            // the opened generation before removing its directory entry.
            let current = fs::symlink_metadata(path)?;
            if !current.is_file()
                || current.len() != metadata.len()
                || current.modified()? != metadata.modified()?
                || current.created().ok() != metadata.created().ok()
            {
                return Err(invalid("log member changed during expiration"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if current.dev() != metadata.dev() || current.ino() != metadata.ino() {
                    return Err(invalid("log member was replaced during expiration"));
                }
            }
            fs::remove_file(path)?;
            result.removed += 1;
        }
        Ok(())
    }

    fn complete_legacy_log(mut file: &File, length: u64, max_bytes: u64) -> io::Result<bool> {
        use std::io::Seek;

        if length == 0 || length > max_bytes {
            return Ok(false);
        }
        // Reading the header advanced the same open file description. Bound the
        // entire validation to the observed extent of this locked generation.
        file.rewind()?;
        let mut reader = BufReader::new(file.take(length));
        let mut row = Vec::new();
        while reader.read_until(b'\n', &mut row)? != 0 {
            if !row.ends_with(b"\n")
                || serde_json::from_slice::<crate::logging::LogRecord>(&row).is_err()
            {
                return Ok(false);
            }
            row.clear();
        }
        Ok(reader.into_inner().limit() == 0)
    }
}

#[cfg(not(verus_keep_ghost))]
impl Write for RollingFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if !crate::verified_rolling::record_fits(bytes.len() as u64, self.max_bytes) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log record exceeds max_bytes; record was not written",
            ));
        }
        let _lock = self.lock()?;
        if let Some(keep_seconds) = self.keep_seconds {
            return self.write_expiring(bytes, keep_seconds);
        }
        let path = self.path(0);
        let length = if regular(&path)? {
            fs::metadata(&path)?.len()
        } else {
            0
        };
        if crate::verified_rolling::needs_rotation(length, self.max_bytes, bytes.len() as u64) {
            self.rotate()?;
        }
        // Reopen after acquiring the family lock so a different process's rotation cannot
        // strand this writer on an old inode. Logger supplies one whole JSONL record.
        let mut file = OpenOptions::new().append(true).create(true).open(path)?;
        file.write_all(bytes)?;
        file.flush()?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // There is no userspace buffer or persistent file handle. No fsync claim is made.
        Ok(())
    }
}
