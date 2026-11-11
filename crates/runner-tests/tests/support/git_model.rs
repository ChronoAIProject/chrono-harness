//! Command-boundary Git fixture for runner snapshots and release source custody.
//! This is finite test data, not a Git implementation: unknown commands fail closed.
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Default, Deserialize, Serialize)]
struct Entry {
    mode: String,
    oid: String,
}
type Tree = BTreeMap<String, Entry>;
#[derive(Default, Deserialize, Serialize)]
struct Repository {
    width: usize,
    head: String,
    index: Tree,
    commits: BTreeMap<String, Tree>,
    refs: BTreeMap<String, String>,
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
        if entry.path() == root.join(".git") {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.is_dir() {
            walk(root, &entry.path(), out)?;
        } else {
            out.push(entry.path().strip_prefix(root).unwrap().to_str().unwrap().into());
        }
    }
    Ok(())
}
fn ignored(root: &Path, path: &str) -> bool {
    fs::read_to_string(root.join(".gitignore"))
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .any(|line| {
            let pattern = line.trim_start_matches('/');
            path == pattern.trim_end_matches('/')
                || (pattern.ends_with('/') && path.starts_with(pattern))
        })
}
fn entry(root: &Path, path: &str, width: usize) -> io::Result<(Entry, Vec<u8>)> {
    let physical = root.join(path);
    let metadata = fs::symlink_metadata(&physical)?;
    let (mode, bytes) = if metadata.file_type().is_symlink() {
        ("120000", fs::read_link(physical)?.as_os_str().as_bytes().to_vec())
    } else {
        (
            if metadata.permissions().mode() & 0o111 != 0 { "100755" } else { "100644" },
            fs::read(physical)?,
        )
    };
    Ok((Entry { mode: mode.into(), oid: hash("blob", &bytes, width) }, bytes))
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
fn resolve<'a>(repo: &'a Repository, name: &'a str) -> &'a str {
    if name == "HEAD" { &repo.head }
    else if let Some(oid) = repo.refs.get(name).or_else(|| repo.refs.get(&format!("refs/heads/{name}"))) { oid }
    else { name }
}
fn tree<'a>(repo: &'a Repository, name: &str) -> Result<&'a Tree, String> {
    repo.commits.get(resolve(repo, name)).ok_or_else(|| format!("missing fixture commit {name}"))
}
fn object(repo: &Repository, query: &str) -> Result<(String, &'static str), String> {
    if let Some((revision, path)) = query.split_once(':') {
        let e = tree(repo, revision)?.get(path).ok_or("missing fixture path")?;
        Ok((e.oid.clone(), "blob"))
    } else if repo.commits.contains_key(resolve(repo, query)) {
        Ok((resolve(repo, query).into(), "commit"))
    } else {
        Ok((query.into(), "blob"))
    }
}
fn save(root: &Path, repo: &Repository) -> Result<(), String> {
    fs::write(root.join(".git/fixture.json"), serde_json::to_vec(repo).unwrap())
        .map_err(|e| e.to_string())
}

pub fn run(mut root: PathBuf, raw: &[String], input: &[u8]) -> Result<Vec<u8>, String> {
    let mut args = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "-C" => { root = PathBuf::from(raw.get(i + 1).ok_or("missing -C")?); i += 2; }
            "-c" => { i += 2; }
            "--no-optional-locks" | "--no-replace-objects" | "--literal-pathspecs" => { i += 1; }
            _ => { args.extend_from_slice(&raw[i..]); break; }
        }
    }
    let command = args.first().ok_or("empty fixture command")?.as_str();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    if a == ["--version"] { return Ok(b"git version fixture\n".to_vec()); }
    if command == "init" {
        fs::create_dir_all(root.join(".git/objects")).map_err(|e| e.to_string())?;
        fs::write(root.join(".git/config"), b"[core]\nrepositoryformatversion = 0\n").map_err(|e| e.to_string())?;
        save(&root, &Repository { width: if a.contains(&"--object-format=sha256") {64} else {40}, ..Default::default() })?;
        return Ok(vec![]);
    }
    if command == "clone" && a.len() == 5 && a[1..3] == ["--no-hardlinks", "--no-checkout"] {
        copy_directory(&Path::new(a[3]).join(".git"), &Path::new(a[4]).join(".git")).map_err(|e| e.to_string())?;
        return Ok(vec![]);
    }
    let mut repo: Repository = serde_json::from_slice(&fs::read(root.join(".git/fixture.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    match command {
        "add" => {
            let force = a.contains(&"-f");
            let paths: Vec<_> = a[1..].iter().filter(|p| !p.starts_with('-')).copied().collect();
            let all = paths.iter().any(|p| *p == ".") || a.contains(&"--all");
            let mut files = Vec::new();
            walk(&root, &root, &mut files).map_err(|e| e.to_string())?;
            if all { repo.index.retain(|p, _| fs::symlink_metadata(root.join(p)).is_ok()); }
            for path in files {
                if !(all || paths.iter().any(|p| path == *p || path.starts_with(&format!("{p}/")))) { continue; }
                if !force && ignored(&root, &path) && !repo.index.contains_key(&path) { continue; }
                let (mut e, mut bytes) = entry(&root, &path, repo.width).map_err(|e| e.to_string())?;
                // These explicit fixture controls model Git's conversion result;
                // the production checkout must still inspect physical raw bytes.
                if path == "payload" {
                    let attrs = fs::read_to_string(root.join(".gitattributes")).unwrap_or_default();
                    if attrs.contains("payload filter=hide") { bytes = b"canonical\n".to_vec(); }
                    if attrs.contains("payload text") { bytes = bytes.replace_crlf(); }
                    e.oid = hash("blob", &bytes, repo.width);
                }
                fs::write(root.join(".git/objects").join(&e.oid), bytes).map_err(|e| e.to_string())?;
                repo.index.insert(path, e);
            }
            save(&root, &repo)?;
        }
        "commit" => {
            let content = serde_json::to_vec(&repo.index).unwrap();
            let tree_oid = hash("tree", &content, repo.width);
            let mut commit = format!("tree {tree_oid}\n");
            if !repo.head.is_empty() { commit.push_str(&format!("parent {}\n", repo.head)); }
            commit.push_str(&format!("\nfixture {}\n", repo.commits.len()));
            let oid = hash("commit", commit.as_bytes(), repo.width);
            fs::write(root.join(".git/objects").join(&oid), commit).map_err(|e| e.to_string())?;
            repo.head = oid.clone();
            repo.commits.insert(oid, repo.index.clone());
            save(&root, &repo)?;
        }
        "rev-parse" => {
            if a == ["rev-parse", "--show-toplevel", "--verify", "HEAD"] {
                tree(&repo, "HEAD")?;
                return Ok(format!("{}\n{}\n", root.display(), repo.head).into_bytes());
            }
            if !(a.len() == 2 || (a.len() == 3 && a[1] == "--verify")) {
                return Err(format!("unhandled rev-parse {a:?}"));
            }
            let value = *a.last().unwrap();
            let text = match value {
                "--show-toplevel" => root.to_str().unwrap().to_owned(),
                "--show-object-format" => if repo.width == 64 {"sha256"} else {"sha1"}.into(),
                _ if value.ends_with("^{tree}") => hash("tree", &serde_json::to_vec(tree(&repo, value.trim_end_matches("^{tree}"))?).unwrap(), repo.width),
                _ => {
                    let name = value.trim_end_matches("^{commit}");
                    tree(&repo, name)?;
                    resolve(&repo, name).into()
                }
            };
            out.extend_from_slice(format!("{text}\n").as_bytes());
        }
        "show" if a.len() == 2 => {
            let (oid, _) = object(&repo, a.last().unwrap())?;
            out = fs::read(root.join(".git/objects").join(oid)).map_err(|e| e.to_string())?;
        }
        "ls-tree" => {
            let (revision, selected) = if a.len() == 4 && a[1..3] == ["-rz", "--full-tree"] {
                (a[3], None)
            } else if a.len() == 5 && a[1] == "-z" && a[3] == "--" {
                (a[2], Some(a[4]))
            } else if a.len() == 5 && a[1] == "-rz" && a[3] == "--" {
                (a[2], Some(a[4]))
            } else { return Err(format!("unhandled ls-tree {a:?}")); };
            for (path, e) in tree(&repo, revision)? {
                if selected.is_some_and(|s| s != path && !path.starts_with(&format!("{}/", s.trim_end_matches('/')))) { continue; }
                out.extend_from_slice(format!("{} blob {}\t{path}\0", e.mode, e.oid).as_bytes());
            }
        }
        "cat-file" => {
            if a.get(1).is_some_and(|s| matches!(*s, "--batch-check" | "--batch"))
                && (a.len() == 2 || (a.len() == 3 && a[2] == "-z")) {
                let zero = a.contains(&"-z");
                for query in input.split(|b| *b == if zero {0} else {b'\n'}).filter(|s| !s.is_empty()) {
                    let query = std::str::from_utf8(query).map_err(|e| e.to_string())?;
                    let (oid, kind) = object(&repo, query)?;
                    let bytes = fs::read(root.join(".git/objects").join(&oid)).map_err(|e| e.to_string())?;
                    out.extend_from_slice(format!("{oid} {kind} {}\n", bytes.len()).as_bytes());
                    if a[1] == "--batch" { out.extend_from_slice(&bytes); out.push(b'\n'); }
                }
            } else if a.len() == 3 && matches!(a[1], "blob" | "commit") {
                let (oid, kind) = object(&repo, a[2])?;
                if kind != a[1] { return Err("wrong fixture object type".into()); }
                out = fs::read(root.join(".git/objects").join(oid)).map_err(|e| e.to_string())?;
            } else { return Err(format!("unhandled cat-file {a:?}")); }
        }
        "ls-files" => {
            let mut files = Vec::new();
            let other = a.contains(&"--others");
            if other { walk(&root, &root, &mut files).map_err(|e| e.to_string())?; }
            else { files.extend(repo.index.keys().cloned()); }
            files.sort();
            let selected = a.iter().position(|s| *s == "--").map(|i| &a[i + 1..]);
            for path in files {
                if other && repo.index.contains_key(&path) { continue; }
                if other && a.contains(&"--exclude-standard") && ignored(&root, &path) != a.contains(&"--ignored") { continue; }
                if let Some(selected) = selected {
                    if selected.iter().filter(|s| !s.starts_with(":(")).any(|s| *s != ".")
                        && !selected.iter().any(|s| *s == path || path.starts_with(&format!("{}/", s.trim_end_matches('/')))) { continue; }
                    if selected.iter().any(|s| s.strip_prefix(":(top,exclude,literal)").is_some_and(|p| path.starts_with(p))) { continue; }
                }
                if a.contains(&"--stage") {
                    let e = &repo.index[&path];
                    out.extend_from_slice(format!("{} {} 0\t", e.mode, e.oid).as_bytes());
                } else if a.contains(&"-v") {
                    out.extend_from_slice(format!("{} ", repo.flags.get(&path).map(String::as_str).unwrap_or("H")).as_bytes());
                }
                out.extend_from_slice(path.as_bytes());
                out.push(if a.contains(&"-z") {0} else {b'\n'});
            }
        }
        "config" if a == ["config", "--local", "--get", "core.repositoryformatversion"] => out = b"0\n".to_vec(),
        "config" if a.len() == 3 => { repo.config.insert(a[1].into(), a[2].into()); save(&root, &repo)?; }
        "diff" if a.len() == 4 && a[1] == "--name-only" && a[3] == "--" => {
            tree(&repo, a[2])?;
            // The checkout regressions deliberately configure Git to hide these
            // differences; this independent external response is the control.
        }
        "update-index" if a.len() == 3 && a[1] == "--assume-unchanged" => {
            repo.flags.insert(a[2].into(), "h".into()); save(&root, &repo)?;
        }
        "update-ref" if a.len() == 3 => {
            tree(&repo, a[2])?;
            repo.refs.insert(a[1].into(), a[2].into()); save(&root, &repo)?;
        }
        "reset" if a.len() == 3 && a[1] == "--mixed" => {
            repo.index = tree(&repo, a[2])?.clone(); save(&root, &repo)?;
        }
        "checkout" if a.len() == 3 && a[1] == "--detach" => {
            let snapshot = tree(&repo, a[2])?.clone();
            for (path, e) in &snapshot {
                let physical = root.join(path);
                fs::create_dir_all(physical.parent().unwrap()).map_err(|e| e.to_string())?;
                let bytes = fs::read(root.join(".git/objects").join(&e.oid)).map_err(|e| e.to_string())?;
                if e.mode == "120000" {
                    std::os::unix::fs::symlink(std::ffi::OsStr::from_bytes(&bytes), physical).map_err(|e| e.to_string())?;
                } else {
                    fs::write(&physical, bytes).map_err(|e| e.to_string())?;
                    fs::set_permissions(physical, fs::Permissions::from_mode(if e.mode == "100755" {0o755} else {0o644})).map_err(|e| e.to_string())?;
                }
            }
            repo.head = resolve(&repo, a[2]).into(); repo.index = snapshot; save(&root, &repo)?;
        }
        "status" if a == ["status", "--porcelain", "--untracked-files=normal"]
            || a == ["status", "--porcelain=v1", "--untracked-files=normal", "--ignore-submodules=none"] => {
            let snapshot = tree(&repo, "HEAD")?;
            let mut files = Vec::new(); walk(&root, &root, &mut files).map_err(|e| e.to_string())?;
            for path in snapshot.keys() {
                if entry(&root, path, repo.width).ok().is_none_or(|(e, _)| e.oid != snapshot[path].oid || e.mode != snapshot[path].mode) {
                    out.extend_from_slice(format!(" M {path}\n").as_bytes());
                }
            }
            for path in files {
                if !snapshot.contains_key(&path) && !ignored(&root, &path) { out.extend_from_slice(format!("?? {path}\n").as_bytes()); }
            }
        }
        _ => return Err(format!("unhandled Git fixture command: {a:?}")),
    }
    Ok(out)
}

trait CrLf { fn replace_crlf(&self) -> Vec<u8>; }
impl CrLf for Vec<u8> {
    fn replace_crlf(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (i, b) in self.iter().enumerate() {
            if *b != b'\r' || self.get(i + 1) != Some(&b'\n') { out.push(*b); }
        }
        out
    }
}

pub fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut input = Vec::new();
    if args.iter().any(|s| s.starts_with("--batch")) { io::stdin().read_to_end(&mut input).unwrap(); }
    match run(std::env::current_dir().unwrap(), &args, &input) {
        Ok(bytes) => { use std::io::Write; io::stdout().write_all(&bytes).unwrap(); }
        Err(error) => { eprintln!("{error}"); std::process::exit(93); }
    }
}
