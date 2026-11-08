//! Bounded, process-safe JSONL files. Only the explicitly selected log family is touched.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const MAX_FILES: usize = 64;

pub struct RollingFile {
    directory: PathBuf,
    max_bytes: u64,
    max_files: usize,
}

fn regular(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "log member must be a regular file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

impl RollingFile {
    /// `directory` is exclusively adopted for this program-owned log family.
    /// `max_files` includes the current file. Existing excess members are retired on open.
    pub fn open(directory: &Path, max_bytes: u64, max_files: usize) -> io::Result<Self> {
        if max_bytes == 0 || !(1..=MAX_FILES).contains(&max_files) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log bounds must be positive; max_files must be at most 64",
            ));
        }
        // Walk explicit components only; never follow a directory alias during disposal.
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
        let writer = Self {
            directory: directory.to_owned(),
            max_bytes,
            max_files,
        };
        let _lock = writer.lock()?;
        // Validate all reserved slots before deleting any, including a changed policy's excess.
        for index in 0..MAX_FILES {
            regular(&writer.path(index))?;
        }
        for index in max_files..MAX_FILES {
            let path = writer.path(index);
            if regular(&path)? {
                fs::remove_file(path)?;
            }
        }
        Ok(writer)
    }

    fn path(&self, index: usize) -> PathBuf {
        self.directory.join(if index == 0 {
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
        for index in 0..self.max_files {
            regular(&self.path(index))?;
        }
        let oldest = self.path(self.max_files - 1);
        if regular(&oldest)? {
            fs::remove_file(oldest)?;
        }
        for index in (1..self.max_files).rev() {
            let source = self.path(index - 1);
            if regular(&source)? {
                fs::rename(source, self.path(index))?;
            }
        }
        Ok(())
    }
}

impl Write for RollingFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if bytes.len() as u64 > self.max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log record exceeds max_bytes; record was not written",
            ));
        }
        let _lock = self.lock()?;
        let path = self.path(0);
        let length = if regular(&path)? {
            fs::metadata(&path)?.len()
        } else {
            0
        };
        if length > self.max_bytes - bytes.len() as u64 {
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
