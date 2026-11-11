//! Compare an explicit Git tree/index inventory with literal filesystem objects.
use crate::{
    facts::{Entry, Tree},
    relative_path,
    verified_identifiers::{git_inventory_records, git_inventory_termination, git_inventory_text},
    verified_status::{
        CheckoutEntryIo, CheckoutEntryMetadata, CheckoutPhysicalObservation,
        checkout_changed_paths, checkout_entry_matches, checkout_hash_stream,
        checkout_index_changes, checkout_object_hash, checkout_physical_path,
        checkout_physical_targets, parse_git_index_rows, parse_git_tree_rows,
        record_checkout_physical_observations,
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
    checkout_physical_path(
        Path::new(path).components(),
        root.to_path_buf(),
        |mut parts: std::path::Components<'_>| {
            let part = parts.next();
            (parts, part)
        },
        |mut current: PathBuf, part: std::path::Component<'_>| {
            current.push(part);
            current
        },
        |current: PathBuf| {
            let observation = fs::symlink_metadata(&current).map(|metadata| metadata.is_dir());
            (current, observation)
        },
        |error: &io::Error| error.kind() == io::ErrorKind::NotFound,
    )
    .map_err(|error| format!("checkout path {path}: {error}"))
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

fn entry_metadata(metadata: fs::Metadata) -> CheckoutEntryMetadata {
    CheckoutEntryMetadata {
        symbolic: metadata.is_symlink(),
        regular: metadata.is_file(),
        size: metadata.len(),
        permissions: metadata.permissions().mode(),
    }
}

fn matches_entry(root: &Path, name: &str, entry: &Entry) -> Result<bool, String> {
    checkout_entry_matches(
        entry.kind.as_str(),
        entry.mode.as_str(),
        entry.oid.as_str(),
        CheckoutEntryIo {
            locate: || physical(root, name),
            metadata: |path: &PathBuf| {
                fs::symlink_metadata(path)
                    .map(entry_metadata)
                    .map_err(|error| (error.kind() == io::ErrorKind::NotFound, error))
            },
            format_metadata_error: |error: io::Error| format!("checkout file {name}: {error}"),
            read_link: |path: PathBuf| fs::read_link(path).map_err(|error| error.to_string()),
            hash_link: |target: PathBuf| {
                let bytes = target.as_os_str().as_bytes();
                object_hash(&entry.oid, bytes.len() as u64, &mut &bytes[..])
            },
            open: |path: PathBuf| {
                // 不跟随在类型观察后替入的链接，也不阻塞在替入的 FIFO 上。
                fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                    .open(path)
                    .map_err(|error| format!("checkout file {name}: {error}"))
            },
            opened_metadata: |file: &fs::File| {
                file.metadata()
                    .map(entry_metadata)
                    .map_err(|error| error.to_string())
            },
            hash_file: |size: u64, mut file: fs::File| object_hash(&entry.oid, size, &mut file),
            changed_error: || format!("checkout file identity changed while opening: {name}"),
        },
    )
}
