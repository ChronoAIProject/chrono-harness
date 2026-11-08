//! 固定归档的本地复用；摘要来自现役声明，网络与进程仍用已有工具。
use crate::{
    bindings::{self, Environment},
    construction::BuildFailure,
    profile, verified_bindings as policy,
};
use chrono_harness::file_identity;
use serde::Serialize;
use std::{fs, io, os::unix::process::ExitStatusExt, path::Path};

#[derive(Debug, Serialize)]
pub struct Archive {
    pub status: &'static str,
    pub path: String,
    pub sha256: String,
}

pub fn prepare(
    root: &Path,
    checksum: &str,
    curl: &Path,
    headers: &str,
    url: &str,
    environment: &Environment,
) -> Result<Archive, BuildFailure> {
    let declaration = profile::read(root, checksum, b".chrono-harness/ci/")?;
    let text = std::str::from_utf8(&declaration).map_err(|error| error.to_string())?;
    let (expected, archive) = text
        .strip_suffix('\n')
        .unwrap_or(text)
        .split_once("  ")
        .ok_or("archive checksum requires one SHA256 and path record")?;
    if !policy::valid_sha256(expected.as_bytes())
        || !policy::byte_prefix(url.as_bytes(), b"https://")
    {
        return Err("invalid archive checksum or HTTPS URL".into());
    }
    let destination = profile::path(root, archive, b".chrono-harness/cache/")?;
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
