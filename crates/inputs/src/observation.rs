//! Factual acquisition for explicit host roots; never registration or work selection.
use chrono_harness::{
    CommandSpec, ProcessEvidence, file_identity, no_symlink_parents, observation::process_success,
    resolve_program, run_process_observed_retained,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    #[serde(default)]
    platforms: Option<Vec<String>>,
    environment: Environment,
    operations: Vec<Operation>,
    #[serde(default)]
    registered_operations: Option<Registered>,
    #[serde(default)]
    stop_on_failure: bool,
    files: Vec<File>,
    directories: Vec<Directory>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registered {
    projects: String,
    operations: Vec<String>,
    tools: BTreeMap<String, Program>,
    #[serde(default)]
    environment: BTreeMap<String, Output>,
    timeout_seconds: u64,
    output_limit_bytes: usize,
    #[serde(default)]
    operation_bounds: BTreeMap<String, Bounds>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bounds {
    timeout_seconds: u64,
    output_limit_bytes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Environment {
    inherit: Vec<String>,
    values: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    id: String,
    program: Program,
    #[serde(default)]
    environment: BTreeMap<String, Output>,
    argv: Vec<String>,
    timeout_seconds: u64,
    output_limit_bytes: usize,
}
#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Program {
    Literal(String),
    Observed(Output),
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Location {
    Path(Literal),
    Stdout(Output),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Literal {
    path: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    stdout: String,
    #[serde(default)]
    suffix: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    id: String,
    location: Location,
    #[serde(default)]
    resolution: FileResolution,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FileResolution {
    #[default]
    Strict,
    CanonicalTarget,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Directory {
    id: String,
    location: Location,
    max_entries: usize,
    #[serde(default)]
    manifest: Option<DirectoryOutput>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryOutput {
    path: String,
    root: String,
}

fn path(
    root: &Path,
    location: &Location,
    processes: &BTreeMap<String, Value>,
) -> Result<PathBuf, String> {
    let path = match location {
        Location::Path(p) => PathBuf::from(&p.path),
        Location::Stdout(o) => {
            let process = processes.get(&o.stdout).ok_or("missing location process")?;
            if process["exit_code"] != 0 || !process["failure"].is_null() {
                return Err("unsuccessful location process".into());
            }
            let raw: Vec<u8> = serde_json::from_value(process["stdout_bytes"].clone())
                .map_err(|e| e.to_string())?;
            let text = std::str::from_utf8(&raw)
                .map_err(|e| e.to_string())?
                .trim_end_matches(['\r', '\n']);
            if text.is_empty() || text.contains(['\r', '\n', '\0']) {
                return Err("location stdout must be one nonempty path".into());
            }
            let path = PathBuf::from(text);
            if o.suffix.is_empty() {
                path
            } else {
                path.join(&o.suffix)
            }
        }
    };
    Ok(root.join(path))
}
// Canonicalization is an explicit acquisition choice, never the shared input
// validator's default. Retain the original spelling and every traversed alias.
fn canonical_target(location: &Path) -> Result<(PathBuf, Vec<Value>), String> {
    let target = fs::canonicalize(location).map_err(|e| e.to_string())?;
    let mut pending: VecDeque<_> = location
        .components()
        .map(|c| c.as_os_str().to_owned())
        .collect();
    let mut current = PathBuf::new();
    let mut links = Vec::new();
    while let Some(part) = pending.pop_front() {
        if part == "." {
            continue;
        }
        if part == ".." {
            current.pop();
            continue;
        }
        current.push(&part);
        if fs::symlink_metadata(&current)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            if links.len() == 64 {
                return Err("selected file alias resolution exceeds bound".into());
            }
            let link = fs::read_link(&current).map_err(|e| e.to_string())?;
            link.to_str().ok_or("selected file alias target UTF8")?;
            current.to_str().ok_or("selected file alias path UTF8")?;
            let metadata = fs::metadata(&current).map_err(|e| e.to_string())?;
            let kind = if metadata.is_file() {
                "file"
            } else if metadata.is_dir() {
                "directory"
            } else {
                return Err("selected file alias has unsupported target kind".into());
            };
            links.push(json!({"path":current,"target":link,"kind":kind}));
            current.pop();
            if link.is_absolute() {
                current.clear();
            }
            for component in link.components().rev() {
                pending.push_front(component.as_os_str().to_owned());
            }
        }
    }
    if current != target {
        return Err("selected file alias resolution changed during observation".into());
    }
    Ok((target, links))
}

fn inventory(
    root: &Path,
    max: usize,
    output: Option<&DirectoryOutput>,
    host: &Path,
    alias_failure: &mut Option<Value>,
) -> Result<(Value, Option<Value>), String> {
    let resolved = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if output.is_some() {
                return Err("required directory manifest root is absent".into());
            }
            return Ok((json!({"location":root,"absent":true}), None));
        }
        Err(e) => return Err(format!("{}: {e}", root.display())),
    };
    if !resolved.is_dir() {
        return Err("inventory root is not a directory".into());
    }
    if let Some(output) = output {
        if fs::canonicalize(host.join(&output.root)).map_err(|e| e.to_string())? != resolved {
            return Err("directory manifest root differs from selected scope".into());
        }
    }
    let mut pending = vec![resolved.clone()];
    let (mut files, mut links, mut count) = (Vec::new(), Vec::new(), 0);
    let (mut topology, mut aliases) = (Vec::new(), Vec::new());
    while let Some(directory) = pending.pop() {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
            count += 1;
            if count > max {
                return Err("explicit inventory entry bound exceeded".into());
            }
            entries.push(entry.map_err(|e| e.to_string())?);
        }
        entries.sort_by_key(|e| e.file_name());
        if output.is_some() {
            let relative = directory
                .strip_prefix(&resolved)
                .map_err(|e| e.to_string())?;
            let names = entries
                .iter()
                .map(|e| {
                    e.file_name()
                        .into_string()
                        .map_err(|_| "inventory child UTF8".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let relative = if relative.as_os_str().is_empty() {
                "."
            } else {
                relative.to_str().ok_or("inventory directory UTF8")?
            };
            topology.push(json!({"path":relative,"entries":names}));
        }
        for entry in entries {
            let p = entry.path();
            let relative = p
                .strip_prefix(&resolved)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("inventory path UTF8")?
                .to_string();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                let target = fs::read_link(&p).map_err(|e| e.to_string())?;
                let target_text = target.to_str().ok_or("link target UTF8")?;
                links.push(json!({"path":relative,"target":target_text}));
                if output.is_some() {
                    let actual = fs::canonicalize(&p);
                    let result = (|| {
                        let actual = actual.as_ref().map_err(|e| e.to_string())?;
                        if target.is_absolute() || !actual.starts_with(&resolved) {
                            return Err(format!(
                                "directory manifest alias escapes scope: {relative}"
                            ));
                        }
                        if actual.is_file() {
                            Ok("file")
                        } else if actual.is_dir() {
                            Ok("directory")
                        } else {
                            Err("directory manifest alias has unsupported target kind".into())
                        }
                    })();
                    let kind = match result {
                        Ok(kind) => kind,
                        Err(error) => {
                            // Failure facts belong to the retained observation, never
                            // a partial guard manifest. Preserve the original rejection.
                            let (destination, resolution_error, target_kind) = match &actual {
                                Ok(actual) => {
                                    let kind = match fs::metadata(actual) {
                                        Ok(metadata) => {
                                            let kind = if metadata.is_file() {
                                                "file"
                                            } else if metadata.is_dir() {
                                                "directory"
                                            } else {
                                                "other"
                                            };
                                            json!({"observed":kind})
                                        }
                                        Err(e) => {
                                            json!({"unavailable":e.to_string(),"raw_os_error":e.raw_os_error()})
                                        }
                                    };
                                    (Some(actual), None, kind)
                                }
                                Err(e) => (
                                    None,
                                    Some(
                                        json!({"message":e.to_string(),"raw_os_error":e.raw_os_error()}),
                                    ),
                                    json!({"unavailable":"canonical resolution failed"}),
                                ),
                            };
                            *alias_failure = Some(
                                json!({"location":root,"root":resolved,"error":error,
                                "alias_failure":{"path":relative,"target":target_text,
                                "literal_absolute":target.is_absolute(),"canonical_destination":destination,
                                "within_root":actual.as_ref().ok().map(|p|p.starts_with(&resolved)),
                                "resolution_error":resolution_error,"target_kind":target_kind}}),
                            );
                            return Err(error);
                        }
                    };
                    aliases.push(json!({"path":relative,"target":target_text,"kind":kind}));
                }
            } else if kind.is_dir() {
                pending.push(p);
            } else if kind.is_file() {
                let (digest, _) = file_identity(&p)?;
                files.push(json!({"path":relative,"sha256":digest}));
            } else {
                return Err(format!("nonregular inventory entry {}", p.display()));
            }
        }
    }
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    links.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    topology.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    aliases.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let manifest = output.map(|o| {
        json!({"schema":"chrono-input-directory/v3","root":o.root,
        "files":files,"symlinks":aliases,"directories":topology})
    });
    Ok((
        json!({"schema":"chrono-input-directory-observation/v1","root":resolved,
        "files":files,"symlinks":links}),
        manifest,
    ))
}

pub fn observe(root: &Path, manifest: &str, output: &str) -> Result<Value, String> {
    let target = super::state(root, output)?;
    if target.exists() {
        return Err("E_OBSERVATION_EXISTS: preserve original; choose a new output".into());
    }
    let raw = fs::read(no_symlink_parents(root, manifest)?).map_err(|e| e.to_string())?;
    let mut original_sources = vec![(manifest.to_string(), raw.clone())];
    let mut m: Manifest = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    if m.schema != "chrono-input-observation/v1" {
        return Err("invalid observation schema".into());
    }
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    if let Some(platforms) = &m.platforms {
        if platforms.is_empty()
            || platforms.iter().collect::<BTreeSet<_>>().len() != platforms.len()
            || !platforms.contains(&platform)
        {
            return Err(format!(
                "observation has no unique registered native platform {platform}"
            ));
        }
    }
    if let Some(registered) = m.registered_operations.take() {
        if registered
            .operation_bounds
            .keys()
            .any(|id| !registered.operations.contains(id))
        {
            return Err("observation bounds require a selected registered operation".into());
        }
        let project_bytes =
            fs::read(no_symlink_parents(root, &registered.projects)?).map_err(|e| e.to_string())?;
        let projects: Value = serde_json::from_slice(&project_bytes).map_err(|e| e.to_string())?;
        original_sources.push((registered.projects.clone(), project_bytes));
        for id in registered.operations {
            let actions: Vec<&Value> = projects["projects"]
                .as_array()
                .ok_or("projects array required")?
                .iter()
                .chain(
                    projects
                        .get("scripts")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten(),
                )
                .flat_map(|p| {
                    p["actions"]
                        .as_object()
                        .into_iter()
                        .flat_map(|a| a.values())
                })
                .filter(|a| a["operation"] == id)
                .collect();
            if actions.len() != 1 {
                return Err(format!("observation operation {id} missing or ambiguous"));
            }
            let action = actions[0];
            let tool = action["tool"].as_str().ok_or("action tool required")?;
            let bounds = registered.operation_bounds.get(&id);
            m.operations.push(Operation {
                id,
                program: registered
                    .tools
                    .get(tool)
                    .ok_or_else(|| format!("unbound observation tool {tool}"))?
                    .clone(),
                environment: registered.environment.clone(),
                argv: serde_json::from_value(action["argv"].clone()).map_err(|e| e.to_string())?,
                timeout_seconds: bounds.map_or(registered.timeout_seconds, |b| b.timeout_seconds),
                output_limit_bytes: bounds
                    .map_or(registered.output_limit_bytes, |b| b.output_limit_bytes),
            });
        }
    }
    let mut ids = BTreeSet::new();
    for op in &m.operations {
        for (key, source) in &op.environment {
            if key.is_empty()
                || key.contains(['=', '\0'])
                || key == "CHRONO_PROCESS_FDS"
                || !ids.contains(&source.stdout)
                || !source.suffix.is_empty()
            {
                return Err(
                    "operation environment requires a preceding path selection and ordinary key"
                        .into(),
                );
            }
        }
        match &op.program {
            Program::Literal(p) if p.is_empty() => return Err("empty observation program".into()),
            Program::Observed(o) if !ids.contains(&o.stdout) || !o.suffix.is_empty() => {
                return Err(
                    "executable selection requires a preceding operation and no suffix".into(),
                );
            }
            _ => {}
        }
        if op.id.is_empty()
            || !ids.insert(op.id.clone())
            || op.timeout_seconds == 0
            || op.timeout_seconds > 900
            || op.output_limit_bytes == 0
            || op.output_limit_bytes > 64 * 1024 * 1024
        {
            return Err("invalid/duplicate observation operation or bounds".into());
        }
    }
    let mut inputs = BTreeSet::new();
    for (id, location) in m
        .files
        .iter()
        .map(|f| (&f.id, &f.location))
        .chain(m.directories.iter().map(|d| (&d.id, &d.location)))
    {
        if id.is_empty() || !inputs.insert(id) {
            return Err("duplicate/empty observation input".into());
        }
        match location {
            Location::Path(p) if p.path.is_empty() => return Err("empty observation path".into()),
            Location::Stdout(o)
                if !ids.contains(&o.stdout)
                    || Path::new(&o.suffix).is_absolute()
                    || Path::new(&o.suffix)
                        .components()
                        .any(|c| !matches!(c, std::path::Component::Normal(_))) =>
            {
                return Err("unregistered location operation or invalid suffix".into());
            }
            _ => {}
        }
    }
    if m.directories.iter().any(|d| d.max_entries == 0) {
        return Err("inventory requires explicit positive entry bound".into());
    }
    let mut outputs = BTreeSet::new();
    for directory in &m.directories {
        if let Some(manifest) = &directory.manifest {
            let path = super::state(root, &manifest.path)?;
            if manifest.root.is_empty()
                || Path::new(&manifest.root).components().any(|c| {
                    !matches!(
                        c,
                        Component::RootDir | Component::Prefix(_) | Component::Normal(_)
                    )
                })
                || manifest.path == output
                || manifest.path == format!("{output}.processes")
                || manifest.path.starts_with(&format!("{output}.processes/"))
                || !outputs.insert(manifest.path.clone())
            {
                return Err("invalid/duplicate directory manifest output or root".into());
            }
            if path.exists() {
                return Err("E_OBSERVATION_EXISTS: preserve original directory manifest".into());
            }
        }
    }
    let mut environment = BTreeMap::new();
    for key in &m.environment.inherit {
        if key == "CHRONO_PROCESS_FDS" || key.is_empty() || environment.contains_key(key) {
            return Err("invalid inherited observation environment".into());
        }
        match std::env::var(key) {
            Ok(v) => {
                environment.insert(key.clone(), v);
            }
            Err(std::env::VarError::NotPresent) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    if m.environment.values.contains_key("CHRONO_PROCESS_FDS") {
        return Err("reserved observation environment".into());
    }
    environment.extend(m.environment.values);
    let mut retained_prefix = format!("{output}.processes");
    let mut directory = super::state(root, &retained_prefix)?;
    fs::create_dir_all(directory.parent().ok_or("process directory parent")?)
        .map_err(|e| e.to_string())?;
    match fs::create_dir(&directory) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // An interrupted producer can leave streams without publishing its
            // immutable report. Preserve those bytes and atomically reserve a
            // fresh attempt; published reports were rejected before this point.
            directory = tempfile::Builder::new()
                .prefix("attempt-")
                .tempdir_in(&directory)
                .map_err(|e| e.to_string())?
                .keep();
            retained_prefix = format!(
                "{retained_prefix}/{}",
                directory
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or("process directory UTF8")?
            );
        }
        Err(e) => return Err(e.to_string()),
    }
    let mut sources = Vec::new();
    for (i, (path, bytes)) in original_sources.into_iter().enumerate() {
        let retained = format!("{retained_prefix}/source-{i}.json");
        fs::write(super::state(root, &retained)?, &bytes).map_err(|e| e.to_string())?;
        sources
            .push(json!({"path":path,"retained":retained,"sha256":chrono_harness::sha256(&bytes)}));
    }
    let mut processes = BTreeMap::new();
    let mut files = BTreeMap::new();
    let mut directories = BTreeMap::new();
    let mut errors = Vec::new();
    for (i, op) in m.operations.iter().enumerate() {
        if m.stop_on_failure && !errors.is_empty() {
            break;
        }
        let result = (|| {
            let mut environment = environment.clone();
            for (key, source) in &op.environment {
                environment.insert(
                    key.clone(),
                    path(root, &Location::Stdout(source.clone()), &processes)?
                        .to_str()
                        .ok_or("selected environment path UTF8")?
                        .to_owned(),
                );
            }
            let selected = match &op.program {
                Program::Literal(p) => p.clone(),
                Program::Observed(o) => path(root, &Location::Stdout(o.clone()), &processes)?
                    .to_str()
                    .ok_or("selected program UTF8")?
                    .to_owned(),
            };
            let executable = resolve_program(
                root,
                &selected,
                Some(environment.get("PATH").map(String::as_str).unwrap_or("")),
            )?;
            let digest = file_identity(&executable)?.0;
            run_process_observed_retained(
                root,
                &CommandSpec {
                    program: executable.to_str().ok_or("program UTF8")?.into(),
                    args: op.argv.clone(),
                    env: environment.clone(),
                    timeout_seconds: op.timeout_seconds,
                    output_limit_bytes: op.output_limit_bytes,
                },
                &[],
                &digest,
                &ProcessEvidence {
                    stdout: &directory.join(format!("{i}.stdout")),
                    stderr: &directory.join(format!("{i}.stderr")),
                    launch: &directory.join(format!("{i}.launch.json")),
                },
            )
        })();
        match result {
            Ok(process) => {
                if let Err(e) = process_success(&process) {
                    errors.push(format!("{}: {e}", op.id));
                }
                processes.insert(
                    op.id.clone(),
                    serde_json::to_value(process).map_err(|e| e.to_string())?,
                );
            }
            Err(e) => errors.push(format!("{}: {e}", op.id)),
        }
    }
    for f in m.files {
        if m.stop_on_failure && !errors.is_empty() {
            break;
        }
        let result = (|| {
            let location = path(root, &f.location, &processes)?;
            let (selected, resolution) = match f.resolution {
                FileResolution::Strict => (location.clone(), None),
                FileResolution::CanonicalTarget => {
                    let (target, aliases) = canonical_target(&location)?;
                    let evidence = json!({"semantics":"canonical-target","location":target,"symlinks":aliases});
                    (target, Some(evidence))
                }
            };
            let value = match chrono_harness::input_file::observe_file(&selected)? {
                Some((digest, length)) => json!({"sha256":digest,"length":length}),
                None if resolution.is_some() => {
                    return Err("selected canonical file disappeared during observation".into());
                }
                None => json!({"absent":true}),
            };
            let mut result =
                json!({"location":location,"observation":value,"absent":value["absent"]==true});
            if let Some(resolution) = resolution {
                result["resolution"] = resolution;
            }
            Ok::<_, String>(result)
        })();
        match result {
            Ok(value) => {
                files.insert(f.id, value);
            }
            Err(e) => errors.push(format!("{}: {e}", f.id)),
        }
    }
    for d in m.directories {
        if m.stop_on_failure && !errors.is_empty() {
            break;
        }
        let mut alias_failure = None;
        let result = (|| {
            let location = path(root, &d.location, &processes)?;
            let (mut observation, manifest) = inventory(
                &location,
                d.max_entries,
                d.manifest.as_ref(),
                root,
                &mut alias_failure,
            )?;
            if let Some((output, manifest)) = d.manifest.as_ref().zip(manifest) {
                super::publish(root, &output.path, &manifest)?;
                observation["manifest"] =
                    json!(chrono_harness::prepared::original(root, &output.path)?);
            }
            Ok::<_, String>(observation)
        })();
        match result {
            Ok(value) => {
                directories.insert(d.id, value);
            }
            Err(e) => {
                if let Some(failure) = alias_failure {
                    directories.insert(d.id.clone(), failure);
                }
                errors.push(format!("{}: {e}", d.id));
            }
        }
    }
    // Locations use the live process bytes above. Publication references the
    // already retained originals instead of serializing those streams again.
    for (i, op) in m.operations.iter().enumerate() {
        if let Some(process) = processes.get_mut(&op.id) {
            let retained = (|| {
                let stdout = chrono_harness::prepared::original(
                    root,
                    &format!("{retained_prefix}/{i}.stdout"),
                )?;
                let stderr = chrono_harness::prepared::original(
                    root,
                    &format!("{retained_prefix}/{i}.stderr"),
                )?;
                let launch = chrono_harness::prepared::original(
                    root,
                    &format!("{retained_prefix}/{i}.launch.json"),
                )?;
                if process["stdout_sha256"] != stdout.sha256
                    || process["stderr_sha256"] != stderr.sha256
                {
                    return Err("observation original stream drift".to_string());
                }
                Ok((stdout, stderr, launch))
            })();
            match retained {
                Ok((stdout, stderr, launch)) => {
                    let fields = process.as_object_mut().unwrap();
                    for field in ["stdout", "stderr", "stdout_bytes", "stderr_bytes"] {
                        fields.remove(field);
                    }
                    fields.insert("stdout_original".into(), json!(stdout));
                    fields.insert("stderr_original".into(), json!(stderr));
                    fields.insert("launch_original".into(), json!(launch));
                }
                Err(e) => errors.push(format!("{}: {e}; inline receipt preserved", op.id)),
            }
        }
    }
    let result = json!({"schema":"chrono-input-observation-result/v2","manifest":{"path":manifest,"sha256":chrono_harness::sha256(&raw)},"platform":platform,"status":if errors.is_empty(){"observed"}else{"failed"},"governance":"not-evaluated","sources":sources,"operations":processes,"files":files,"directories":directories,"errors":errors});
    super::publish(root, output, &result)?;
    if !errors.is_empty() {
        return Err(format!(
            "E_OBSERVATION: original failed acquisition retained at {output}"
        ));
    }
    Ok(result)
}
