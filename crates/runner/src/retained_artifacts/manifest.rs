//! A sealed producer tree, not a discovery list assembled during deletion.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
fn sealed_member_matches(same_identity: bool, directory: bool, same_stamp: bool,
    batch_file: bool, same_data_stamp: bool) -> (accepted: bool)
    ensures
        accepted == (same_identity && (directory || same_stamp || (batch_file && same_data_stamp))),
        !same_identity ==> !accepted,
        !directory && !same_stamp && accepted ==> batch_file && same_data_stamp,
{
    same_identity && (directory || same_stamp || (batch_file && same_data_stamp))
}
}

#[cfg(not(verus_keep_ghost))]
use crate::artifact_disposal::{entry_path, identity};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub path: String,
    pub identity: String,
    pub stamp: String,
    pub bytes: u64,
    pub directory: bool,
    #[serde(default)]
    pub allocated_bytes: Option<u64>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cursor {
    entry: Entry,
    offset: usize,
    children: Option<Vec<String>>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub entries: Vec<Entry>,
    pending: Vec<Cursor>,
    pub sealed: bool,
    pub bytes: u64,
}
#[cfg(not(verus_keep_ghost))]
fn stamp(m: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!(
        "{}:{}:{}:{}:{}",
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec()
    )
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn observe(root: &Path, path: &str) -> Result<Entry, String> {
    let physical = entry_path(root, path)?;
    let m = fs::symlink_metadata(&physical).map_err(|e| e.to_string())?;
    if !m.is_dir() && !m.is_file() && !m.file_type().is_symlink() {
        return Err("special tree entry; preserve original".into());
    }
    Ok(Entry {
        path: path.into(),
        identity: identity(&physical)?,
        stamp: stamp(&m),
        bytes: if m.is_dir() { 0 } else { m.len() },
        directory: m.is_dir(),
        allocated_bytes: {
            use std::os::unix::fs::MetadataExt;
            m.blocks().checked_mul(512)
        },
    })
}
#[cfg(not(verus_keep_ghost))]
impl Manifest {
    /// Each step consumes one declared child after a finite directory snapshot.
    /// Snapshot names and positions survive interruption in owner metadata.
    pub fn step(&mut self, root: &Path, path: &str, limit: usize) -> Result<(), String> {
        if self.sealed {
            return Ok(());
        }
        if self.entries.is_empty() {
            let entry = observe(root, path)?;
            self.bytes = entry.bytes;
            self.entries.push(entry.clone());
            if entry.directory {
                self.pending.push(Cursor {
                    entry,
                    offset: 0,
                    children: None,
                });
            } else {
                self.sealed = true;
            }
            return Ok(());
        }
        let Some(cursor) = self.pending.last_mut() else {
            self.sealed = true;
            return Ok(());
        };
        let current = observe(root, &cursor.entry.path)?;
        if current.identity != cursor.entry.identity || current.stamp != cursor.entry.stamp {
            return Err("tree changed while sealing identities; preserve original".into());
        }
        if cursor.children.is_none() {
            // One finite directory snapshot, then one exact declared child per
            // step. Publication charges these names to the global manifest cap.
            // It never opens or follows descendant links.
            let physical = entry_path(root, &cursor.entry.path)?;
            let mut names = Vec::new();
            for item in fs::read_dir(physical).map_err(|e| e.to_string())? {
                if names.len() >= limit {
                    return Err("directory identity capacity; preserve original".into());
                }
                let name = item
                    .map_err(|e| e.to_string())?
                    .file_name()
                    .into_string()
                    .map_err(|_| "tree path UTF8")?;
                names.push(name);
            }
            cursor.children = Some(names);
        }
        let child = cursor
            .children
            .as_ref()
            .and_then(|names| names.get(cursor.offset))
            .cloned();
        let next_offset = cursor.offset + 1;
        match child {
            None => {
                self.pending.pop();
                if self.pending.is_empty() {
                    self.sealed = true;
                }
            }
            Some(name) if name == "." || name == ".." => {
                cursor.offset = next_offset;
            }
            Some(name) => {
                if self.entries.len() >= limit {
                    return Err("tree identity manifest capacity; preserve original".into());
                }
                let entry = observe(root, &format!("{}/{name}", cursor.entry.path))?;
                let bytes = self
                    .bytes
                    .checked_add(entry.bytes)
                    .ok_or("tree byte overflow")?;
                self.bytes = bytes;
                cursor.offset = next_offset;
                self.entries.push(entry.clone());
                if entry.directory {
                    self.pending.push(Cursor {
                        entry,
                        offset: 0,
                        children: None,
                    });
                }
            }
        }
        Ok(())
    }
    pub fn validate(entry: &Entry, root: &Path) -> Result<(), String> {
        Self::validate_for_disposal(entry, root, false)
    }

    pub fn validate_for_disposal(
        entry: &Entry,
        root: &Path,
        batch_directory: bool,
    ) -> Result<(), String> {
        let current = observe(root, &entry.path)?;
        // Unlinking a sibling name changes ctime on the same inode. In a
        // disposable batch the producer owns the name, including on interruption.
        // Keep length and mtime checks, and always check the sealed inode and type.
        let batch_file = batch_directory && entry.identity.ends_with(":file");
        let same_data_stamp = batch_file
            && entry
                .stamp
                .rsplitn(3, ':')
                .nth(2)
                .is_some_and(|stamp| current.stamp.rsplitn(3, ':').nth(2) == Some(stamp));
        if !sealed_member_matches(
            current.identity == entry.identity,
            entry.directory,
            entry.stamp == current.stamp,
            batch_file,
            same_data_stamp,
        ) {
            return Err("sealed descendant identity/content changed; preserve replacement".into());
        }
        Ok(())
    }
}
