//! Distribution knows explicit release members and destinations, never host projects.
use chrono_harness::{file_identity, json, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json as value};
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

type Result<T> = std::result::Result<T, String>;
const LAUNCHER: &str = include_str!("../../../assets/distribution/bootstrap.py");
const LOCK_PATH: &str = ".chrono-harness/distribution.json";
const BOOTSTRAP_PATH: &str = ".chrono-harness/install.py";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub file: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub schema: String,
    pub version: String,
    pub source_commit: String,
    pub source_tree: String,
    pub platforms: BTreeMap<String, BTreeMap<String, Asset>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Source {
    Https { base_url: String, curl: String },
    Directory { path: PathBuf },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub schema: String,
    pub version: String,
    pub source: Source,
    pub manifest: Asset,
    pub installers: BTreeMap<String, Asset>,
    pub install: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    version: String,
    assets: BTreeMap<String, String>,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_value(json(&fs::read(path).map_err(|e| e.to_string())?)?)
        .map_err(|e| e.to_string())
}
fn write<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|e| e.to_string())
}
fn name(s: &str) -> Result<()> {
    if s.is_empty()
        || s == "."
        || s == ".."
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(format!("invalid release name: {s:?}"));
    }
    Ok(())
}
fn hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn asset(a: &Asset) -> Result<()> {
    name(&a.file)?;
    if !hex(&a.sha256, 64) || a.size == 0 {
        return Err("invalid asset digest/size".into());
    }
    Ok(())
}
fn inspect(path: &Path, file: String) -> Result<Asset> {
    let (sha256, size) = file_identity(path)?;
    let a = Asset { file, sha256, size };
    asset(&a)?;
    Ok(a)
}
fn verify(path: &Path, a: &Asset) -> Result<()> {
    asset(a)?;
    if file_identity(path)? != (a.sha256.clone(), a.size) {
        return Err(format!("integrity mismatch: {}", a.file));
    }
    Ok(())
}
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
fn release(r: &Release) -> Result<()> {
    if r.schema != "chrono-release/v1"
        || !hex(&r.source_commit, 40)
        || !hex(&r.source_tree, 40)
        || r.platforms.is_empty()
    {
        return Err("invalid release identity".into());
    }
    name(&r.version)?;
    let mut files = BTreeSet::new();
    let mut members = None;
    for (p, tools) in &r.platforms {
        name(p)?;
        if !tools.contains_key("chrono-distribution") {
            return Err("release has no installer".into());
        }
        let names: BTreeSet<_> = tools.keys().collect();
        if members.as_ref().is_some_and(|old| old != &names) {
            return Err("platform release members differ".into());
        }
        members = Some(names);
        for (n, a) in tools {
            name(n)?;
            asset(a)?;
            if a.file != format!("{n}-{p}") || !files.insert(&a.file) {
                return Err("ambiguous release asset".into());
            }
        }
    }
    Ok(())
}
fn source(s: &Source) -> Result<()> {
    match s {
        Source::Directory { path } if path.is_absolute() && path.is_dir() => Ok(()),
        Source::Https { base_url, curl }
            if base_url.starts_with("https://")
                && base_url.len() > 8
                && !base_url.contains(['?', '#', '\n', '\r', '@'])
                && !curl.is_empty() =>
        {
            Ok(())
        }
        _ => {
            Err("source must be an absolute directory or HTTPS base URL with explicit curl".into())
        }
    }
}
fn lock(l: &Lock) -> Result<()> {
    if l.schema != "chrono-install/v1" || l.install.is_empty() || l.installers.is_empty() {
        return Err("invalid install declaration".into());
    }
    name(&l.version)?;
    asset(&l.manifest)?;
    source(&l.source)?;
    let mut destinations = BTreeSet::new();
    for (n, dst) in &l.install {
        name(n)?;
        relative_path(dst)?;
        let Some(leaf) = dst.strip_prefix(".chrono-harness/bin/") else {
            return Err("tools must install under .chrono-harness/bin/".into());
        };
        name(leaf)?;
        if !destinations.insert(dst) {
            return Err("duplicate install destination".into());
        }
    }
    for (p, a) in &l.installers {
        name(p)?;
        asset(a)?;
        if a.file != format!("chrono-distribution-{p}") {
            return Err("installer platform mismatch".into());
        }
    }
    Ok(())
}
fn fetch(s: &Source, a: &Asset, to: &Path) -> Result<()> {
    asset(a)?;
    source(s)?;
    match s {
        Source::Directory { path } => {
            let from = no_symlink_parents(path, &a.file)?;
            fs::copy(from, to).map_err(|e| e.to_string())?;
        }
        Source::Https { base_url, curl } => {
            let status = Command::new(curl)
                .args([
                    "--disable",
                    "--fail",
                    "--location",
                    "--silent",
                    "--show-error",
                    "--proto",
                    "=https",
                    "--proto-redir",
                    "=https",
                    "--connect-timeout",
                    "30",
                    "--max-time",
                    "300",
                    "--output",
                ])
                .arg(to)
                .arg("--url")
                .arg(format!("{}/{}", base_url.trim_end_matches('/'), a.file))
                .status()
                .map_err(|e| format!("download: {e}"))?;
            if !status.success() {
                return Err(format!("download failed: {} ({status})", a.file));
            }
        }
    }
    verify(to, a)
}
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().into())
        .map_err(|e| e.to_string())
}
/// Package only the plan's already built files from a clean, fixed source tree.
/// Build provenance/SDK closure is a separate obligation; SHA-256 is byte identity.
fn pack(root: &Path, plan: &Path, out: &Path) -> Result<Value> {
    let plan: Plan = read(plan)?;
    if plan.schema != "chrono-release-plan/v1" || plan.assets.is_empty() {
        return Err("invalid release plan".into());
    }
    name(&plan.version)?;
    if !git(root, &["status", "--porcelain"])?.is_empty() {
        return Err("release source tree is dirty".into());
    }
    let commit = git(root, &["rev-parse", "HEAD"])?;
    let tree = git(root, &["rev-parse", "HEAD^{tree}"])?;
    if out.exists() {
        return Err("release output already exists".into());
    }
    let parent = out.parent().ok_or("output has no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    let p = platform();
    let mut tools = BTreeMap::new();
    for (n, path) in plan.assets {
        name(&n)?;
        let from = no_symlink_parents(root, &path)?;
        if fs::metadata(&from)
            .map_err(|e| e.to_string())?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(format!("not executable: {path}"));
        }
        let file = format!("{n}-{p}");
        fs::copy(from, stage.path().join(&file)).map_err(|e| e.to_string())?;
        tools.insert(n, inspect(&stage.path().join(&file), file)?);
    }
    let r = Release {
        schema: "chrono-release/v1".into(),
        version: plan.version,
        source_commit: commit,
        source_tree: tree,
        platforms: BTreeMap::from([(p.clone(), tools)]),
    };
    release(&r)?;
    write(&stage.path().join("release.json"), &r)?;
    if !git(root, &["status", "--porcelain"])?.is_empty()
        || git(root, &["rev-parse", "HEAD"])? != r.source_commit
    {
        return Err("release source changed while packaging".into());
    }
    fs::rename(stage.path(), out).map_err(|e| e.to_string())?;
    Ok(
        value!({"platform":p,"version":r.version,"source_commit":r.source_commit,"source_tree":r.source_tree}),
    )
}
/// Combine explicitly supplied, matching platform packages; no directory discovery.
fn assemble(inputs: &[PathBuf], out: &Path) -> Result<Value> {
    if inputs.is_empty() || out.exists() {
        return Err("need inputs and absent output".into());
    }
    let parent = out.parent().ok_or("output has no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    let mut combined: Option<Release> = None;
    for dir in inputs {
        let r: Release = read(&dir.join("release.json"))?;
        release(&r)?;
        if let Some(old) = &combined {
            if (
                old.version.as_str(),
                old.source_commit.as_str(),
                old.source_tree.as_str(),
            ) != (
                r.version.as_str(),
                r.source_commit.as_str(),
                r.source_tree.as_str(),
            ) {
                return Err("platform source/version mismatch".into());
            }
        } else {
            let mut first = r.clone();
            first.platforms.clear();
            combined = Some(first);
        }
        let c = combined.as_mut().unwrap();
        for (p, members) in r.platforms {
            if c.platforms.contains_key(&p) {
                return Err(format!("duplicate platform: {p}"));
            }
            for a in members.values() {
                let from = no_symlink_parents(dir, &a.file)?;
                verify(&from, a)?;
                fs::copy(from, stage.path().join(&a.file)).map_err(|e| e.to_string())?;
            }
            c.platforms.insert(p, members);
        }
    }
    let r = combined.unwrap();
    release(&r)?;
    write(&stage.path().join("release.json"), &r)?;
    let identity = inspect(&stage.path().join("release.json"), "release.json".into())?;
    fs::rename(stage.path(), out).map_err(|e| e.to_string())?;
    Ok(
        value!({"manifest":identity,"version":r.version,"platforms":r.platforms.keys().collect::<Vec<_>>()}),
    )
}

struct Guard(PathBuf);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn guard(root: &Path) -> Result<Guard> {
    let path = no_symlink_parents(root, ".chrono-harness/.distribution-lock")?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("installation lock {}: {e}", path.display()))?;
    Ok(Guard(path))
}
/// Roll back ordinary rename failures, retaining backups if recovery itself fails.
/// This does not promise crash atomicity or cooperation from unrelated writers.
fn publish(root: &Path, stage: &Path, entries: &[(PathBuf, String)]) -> Result<()> {
    let mut applied: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    for (i, (from, relative)) in entries.iter().enumerate() {
        let action = (|| {
            let to = no_symlink_parents(root, relative)?;
            if to.exists() && !to.is_file() {
                return Err(format!("destination is not a file: {relative}"));
            }
            fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
            let backup = if to.exists() {
                let saved = stage.join(format!("backup-{i}"));
                fs::rename(&to, &saved).map_err(|e| e.to_string())?;
                Some(saved)
            } else {
                None
            };
            applied.push((to.clone(), backup));
            fs::rename(from, to).map_err(|e| e.to_string())
        })();
        if let Err(error) = action {
            let mut recovery = Vec::new();
            for (to, backup) in applied.into_iter().rev() {
                if to.exists()
                    && let Err(e) = fs::remove_file(&to)
                {
                    recovery.push(format!("{}: {e}", to.display()));
                    continue;
                }
                if let Some(saved) = backup
                    && let Err(e) = fs::rename(&saved, &to)
                {
                    recovery.push(format!("{} -> {}: {e}", saved.display(), to.display()));
                }
            }
            return Err(format!("{error}; rollback errors: {recovery:?}"));
        }
    }
    Ok(())
}
pub fn adopt(
    root: &Path,
    manifest: &Path,
    origin: Source,
    members: &[String],
    replace: bool,
) -> Result<Value> {
    source(&origin)?;
    let r: Release = read(manifest)?;
    release(&r)?;
    let install: BTreeMap<_, _> = members
        .iter()
        .map(|n| (n.clone(), format!(".chrono-harness/bin/{n}")))
        .collect();
    if install.len() != members.len() {
        return Err("duplicate selected member".into());
    }
    for tools in r.platforms.values() {
        for n in members {
            if !tools.contains_key(n) {
                return Err(format!("unknown release member: {n}"));
            }
        }
    }
    let l = Lock {
        schema: "chrono-install/v1".into(),
        version: r.version,
        source: origin,
        manifest: inspect(manifest, "release.json".into())?,
        installers: r
            .platforms
            .iter()
            .map(|(p, t)| (p.clone(), t["chrono-distribution"].clone()))
            .collect(),
        install,
    };
    lock(&l)?;
    let _guard = guard(root)?;
    let stage = tempfile::tempdir_in(root.join(".chrono-harness")).map_err(|e| e.to_string())?;
    write(&stage.path().join("lock"), &l)?;
    fs::write(stage.path().join("launcher"), LAUNCHER).map_err(|e| e.to_string())?;
    let entries: Vec<(PathBuf, String)> = vec![
        (stage.path().join("lock"), LOCK_PATH.into()),
        (stage.path().join("launcher"), BOOTSTRAP_PATH.into()),
    ];
    for (from, dst) in &entries {
        let path = no_symlink_parents(root, dst)?;
        if path.exists()
            && !replace
            && fs::read(path).map_err(|e| e.to_string())?
                != fs::read(from).map_err(|e| e.to_string())?
        {
            return Err(format!(
                "existing adoption differs: {dst}; use --replace for an explicit update"
            ));
        }
    }
    let result = publish(root, stage.path(), &entries);
    if result.is_err() {
        let saved = stage.keep();
        return Err(format!(
            "{}; recovery directory: {}",
            result.unwrap_err(),
            saved.display()
        ));
    }
    Ok(value!({"version":l.version,"lock":LOCK_PATH,"bootstrap":BOOTSTRAP_PATH}))
}
pub fn install(root: &Path) -> Result<Value> {
    let l: Lock = read(&no_symlink_parents(root, LOCK_PATH)?)?;
    lock(&l)?;
    let p = platform();
    let bootstrap = l
        .installers
        .get(&p)
        .ok_or_else(|| format!("unregistered platform: {p}"))?;
    let _guard = guard(root)?;
    let stage = tempfile::tempdir_in(root.join(".chrono-harness")).map_err(|e| e.to_string())?;
    let manifest = stage.path().join("release.json");
    fetch(&l.source, &l.manifest, &manifest)?;
    let r: Release = read(&manifest)?;
    release(&r)?;
    if r.version != l.version {
        return Err("locked release version mismatch".into());
    }
    let tools = r
        .platforms
        .get(&p)
        .ok_or("release lacks current platform")?;
    if tools.get("chrono-distribution") != Some(bootstrap) {
        return Err("locked installer identity mismatch".into());
    }
    let mut entries = Vec::new();
    let mut installed = Vec::new();
    for (n, dst) in &l.install {
        let a = tools
            .get(n)
            .ok_or_else(|| format!("unregistered release member: {n}"))?;
        let to = no_symlink_parents(root, dst)?;
        if to.exists() && !to.is_file() {
            return Err(format!("destination is not a file: {dst}"));
        }
        // Existing bytes are a cache only after verification; executable mode is part of usable installation.
        let reusable = to.is_file()
            && verify(&to, a).is_ok()
            && fs::metadata(&to)
                .map_err(|e| e.to_string())?
                .permissions()
                .mode()
                & 0o111
                != 0;
        if !reusable {
            let from = stage.path().join(&a.file);
            fetch(&l.source, a, &from)?;
            fs::set_permissions(&from, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            entries.push((from, dst.clone()));
        }
        installed.push(value!({"name":n,"path":dst,"sha256":a.sha256,"size":a.size}));
    }
    let report = value!({"schema":"chrono-install-result/v1","version":r.version,"source_commit":r.source_commit,"source_tree":r.source_tree,"platform":p,"manifest":l.manifest,"installed":installed});
    let receipt = stage.path().join("receipt");
    write(&receipt, &report)?;
    entries.push((receipt, ".chrono-harness/state/distribution.json".into()));
    let result = publish(root, stage.path(), &entries);
    if result.is_err() {
        let saved = stage.keep();
        return Err(format!(
            "{}; recovery directory: {}",
            result.unwrap_err(),
            saved.display()
        ));
    }
    Ok(report)
}

pub fn run(args: &[String]) -> Result<Value> {
    if args == ["--version"] {
        return Ok(value!(concat!(
            "chrono-distribution ",
            env!("CARGO_PKG_VERSION")
        )));
    }
    let command = args.first().ok_or("expected pack/assemble/adopt/install")?;
    let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let key = &args[i];
        if key == "--replace" {
            values.entry(key.clone()).or_default().push("true".into());
            i += 1;
        } else {
            let val = args
                .get(i + 1)
                .ok_or_else(|| format!("missing value for {key}"))?;
            values.entry(key.clone()).or_default().push(val.clone());
            i += 2;
        }
    }
    let allowed: &[&str] = match command.as_str() {
        "pack" => &["--root", "--plan", "--output"],
        "assemble" => &["--input", "--output"],
        "adopt" => &[
            "--host-root",
            "--manifest",
            "--source-dir",
            "--base-url",
            "--curl",
            "--tool",
            "--replace",
        ],
        "install" => &["--host-root"],
        _ => return Err("expected pack/assemble/adopt/install".into()),
    };
    for (k, v) in &values {
        if !allowed.contains(&k.as_str()) || (v.len() != 1 && k != "--input" && k != "--tool") {
            return Err(format!("unknown/duplicate option: {k}"));
        }
    }
    let get = |key: &str| -> Result<&str> {
        values
            .get(key)
            .and_then(|v| v.first())
            .map(|s| s.as_str())
            .ok_or_else(|| format!("missing {key}"))
    };
    match command.as_str() {
        "pack" => pack(
            Path::new(get("--root")?),
            Path::new(get("--plan")?),
            Path::new(get("--output")?),
        ),
        "assemble" => assemble(
            &values
                .get("--input")
                .ok_or("missing --input")?
                .iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>(),
            Path::new(get("--output")?),
        ),
        "install" => install(Path::new(get("--host-root")?)),
        "adopt" => {
            let s = match (
                values.contains_key("--source-dir"),
                values.contains_key("--base-url"),
            ) {
                (true, false) if !values.contains_key("--curl") => Source::Directory {
                    path: PathBuf::from(get("--source-dir")?)
                        .canonicalize()
                        .map_err(|e| e.to_string())?,
                },
                (false, true) => Source::Https {
                    base_url: get("--base-url")?.into(),
                    curl: get("--curl")?.into(),
                },
                _ => return Err("choose one source: --source-dir or --base-url with --curl".into()),
            };
            adopt(
                Path::new(get("--host-root")?),
                Path::new(get("--manifest")?),
                s,
                values.get("--tool").ok_or("missing --tool")?,
                values.contains_key("--replace"),
            )
        }
        _ => unreachable!(),
    }
}
