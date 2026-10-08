//! Factual acquisition for explicit host roots; never registration or work selection.
use chrono_harness::{
    CommandSpec, ProcessEvidence, file_identity, no_symlink_parents, observation::process_success,
    resolve_program, run_process_observed_retained,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    environment: Environment,
    operations: Vec<Operation>,
    #[serde(default)]
    registered_operations: Option<Registered>,
    files: Vec<File>,
    directories: Vec<Directory>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registered {
    projects: String,
    operations: Vec<String>,
    tools: BTreeMap<String, String>,
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
    program: String,
    argv: Vec<String>,
    timeout_seconds: u64,
    output_limit_bytes: usize,
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
#[derive(Deserialize)]
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
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Directory {
    id: String,
    location: Location,
    max_entries: usize,
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
            PathBuf::from(text).join(&o.suffix)
        }
    };
    Ok(root.join(path))
}
fn inventory(root: &Path, max: usize) -> Result<Value, String> {
    let resolved = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"location":root,"absent":true}));
        }
        Err(e) => return Err(format!("{}: {e}", root.display())),
    };
    if !resolved.is_dir() {
        return Err("inventory root is not a directory".into());
    }
    let mut pending = vec![resolved.clone()];
    let (mut files, mut links, mut count) = (Vec::new(), Vec::new(), 0);
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            count += 1;
            if count > max {
                return Err("explicit inventory entry bound exceeded".into());
            }
            let p = entry.path();
            let relative = p
                .strip_prefix(&resolved)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("inventory path UTF8")?
                .to_string();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                links.push(json!({"path":relative,"target":fs::read_link(&p).map_err(|e| e.to_string())?.to_str().ok_or("link target UTF8")?}));
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
    Ok(json!({"schema":"chrono-input-directory/v2","root":resolved,"files":files,"symlinks":links}))
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
    if let Some(registered) = m.registered_operations.take() {
        let project_bytes =
            fs::read(no_symlink_parents(root, &registered.projects)?).map_err(|e| e.to_string())?;
        let projects: Value = serde_json::from_slice(&project_bytes).map_err(|e| e.to_string())?;
        original_sources.push((registered.projects.clone(), project_bytes));
        for id in registered.operations {
            let actions: Vec<&Value> = projects["projects"]
                .as_array()
                .ok_or("projects array required")?
                .iter()
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
            m.operations.push(Operation {
                id,
                program: registered
                    .tools
                    .get(tool)
                    .ok_or_else(|| format!("unbound observation tool {tool}"))?
                    .clone(),
                argv: serde_json::from_value(action["argv"].clone()).map_err(|e| e.to_string())?,
                timeout_seconds: registered.timeout_seconds,
                output_limit_bytes: registered.output_limit_bytes,
            });
        }
    }
    let mut ids = BTreeSet::new();
    for op in &m.operations {
        if op.id.is_empty()
            || !ids.insert(op.id.clone())
            || op.program.is_empty()
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
    let directory = super::state(root, &format!("{output}.processes"))?;
    if directory.exists() {
        return Err("E_OBSERVATION_EXISTS: preserve original process directory".into());
    }
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut sources = Vec::new();
    for (i, (path, bytes)) in original_sources.into_iter().enumerate() {
        let retained = format!("{output}.processes/source-{i}.json");
        fs::write(super::state(root, &retained)?, &bytes).map_err(|e| e.to_string())?;
        sources
            .push(json!({"path":path,"retained":retained,"sha256":chrono_harness::sha256(&bytes)}));
    }
    let mut processes = BTreeMap::new();
    let mut files = BTreeMap::new();
    let mut directories = BTreeMap::new();
    let mut errors = Vec::new();
    for (i, op) in m.operations.iter().enumerate() {
        let result = (|| {
            let executable = resolve_program(
                root,
                &op.program,
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
        let result = (|| {
            let location = path(root, &f.location, &processes)?;
            let value = match chrono_harness::input_file::observe_file(&location)? {
                Some((digest, length)) => json!({"sha256":digest,"length":length}),
                None => json!({"absent":true}),
            };
            Ok::<_, String>(
                json!({"location":location,"observation":value,"absent":value["absent"]==true}),
            )
        })();
        match result {
            Ok(value) => {
                files.insert(f.id, value);
            }
            Err(e) => errors.push(format!("{}: {e}", f.id)),
        }
    }
    for d in m.directories {
        let result = (|| inventory(&path(root, &d.location, &processes)?, d.max_entries))();
        match result {
            Ok(value) => {
                directories.insert(d.id, value);
            }
            Err(e) => errors.push(format!("{}: {e}", d.id)),
        }
    }
    let result = json!({"schema":"chrono-input-observation-result/v1","manifest":{"path":manifest,"sha256":chrono_harness::sha256(&raw)},"platform":format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH),"status":if errors.is_empty(){"observed"}else{"failed"},"governance":"not-evaluated","sources":sources,"operations":processes,"files":files,"directories":directories,"errors":errors});
    super::publish(root, output, &result)?;
    if !errors.is_empty() {
        return Err(format!(
            "E_OBSERVATION: original failed acquisition retained at {output}"
        ));
    }
    Ok(result)
}
