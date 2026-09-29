//! One preservation and attachment path, shared by initial and resumed rebind.
use crate::{
    rebind_inputs::{self as inputs, Expected},
    start::{self, Runner},
};
use serde_json::{Value, json as value};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(crate) struct Scope<'a> {
    pub root: &'a Path,
    pub target: &'a Path,
    pub metadata: &'a Path,
    pub backup: &'a Path,
    pub donor: &'a Path,
    pub branch: &'a str,
    pub head: &'a str,
    pub tree: &'a str,
    pub token: &'a str,
    pub expected: &'a Expected,
}
fn text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "rebind path is not UTF-8".into())
}
fn directory(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
        Ok(m) if m.is_dir() => Ok(true),
        Ok(_) => Err(format!(
            "rebind directory is not physical: {}",
            path.display()
        )),
    }
}
fn entries(path: &Path, allowed: &[&str]) -> Result<(), String> {
    if directory(path)? {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !allowed.iter().any(|s| entry.file_name() == *s) {
                return Err(format!(
                    "preserve unexpected recovery entry: {}",
                    entry.path().display()
                ));
            }
        }
    }
    Ok(())
}
fn bytes(path: &Path) -> Result<Vec<u8>, String> {
    inputs::identity(path)?.ok_or("required rebind file is absent")?;
    fs::read(path).map_err(|e| e.to_string())
}
fn pointer(checkout: &Path) -> Result<Option<PathBuf>, String> {
    if inputs::identity(&checkout.join(".git"))?.is_none() {
        return Ok(None);
    }
    let raw = bytes(&checkout.join(".git"))?;
    let raw = std::str::from_utf8(&raw).map_err(|_| "new Git pointer is not UTF-8")?;
    let path = raw
        .strip_prefix("gitdir: ")
        .ok_or("new Git pointer is malformed")?;
    Ok(Some(inputs::pointer(
        checkout,
        path.trim_end_matches('\n'),
    )?))
}
fn digest(path: &Path) -> Result<Option<String>, String> {
    if directory(path)? {
        Ok(Some(
            inputs::snapshot(path, false)?["sha256"]
                .as_str()
                .unwrap()
                .into(),
        ))
    } else {
        Ok(None)
    }
}
impl Scope<'_> {
    fn preserved(&self, complete: bool) -> Result<(), String> {
        entries(self.backup, &["gitfile", "metadata"])?;
        let saved_pointer = inputs::identity(&self.backup.join("gitfile"))?;
        if saved_pointer != self.expected.gitfile {
            if complete
                || saved_pointer.is_some()
                || inputs::identity(&self.target.join(".git"))? != self.expected.gitfile
            {
                return Err("preserved Git pointer identity changed or is missing".into());
            }
        }
        let saved = digest(&self.backup.join("metadata"))?;
        let original = digest(self.metadata)?;
        if saved == self.expected.metadata && original.is_none() {
            return Ok(());
        }
        if !complete && saved.is_none() && original == self.expected.metadata {
            return Ok(());
        }
        Err("preserved metadata identity changed, duplicated or is missing".into())
    }
    fn visible(&self) -> Result<(), String> {
        if inputs::snapshot(self.target, true)?["sha256"] != self.expected.visible {
            return Err("visible work changed during metadata rebind".into());
        }
        Ok(())
    }
    fn allocation(&self) -> Result<Option<(PathBuf, bool)>, String> {
        entries(self.donor, &[".git"])?;
        let old_pointer = inputs::identity(&self.target.join(".git"))? == self.expected.gitfile;
        let from_donor = pointer(self.donor)?;
        let from_target = if old_pointer {
            None
        } else {
            pointer(self.target)?
        };
        if !old_pointer && from_target.is_none() {
            return Err("target Git pointer changed".into());
        }
        if from_donor.is_some() && from_target.is_some() && from_donor != from_target {
            return Err("donor and target point to different metadata".into());
        }
        let Some(metadata) = from_donor.or(from_target.clone()) else {
            return Ok(None);
        };
        if metadata.parent() != self.metadata.parent()
            || metadata == self.metadata
            || !directory(&metadata)?
        {
            return Err("new metadata is not a distinct physical common-repository member".into());
        }
        let common = self.metadata.parent().unwrap().parent().unwrap();
        let raw = bytes(&metadata.join("commondir"))?;
        let raw = std::str::from_utf8(&raw).map_err(|_| "common pointer is not UTF-8")?;
        if inputs::pointer(&metadata, raw.trim_end_matches('\n'))? != common {
            return Err("donor belongs to another repository".into());
        }
        let raw = bytes(&metadata.join("gitdir"))?;
        let raw = std::str::from_utf8(&raw).map_err(|_| "metadata backlink is not UTF-8")?;
        let backlink = inputs::pointer(&metadata, raw.trim_end_matches('\n'))?;
        if backlink != self.donor.join(".git") && backlink != self.target.join(".git") {
            return Err("new metadata belongs to another checkout".into());
        }
        let lock = inputs::identity(&metadata.join("locked"))?;
        let locked = lock.is_some();
        if locked {
            if bytes(&metadata.join("locked"))? != format!("{}\n", self.token).as_bytes() {
                return Err("rebind ownership lock mismatch".into());
            }
        } else if from_target.as_ref() != Some(&metadata)
            || directory(self.donor)?
            || backlink != self.target.join(".git")
        {
            return Err("unlocked rebind is not a completed attachment".into());
        }
        let head = bytes(&metadata.join("HEAD"))?;
        if head != format!("{}\n", self.head).as_bytes()
            && head != format!("ref: {}\n", self.branch).as_bytes()
        {
            return Err("donor HEAD or branch changed".into());
        }
        for name in ["index.lock", "HEAD.lock"] {
            if fs::symlink_metadata(metadata.join(name)).is_ok() {
                return Err("preserve interrupted Git lock; reconcile it before resuming".into());
            }
        }
        Ok(Some((metadata, locked)))
    }
    fn index(&self, r: &mut Runner, metadata: &Path) -> Result<bool, String> {
        if inputs::identity(&metadata.join("index"))?.is_none() {
            return Ok(false);
        }
        let location = if pointer(self.donor)?.as_deref() == Some(metadata) {
            self.donor
        } else {
            self.target
        };
        if r.text(location, &["write-tree"])?.trim() != self.tree {
            return Err("rebound index differs from explicit tree; preserve it".into());
        }
        Ok(true)
    }
    pub(crate) fn finish(
        &self,
        r: &mut Runner,
        report: &mut Value,
        stable: impl Fn(&mut Runner, &Value) -> Result<(), String>,
    ) -> Result<(), String> {
        // Reject conflicts before any preservation or attachment effect.
        self.visible()?;
        self.preserved(false)?;
        let allocation = self.allocation()?;
        if let Some((metadata, _)) = &allocation {
            self.index(r, metadata)?;
        }
        if allocation.is_none()
            && r.inventory(self.root)?.iter().any(|row| {
                row.get("worktree")
                    .is_some_and(|p| Path::new(p) == self.donor)
            })
        {
            return Err("donor metadata is incomplete; reconcile the retained allocation".into());
        }
        stable(r, report)?;
        if !directory(self.backup)? {
            fs::create_dir(self.backup).map_err(|e| format!("rebind backup: {e}"))?;
        }
        report["phase"] = value!("backup-created");
        if inputs::identity(&self.backup.join("gitfile"))?.is_none()
            && self.expected.gitfile.is_some()
        {
            let original = bytes(&self.target.join(".git"))?;
            // Publish complete bytes atomically; incomplete temporary data never replaces the saved pointer.
            let mut f = tempfile::Builder::new()
                .prefix("gitfile-")
                .tempfile_in(self.backup)
                .map_err(|e| e.to_string())?;
            f.write_all(&original).map_err(|e| e.to_string())?;
            f.as_file()
                .set_permissions(fs::Permissions::from_mode(
                    self.expected.gitfile.as_ref().unwrap().mode,
                ))
                .map_err(|e| e.to_string())?;
            f.as_file().sync_all().map_err(|e| e.to_string())?;
            f.persist_noclobber(self.backup.join("gitfile"))
                .map_err(|e| e.to_string())?;
        }
        if directory(self.metadata)? {
            self.preserved(false)?;
            fs::rename(self.metadata, self.backup.join("metadata"))
                .map_err(|e| format!("preserve original metadata: {e}"))?;
        }
        self.preserved(true)?;
        report["phase"] = value!("original-metadata-preserved");
        self.visible()?;
        stable(r, report)?;
        if allocation.is_none() {
            r.git(
                self.root,
                &[
                    "worktree",
                    "add",
                    "--detach",
                    "--no-checkout",
                    "--lock",
                    "--reason",
                    self.token,
                    "--",
                    text(self.donor)?,
                    self.head,
                ],
            )?;
        }
        report["phase"] = value!("donor-created");
        let (metadata, locked) = self
            .allocation()?
            .ok_or("donor pointer missing after creation")?;
        report["new_metadata_path"] = value!(metadata);
        let location = if pointer(self.donor)?.as_ref() == Some(&metadata) {
            self.donor
        } else {
            self.target
        };
        self.visible()?;
        self.preserved(true)?;
        stable(r, report)?;
        if !locked {
            if bytes(&metadata.join("HEAD"))? != format!("ref: {}\n", self.branch).as_bytes()
                || !self.index(r, &metadata)?
            {
                return Err("unlocked attachment has wrong branch or index".into());
            }
        } else {
            if bytes(&metadata.join("HEAD"))? != format!("ref: {}\n", self.branch).as_bytes() {
                r.git(location, &["symbolic-ref", "HEAD", self.branch])?;
            }
            if !self.index(r, &metadata)? {
                r.git(location, &["read-tree", self.tree])?;
            }
            if pointer(self.target).ok().flatten().as_ref() != Some(&metadata) {
                if inputs::identity(&self.target.join(".git"))? != self.expected.gitfile {
                    return Err("original work changed before attachment".into());
                }
                self.visible()?;
                self.preserved(true)?;
                stable(r, report)?;
                let new_gitfile = bytes(&self.donor.join(".git"))?;
                let pointer_mode = inputs::identity(&self.donor.join(".git"))?
                    .ok_or("donor pointer disappeared")?
                    .mode;
                let mut replacement = tempfile::Builder::new()
                    .prefix(".chrono-rebind-")
                    .tempfile_in(self.target)
                    .map_err(|e| e.to_string())?;
                replacement
                    .write_all(&new_gitfile)
                    .map_err(|e| e.to_string())?;
                replacement
                    .as_file()
                    .set_permissions(fs::Permissions::from_mode(pointer_mode))
                    .map_err(|e| e.to_string())?;
                replacement
                    .as_file()
                    .sync_all()
                    .map_err(|e| e.to_string())?;
                replacement
                    .persist(self.target.join(".git"))
                    .map_err(|e| format!("new Git pointer publication: {e}"))?;
            }
            report["phase"] = value!("pointer-published");
            self.visible()?;
            self.preserved(true)?;
            stable(r, report)?;
            r.git(self.root, &["worktree", "repair", "--", text(self.target)?])?;
            start::ensure_identity(
                r,
                self.root,
                self.target,
                self.branch.strip_prefix("refs/heads/").unwrap(),
                self.head,
                self.token,
            )?;
            if !self.index(r, &metadata)? {
                return Err("rebound index is absent".into());
            }
            self.allocation()?;
            if directory(self.donor)? {
                if let Some(path) = pointer(self.donor)? {
                    if path != metadata {
                        return Err("donor pointer changed; preserve it".into());
                    }
                    fs::remove_file(self.donor.join(".git")).map_err(|e| e.to_string())?;
                }
                fs::remove_dir(self.donor).map_err(|e| format!("preserve nonempty donor: {e}"))?;
            }
            report["phase"] = value!("attached");
            self.visible()?;
            self.preserved(true)?;
            stable(r, report)?;
            start::unlock(r, self.root, text(self.target)?, self.branch, self.head)?;
        }
        self.visible()?;
        self.preserved(true)?;
        stable(r, report)?;
        let (after, lock) = self
            .allocation()?
            .ok_or("completed attachment is missing")?;
        if after != metadata || lock || !self.index(r, &after)? {
            return Err("attachment changed after unlock".into());
        }
        if !r.inventory(self.root)?.iter().any(|row| {
            row.get("worktree")
                .is_some_and(|p| Path::new(p) == self.target)
                && row.get("HEAD").map(String::as_str) == Some(self.head)
                && row.get("branch").map(String::as_str) == Some(self.branch)
                && !row.contains_key("locked")
                && !row.contains_key("prunable")
        }) {
            return Err("completed attachment identity changed".into());
        }
        report["visible_after"] = value!(self.expected.visible);
        report["phase"] = value!("verified");
        report["backup_preserved"] = value!(true);
        report["donor_removed"] = value!(true);
        Ok(())
    }
}
