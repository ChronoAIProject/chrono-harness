//! Compare an explicit Git tree/index inventory with literal filesystem objects.
use crate::{
    facts::{Entry, Tree},
    relative_path,
    verified_identifiers::{
        equal_text, git_inventory_records, git_inventory_termination, git_inventory_text,
    },
    verified_status::{
        CheckoutPhysicalObservation, checkout_changed_paths, checkout_entry_supported,
        checkout_hash_stream, checkout_index_changes, checkout_metadata_observation,
        checkout_object_hash, checkout_opened_file_matches, checkout_parent_observation,
        checkout_physical_targets, checkout_regular_file_matches, checkout_symbolic_link,
        parse_git_index_rows, parse_git_tree_rows, record_checkout_physical_observations,
    },
};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};

fn rows(raw: &[u8]) -> Result<Vec<&str>, String> {
    git_inventory_termination(raw)?;
    Ok(git_inventory_records(git_inventory_text(
        std::str::from_utf8(raw).ok(),
    )?))
}

pub fn tree(raw: &[u8]) -> Result<Tree, String> {
    parse_git_tree_rows(&rows(raw)?)
        .map_err(|error| error.render(|path| format!("invalid relative path: {path:?}")))
}

/// Index stages are compared as data; Git filters/configuration never decide equality.
pub fn changes(root: &Path, expected: &Tree, index: &[u8]) -> Result<Vec<String>, String> {
    let inventory = parse_git_index_rows(&rows(index)?)
        .map_err(|error| error.render(|path| format!("invalid relative path: {path:?}")))?;
    let mut changed = checkout_index_changes(expected, inventory.rows.as_slice());
    let targets = checkout_physical_targets(expected, &inventory.stages, &mut changed);
    let observations = targets
        .into_iter()
        .map(|(path, entry)| {
            matches_entry(root, path, entry).map(|matches| CheckoutPhysicalObservation {
                path: path.as_str(),
                matches,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    record_checkout_physical_observations(&mut changed, &observations);
    Ok(checkout_changed_paths(&changed))
}

fn physical(root: &Path, path: &str) -> Result<Option<PathBuf>, String> {
    relative_path(path)?;
    let mut current = root.to_path_buf();
    let mut parts = Path::new(path).components().peekable();
    while let Some(part) = parts.next() {
        current.push(part);
        if parts.peek().is_some() {
            if !checkout_parent_observation(
                fs::symlink_metadata(&current).map(|metadata| metadata.is_dir()),
                |error| error.kind() == io::ErrorKind::NotFound,
            )
            .map_err(|error| format!("checkout path {path}: {error}"))?
            {
                return Ok(None);
            }
        }
    }
    Ok(Some(current))
}

fn hash<D: Digest>(size: u64, source: &mut impl Read) -> Result<String, String> {
    let mut digest = D::new();
    digest.update(format!("blob {size}\0").as_bytes());
    let mut buffer = [0u8; 65536];
    checkout_hash_stream(
        size,
        source,
        digest,
        &mut buffer,
        |source: &mut _, buffer: &mut [u8; 65536]| {
            let result = Read::read(source, buffer).map_err(|error| error.to_string());
            (source, buffer, result)
        },
        AsRef::<[u8]>::as_ref,
        |mut digest: D, bytes: &[u8]| {
            digest.update(bytes);
            digest
        },
        |digest: D| {
            digest
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        },
    )
}
fn object_hash(oid: &str, size: u64, source: &mut impl Read) -> Result<String, String> {
    checkout_object_hash(oid, size, source, hash::<Sha1>, hash::<Sha256>)
}
fn matches_entry(root: &Path, name: &str, entry: &Entry) -> Result<bool, String> {
    if !checkout_entry_supported(entry.kind.as_str(), entry.mode.as_str()) {
        // Unsupported entries never match. Keep observations available so the
        // registration judge can emit its structured unsupported-input finding.
        return Ok(false);
    }
    let Some(path) = physical(root, name)? else {
        return Ok(false);
    };
    let Some(metadata) = checkout_metadata_observation(fs::symlink_metadata(&path), |error| {
        error.kind() == io::ErrorKind::NotFound
    })
    .map_err(|error| format!("checkout file {name}: {error}"))?
    else {
        return Ok(false);
    };
    if checkout_symbolic_link(entry.mode.as_str()) {
        if !metadata.is_symlink() {
            return Ok(false);
        }
        let target = fs::read_link(path).map_err(|e| e.to_string())?;
        let bytes = target.as_os_str().as_bytes();
        return Ok(equal_text(
            object_hash(&entry.oid, bytes.len() as u64, &mut &bytes[..])?.as_str(),
            entry.oid.as_str(),
        ));
    }
    if !checkout_regular_file_matches(
        metadata.is_file(),
        metadata.permissions().mode(),
        entry.mode.as_str(),
    ) {
        return Ok(false);
    }
    // Never open a symlink or block on a FIFO substituted after the type observation.
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| format!("checkout file {name}: {e}"))?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !checkout_opened_file_matches(
        opened.is_file(),
        opened.len(),
        opened.permissions().mode(),
        metadata.len(),
        metadata.permissions().mode(),
    ) {
        return Err(format!(
            "checkout file identity changed while opening: {name}"
        ));
    }
    Ok(equal_text(
        object_hash(&entry.oid, opened.len(), &mut file)?.as_str(),
        entry.oid.as_str(),
    ))
}
