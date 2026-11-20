//! 固定归档的本地复用；摘要来自现役声明，网络与进程仍用已有工具。
use crate::{
    bindings::{self, Environment},
    construction::BuildFailure,
    profile, verified_bindings as policy,
};
use chrono_harness::file_identity;
use serde::Serialize;
use std::{
    fs, io,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
pub struct Archive {
    pub status: &'static str,
    pub path: String,
    pub sha256: String,
}

pub fn installation(
    root: &Path,
    checksum: &str,
    environment: &Environment,
) -> Result<(PathBuf, String), String> {
    use std::ffi::OsStr;
    let declaration = profile::read(root, checksum, b".chrono-harness/ci/")?;
    let text = std::str::from_utf8(&declaration).map_err(|error| error.to_string())?;
    let digest = text
        .trim_end_matches('\n')
        .strip_suffix("  package.zip")
        .ok_or("expected one pinned package.zip checksum")?;
    if !policy::valid_sha256(digest.as_bytes()) {
        return Err("invalid Verus package digest".into());
    }
    let storage = PathBuf::from(
        environment
            .get(OsStr::new("CHRONO_TOOLCHAIN_CACHE"))
            .ok_or("missing shared toolchain directory")?,
    );
    if !storage.is_absolute() || storage.starts_with(root) {
        return Err("toolchain directory must be outside the checkout".into());
    }
    fs::create_dir_all(&storage).map_err(|e| e.to_string())?;
    let storage = storage.canonicalize().map_err(|e| e.to_string())?;
    if storage.starts_with(root) {
        return Err("shared toolchain directory resolves inside checkout".into());
    }
    let directory = chrono_harness::no_symlink_parents(&storage, &format!("verus/{digest}"))?;
    Ok((directory, digest.to_owned()))
}

pub fn prepare(
    root: &Path,
    checksum: &str,
    curl: &Path,
    headers: &str,
    url: &str,
    environment: &Environment,
) -> Result<Archive, BuildFailure> {
    let (directory, expected) = installation(root, checksum, environment)?;
    if !policy::byte_prefix(url.as_bytes(), b"https://") {
        return Err("invalid HTTPS URL".into());
    }
    let destination = chrono_harness::no_symlink_parents(&directory, "package.zip")?;
    let archive = destination.to_str().ok_or("non UTF-8 archive path")?;
    let header_path = profile::path(root, headers, b".chrono-harness/state/")?;
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            let digest = file_identity(&destination)?.0;
            if policy::archive_matches(digest.as_bytes(), expected.as_bytes()) {
                return Ok(Archive {
                    status: "cache-hit",
                    path: archive.into(),
                    sha256: digest,
                });
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string().into()),
    }
    let parent = destination.parent().ok_or("missing archive parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::create_dir_all(header_path.parent().ok_or("missing headers parent")?)
        .map_err(|error| error.to_string())?;
    // 同一共享卷内验证并原子发布，工作树只保存本次响应头。
    let temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    let output = temporary
        .path()
        .to_str()
        .ok_or("non UTF-8 archive temporary path")?;
    let headers = header_path.to_str().ok_or("non UTF-8 headers path")?;
    let status = bindings::command(
        root,
        curl,
        &[
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--tlsv1.2",
            "--header",
            "Accept: application/octet-stream",
            "--dump-header",
            headers,
            "--output",
            output,
            url,
        ],
        environment,
    )?
    .status()
    .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(BuildFailure {
            message: format!("archive download failed: {status}"),
            exit_code: policy::process_exit(status.code(), status.signal()),
        });
    }
    let digest = file_identity(temporary.path())?.0;
    if !policy::archive_matches(digest.as_bytes(), expected.as_bytes()) {
        return Err("downloaded archive checksum mismatch".into());
    }
    temporary
        .persist(&destination)
        .map_err(|error| error.to_string())?;
    Ok(Archive {
        status: "downloaded",
        path: archive.into(),
        sha256: digest,
    })
}

/// 完成解包后才发布整个目录。相同版本的并发启动共用一份已发布安装。
pub fn extract(
    root: &Path,
    checksum: &str,
    tar: &Path,
    environment: &Environment,
) -> Result<(), BuildFailure> {
    let (directory, expected) = installation(root, checksum, environment)?;
    let archive = chrono_harness::no_symlink_parents(&directory, "package.zip")?;
    if file_identity(&archive)?.0 != expected {
        return Err("shared archive checksum mismatch".into());
    }
    let target = chrono_harness::no_symlink_parents(&directory, "toolchain")?;
    let lock = fs::File::open(&directory).map_err(|e| e.to_string())?;
    lock.lock().map_err(|e| e.to_string())?;
    if !target.try_exists().map_err(|e| e.to_string())? {
        let stage = directory.join("extracting");
        if stage.try_exists().map_err(|e| e.to_string())? {
            fs::remove_dir_all(&stage).map_err(|e| e.to_string())?;
        }
        fs::create_dir(&stage).map_err(|e| e.to_string())?;
        let status = bindings::command(
            root,
            tar,
            &[
                "-xf",
                archive.to_str().ok_or("non UTF-8 archive")?,
                "-C",
                stage.to_str().ok_or("non UTF-8 staging path")?,
                "--strip-components",
                "1",
                "--no-same-owner",
            ],
            environment,
        )?
        .status()
        .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(BuildFailure {
                message: format!("Verus extraction failed: {status}"),
                exit_code: policy::process_exit(status.code(), status.signal()),
            });
        }
        validate_installation(&stage)?;
        fs::rename(&stage, &target).map_err(|e| e.to_string())?;
    }
    validate_installation(&target)?;
    let bin = profile::path(root, ".chrono-harness/bin", b".chrono-harness/")?;
    fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let link = bin.join("verus-compiler");
    let executable = target.join("verus");
    match fs::read_link(&link) {
        Ok(existing) if existing == executable => return Ok(()),
        Ok(_) => (),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.to_string().into()),
    }
    let stage = tempfile::tempdir_in(&bin).map_err(|e| e.to_string())?;
    let selected = stage.path().join("verus-compiler");
    std::os::unix::fs::symlink(executable, &selected).map_err(|e| e.to_string())?;
    fs::rename(&selected, link).map_err(|e| e.to_string())?;
    Ok(())
}

fn validate_installation(directory: &Path) -> Result<(), String> {
    for member in [
        "cargo-verus",
        "verus",
        "rust_verify",
        "z3",
        "libvstd.rlib",
        "vstd.vir",
    ] {
        if !chrono_harness::no_symlink_parents(directory, member)?.is_file() {
            return Err(format!("incomplete shared Verus installation: {member}"));
        }
    }
    Ok(())
}
