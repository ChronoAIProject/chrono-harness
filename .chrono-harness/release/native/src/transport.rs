//! 发布单执行物的流式 tar。数据容量由文件系统承载，不经过 JSON 字节数组。
use crate::verified_recipe as policy;
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableIdentity {
    pub sha256: String,
    pub size: u64,
    pub mode: u32,
}

impl ExecutableIdentity {
    pub fn from_receipt(value: &Value) -> Result<Self, String> {
        let sha256 = value["sha256"]
            .as_str()
            .filter(|s| chrono_identifiers::valid_digest_text(s))
            .ok_or_else(|| error("invalid executable digest"))?
            .to_owned();
        let size = value["size"]
            .as_u64()
            .ok_or_else(|| error("invalid executable size"))?;
        let mode = value["mode"]
            .as_u64()
            .filter(|m| *m <= 0o7777)
            .ok_or_else(|| error("invalid executable mode"))? as u32;
        if !policy::executable_file(true, mode) {
            return Err(error("missing executable mode"));
        }
        Ok(Self { sha256, size, mode })
    }

    pub fn receipt(&self, path: &str) -> Value {
        json!({"path": path, "sha256": self.sha256, "size": self.size,
            "mode": self.mode, "error": null})
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveIdentity {
    pub sha256: String,
    pub size: u64,
}

impl ArchiveIdentity {
    pub fn receipt(&self) -> Value {
        json!({"path": "executable.tar", "sha256": self.sha256, "size": self.size})
    }
}

pub struct PackedExecutable {
    pub executable: ExecutableIdentity,
    pub archive: ArchiveIdentity,
}

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_TRANSPORT: {message}")
}

/// 调用方提供自己的源码／clone 根与登记路径；不扫描或推断资产。
pub fn observe(root: &Path, relative: &str) -> Result<ExecutableIdentity, String> {
    let path = chrono_harness::no_symlink_parents(root, relative)?;
    let metadata = fs::symlink_metadata(&path).map_err(error)?;
    let mode = metadata.permissions().mode() & 0o7777;
    if !policy::executable_file(metadata.is_file(), mode) {
        return Err(error(format!("not a regular executable: {relative}")));
    }
    let (sha256, size) = chrono_harness::file_identity(&path)?;
    Ok(ExecutableIdentity { sha256, size, mode })
}

/// archive 可以位于源码根外，必须不存在。失败的部分原件由发布单元保管。
pub fn pack(root: &Path, relative: &str, archive: &Path) -> Result<PackedExecutable, String> {
    let executable = observe(root, relative)?;
    let source = chrono_harness::no_symlink_parents(root, relative)?;
    let mut input = File::open(source).map_err(error)?;
    let metadata = input.metadata().map_err(error)?;
    if !policy::executable_entry(
        b"executable",
        metadata.is_file(),
        metadata.len(),
        metadata.permissions().mode() & 0o7777,
        executable.size,
        executable.mode,
    ) {
        return Err(error("source metadata changed before archiving"));
    }
    let output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(archive)
        .map_err(error)?;
    let mut builder = tar::Builder::new(output);
    let mut header = tar::Header::new_gnu();
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(executable.mode);
    header.set_size(executable.size);
    header.set_cksum();
    builder
        .append_data(&mut header, "executable", &mut input)
        .map_err(error)?;
    let mut output = builder.into_inner().map_err(error)?;
    output.flush().map_err(error)?;
    drop(output);
    let (sha256, size) = chrono_harness::file_identity(archive)?;
    Ok(PackedExecutable {
        executable,
        archive: ArchiveIdentity { sha256, size },
    })
}

/// 先核完整归档，再流式核单一文件，最后原子替换登记的消费路径。
/// 原生 artifact/run/attempt 和 receipt 由上层在调用前核对。
pub fn import(
    archive: &Path,
    expected_archive: &ArchiveIdentity,
    expected: &ExecutableIdentity,
    root: &Path,
    relative: &str,
) -> Result<ExecutableIdentity, String> {
    if !fs::symlink_metadata(archive).map_err(error)?.is_file() {
        return Err(error("archive is not a regular file"));
    }
    let (sha256, size) = chrono_harness::file_identity(archive)?;
    if !policy::same_content(
        &sha256,
        size,
        &expected_archive.sha256,
        expected_archive.size,
    ) {
        return Err(error("selected executable archive mismatch"));
    }
    let mut package = tar::Archive::new(File::open(archive).map_err(error)?);
    let mut entries = package.entries().map_err(error)?;
    let mut entry = entries
        .next()
        .ok_or_else(|| error("missing executable entry"))?
        .map_err(error)?;
    let mode = entry.header().mode().map_err(error)?;
    if !policy::executable_entry(
        entry.path_bytes().as_ref(),
        entry.header().entry_type().is_file(),
        entry.size(),
        mode,
        expected.size,
        expected.mode,
    ) || !policy::executable_file(true, mode)
    {
        return Err(error("invalid executable archive member/path/mode"));
    }
    let destination = chrono_harness::prepare_publication(root, relative)?;
    let parent = destination
        .parent()
        .ok_or_else(|| error("missing destination parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    let (sha256, size) = chrono_harness::copy_hashed(&mut entry, temporary.as_file_mut())?;
    drop(entry);
    match entries.next() {
        Some(Err(failure)) => return Err(error(failure)),
        Some(Ok(_)) => return Err(error("multiple executable archive entries")),
        None => {}
    }
    temporary.as_file_mut().flush().map_err(error)?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode))
        .map_err(error)?;
    let actual_mode = temporary
        .as_file()
        .metadata()
        .map_err(error)?
        .permissions()
        .mode()
        & 0o7777;
    if !policy::same_executable(
        &sha256,
        size,
        actual_mode,
        &expected.sha256,
        expected.size,
        expected.mode,
    ) {
        return Err(error("transported executable identity mismatch"));
    }
    chrono_harness::no_symlink_parents(root, relative)?;
    temporary.persist(destination).map_err(error)?;
    Ok(ExecutableIdentity {
        sha256,
        size,
        mode: actual_mode,
    })
}
