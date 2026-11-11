//! Finite command-boundary Git data for the explicitly registered test consumers.
//! This is finite test data, not a Git implementation: unknown commands fail closed.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn known_git_command(command: &str) -> (allowed: bool)
    ensures allowed == (command@ == "--version"@ || command@ == "check-ref-format"@ || command@ == "init"@ || command@ == "clone"@ || command@ == "add"@ || command@ == "commit"@ || command@ == "commit-tree"@ || command@ == "rev-parse"@ || command@ == "show"@ || command@ == "ls-tree"@ || command@ == "cat-file"@ || command@ == "ls-files"@ || command@ == "config"@ || command@ == "diff"@ || command@ == "update-index"@ || command@ == "update-ref"@ || command@ == "reset"@ || command@ == "checkout"@ || command@ == "switch"@ || command@ == "status"@ || command@ == "symbolic-ref"@ || command@ == "branch"@ || command@ == "show-ref"@ || command@ == "for-each-ref"@ || command@ == "remote"@ || command@ == "ls-remote"@ || command@ == "push"@ || command@ == "fetch"@ || command@ == "merge-base"@ || command@ == "rev-list"@ || command@ == "merge"@ || command@ == "write-tree"@ || command@ == "read-tree"@ || command@ == "mv"@ || command@ == "rm"@ || command@ == "replace"@ || command@ == "reflog"@ || command@ == "worktree"@),
{ identifiers::equal_text(command, "--version") || identifiers::equal_text(command, "check-ref-format") || identifiers::equal_text(command, "init") || identifiers::equal_text(command, "clone") || identifiers::equal_text(command, "add") || identifiers::equal_text(command, "commit") || identifiers::equal_text(command, "commit-tree") || identifiers::equal_text(command, "rev-parse") || identifiers::equal_text(command, "show") || identifiers::equal_text(command, "ls-tree") || identifiers::equal_text(command, "cat-file") || identifiers::equal_text(command, "ls-files") || identifiers::equal_text(command, "config") || identifiers::equal_text(command, "diff") || identifiers::equal_text(command, "update-index") || identifiers::equal_text(command, "update-ref") || identifiers::equal_text(command, "reset") || identifiers::equal_text(command, "checkout") || identifiers::equal_text(command, "switch") || identifiers::equal_text(command, "status") || identifiers::equal_text(command, "symbolic-ref") || identifiers::equal_text(command, "branch") || identifiers::equal_text(command, "show-ref") || identifiers::equal_text(command, "for-each-ref") || identifiers::equal_text(command, "remote") || identifiers::equal_text(command, "ls-remote") || identifiers::equal_text(command, "push") || identifiers::equal_text(command, "fetch") || identifiers::equal_text(command, "merge-base") || identifiers::equal_text(command, "rev-list") || identifiers::equal_text(command, "merge") || identifiers::equal_text(command, "write-tree") || identifiers::equal_text(command, "read-tree") || identifiers::equal_text(command, "mv") || identifiers::equal_text(command, "rm") || identifiers::equal_text(command, "replace") || identifiers::equal_text(command, "reflog") || identifiers::equal_text(command, "worktree") }
}

#[cfg(not(any(verus_keep_ghost, test)))]
#[path = "."]
mod runtime {
    use serde::{Deserialize, Serialize};
    use sha1::{Digest, Sha1};
    use std::{
        collections::BTreeMap,
        fs,
        io::{self, Read},
        os::unix::{ffi::OsStrExt, fs::PermissionsExt},
        path::{Path, PathBuf},
    };

    #[derive(Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
    struct Entry {
        mode: String,
        oid: String,
    }
    type Tree = BTreeMap<String, Entry>;
    #[derive(Default, Deserialize, Serialize)]
    #[serde(default)]
    struct Repository {
        branch: String,
        remotes: BTreeMap<String, String>,
        worktrees: BTreeMap<String, String>,
        parents: BTreeMap<String, Vec<String>>,
        trees: BTreeMap<String, Tree>,
        bare: bool,
        main_root: String,
        width: usize,
        unmerged: bool,
        head: String,
        index: Tree,
        commits: BTreeMap<String, Tree>,
        refs: BTreeMap<String, String>,
        symbolic: BTreeMap<String, String>,
        flags: BTreeMap<String, String>,
        config: BTreeMap<String, String>,
    }

    fn hash(kind: &str, bytes: &[u8], width: usize) -> String {
        let mut framed = format!("{kind} {}\0", bytes.len()).into_bytes();
        framed.extend_from_slice(bytes);
        if width == 64 {
            chrono_harness::sha256(&framed)
        } else {
            format!("{:x}", Sha1::digest(&framed))
        }
    }
    fn walk(root: &Path, path: &Path, out: &mut Vec<String>) -> io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_name() == ".git" || (path == root && entry.file_name() == "fixture.json")
            {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.is_dir() {
                walk(root, &entry.path(), out)?;
            } else {
                out.push(
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .into(),
                );
            }
        }
        Ok(())
    }
    fn ignored(root: &Path, path: &str) -> bool {
        (fs::read_to_string(root.join(".gitignore")).unwrap_or_default()
            + "\n"
            + &fs::read_to_string(common(root).join("info/exclude")).unwrap_or_default())
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .any(|line| {
                let pattern = line.trim_start_matches('/');
                (pattern.starts_with("*.") && path.ends_with(&pattern[1..]))
                    || path == pattern.trim_end_matches('/')
                    || (pattern.ends_with('/') && path.starts_with(pattern))
            })
    }
    fn entry(root: &Path, path: &str, width: usize) -> io::Result<(Entry, Vec<u8>)> {
        let physical = root.join(path);
        let metadata = fs::symlink_metadata(&physical)?;
        let (mode, bytes) = if metadata.file_type().is_symlink() {
            (
                "120000",
                fs::read_link(physical)?.as_os_str().as_bytes().to_vec(),
            )
        } else {
            (
                if metadata.permissions().mode() & 0o111 != 0 {
                    "100755"
                } else {
                    "100644"
                },
                fs::read(physical)?,
            )
        };
        Ok((
            Entry {
                mode: mode.into(),
                oid: hash("blob", &bytes, width),
            },
            bytes,
        ))
    }
    fn copy_directory(source: &Path, target: &Path) -> io::Result<()> {
        fs::create_dir_all(target)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            let target = target.join(item.file_name());
            if item.file_type()?.is_dir() {
                copy_directory(&item.path(), &target)?;
            } else {
                fs::copy(item.path(), target)?;
            }
        }
        Ok(())
    }
    fn metadata(root: &Path) -> PathBuf {
        let path = root.join(".git");
        if path.is_file() {
            let text = fs::read_to_string(path).unwrap();
            let selected = PathBuf::from(
                text.trim()
                    .strip_prefix("gitdir: ")
                    .expect("fixture gitdir"),
            );
            if selected.is_absolute() {
                selected
            } else {
                root.join(selected)
            }
        } else if path.is_dir() {
            path
        } else {
            root.to_path_buf()
        }
    }
    fn common(root: &Path) -> PathBuf {
        let dir = metadata(root);
        if let Ok(path) = fs::read_to_string(dir.join("commondir")) {
            fs::canonicalize(dir.join(path.trim())).unwrap()
        } else {
            dir
        }
    }
    fn load(root: &Path) -> Result<Repository, String> {
        let dir = metadata(root);
        let mut repo: Repository = serde_json::from_slice(
            &fs::read(common(root).join("fixture.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        for name in repo.refs.clone().keys() {
            match fs::read_to_string(common(root).join(name)) {
                Ok(text) if text.starts_with("ref: ") => {
                    let target = text.trim().trim_start_matches("ref: ");
                    repo.symbolic.insert(name.clone(), target.into());
                    repo.refs.insert(name.clone(), resolve(&repo, target));
                }
                Ok(text) => {
                    repo.refs.insert(name.clone(), text.trim().into());
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    repo.refs.remove(name);
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        if let Ok(head) = fs::read_to_string(dir.join("HEAD")) {
            if let Some(branch) = head.trim().strip_prefix("ref: ") {
                repo.branch = branch.into();
                repo.head = repo.refs.get(branch).cloned().unwrap_or_default();
            } else {
                repo.branch.clear();
                repo.head = head.trim().into();
            }
        }
        if let Ok(index) = fs::read(dir.join("index")) {
            repo.index = serde_json::from_slice(&index).map_err(|e| e.to_string())?;
        }
        Ok(repo)
    }
    fn resolve(repo: &Repository, name: &str) -> String {
        let name = name.trim_end_matches("^{commit}");
        if let Some((base, count)) = name.rsplit_once('~') {
            let mut oid = resolve(repo, base);
            for _ in 0..count.parse::<usize>().unwrap_or(1) {
                oid = repo
                    .parents
                    .get(&oid)
                    .and_then(|p| p.first())
                    .cloned()
                    .unwrap_or_default();
            }
            return oid;
        }
        if let Some((base, count)) = name.rsplit_once('^') {
            if count.chars().all(|c| c.is_ascii_digit()) {
                return repo
                    .parents
                    .get(&resolve(repo, base))
                    .and_then(|p| p.get(count.parse::<usize>().unwrap_or(1).saturating_sub(1)))
                    .cloned()
                    .unwrap_or_default();
            }
        }
        if name == "HEAD" {
            repo.head.clone()
        } else if let Some(oid) = repo
            .refs
            .get(name)
            .or_else(|| repo.refs.get(&format!("refs/heads/{name}")))
            .or_else(|| repo.refs.get(&format!("refs/remotes/{name}")))
        {
            oid.clone()
        } else {
            name.into()
        }
    }
    fn tree<'a>(repo: &'a Repository, name: &str) -> Result<&'a Tree, String> {
        repo.commits
            .get(&resolve(repo, name))
            .ok_or_else(|| format!("missing fixture commit {name}"))
    }
    fn object(repo: &Repository, query: &str) -> Result<(String, &'static str), String> {
        if let Some((revision, path)) = query.split_once(':') {
            let e = tree(repo, revision)?
                .get(path)
                .ok_or("missing fixture path")?;
            Ok((e.oid.clone(), "blob"))
        } else if repo.commits.contains_key(&resolve(repo, query)) {
            Ok((resolve(repo, query), "commit"))
        } else if repo.trees.contains_key(query) {
            Ok((query.into(), "tree"))
        } else {
            Ok((query.into(), "blob"))
        }
    }
    fn save(root: &Path, repo: &Repository) -> Result<(), String> {
        let dir = metadata(root);
        let common = common(root);
        fs::write(
            common.join("fixture.json"),
            serde_json::to_vec(repo).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        fs::write(dir.join("index"), serde_json::to_vec(&repo.index).unwrap())
            .map_err(|e| e.to_string())?;
        let previous = fs::read_to_string(dir.join("HEAD")).unwrap_or_default();
        let mut reflog: Vec<String> = fs::read(dir.join("fixture-reflog.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let previous_oid = if previous.starts_with("ref: ") {
            fs::read_to_string(common.join(previous.trim().trim_start_matches("ref: ")))
                .unwrap_or_default()
                .trim()
                .to_owned()
        } else {
            previous.trim().to_owned()
        };
        if !repo.head.is_empty() && (previous_oid != repo.head || reflog.is_empty()) {
            reflog.push(repo.head.clone());
        }
        fs::write(
            dir.join("fixture-reflog.json"),
            serde_json::to_vec(&reflog).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            dir.join("HEAD"),
            if repo.branch.is_empty() {
                format!("{}\n", repo.head)
            } else {
                format!("ref: {}\n", repo.branch)
            },
        )
        .map_err(|e| e.to_string())?;
        for (name, oid) in &repo.refs {
            let path = common.join(name);
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            let value = repo
                .symbolic
                .get(name)
                .map(|target| format!("ref: {target}\n"))
                .unwrap_or_else(|| format!("{oid}\n"));
            fs::write(path, value).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn checkout(root: &Path, repo: &mut Repository, revision: &str) -> Result<(), String> {
        let snapshot = tree(repo, revision)?.clone();
        for path in repo.index.keys() {
            if !snapshot.contains_key(path) {
                let _ = fs::remove_file(root.join(path));
            }
        }
        for (path, e) in &snapshot {
            let physical = root.join(path);
            fs::create_dir_all(physical.parent().unwrap()).map_err(|e| e.to_string())?;
            let bytes =
                fs::read(common(root).join("objects").join(&e.oid)).map_err(|e| e.to_string())?;
            if fs::symlink_metadata(&physical).is_ok() {
                fs::remove_file(&physical).map_err(|e| e.to_string())?;
            }
            if e.mode == "120000" {
                std::os::unix::fs::symlink(std::ffi::OsStr::from_bytes(&bytes), physical)
                    .map_err(|e| e.to_string())?;
            } else {
                fs::write(&physical, bytes).map_err(|e| e.to_string())?;
                fs::set_permissions(
                    physical,
                    fs::Permissions::from_mode(if e.mode == "100755" { 0o755 } else { 0o644 }),
                )
                .map_err(|e| e.to_string())?;
            }
        }
        repo.head = resolve(repo, revision);
        repo.index = snapshot;
        save(root, repo)
    }
    fn commit(
        root: &Path,
        repo: &mut Repository,
        snapshot: Tree,
        parents: Vec<String>,
    ) -> Result<String, String> {
        let content = serde_json::to_vec(&snapshot).unwrap();
        let tree_oid = hash("tree", &content, repo.width);
        repo.trees.insert(tree_oid.clone(), snapshot.clone());
        fs::write(common(root).join("objects").join(&tree_oid), &content)
            .map_err(|e| e.to_string())?;
        let mut bytes = format!("tree {tree_oid}\n");
        for parent in &parents {
            bytes.push_str(&format!("parent {parent}\n"));
        }
        bytes.push_str(&format!("\nfixture {}\n", repo.commits.len()));
        let oid = hash("commit", bytes.as_bytes(), repo.width);
        fs::write(common(root).join("objects").join(&oid), bytes).map_err(|e| e.to_string())?;
        repo.commits.insert(oid.clone(), snapshot);
        repo.parents.insert(oid.clone(), parents);
        Ok(oid)
    }
    fn import(root: &Path, repo: &mut Repository, source: &Path) -> Result<Repository, String> {
        let other = load(source)?;
        copy_directory(
            &common(source).join("objects"),
            &common(root).join("objects"),
        )
        .map_err(|e| e.to_string())?;
        repo.commits.extend(other.commits.clone());
        repo.trees.extend(other.trees.clone());
        repo.parents.extend(other.parents.clone());
        Ok(other)
    }
    fn remote(repo: &Repository, name: &str) -> PathBuf {
        PathBuf::from(
            repo.remotes
                .get(name)
                .map(String::as_str)
                .unwrap_or(name)
                .trim_start_matches("file://"),
        )
    }

    pub fn run(mut root: PathBuf, raw: &[String], input: &[u8]) -> Result<Vec<u8>, String> {
        let mut args = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            match raw[i].as_str() {
                "-C" => {
                    let p = PathBuf::from(raw.get(i + 1).ok_or("missing -C")?);
                    root = if p.is_absolute() { p } else { root.join(p) };
                    i += 2;
                }
                value if value.starts_with("--git-dir=") => {
                    root = PathBuf::from(value.trim_start_matches("--git-dir="));
                    i += 1;
                }
                "-c" => {
                    i += 2;
                }
                "--no-optional-locks" | "--no-replace-objects" | "--literal-pathspecs" => {
                    i += 1;
                }
                _ => {
                    args.extend_from_slice(&raw[i..]);
                    break;
                }
            }
        }
        let command = args.first().ok_or("empty fixture command")?.as_str();
        if !super::known_git_command(command) {
            return Err(format!("unhandled Git fixture command: {args:?}"));
        }
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        if a == ["--version"] {
            return Ok(b"git version fixture\n".to_vec());
        }
        // Close the option domain at the actual command boundary as well as the
        // command-name gate. Operands remain fixture paths/messages, never tools.
        if command == "add"
            && a[1..]
                .iter()
                .any(|arg| arg.starts_with('-') && !matches!(*arg, "-A" | "--all" | "-f" | "--"))
        {
            return Err(format!("unhandled Git fixture argv: {a:?}"));
        }
        if command == "ls-files"
            && a[1..].iter().any(|arg| {
                arg.starts_with('-')
                    && !matches!(
                        *arg,
                        "--stage"
                            | "-z"
                            | "--others"
                            | "--exclude-standard"
                            | "--ignored"
                            | "-v"
                            | "--cached"
                            | "--unmerged"
                            | "--"
                    )
            })
        {
            return Err(format!("unhandled Git fixture argv: {a:?}"));
        }
        if command == "commit" {
            let mut at = 1;
            while at < a.len() {
                match a[at] {
                    "-q"
                    | "--quiet"
                    | "-a"
                    | "-qa"
                    | "--allow-empty"
                    | "--allow-empty-message"
                    | "--no-gpg-sign" => at += 1,
                    "-m" | "-qm" | "-am" | "-qam" | "-aqm" => {
                        if a.get(at + 1).is_none() {
                            return Err("missing fixture commit message".into());
                        }
                        at += 2;
                    }
                    _ => return Err(format!("unhandled Git fixture argv: {a:?}")),
                }
            }
        }
        if command == "check-ref-format" {
            let value = a.last().ok_or("missing ref")?;
            if value.contains(' ')
                || value.contains("..")
                || value.ends_with('/')
                || !value.contains('/')
            {
                return Err("invalid fixture reference".into());
            }
            return Ok(vec![]);
        }
        if command == "init" {
            if let Some(path) = a.get(1).filter(|p| !p.starts_with('-')) {
                root = root.join(path);
            }
            let bare = a.contains(&"--bare");
            let dir = if bare {
                root.clone()
            } else {
                root.join(".git")
            };
            fs::create_dir_all(dir.join("objects")).map_err(|e| e.to_string())?;
            fs::create_dir_all(dir.join("hooks")).map_err(|e| e.to_string())?;
            fs::create_dir_all(dir.join("info")).map_err(|e| e.to_string())?;
            fs::write(dir.join("config"), b"[core]\nrepositoryformatversion = 0\n")
                .map_err(|e| e.to_string())?;
            let branch = a
                .iter()
                .find_map(|arg| arg.strip_prefix("--initial-branch="))
                .or_else(|| {
                    a.iter()
                        .position(|p| matches!(*p, "-b" | "--initial-branch"))
                        .and_then(|i| a.get(i + 1).copied())
                })
                .unwrap_or("dev");
            let repo = Repository {
                width: if a.contains(&"--object-format=sha256") {
                    64
                } else {
                    40
                },
                branch: format!("refs/heads/{branch}"),
                bare,
                main_root: fs::canonicalize(&root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .unwrap()
                    .into(),
                ..Default::default()
            };
            save(&root, &repo)?;
            return Ok(vec![]);
        }
        if command == "clone" {
            let mut operands = Vec::new();
            let mut branch = None;
            let mut depth = None;
            let mut i = 1;
            while i < a.len() {
                match a[i] {
                    "--branch" | "-b" => {
                        branch = Some(*a.get(i + 1).ok_or("missing clone branch")?);
                        i += 2;
                    }
                    "--depth" => {
                        depth = Some(*a.get(i + 1).ok_or("missing clone depth")?);
                        i += 2;
                    }
                    "--quiet" | "-q" | "--bare" | "--no-local" | "--no-hardlinks"
                    | "--no-checkout" => i += 1,
                    v if v.starts_with("--depth=") => {
                        depth = Some(v.trim_start_matches("--depth="));
                        i += 1;
                    }
                    v if v.starts_with('-') => {
                        return Err(format!("unsupported fixture clone option {v}"));
                    }
                    v => {
                        operands.push(v);
                        i += 1;
                    }
                }
            }
            if operands.len() != 2 {
                return Err(format!("unhandled clone {a:?}"));
            }
            let source = PathBuf::from(operands[0].trim_start_matches("file://"));
            let target = root.join(operands[1]);
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            let bare = a.contains(&"--bare");
            copy_directory(
                &common(&source),
                &if bare {
                    target.clone()
                } else {
                    target.join(".git")
                },
            )
            .map_err(|e| e.to_string())?;
            let mut repo = load(&target)?;
            repo.bare = bare;
            repo.main_root = fs::canonicalize(&target)
                .map_err(|e| e.to_string())?
                .to_str()
                .unwrap()
                .into();
            repo.worktrees.clear();
            repo.remotes
                .insert("origin".into(), source.to_str().unwrap().into());
            for (name, oid) in repo.refs.clone() {
                if let Some(branch) = name.strip_prefix("refs/heads/") {
                    repo.refs
                        .insert(format!("refs/remotes/origin/{branch}"), oid);
                }
            }
            if let Some(branch) = branch {
                repo.branch = format!("refs/heads/{branch}");
                repo.head = resolve(&repo, branch);
                tree(&repo, &repo.head)?;
            }
            if !bare && !a.contains(&"--no-checkout") {
                let head = repo.head.clone();
                repo.index.clear();
                checkout(&target, &mut repo, &head)?;
            }
            if depth.is_some() {
                fs::write(common(&target).join("shallow"), format!("{}\n", repo.head))
                    .map_err(|e| e.to_string())?;
            }
            save(&target, &repo)?;
            return Ok(vec![]);
        }
        // Current consumers sometimes invoke legacy acquisition from a subdirectory.
        while !root.join(".git").exists() && !root.join("fixture.json").exists() {
            if !root.pop() {
                return Err("outside fixture repository".into());
            }
        }
        let command_lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(common(&root).join("fixture-command.lock"))
            .map_err(|e| e.to_string())?;
        command_lock.lock().map_err(|e| e.to_string())?;
        let mut repo = load(&root)?;
        let mut out = Vec::new();
        match command {
            "add" => {
                let force = a.contains(&"-f");
                let paths: Vec<_> = a[1..]
                    .iter()
                    .filter(|p| !p.starts_with('-'))
                    .copied()
                    .collect();
                let all =
                    paths.iter().any(|p| *p == ".") || a.contains(&"--all") || a.contains(&"-A");
                let mut files = Vec::new();
                walk(&root, &root, &mut files).map_err(|e| e.to_string())?;
                if all {
                    repo.index
                        .retain(|p, _| fs::symlink_metadata(root.join(p)).is_ok());
                }
                for path in files {
                    if !(all
                        || paths
                            .iter()
                            .any(|p| path == *p || path.starts_with(&format!("{p}/"))))
                    {
                        continue;
                    }
                    if !force && ignored(&root, &path) && !repo.index.contains_key(&path) {
                        continue;
                    }
                    let (mut e, mut bytes) =
                        entry(&root, &path, repo.width).map_err(|e| e.to_string())?;
                    // These explicit fixture controls model Git's conversion result;
                    // the production checkout must still inspect physical raw bytes.
                    if path == "payload" {
                        let attrs =
                            fs::read_to_string(root.join(".gitattributes")).unwrap_or_default();
                        if attrs.contains("payload filter=hide") {
                            bytes = b"canonical\n".to_vec();
                        }
                        if attrs.contains("payload text") {
                            bytes = bytes.replace_crlf();
                        }
                        e.oid = hash("blob", &bytes, repo.width);
                    }
                    fs::write(common(&root).join("objects").join(&e.oid), bytes)
                        .map_err(|e| e.to_string())?;
                    repo.index.insert(path, e);
                }
                save(&root, &repo)?;
            }
            "commit" => {
                if a.iter()
                    .any(|v| matches!(*v, "-a" | "-am" | "-qam" | "-aqm" | "-qa"))
                {
                    let paths: Vec<_> = repo.index.keys().cloned().collect();
                    for path in paths {
                        if fs::symlink_metadata(root.join(&path)).is_ok() {
                            let (entry, bytes) =
                                entry(&root, &path, repo.width).map_err(|e| e.to_string())?;
                            fs::write(common(&root).join("objects").join(&entry.oid), bytes)
                                .map_err(|e| e.to_string())?;
                            repo.index.insert(path, entry);
                        } else {
                            repo.index.remove(&path);
                        }
                    }
                }
                let parents = if repo.head.is_empty() {
                    vec![]
                } else {
                    vec![repo.head.clone()]
                };
                let snapshot = repo.index.clone();
                let oid = commit(&root, &mut repo, snapshot, parents)?;
                repo.head = oid.clone();
                if !repo.branch.is_empty() {
                    repo.refs.insert(repo.branch.clone(), oid);
                }
                save(&root, &repo)?;
            }
            "commit-tree" => {
                let snapshot = repo
                    .trees
                    .get(*a.get(1).ok_or("missing tree")?)
                    .cloned()
                    .ok_or("missing fixture tree")?;
                let parents = a
                    .windows(2)
                    .filter(|p| p[0] == "-p")
                    .map(|p| resolve(&repo, p[1]))
                    .collect();
                let oid = commit(&root, &mut repo, snapshot, parents)?;
                save(&root, &repo)?;
                out = format!("{oid}\n").into_bytes();
            }
            "rev-parse" => {
                let mut symbolic = false;
                let mut abbrev = false;
                for value in &a[1..] {
                    if *value == "--symbolic-full-name" {
                        symbolic = true;
                        continue;
                    }
                    if *value == "--abbrev-ref" {
                        abbrev = true;
                        continue;
                    }
                    if matches!(
                        *value,
                        "--verify" | "--end-of-options" | "--path-format=absolute"
                    ) {
                        continue;
                    }
                    let text = match *value {
                        "--show-toplevel" => root.to_str().unwrap().into(),
                        "--git-common-dir" => common(&root).to_str().unwrap().into(),
                        "--absolute-git-dir" | "--git-dir" => {
                            metadata(&root).to_str().unwrap().into()
                        }
                        "--is-bare-repository" => repo.bare.to_string(),
                        "--is-inside-work-tree" => (!repo.bare).to_string(),
                        "--is-shallow-repository" => {
                            common(&root).join("shallow").is_file().to_string()
                        }
                        "--show-object-format" => {
                            if repo.width == 64 { "sha256" } else { "sha1" }.into()
                        }
                        "HEAD" if abbrev => {
                            if repo.branch.is_empty() {
                                "HEAD".into()
                            } else {
                                repo.branch.trim_start_matches("refs/heads/").into()
                            }
                        }
                        "HEAD" if symbolic => {
                            if repo.branch.is_empty() {
                                "HEAD".into()
                            } else {
                                repo.branch.clone()
                            }
                        }
                        value if value.ends_with("^{tree}") => hash(
                            "tree",
                            &serde_json::to_vec(tree(&repo, value.trim_end_matches("^{tree}"))?)
                                .unwrap(),
                            repo.width,
                        ),
                        value if value.starts_with('-') => {
                            return Err(format!("unhandled rev-parse {a:?}"));
                        }
                        value if value.contains(':') => object(&repo, value)?.0,
                        value => {
                            tree(&repo, value)?;
                            resolve(&repo, value)
                        }
                    };
                    out.extend_from_slice(format!("{text}\n").as_bytes());
                }
            }
            "show" if a.len() == 2 => {
                let (oid, _) = object(&repo, a.last().unwrap())?;
                out =
                    fs::read(common(&root).join("objects").join(oid)).map_err(|e| e.to_string())?;
            }
            "ls-tree" => {
                let (revision, selected) = if a.len() == 4 && a[1..3] == ["-rz", "--full-tree"] {
                    (a[3], None)
                } else if a.len() == 5 && a[1] == "-z" && a[3] == "--" {
                    (a[2], Some(a[4]))
                } else if a.len() == 5 && a[1] == "-rz" && a[3] == "--" {
                    (a[2], Some(a[4]))
                } else {
                    return Err(format!("unhandled ls-tree {a:?}"));
                };
                for (path, e) in tree(&repo, revision)? {
                    if selected.is_some_and(|s| {
                        s != path && !path.starts_with(&format!("{}/", s.trim_end_matches('/')))
                    }) {
                        continue;
                    }
                    out.extend_from_slice(
                        format!("{} blob {}\t{path}\0", e.mode, e.oid).as_bytes(),
                    );
                }
            }
            "cat-file" => {
                if a.get(1)
                    .is_some_and(|s| *s == "--batch" || s.starts_with("--batch-check"))
                    && (a.len() == 2 || (a.len() == 3 && a[2] == "-z"))
                {
                    let zero = a.contains(&"-z");
                    for query in input
                        .split(|b| *b == if zero { 0 } else { b'\n' })
                        .filter(|s| !s.is_empty())
                    {
                        let query = std::str::from_utf8(query).map_err(|e| e.to_string())?;
                        let found = object(&repo, query).ok().and_then(|(oid, kind)| {
                            fs::read(common(&root).join("objects").join(&oid))
                                .ok()
                                .map(|bytes| (oid, kind, bytes))
                        });
                        let Some((oid, kind, bytes)) = found else {
                            out.extend_from_slice(format!("{query} missing\n").as_bytes());
                            continue;
                        };
                        if a[1] == "--batch-check=%(objectname) %(objecttype)" {
                            out.extend_from_slice(format!("{oid} {kind}\n").as_bytes());
                        } else {
                            out.extend_from_slice(
                                format!("{oid} {kind} {}\n", bytes.len()).as_bytes(),
                            );
                        }
                        if a[1] == "--batch" {
                            out.extend_from_slice(&bytes);
                            out.push(b'\n');
                        }
                    }
                } else if a.len() == 3 && matches!(a[1], "-e" | "-t" | "-p") {
                    let (oid, kind) = object(&repo, a[2])?;
                    let bytes = fs::read(common(&root).join("objects").join(oid))
                        .map_err(|e| e.to_string())?;
                    if a[1] == "-t" {
                        out = format!("{kind}\n").into_bytes();
                    } else if a[1] == "-p" {
                        out = bytes;
                    }
                } else if a.len() == 3 && matches!(a[1], "blob" | "commit") {
                    let (oid, kind) = object(&repo, a[2])?;
                    if kind != a[1] {
                        return Err("wrong fixture object type".into());
                    }
                    out = fs::read(common(&root).join("objects").join(oid))
                        .map_err(|e| e.to_string())?;
                } else {
                    return Err(format!("unhandled cat-file {a:?}"));
                }
            }
            "ls-files" => {
                if a.contains(&"--unmerged") {
                    return Ok(if repo.unmerged {
                        b"100644 fixture 1\tconflict\n".to_vec()
                    } else {
                        vec![]
                    });
                }
                let mut files = Vec::new();
                let other = a.contains(&"--others");
                if other {
                    walk(&root, &root, &mut files).map_err(|e| e.to_string())?;
                } else {
                    files.extend(repo.index.keys().cloned());
                }
                files.sort();
                let selected = a.iter().position(|s| *s == "--").map(|i| &a[i + 1..]);
                let operands: Vec<_> = a[1..]
                    .iter()
                    .filter(|p| !p.starts_with('-'))
                    .copied()
                    .collect();
                for path in files {
                    if other && repo.index.contains_key(&path) {
                        continue;
                    }
                    if other
                        && a.contains(&"--exclude-standard")
                        && ignored(&root, &path) != a.contains(&"--ignored")
                    {
                        continue;
                    }
                    if selected.is_none()
                        && !operands.is_empty()
                        && !operands.iter().any(|p| {
                            path == *p || path.starts_with(&format!("{}/", p.trim_end_matches('/')))
                        })
                    {
                        continue;
                    }
                    if let Some(selected) = selected {
                        if selected
                            .iter()
                            .filter(|s| !s.starts_with(":("))
                            .any(|s| *s != ".")
                            && !selected.iter().any(|s| {
                                *s == path
                                    || path.starts_with(&format!("{}/", s.trim_end_matches('/')))
                            })
                        {
                            continue;
                        }
                        if selected.iter().any(|s| {
                            s.strip_prefix(":(top,exclude,literal)")
                                .is_some_and(|p| path.starts_with(p))
                        }) {
                            continue;
                        }
                    }
                    if a.contains(&"--stage") {
                        let e = &repo.index[&path];
                        out.extend_from_slice(format!("{} {} 0\t", e.mode, e.oid).as_bytes());
                    } else if a.contains(&"-v") {
                        out.extend_from_slice(
                            format!(
                                "{} ",
                                repo.flags.get(&path).map(String::as_str).unwrap_or("H")
                            )
                            .as_bytes(),
                        );
                    }
                    out.extend_from_slice(path.as_bytes());
                    out.push(if a.contains(&"-z") { 0 } else { b'\n' });
                }
            }
            "config" if a == ["config", "--local", "--get", "core.repositoryformatversion"] => {
                out = b"0\n".to_vec()
            }
            "config" if a.len() == 4 && a[1] == "--add" => {
                repo.config.insert(a[2].into(), a[3].into());
                save(&root, &repo)?;
            }
            "config" if a.len() == 3 && !a[1].starts_with('-') => {
                repo.config.insert(a[1].into(), a[2].into());
                save(&root, &repo)?;
            }
            "config" if a.len() == 3 && a[1] == "--get" => {
                out = format!(
                    "{}\n",
                    repo.config
                        .get(a[2])
                        .ok_or("missing fixture configuration")?
                )
                .into_bytes();
            }
            "diff" if a.len() == 4 && a[1] == "--name-only" && a[3] == "--" => {
                tree(&repo, a[2])?;
                out = changed_paths(&root, &repo, tree(&repo, a[2])?, false)?.into_bytes();
            }
            "update-index" => {
                let args: Vec<_> = a[1..].iter().filter(|v| **v != "--").copied().collect();
                match args.as_slice() {
                    [flag, path]
                        if matches!(
                            *flag,
                            "--assume-unchanged"
                                | "--skip-worktree"
                                | "--no-assume-unchanged"
                                | "--no-skip-worktree"
                        ) =>
                    {
                        repo.flags.insert(
                            (*path).into(),
                            match *flag {
                                "--skip-worktree" => "S",
                                "--assume-unchanged" => "h",
                                _ => "H",
                            }
                            .into(),
                        );
                    }
                    ["--chmod=+x", path] => {
                        repo.index
                            .get_mut(*path)
                            .ok_or("missing fixture index path")?
                            .mode = "100755".into()
                    }
                    ["--add", "--cacheinfo", value] => {
                        let parts: Vec<_> = value.split(',').collect();
                        if parts.len() != 3 || parts[0] != "160000" {
                            return Err("unsupported fixture cacheinfo".into());
                        }
                        repo.index.insert(
                            parts[2].into(),
                            Entry {
                                mode: parts[0].into(),
                                oid: parts[1].into(),
                            },
                        );
                    }
                    _ => return Err(format!("unsupported update-index {a:?}")),
                }
                save(&root, &repo)?;
            }
            "update-ref" => {
                let args: Vec<_> = a[1..]
                    .iter()
                    .filter(|v| **v != "--no-deref")
                    .copied()
                    .collect();
                let delete = args.first() == Some(&"-d");
                let at = usize::from(delete);
                let name = *args.get(at).ok_or("missing update-ref name")?;
                let expected = args.get(at + if delete { 1 } else { 2 });
                if let Some(expected) = expected {
                    if repo.refs.get(name).map(String::as_str).unwrap_or("") != *expected
                        && !(expected.chars().all(|c| c == '0') && !repo.refs.contains_key(name))
                    {
                        return Err("fixture update-ref compare failed".into());
                    }
                }
                if delete {
                    repo.refs.remove(name);
                    let _ = fs::remove_file(common(&root).join(name));
                } else {
                    let oid = resolve(&repo, args.get(at + 1).ok_or("missing update-ref object")?);
                    tree(&repo, &oid)?;
                    repo.symbolic.remove(name);
                    repo.refs.insert(name.into(), oid.clone());
                    if repo.branch == name {
                        repo.head = oid;
                    }
                }
                save(&root, &repo)?;
            }
            "reset" if a.len() == 3 && matches!(a[1], "--mixed" | "--hard" | "--soft") => {
                if a[1] == "--hard" {
                    checkout(&root, &mut repo, a[2])?;
                } else {
                    if a[1] != "--soft" {
                        repo.index = tree(&repo, a[2])?.clone();
                    }
                    repo.head = resolve(&repo, a[2]);
                }
                if !repo.branch.is_empty() {
                    repo.refs.insert(repo.branch.clone(), repo.head.clone());
                }
                save(&root, &repo)?;
            }
            "checkout" | "switch" => {
                let args: Vec<_> = a[1..].iter().filter(|v| **v != "-q").copied().collect();
                if args.first() == Some(&"--orphan") {
                    repo.branch =
                        format!("refs/heads/{}", args.get(1).ok_or("missing orphan name")?);
                    repo.head.clear();
                    save(&root, &repo)?;
                } else if let Some(at) = args.iter().position(|v| *v == "--") {
                    let revision = args
                        .first()
                        .filter(|s| **s != "--")
                        .copied()
                        .unwrap_or("HEAD");
                    let snap = tree(&repo, revision)?.clone();
                    for path in &args[at + 1..] {
                        let e = snap.get(*path).ok_or("missing checkout fixture path")?;
                        fs::write(
                            root.join(path),
                            fs::read(common(&root).join("objects").join(&e.oid))
                                .map_err(|e| e.to_string())?,
                        )
                        .map_err(|e| e.to_string())?;
                        repo.index.insert((*path).into(), e.clone());
                    }
                    save(&root, &repo)?;
                } else {
                    let new = args
                        .iter()
                        .position(|v| matches!(*v, "-b" | "-B" | "-qb" | "-c"));
                    let (revision, branch) = if let Some(at) = new {
                        let name = args.get(at + 1).ok_or("missing branch")?;
                        (
                            args.get(at + 2).copied().unwrap_or("HEAD"),
                            format!("refs/heads/{name}"),
                        )
                    } else {
                        let revision = args
                            .iter()
                            .find(|v| !v.starts_with('-'))
                            .copied()
                            .unwrap_or("HEAD");
                        let branch = format!("refs/heads/{revision}");
                        (
                            revision,
                            if !args.contains(&"--detach") && repo.refs.contains_key(&branch) {
                                branch
                            } else {
                                String::new()
                            },
                        )
                    };
                    let oid = resolve(&repo, revision);
                    checkout(&root, &mut repo, &oid)?;
                    repo.branch = branch;
                    if !repo.branch.is_empty() {
                        repo.refs.insert(repo.branch.clone(), oid);
                    }
                    save(&root, &repo)?;
                    drop(command_lock);
                    run_hook(&root)?;
                }
            }
            "status"
                if a[1..].iter().all(|v| {
                    matches!(
                        *v,
                        "--porcelain"
                            | "--porcelain=v1"
                            | "--untracked-files=normal"
                            | "--untracked-files=all"
                            | "--ignore-submodules=none"
                            | "-z"
                            | "--short"
                            | "--ignored"
                    )
                }) =>
            {
                let empty = Tree::new();
                let snapshot = if repo.head.is_empty() {
                    &empty
                } else {
                    tree(&repo, "HEAD")?
                };
                let mut files = Vec::new();
                walk(&root, &root, &mut files).map_err(|e| e.to_string())?;
                let paths: std::collections::BTreeSet<_> =
                    snapshot.keys().chain(repo.index.keys()).cloned().collect();
                for path in paths {
                    let staged = match (snapshot.get(&path), repo.index.get(&path)) {
                        (None, Some(_)) => 'A',
                        (Some(_), None) => 'D',
                        (Some(x), Some(y)) if x.oid != y.oid || x.mode != y.mode => 'M',
                        _ => ' ',
                    };
                    let actual = entry(&root, &path, repo.width).ok();
                    let working = match (repo.index.get(&path), actual) {
                        (Some(_), None) => 'D',
                        (Some(x), Some((y, _))) if x.oid != y.oid || x.mode != y.mode => 'M',
                        _ => ' ',
                    };
                    if staged != ' ' || working != ' ' {
                        out.extend_from_slice(format!("{staged}{working} {path}").as_bytes());
                        out.push(if a.contains(&"-z") { 0 } else { b'\n' });
                    }
                }
                for path in files {
                    if !snapshot.contains_key(&path) && !repo.index.contains_key(&path) {
                        let ignored = ignored(&root, &path);
                        if !ignored || a.contains(&"--ignored") {
                            out.extend_from_slice(
                                format!("{} {path}", if ignored { "!!" } else { "??" }).as_bytes(),
                            );
                            out.push(if a.contains(&"-z") { 0 } else { b'\n' });
                        }
                    }
                }
            }
            "worktree" if a.get(1) == Some(&"add") => {
                let previous = repo.worktrees.clone();
                let output = worktree(&root, &mut repo, &a)?;
                let added = repo
                    .worktrees
                    .keys()
                    .find(|path| !previous.contains_key(*path))
                    .cloned()
                    .ok_or("missing new fixture worktree")?;
                drop(command_lock);
                run_hook(Path::new(&added))?;
                return Ok(output);
            }
            _ => return extra(&root, &mut repo, &a, input),
        }
        Ok(out)
    }

    trait CrLf {
        fn replace_crlf(&self) -> Vec<u8>;
    }
    impl CrLf for Vec<u8> {
        fn replace_crlf(&self) -> Vec<u8> {
            let mut out = Vec::new();
            for (i, b) in self.iter().enumerate() {
                if *b != b'\r' || self.get(i + 1) != Some(&b'\n') {
                    out.push(*b);
                }
            }
            out
        }
    }

    pub fn main() {
        dispatch(&std::env::args().skip(1).collect::<Vec<_>>());
    }

    pub fn dispatch(args: &[String]) {
        let mut input = Vec::new();
        if args.iter().any(|s| s.starts_with("--batch")) {
            io::stdin().read_to_end(&mut input).unwrap();
        }
        match run(std::env::current_dir().unwrap(), args, &input) {
            Ok(bytes) => {
                use std::io::Write;
                io::stdout().write_all(&bytes).unwrap();
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(match error.as_str() {
                    "missing fixture reference"
                    | "not a fixture ancestor"
                    | "not a symbolic fixture ref" => 1,
                    "missing remote fixture reference" => 2,
                    _ => 93,
                });
            }
        }
    }

    // The model only supplies interfaces used by the registered snapshot, event,
    // source-custody and lifecycle cases. It never discovers or executes Git.
    fn run_hook(root: &Path) -> Result<(), String> {
        let path = common(root).join("hooks/post-checkout");
        if path.is_file()
            && fs::metadata(&path)
                .map_err(|e| e.to_string())?
                .permissions()
                .mode()
                & 0o111
                != 0
        {
            let mut command = std::process::Command::new(path);
            command
                .current_dir(root)
                .args(["fixture-old", "fixture-new", "1"]);
            chrono_harness::process_fds::forward_command(&mut command, &[])?;
            let output = command.output().map_err(|e| e.to_string())?;
            use std::io::Write;
            io::stdout()
                .write_all(&output.stdout)
                .map_err(|e| e.to_string())?;
            io::stderr()
                .write_all(&output.stderr)
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("fixture post-checkout hook failed".into());
            }
        }
        Ok(())
    }
    fn ancestor(repo: &Repository, base: &str, head: &str) -> bool {
        let base = resolve(repo, base);
        let mut pending = vec![resolve(repo, head)];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(oid) = pending.pop() {
            if oid == base {
                return true;
            }
            if seen.insert(oid.clone()) {
                pending.extend(repo.parents.get(&oid).into_iter().flatten().cloned());
            }
        }
        false
    }
    fn extra(
        root: &Path,
        repo: &mut Repository,
        a: &[&str],
        _input: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        match a[0] {
            "symbolic-ref" => {
                let args: Vec<_> = a[1..]
                    .iter()
                    .filter(|v| **v != "--quiet" && **v != "--short")
                    .copied()
                    .collect();
                match args.as_slice() {
                    ["HEAD"] if !repo.branch.is_empty() => {
                        out = format!(
                            "{}\n",
                            if a.contains(&"--short") {
                                repo.branch.trim_start_matches("refs/heads/")
                            } else {
                                &repo.branch
                            }
                        )
                        .into_bytes()
                    }
                    ["HEAD", reference] => {
                        repo.branch = (*reference).into();
                        repo.head = resolve(repo, reference);
                        save(root, repo)?;
                    }
                    [reference] => {
                        let path = common(root).join(reference);
                        let bytes =
                            fs::read_to_string(path).map_err(|_| "not a symbolic fixture ref")?;
                        let target = bytes
                            .trim()
                            .strip_prefix("ref: ")
                            .ok_or("not a symbolic fixture ref")?;
                        out = format!("{target}\n").into_bytes();
                    }
                    [reference, target] => {
                        repo.symbolic.insert((*reference).into(), (*target).into());
                        repo.refs.insert((*reference).into(), resolve(repo, target));
                        let path = common(root).join(reference);
                        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                        fs::write(path, format!("ref: {target}\n")).map_err(|e| e.to_string())?;
                    }
                    _ => return Err(format!("unhandled symbolic-ref {a:?}")),
                }
            }
            "branch" => {
                if a == ["branch", "--show-current"] {
                    return Ok(
                        format!("{}\n", repo.branch.trim_start_matches("refs/heads/")).into_bytes(),
                    );
                }
                let args: Vec<_> = a[1..]
                    .iter()
                    .filter(|v| !v.starts_with('-'))
                    .copied()
                    .collect();
                if a.iter().any(|v| matches!(*v, "-d" | "-D" | "--delete")) {
                    for name in args {
                        let name = format!("refs/heads/{name}");
                        repo.refs.remove(&name);
                        let _ = fs::remove_file(common(root).join(name));
                    }
                } else if a.contains(&"-m") || a.contains(&"-M") {
                    let (old, new) = match args.as_slice() {
                        [new] => (
                            repo.branch.trim_start_matches("refs/heads/").to_owned(),
                            *new,
                        ),
                        [old, new] => ((*old).into(), *new),
                        _ => return Err("unsupported branch rename".into()),
                    };
                    let old = format!("refs/heads/{old}");
                    let new = format!("refs/heads/{new}");
                    let oid = repo.refs.remove(&old).ok_or("missing branch")?;
                    let _ = fs::remove_file(common(root).join(&old));
                    repo.refs.insert(new.clone(), oid);
                    if repo.branch == old {
                        repo.branch = new;
                    }
                } else {
                    let name = args.first().ok_or("missing branch")?;
                    let oid = resolve(repo, args.get(1).copied().unwrap_or("HEAD"));
                    tree(repo, &oid)?;
                    repo.refs.insert(format!("refs/heads/{name}"), oid);
                }
                save(root, repo)?;
            }
            "show-ref" | "for-each-ref" => {
                let selected = a.iter().find(|v| v.starts_with("refs/"));
                for (name, oid) in &repo.refs {
                    if selected.is_some_and(|prefix| {
                        if a[0] == "show-ref" {
                            name != *prefix
                        } else {
                            !name.starts_with(*prefix)
                        }
                    }) {
                        continue;
                    }
                    let line = if a.contains(&"--format=%(refname)") {
                        format!("{name}\n")
                    } else if a.contains(&"--format=%(refname) %(objectname) %(symref)") {
                        format!("{name} {oid} \n")
                    } else if a.contains(&"--hash") {
                        format!("{oid}\n")
                    } else {
                        format!(
                            "{oid} {}{name}\n",
                            if a[0] == "for-each-ref" {
                                "commit\t"
                            } else {
                                ""
                            }
                        )
                    };
                    out.extend_from_slice(line.as_bytes());
                }
                if out.is_empty() && a[0] == "show-ref" {
                    return Err("missing fixture reference".into());
                }
            }
            "remote" => match a {
                ["remote", "add" | "set-url", name, url] => {
                    repo.remotes.insert((*name).into(), (*url).into());
                    save(root, repo)?;
                }
                ["remote", "get-url", name] => {
                    out = format!("{}\n", repo.remotes.get(*name).ok_or("missing remote")?)
                        .into_bytes()
                }
                ["remote", "get-url", "--push", "--all", name] => {
                    let key = format!("remote.{name}.pushurl");
                    let primary = repo.remotes.get(*name).ok_or("missing remote")?;
                    out = if let Some(extra) = repo.config.get(&key) {
                        format!("{primary}\n{extra}\n")
                    } else {
                        format!("{primary}\n")
                    }
                    .into_bytes();
                }
                _ => return Err(format!("unhandled remote {a:?}")),
            },
            "ls-remote" => {
                let args: Vec<_> = a[1..]
                    .iter()
                    .filter(|v| !v.starts_with('-'))
                    .copied()
                    .collect();
                let other = load(&remote(repo, args.first().ok_or("missing remote")?))?;
                for (name, oid) in &other.refs {
                    if args.len() == 1
                        || args[1..].iter().any(|selected| {
                            name == *selected || name.ends_with(&format!("/{selected}"))
                        })
                    {
                        out.extend_from_slice(format!("{oid}\t{name}\n").as_bytes());
                    }
                }
                if out.is_empty() && a.contains(&"--exit-code") {
                    return Err("missing remote fixture reference".into());
                }
            }
            "push" | "fetch" => {
                let args: Vec<_> = a[1..]
                    .iter()
                    .filter(|v| !v.starts_with('-'))
                    .copied()
                    .collect();
                let name = args.first().ok_or("missing remote")?;
                let path = remote(repo, name);
                if a[0] == "fetch" {
                    let other = import(root, repo, &path)
                        .map_err(|e| format!("unhandled Git fixture command: {a:?}; {e}"))?;
                    let spec = args
                        .get(1)
                        .copied()
                        .unwrap_or("dev")
                        .trim_start_matches('+');
                    let (from, to) = spec.split_once(':').unwrap_or((spec, "FETCH_HEAD"));
                    let oid = resolve(&other, from);
                    tree(&other, &oid)?;
                    repo.refs.insert(to.into(), oid.clone());
                    if to == "FETCH_HEAD" {
                        repo.refs.insert(
                            format!(
                                "refs/remotes/{name}/{}",
                                from.trim_start_matches("refs/heads/")
                            ),
                            oid.clone(),
                        );
                    }
                    fs::write(common(root).join("FETCH_HEAD"), format!("{oid}\n"))
                        .map_err(|e| e.to_string())?;
                    if a.contains(&"--unshallow") {
                        let _ = fs::remove_file(common(root).join("shallow"));
                    }
                    if a.iter().any(|v| v.starts_with("--depth=")) {
                        fs::write(common(root).join("shallow"), format!("{oid}\n"))
                            .map_err(|e| e.to_string())?;
                    }
                    save(root, repo)?;
                } else {
                    let mut other = load(&path)?;
                    import(&path, &mut other, root)?;
                    for spec in &args[1..] {
                        let (from, to) = spec.split_once(':').unwrap_or((spec, spec));
                        let to = if to.starts_with("refs/") {
                            to.into()
                        } else {
                            format!("refs/heads/{to}")
                        };
                        for lease in a
                            .iter()
                            .filter_map(|v| v.strip_prefix("--force-with-lease="))
                        {
                            let (reference, expected) =
                                lease.split_once(':').ok_or("missing fixture lease")?;
                            if reference == to
                                && other.refs.get(reference).map(String::as_str).unwrap_or("")
                                    != expected
                            {
                                return Err("fixture remote lease rejected".into());
                            }
                        }
                        if from.is_empty() || a.contains(&"--delete") {
                            other.refs.remove(&to);
                            let _ = fs::remove_file(common(&path).join(&to));
                        } else {
                            let from = from.trim_start_matches('+');
                            let oid = resolve(repo, from);
                            tree(repo, &oid)?;
                            let forced = spec.starts_with('+')
                                || a.contains(&"--force")
                                || a.contains(&"-f")
                                || a.iter().any(|arg| arg.starts_with("--force-with-lease="));
                            if !forced
                                && other
                                    .refs
                                    .get(&to)
                                    .is_some_and(|old| !ancestor(&other, old, &oid))
                            {
                                return Err("fixture remote non-fast-forward rejected".into());
                            }
                            other.refs.insert(to.clone(), oid.clone());
                            repo.refs.insert(
                                format!(
                                    "refs/remotes/{name}/{}",
                                    to.trim_start_matches("refs/heads/")
                                ),
                                oid.clone(),
                            );
                            if other.branch == to {
                                other.head = oid;
                            }
                        }
                    }
                    save(&path, &other)?;
                    save(root, repo)?;
                }
            }
            "merge-base" if a.len() == 4 && a[1] == "--is-ancestor" => {
                if !ancestor(repo, a[2], a[3]) {
                    return Err("not a fixture ancestor".into());
                }
            }
            "rev-list" => {
                let revision = a.last().ok_or("missing revision")?;
                let selected = revision
                    .split_once("..")
                    .map(|(_, head)| head)
                    .unwrap_or(revision);
                let oid = resolve(repo, selected);
                tree(repo, &oid)?;
                if a.contains(&"--count") {
                    let count = if let Some((base, head)) = revision.split_once("..") {
                        let mut pending = vec![resolve(repo, head)];
                        let mut seen = std::collections::BTreeSet::new();
                        while let Some(oid) = pending.pop() {
                            if ancestor(repo, &oid, base) || !seen.insert(oid.clone()) {
                                continue;
                            }
                            pending.extend(repo.parents.get(&oid).into_iter().flatten().cloned());
                        }
                        seen.len()
                    } else {
                        let mut pending = vec![oid.clone()];
                        let mut seen = std::collections::BTreeSet::new();
                        while let Some(next) = pending.pop() {
                            if seen.insert(next.clone()) {
                                pending
                                    .extend(repo.parents.get(&next).into_iter().flatten().cloned());
                            }
                        }
                        seen.len()
                    };
                    return Ok(format!("{count}\n").into_bytes());
                }
                out.extend_from_slice(oid.as_bytes());
                if a.contains(&"--parents") && !common(root).join("shallow").is_file() {
                    for parent in repo.parents.get(&oid).into_iter().flatten() {
                        out.extend_from_slice(format!(" {parent}").as_bytes());
                    }
                }
                out.push(b'\n');
            }
            "merge" => {
                let head = resolve(repo, a.last().ok_or("missing merge head")?);
                if a.contains(&"--ff-only") {
                    if !ancestor(repo, &repo.head, &head) {
                        return Err("fixture merge is not fast-forward".into());
                    }
                    checkout(root, repo, &head)?;
                    if !repo.branch.is_empty() {
                        repo.refs.insert(repo.branch.clone(), head);
                    }
                    save(root, repo)?;
                } else {
                    let mut snapshot = tree(repo, "HEAD")?.clone();
                    snapshot.extend(tree(repo, &head)?.clone());
                    let parents = vec![repo.head.clone(), head];
                    let oid = commit(root, repo, snapshot, parents)?;
                    checkout(root, repo, &oid)?;
                    if !repo.branch.is_empty() {
                        repo.refs.insert(repo.branch.clone(), oid);
                    }
                    save(root, repo)?;
                }
            }
            "write-tree" if a.len() == 1 => {
                if repo.unmerged {
                    return Err("fixture index has unmerged entries".into());
                }
                let bytes = serde_json::to_vec(&repo.index).unwrap();
                let oid = hash("tree", &bytes, repo.width);
                repo.trees.insert(oid.clone(), repo.index.clone());
                fs::write(common(root).join("objects").join(&oid), bytes)
                    .map_err(|e| e.to_string())?;
                save(root, repo)?;
                out = format!("{oid}\n").into_bytes();
            }
            "read-tree" => {
                let get = |name: &str| {
                    repo.trees
                        .get(name)
                        .or_else(|| repo.commits.get(&resolve(repo, name)))
                        .cloned()
                        .ok_or_else(|| format!("missing fixture tree {name}"))
                };
                match a {
                    ["read-tree", revision] => {
                        repo.index = get(revision)?;
                        repo.unmerged = false;
                    }
                    ["read-tree", "-m", base, ours, theirs] => {
                        let base = get(base)?;
                        let ours = get(ours)?;
                        let theirs = get(theirs)?;
                        let paths: std::collections::BTreeSet<_> = base
                            .keys()
                            .chain(ours.keys())
                            .chain(theirs.keys())
                            .cloned()
                            .collect();
                        let mut index = Tree::new();
                        let mut unmerged = false;
                        for path in paths {
                            let (b, o, t) = (base.get(&path), ours.get(&path), theirs.get(&path));
                            let selected = if o == t || t == b {
                                o
                            } else if o == b {
                                t
                            } else {
                                unmerged = true;
                                o
                            };
                            if let Some(entry) = selected {
                                index.insert(path, entry.clone());
                            }
                        }
                        repo.index = index;
                        repo.unmerged = unmerged;
                    }
                    _ => return Err(format!("unhandled fixture read-tree {a:?}")),
                }
                save(root, repo)?;
            }
            "mv" if a.len() == 3 => {
                fs::rename(root.join(a[1]), root.join(a[2])).map_err(|e| e.to_string())?;
                let e = repo.index.remove(a[1]).ok_or("missing fixture mv path")?;
                repo.index.insert(a[2].into(), e);
                save(root, repo)?;
            }
            "rm" if a.len() == 2 => {
                fs::remove_file(root.join(a[1])).map_err(|e| e.to_string())?;
                repo.index.remove(a[1]);
                save(root, repo)?;
            }
            "replace" if a.len() == 3 => {
                let original = resolve(repo, a[1]);
                let replacement = resolve(repo, a[2]);
                tree(repo, &original)?;
                tree(repo, &replacement)?;
                repo.refs
                    .insert(format!("refs/replace/{original}"), replacement);
                save(root, repo)?;
            }
            "diff"
                if a == ["diff", "--name-only"]
                    || a == ["diff", "--cached", "--name-only"]
                    || a == ["diff", "--raw", "-z"]
                    || a == ["diff", "--binary"] =>
            {
                out = changed_paths(root, repo, tree(repo, "HEAD")?, a.contains(&"--cached"))?
                    .into_bytes();
            }
            "reflog" => {
                if a != ["reflog", "show", "--format=%H", "HEAD"]
                    && a != ["reflog", "show", "--format=%H"]
                {
                    return Err(format!("unhandled fixture reflog {a:?}"));
                }
                let bytes = fs::read(metadata(root).join("fixture-reflog.json"))
                    .map_err(|e| e.to_string())?;
                let entries: Vec<String> =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                for oid in entries.iter().rev() {
                    out.extend_from_slice(format!("{oid}\n").as_bytes());
                }
            }
            "worktree" => return worktree(root, repo, a),
            _ => return Err(format!("unhandled Git fixture command: {a:?}")),
        }
        Ok(out)
    }
    fn worktree(root: &Path, repo: &mut Repository, a: &[&str]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let common = common(root);
        match a.get(1).copied().ok_or("missing worktree operation")? {
            "list" => {
                let mut paths = vec![repo.main_root.clone()];
                paths.extend(repo.worktrees.keys().cloned());
                let zero = a.contains(&"-z");
                for path in paths {
                    let path = Path::new(&path);
                    let state = load(path);
                    let state = match state {
                        Ok(v) => v,
                        Err(_) => {
                            out.extend_from_slice(
                                format!(
                                    "worktree {}\nprunable fixture missing metadata\n\n",
                                    path.display()
                                )
                                .as_bytes(),
                            );
                            continue;
                        }
                    };
                    let mut fields = vec![
                        format!("worktree {}", path.display()),
                        format!("HEAD {}", state.head),
                        if state.branch.is_empty() {
                            "detached".into()
                        } else {
                            format!("branch {}", state.branch)
                        },
                    ];
                    if let Ok(reason) = fs::read_to_string(metadata(path).join("locked")) {
                        fields.push(format!("locked {}", reason.trim()));
                    }
                    for field in fields {
                        out.extend_from_slice(field.as_bytes());
                        out.push(if zero { 0 } else { b'\n' });
                    }
                    out.push(if zero { 0 } else { b'\n' });
                }
            }
            "add" => {
                let at = a.iter().position(|p| *p == "--");
                let operands: Vec<_> = if let Some(i) = at {
                    a[i + 1..].to_vec()
                } else {
                    let mut v = vec![];
                    let mut i = 2;
                    while i < a.len() {
                        if matches!(a[i], "-b" | "-B" | "-qb" | "--reason") {
                            i += 2;
                        } else if a[i].starts_with('-') {
                            i += 1;
                        } else {
                            v.push(a[i]);
                            i += 1;
                        }
                    }
                    v
                };
                let path = PathBuf::from(operands.first().ok_or("missing worktree path")?);
                let revision = operands.get(1).copied().unwrap_or("HEAD");
                let oid = resolve(repo, revision);
                tree(repo, &oid)?;
                fs::create_dir_all(&path).map_err(|e| e.to_string())?;
                let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
                let name = path.file_name().unwrap().to_str().unwrap();
                let dir = common.join("worktrees").join(name);
                if dir.exists() {
                    return Err("fixture metadata already exists".into());
                }
                fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                fs::write(path.join(".git"), format!("gitdir: {}\n", dir.display()))
                    .map_err(|e| e.to_string())?;
                fs::write(dir.join("commondir"), "../..\n").map_err(|e| e.to_string())?;
                fs::write(
                    dir.join("gitdir"),
                    format!("{}\n", path.join(".git").display()),
                )
                .map_err(|e| e.to_string())?;
                repo.worktrees
                    .insert(path.to_str().unwrap().into(), dir.to_str().unwrap().into());
                save(root, repo)?;
                let mut linked = load(root)?;
                linked.index.clear();
                linked.branch =
                    if let Some(i) = a.iter().position(|p| matches!(*p, "-b" | "-B" | "-qb")) {
                        format!("refs/heads/{}", a[i + 1])
                    } else if a.contains(&"--detach") {
                        String::new()
                    } else {
                        format!("refs/heads/{revision}")
                    };
                if !linked.branch.is_empty() {
                    linked.refs.insert(linked.branch.clone(), oid.clone());
                }
                linked.head = oid.clone();
                checkout(&path, &mut linked, &oid)?;
                if a.contains(&"--lock") {
                    let reason = a
                        .iter()
                        .position(|p| *p == "--reason")
                        .map(|i| a[i + 1])
                        .unwrap_or("initializing");
                    fs::write(dir.join("locked"), reason).map_err(|e| e.to_string())?;
                }
            }
            "lock" | "unlock" => {
                let path = Path::new(a.last().ok_or("missing worktree path")?);
                let locked = metadata(path).join("locked");
                if a[1] == "lock" {
                    let reason = a
                        .iter()
                        .position(|p| *p == "--reason")
                        .map(|i| a[i + 1])
                        .unwrap_or("locked");
                    fs::write(locked, reason).map_err(|e| e.to_string())?;
                } else {
                    fs::remove_file(locked).map_err(|e| e.to_string())?;
                }
            }
            "remove" => {
                let path = Path::new(a.last().ok_or("missing worktree path")?);
                let dir = metadata(path);
                if dir.join("locked").is_file() {
                    return Err("fixture worktree is locked".into());
                }
                fs::remove_dir_all(path).map_err(|e| e.to_string())?;
                fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
                repo.worktrees.remove(path.to_str().unwrap());
                save(root, repo)?;
            }
            "prune" => {
                repo.worktrees.retain(|path, dir| {
                    if Path::new(path).join(".git").exists() {
                        true
                    } else {
                        let _ = fs::remove_dir_all(dir);
                        false
                    }
                });
                save(root, repo)?;
            }
            "repair" => {
                let path = Path::new(a.last().ok_or("missing repair target")?);
                let dir = metadata(path);
                fs::write(
                    dir.join("gitdir"),
                    format!("{}\n", path.join(".git").display()),
                )
                .map_err(|e| e.to_string())?;
                repo.worktrees.retain(|_, stored| Path::new(stored) != dir);
                repo.worktrees
                    .insert(path.to_str().unwrap().into(), dir.to_str().unwrap().into());
                save(root, repo)?;
            }
            "move" => {
                if a.len() != 4 {
                    return Err("unsupported fixture move".into());
                }
                let from = Path::new(a[2]);
                let to = Path::new(a[3]);
                let dir = metadata(from);
                fs::rename(from, to).map_err(|e| e.to_string())?;
                fs::write(
                    dir.join("gitdir"),
                    format!("{}\n", to.join(".git").display()),
                )
                .map_err(|e| e.to_string())?;
                repo.worktrees.remove(a[2]);
                repo.worktrees
                    .insert(a[3].into(), dir.to_str().unwrap().into());
                save(root, repo)?;
            }
            _ => return Err(format!("unhandled fixture worktree {a:?}")),
        }
        Ok(out)
    }

    fn changed_paths(
        root: &Path,
        repo: &Repository,
        snapshot: &Tree,
        cached: bool,
    ) -> Result<String, String> {
        let paths: std::collections::BTreeSet<_> =
            snapshot.keys().chain(repo.index.keys()).cloned().collect();
        let mut out = String::new();
        for path in paths {
            if !cached && repo.flags.get(&path).is_some_and(|v| v == "h" || v == "S") {
                continue;
            }
            if !cached
                && path == "payload"
                && fs::read_to_string(root.join(".gitattributes"))
                    .unwrap_or_default()
                    .contains("payload filter=hide")
            {
                continue;
            }
            let actual = if cached {
                repo.index.get(&path).cloned()
            } else {
                entry(root, &path, repo.width).ok().map(|(e, _)| e)
            };
            if match (snapshot.get(&path), actual) {
                (Some(x), Some(y)) => x.oid != y.oid || x.mode != y.mode,
                (None, None) => false,
                _ => true,
            } {
                out.push_str(&path);
                out.push('\n');
            }
        }
        Ok(out)
    }
}
#[cfg(not(any(verus_keep_ghost, test)))]
pub use runtime::{dispatch, run};
#[cfg(not(any(verus_keep_ghost, test)))]
pub fn main() {
    runtime::main();
}
