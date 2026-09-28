//! Optional Cargo-specific consistency checks. FILEMAP alone declares selection.
mod configuration;
pub mod guard;
mod inputs;
use chrono_harness::{
    facts, json, no_symlink_parents, relative_path,
    wire::{Finding, Request, Response, Status},
};
use chrono_judge_registration::Registrations;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

type Result<T = ()> = std::result::Result<T, String>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub project: String,
    pub manifest: String,
    pub lockfile: String,
    pub output: String,
    pub ancestor_manifests: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub projects: Vec<Project>,
}
impl Policy {
    pub fn validate(&self) -> Result {
        if self.schema != "chrono-cargo-projects/v1" {
            return Err("E_CARGO_POLICY: unsupported schema".into());
        }
        let mut ids = BTreeSet::new();
        for p in &self.projects {
            if p.project.is_empty() || !ids.insert(&p.project) {
                return Err("E_CARGO_POLICY: empty/duplicate project".into());
            }
            for path in [&p.manifest, &p.lockfile] {
                relative_path(path)?;
            }
            if !p.output.ends_with('/') {
                return Err("E_CARGO_POLICY: output must be a registered directory".into());
            }
            relative_path(p.output.trim_end_matches('/'))?;
            let mut seen = BTreeSet::new();
            for ancestor in &p.ancestor_manifests {
                relative_path(ancestor)?;
                if !seen.insert(ancestor) {
                    return Err("E_CARGO_POLICY: duplicate ancestor".into());
                }
            }
        }
        Ok(())
    }
}
fn normalized(path: &Path) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err("E_CARGO_POLICY: dependency escapes host".into());
                }
            }
            Component::Normal(s) => out.push(s),
            _ => return Err("E_CARGO_POLICY: dependency must be host-relative".into()),
        }
    }
    Ok(out)
}
fn table(root: &Path, path: &str) -> Result<toml::Value> {
    fs::read_to_string(no_symlink_parents(root, path)?)
        .map_err(|e| format!("E_MANIFEST: {path}: {e}"))?
        .parse()
        .map_err(|e| format!("E_MANIFEST: {path}: {e}"))
}
/// `selected` is an explicit project ID set, never computed from Cargo metadata.
pub fn check(
    root: &Path,
    r: &Registrations,
    policy: &Policy,
    selected: &BTreeSet<String>,
) -> Result<Vec<String>> {
    check_policy(root, r, policy, selected, false)
}
// External dependencies are admitted only inside the metadata guard, which
// validates their explicit retained inventory and actual resolution before use.
fn check_policy(
    root: &Path,
    r: &Registrations,
    policy: &Policy,
    selected: &BTreeSet<String>,
    metadata_guard: bool,
) -> Result<Vec<String>> {
    policy.validate()?;
    let projects = r.projects()["projects"].as_array().unwrap();
    let files = r.filemap()["files"].as_array().unwrap();
    let edge = |from: &str, to: &str| {
        r.filemap()["project_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from"] == from && e["kind"] == "compile" && e["to"] == to)
    };
    let mut checked = Vec::new();
    for p in &policy.projects {
        if !selected.contains(&p.project) {
            continue;
        }
        if !projects.iter().any(|row| row["id"] == p.project) {
            return Err(format!("E_CARGO_POLICY: missing project {}", p.project));
        }
        for path in [&p.manifest, &p.lockfile] {
            if files
                .iter()
                .filter(|f| f["path"] == *path && f["owner"] == p.project)
                .count()
                != 1
                || !no_symlink_parents(root, path)?.is_file()
            {
                return Err(format!("E_TEST_PAIR: missing owned Cargo input {path}"));
            }
        }
        if r.config()["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["path"] == p.output && a["owner"] == p.project)
            .count()
            != 1
        {
            return Err(format!(
                "E_TEST_PAIR: missing declared Cargo output {}",
                p.output
            ));
        }
        let manifest = table(root, &p.manifest)?;
        if manifest.get("workspace").is_some()
            || manifest
                .get("package")
                .and_then(|p| p.get("workspace"))
                .is_some()
        {
            return Err(format!(
                "E_TEST_PAIR: Cargo workspace aggregation: {}",
                p.manifest
            ));
        }
        // Validate the explicit ancestor inventory against registered inputs. This
        // is a consistency check; discovering an omitted input never registers it.
        let ancestors: BTreeSet<_> = Path::new(&p.manifest)
            .parent()
            .unwrap()
            .ancestors()
            .skip(1)
            .map(|a| a.join("Cargo.toml"))
            .filter_map(|a| a.to_str().map(String::from))
            .filter(|a| files.iter().any(|f| f["path"] == *a))
            .collect();
        let declared: BTreeSet<_> = p.ancestor_manifests.iter().cloned().collect();
        if ancestors != declared {
            return Err(format!(
                "E_INPUT_UNDECLARED: Cargo ancestor inventory for {}: registered={ancestors:?}, declared={declared:?}",
                p.project
            ));
        }
        for ancestor in &p.ancestor_manifests {
            if table(root, ancestor)?.get("workspace").is_some() {
                return Err(format!(
                    "E_TEST_PAIR: registered ancestor workspace {ancestor} for {}",
                    p.manifest
                ));
            }
        }
        let mut tables = vec![&manifest];
        if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
            tables.extend(targets.values());
        }
        for table in tables {
            for kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(deps) = table.get(kind).and_then(toml::Value::as_table) {
                    for (name, dep) in deps {
                        if dep.get("workspace").is_some() {
                            return Err("E_TEST_PAIR: workspace dependency".into());
                        }
                        let Some(relative) = dep.get("path").and_then(toml::Value::as_str) else {
                            if metadata_guard {
                                continue;
                            }
                            return Err(format!(
                                "E_INPUT_UNDECLARED: retained registry/git Cargo input closure unresolved for {name}"
                            ));
                        };
                        let target = normalized(
                            &Path::new(&p.manifest)
                                .parent()
                                .unwrap()
                                .join(relative)
                                .join("Cargo.toml"),
                        )?;
                        let targets: Vec<_> = policy
                            .projects
                            .iter()
                            .filter(|p| Some(p.manifest.as_str()) == target.to_str())
                            .collect();
                        if targets.len() != 1 {
                            return Err(format!("E_INPUT_UNDECLARED: Cargo dependency {name}"));
                        }
                        let target = targets[0];
                        if !projects.iter().any(|row| row["id"] == target.project) {
                            return Err(format!(
                                "E_CARGO_POLICY: dependency project {} missing",
                                target.project
                            ));
                        }
                        if !edge(
                            &format!("project:{}", target.project),
                            &format!("project:{}", p.project),
                        ) {
                            return Err(format!(
                                "E_DANGLING_EDGE: undeclared manifest dependency {name}"
                            ));
                        }
                    }
                }
            }
        }
        checked.push(p.project.clone());
    }
    Ok(checked)
}
fn read_policy(root: &Path, path: &str) -> Result<Policy> {
    if !path.starts_with(".chrono-harness/") {
        return Err("E_CARGO_POLICY: policy must live below .chrono-harness/".into());
    }
    let data = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    serde_json::from_value(json(&data)?).map_err(|e| format!("E_CARGO_POLICY: {e}"))
}
pub fn judge(req: &Request, path: &str) -> Response {
    let mut response = req.response(Status::Pass);
    let reader = facts::Reader::for_request(req);
    let result = (|| -> Result<Value> {
        req.validate()?;
        let reader = reader.as_ref().map_err(Clone::clone)?;
        let (old, r, _) = chrono_judge_registration::views_with_reader(req, reader)?;
        chrono_judge_registration::inputs::validate(req, &old, &r)?;
        if !r.filemap()["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == path)
        {
            return Err("E_CARGO_POLICY: policy file unregistered".into());
        }
        let policy = read_policy(&req.candidate.root, path)?;
        if fs::read(req.candidate.root.join(path)).map_err(|e| e.to_string())?
            != reader.blob(&req.candidate.root, &req.candidate.commit, path)?
        {
            return Err("E_SNAPSHOT_DIRTY: Cargo policy changed".into());
        }
        let impact: chrono_judge_filemap::Impact =
            serde_json::from_value(req.impact.clone()).map_err(|e| e.to_string())?;
        let selected: BTreeSet<_> = impact
            .closure
            .reached
            .keys()
            .filter_map(|id| {
                id.strip_prefix("project:")
                    .or_else(|| id.strip_prefix("test:"))
            })
            .map(String::from)
            .collect();
        let checked = check(&req.candidate.root, &r, &policy, &selected)?;
        Ok(serde_json::json!({"policy":path,"checked":checked,"input_closure_complete":false}))
    })();
    if let Ok(reader) = &reader {
        reader.record(&mut response);
    }
    match result {
        Ok(value) => {
            response.outputs.insert("cargo_projects".into(), value);
        }
        Err(message) => {
            response.status = Status::Fail;
            response.findings.push(Finding {
                code: message.split(':').next().unwrap_or("E_CARGO").into(),
                level: "error".into(),
                message,
                delta_refs: vec![format!("file:{path}")],
                causes: vec![],
            });
        }
    }
    response
}
/// A scoped host can register this exact command as an ordinary prerequisite.
pub fn standalone(
    root: &Path,
    config: &str,
    policy: &str,
    selected: &BTreeSet<String>,
) -> Result<Value> {
    if selected.is_empty() {
        return Err("E_CARGO_POLICY: explicit project selection required".into());
    }
    let config_value =
        json(&fs::read(no_symlink_parents(root, config)?).map_err(|e| e.to_string())?)?;
    let values: BTreeMap<_, _> = facts::registry_paths(&config_value, config)?
        .into_iter()
        .map(|p| {
            Ok((
                p.clone(),
                json(&fs::read(no_symlink_parents(root, &p)?).map_err(|e| e.to_string())?)?,
            ))
        })
        .collect::<Result<_>>()?;
    let r = Registrations::load(&values, config)?;
    if !r.filemap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == policy)
    {
        return Err("E_CARGO_POLICY: policy file unregistered".into());
    }
    let p = read_policy(root, policy)?;
    for id in selected {
        if !p.projects.iter().any(|p| p.project == *id) {
            return Err(format!(
                "E_CARGO_POLICY: selected project has no Cargo declaration: {id}"
            ));
        }
    }
    Ok(serde_json::json!({"checked":check(root,&r,&p,selected)?,"input_closure_complete":false}))
}
