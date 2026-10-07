use super::{DirectoryEvidence, DirectoryInventory, Result, error};
use chrono_harness::{file_identity, json};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
};

pub(super) struct Binding {
    source: PathBuf,
    root: PathBuf,
    files: Vec<(PathBuf, String)>,
    links: BTreeMap<String, Link>,
    directories: Vec<(PathBuf, Vec<String>)>,
    file_targets: BTreeSet<PathBuf>,
    directory_targets: BTreeSet<PathBuf>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Link {
    path: String,
    target: String,
    kind: Kind,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    File,
    Directory,
}

impl Binding {
    pub(super) fn unchanged(&self) -> Result {
        crate::guard::measure(
            &format!("directory-unchanged:{}", self.source.display()),
            || self.unchanged_inner(),
        )
    }
    fn unchanged_inner(&self) -> Result {
        if fs::canonicalize(&self.source).map_err(error)? != self.root {
            return Err(error(format!(
                "toolchain directory root changed during guarded operation: {}",
                self.source.display()
            )));
        }
        if !self.root.is_dir() {
            return Err(error(format!(
                "toolchain directory disappeared: {}",
                self.root.display()
            )));
        }
        for (path, digest) in &self.files {
            let relative = path
                .strip_prefix(&self.root)
                .map_err(error)?
                .to_str()
                .ok_or_else(|| error("non UTF-8 directory input"))?;
            regular_path(&self.root, relative)?;
            if file_identity(path).map_err(error)?.0 != *digest {
                return Err(error(format!(
                    "toolchain directory input changed during guarded operation: {}",
                    path.display()
                )));
            }
        }
        for (path, entries) in &self.directories {
            let relative = path
                .strip_prefix(&self.root)
                .map_err(error)?
                .to_str()
                .ok_or_else(|| error("non UTF-8 empty directory"))?;
            directory_entries(&self.root, relative, entries)?;
        }
        self.check_links()
    }

    fn check_links(&self) -> Result {
        for link in self.links.values() {
            let path = self.root.join(&link.path);
            // Declared aliases have literal physical parents; aliases reached in
            // their target expressions are resolved only through this registry.
            let parent = Path::new(&link.path).parent().unwrap();
            if !parent.as_os_str().is_empty() {
                let mut current = self.root.clone();
                for component in parent.components() {
                    current.push(component);
                    if fs::symlink_metadata(&current)
                        .map_err(error)?
                        .file_type()
                        .is_symlink()
                    {
                        return Err(error("directory link declaration has a symlink parent"));
                    }
                }
            }
            if fs::read_link(&path).map_err(error)?.as_os_str()
                != Path::new(&link.target).as_os_str()
            {
                return Err(error(format!(
                    "directory link target changed: {}",
                    path.display()
                )));
            }
            let target = self.resolve(&link.path)?;
            if fs::canonicalize(&path).map_err(error)? != target {
                return Err(error("directory link resolution changed"));
            }
            let covered = match link.kind {
                Kind::File => target.is_file() && self.file_targets.contains(&target),
                Kind::Directory => target.is_dir() && self.directory_targets.contains(&target),
            };
            if !covered {
                return Err(error(
                    "directory link target kind or registered file coverage differs",
                ));
            }
        }
        Ok(())
    }

    fn resolve(&self, relative: &str) -> Result<PathBuf> {
        let mut pending: VecDeque<_> = Path::new(relative)
            .components()
            .map(|c| c.as_os_str().to_owned())
            .collect();
        let mut resolved = PathBuf::new();
        let mut followed = 0;
        while let Some(component) = pending.pop_front() {
            if component == "." {
                continue;
            }
            if component == ".." {
                if !resolved.pop() {
                    return Err(error("directory link target escapes inventory root"));
                }
                continue;
            }
            resolved.push(&component);
            let path = self.root.join(&resolved);
            let metadata = fs::symlink_metadata(&path).map_err(error)?;
            if metadata.file_type().is_symlink() {
                let id = resolved
                    .to_str()
                    .ok_or_else(|| error("non UTF-8 directory link"))?;
                let declaration = self
                    .links
                    .get(id)
                    .ok_or_else(|| error(format!("undeclared directory link: {id}")))?;
                let target = fs::read_link(&path).map_err(error)?;
                if target.as_os_str() != Path::new(&declaration.target).as_os_str() {
                    return Err(error(format!("directory link target changed: {id}")));
                }
                followed += 1;
                if followed > 64 {
                    return Err(error("directory link resolution exceeds bound or cycles"));
                }
                resolved.pop();
                for part in target.components().rev() {
                    match part {
                        Component::Normal(_) | Component::CurDir | Component::ParentDir => {
                            pending.push_front(part.as_os_str().to_owned())
                        }
                        _ => return Err(error("directory link target must be relative")),
                    }
                }
            }
        }
        Ok(self.root.join(resolved))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryManifest {
    schema: String,
    root: String,
    files: Vec<DirectoryFile>,
    symlinks: Option<Vec<Link>>,
    directories: Option<Vec<DirectoryEntries>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryEntries {
    path: String,
    entries: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryFile {
    path: String,
    sha256: String,
}

fn physical_path(root: &Path, relative: &str) -> Result<PathBuf> {
    chrono_harness::relative_path(relative)?;
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(error(format!(
                    "directory inventory contains symlink: {}",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(error(format!(
                    "directory inventory file is missing: {}",
                    path.display()
                )));
            }
            Err(e) => return Err(error(e)),
        }
    }
    Ok(path)
}
fn regular_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = physical_path(root, relative)?;
    if !path.is_file() {
        return Err(error(format!(
            "directory inventory entry is not a file: {}",
            path.display()
        )));
    }
    Ok(path)
}
fn directory_entries(root: &Path, relative: &str, expected: &[String]) -> Result<PathBuf> {
    let path = physical_path(root, relative)?;
    let declared: BTreeSet<_> = expected.iter().cloned().collect();
    if declared.len() != expected.len()
        || expected.iter().any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.contains('/')
                || s.contains('\\')
                || s.contains('\0')
        })
    {
        return Err(error("invalid/duplicate directory child declaration"));
    }
    let actual: BTreeSet<_> = fs::read_dir(&path)
        .map_err(error)?
        .map(|entry| {
            entry
                .map_err(error)?
                .file_name()
                .into_string()
                .map_err(|_| error("non UTF-8 directory child"))
        })
        .collect::<Result<_>>()?;
    if actual != declared {
        return Err(error(format!(
            "directory children differ from declaration: {}",
            path.display()
        )));
    }
    Ok(path)
}

pub(super) fn bind(
    root: &Path,
    declaration: &DirectoryInventory,
    manifest: &Path,
) -> Result<(Binding, DirectoryEvidence)> {
    crate::guard::measure(&format!("directory-bind:{}", declaration.root), || {
        bind_inner(root, declaration, manifest)
    })
}
fn bind_inner(
    root: &Path,
    declaration: &DirectoryInventory,
    manifest: &Path,
) -> Result<(Binding, DirectoryEvidence)> {
    if declaration.root.is_empty() || declaration.manifest.is_empty() {
        return Err(error("directory inventory root/manifest is empty"));
    }
    let source = if Path::new(&declaration.root).is_absolute() {
        PathBuf::from(&declaration.root)
    } else {
        root.join(&declaration.root)
    };
    let directory = fs::canonicalize(&source).map_err(error)?;
    if !directory.is_dir() {
        return Err(error(format!(
            "directory inventory root is not a directory: {}",
            directory.display()
        )));
    }
    let value = json(&fs::read(manifest).map_err(error)?)?;
    let has_links = value.get("symlinks").is_some();
    let has_directories = value.get("directories").is_some();
    let parsed: DirectoryManifest = serde_json::from_value(value).map_err(error)?;
    if !matches!(
        parsed.schema.as_str(),
        "chrono-input-directory/v1" | "chrono-input-directory/v2" | "chrono-input-directory/v3"
    ) || parsed.root != declaration.root
        || (parsed.schema == "chrono-input-directory/v1" && has_links)
        || (parsed.schema != "chrono-input-directory/v1" && parsed.symlinks.is_none())
        || (parsed.schema != "chrono-input-directory/v3" && has_directories)
        || (parsed.schema == "chrono-input-directory/v3" && parsed.directories.is_none())
    {
        return Err(error("directory inventory manifest schema/root mismatch"));
    }
    let mut seen = BTreeSet::new();
    let mut files = Vec::new();
    for entry in parsed.files {
        chrono_harness::wire::is_digest(&entry.sha256)
            .then_some(())
            .ok_or_else(|| error("directory inventory digest is invalid"))?;
        if !seen.insert(entry.path.clone()) {
            return Err(error("directory inventory contains a duplicate path"));
        }
        let path = regular_path(&directory, &entry.path)?;
        let actual = file_identity(&path).map_err(error)?.0;
        if actual != entry.sha256 {
            return Err(error(format!(
                "directory inventory digest differs: {}",
                path.display()
            )));
        }
        files.push((path, entry.sha256));
    }
    if files.is_empty() {
        return Err(error("directory inventory must contain a file"));
    }
    let file_count = files.len();
    let mut directories = Vec::new();
    for entry in parsed.directories.unwrap_or_default() {
        if !seen.insert(entry.path.clone()) {
            return Err(error("directory inventory contains a duplicate path"));
        }
        directories.push((
            directory_entries(&directory, &entry.path, &entry.entries)?,
            entry.entries,
        ));
    }
    let mut links = BTreeMap::new();
    for link in parsed.symlinks.unwrap_or_default() {
        chrono_harness::relative_path(&link.path)?;
        if link.target.is_empty()
            || Path::new(&link.target).is_absolute()
            || !seen.insert(link.path.clone())
        {
            return Err(error("directory link path/target is invalid or duplicated"));
        }
        links.insert(link.path.clone(), link);
    }
    // Index exactly the existing declared coverage; this does not scan or
    // expand the whitelist. SDK alias checks otherwise repeatedly scan files.
    let file_targets = files.iter().map(|(p, _)| p.clone()).collect();
    let mut directory_targets: BTreeSet<_> = directories.iter().map(|(p, _)| p.clone()).collect();
    for (path, _) in &files {
        for parent in path
            .ancestors()
            .skip(1)
            .take_while(|p| p.starts_with(&directory))
        {
            directory_targets.insert(parent.to_path_buf());
        }
    }
    let binding = Binding {
        source,
        root: directory,
        files,
        links,
        directories,
        file_targets,
        directory_targets,
    };
    binding.check_links()?;
    Ok((
        binding,
        DirectoryEvidence {
            root: declaration.root.clone(),
            manifest: declaration.manifest.clone(),
            files: file_count,
        },
    ))
}
