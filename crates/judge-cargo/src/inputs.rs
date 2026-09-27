//! Explicit retained Cargo package contracts and metadata comparison.
use crate::{Policy, Project};
use chrono_harness::{facts, file_identity, json, no_symlink_parents, wire};
use chrono_judge_filemap::{
    edges,
    graph::{self, EdgeKind, Seed},
};
use chrono_judge_registration::Registrations;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
type Result<T = ()> = std::result::Result<T, String>;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Command {
    pub tool: String,
    pub argv: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Compiler {
    pub tool: String,
    input: String,
}
pub(crate) struct ToolInput {
    pub path: PathBuf,
    pub sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Contract {
    schema: String,
    root: String,
    projects: Vec<Project>,
    pub metadata: Command,
    pub operations: BTreeMap<String, Command>,
    target: String,
    configuration_files: Vec<crate::configuration::Declaration>,
    configuration_ancestors: Option<crate::configuration::Ancestors>,
    cargo_input: Option<String>,
    pub compiler: Option<Compiler>,
    packages: Vec<Package>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    id: String,
    name: String,
    version: String,
    source: Option<String>,
    project: Option<String>,
    manifest_input: Option<String>,
    inputs: Vec<String>,
    checksum: Option<String>,
    features: Vec<String>,
    dependencies: Vec<Dependency>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Dependency {
    package: String,
    name: String,
    kinds: Vec<Kind>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Kind {
    kind: Option<String>,
    target: Option<String>,
}
pub(crate) struct Check {
    pub contract: Contract,
    pub configuration: crate::configuration::Checked,
    pub tool_inputs: BTreeMap<String, ToolInput>,
    manifests: BTreeMap<String, PathBuf>,
    sources: BTreeMap<String, BTreeSet<PathBuf>>,
    target_directory: PathBuf,
    inputs: BTreeMap<PathBuf, (PathBuf, String)>,
    absent_inputs: Vec<PathBuf>,
    strict_inputs: Vec<PathBuf>,
}
fn error(s: impl std::fmt::Display) -> String {
    format!("E_CARGO_INPUT: {s}")
}
fn strings(values: &[String]) -> Result<BTreeSet<String>> {
    let set: BTreeSet<_> = values.iter().cloned().collect();
    if set.len() != values.len() || values.iter().any(|s| s.is_empty()) {
        return Err(error("empty/duplicate identity"));
    }
    Ok(set)
}
fn table(path: &Path) -> Result<toml::Value> {
    fs::read_to_string(path)
        .map_err(error)?
        .parse()
        .map_err(error)
}
#[derive(Default, PartialEq, Eq)]
struct Options {
    configurations: Vec<String>,
    features: BTreeSet<String>,
    all: bool,
    no_default: bool,
}
fn command(argv: &[String], manifest: &str, target: &str, metadata: bool) -> Result<Options> {
    let first = argv
        .first()
        .map(String::as_str)
        .ok_or_else(|| error("empty Cargo argv"))?;
    if metadata && first != "metadata"
        || !metadata
            && !["build", "check", "test", "run", "bench", "doc", "clippy"].contains(&first)
    {
        return Err(error("unsupported Cargo consuming command"));
    }
    let (mut locked, mut offline, mut seen_manifest, mut seen_target, mut format) =
        (false, false, false, false, false);
    let mut out = Options::default();
    let mut i = 1;
    while i < argv.len() {
        let flag = argv[i].as_str();
        i += 1;
        match flag {
            "--locked" => locked = true,
            "--offline" => offline = true,
            "--frozen" => {
                locked = true;
                offline = true;
            }
            "--all-features" => out.all = true,
            "--no-default-features" => out.no_default = true,
            "--release" | "--lib" | "--bins" | "--tests" | "--all-targets" | "--examples"
            | "--benches"
                if !metadata => {}
            "--config" | "--features" | "--manifest-path" | "--filter-platform" | "--target"
            | "--format-version" | "--profile" => {
                let value = argv
                    .get(i)
                    .ok_or_else(|| error("Cargo option requires a value"))?;
                i += 1;
                match flag {
                    "--config" => {
                        out.configurations.push(value.clone());
                    }
                    "--features" => {
                        out.features.extend(
                            value
                                .split([',', ' '])
                                .filter(|s| !s.is_empty())
                                .map(String::from),
                        );
                    }
                    "--manifest-path" if value == manifest && !seen_manifest => {
                        seen_manifest = true
                    }
                    "--filter-platform" if metadata && value == target && !seen_target => {
                        seen_target = true
                    }
                    "--target" if !metadata && value == target && !seen_target => {
                        seen_target = true
                    }
                    "--format-version" if metadata && value == "1" && !format => format = true,
                    "--profile" if !metadata => {}
                    _ => return Err(error(format!("mismatched/duplicate Cargo option {flag}"))),
                }
            }
            _ => return Err(error(format!("unsupported Cargo option {flag}"))),
        }
    }
    if !(locked && offline && seen_manifest && seen_target && (!metadata || format)) {
        return Err(error(
            "Cargo requires locked offline mode, exact manifest/target and metadata v1",
        ));
    }
    Ok(out)
}
fn dependency_set(deps: &[Dependency]) -> Result<BTreeSet<Dependency>> {
    let mut out = BTreeSet::new();
    for d in deps {
        let mut d = d.clone();
        if d.name.is_empty() || d.package.is_empty() || d.kinds.is_empty() {
            return Err(error("incomplete dependency"));
        }
        let kinds: BTreeSet<_> = d.kinds.iter().cloned().collect();
        if kinds.len() != d.kinds.len()
            || kinds
                .iter()
                .any(|k| !matches!(k.kind.as_deref(), None | Some("dev" | "build")))
        {
            return Err(error("invalid dependency kinds"));
        }
        d.kinds = kinds.into_iter().collect();
        if !out.insert(d) {
            return Err(error("duplicate dependency"));
        }
    }
    Ok(out)
}

// Inventory checks compare the declared package against actual files; they never
// add an input, dependency, project or test to a registration.
fn inventory(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(error)? {
            let entry = entry.map_err(error)?;
            let kind = entry.file_type().map_err(error)?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.insert(entry.path());
            } else {
                return Err(error(format!(
                    "unsupported package entry {}",
                    entry.path().display()
                )));
            }
        }
    }
    Ok(files)
}
pub(crate) fn prepare(
    root: &Path,
    r: &Registrations,
    config: &str,
    policy: &str,
    operation: &str,
    environment: &BTreeMap<String, String>,
) -> Result<Check> {
    use std::cell::RefCell;
    if !policy.starts_with(".chrono-harness/") {
        return Err(error("policy must be under .chrono-harness/"));
    }
    let policy_path = no_symlink_parents(root, policy)?;
    let value = json(&fs::read(&policy_path).map_err(error)?)?;
    let contract = Contract::deserialize(&value).map_err(error)?;
    let version_valid = match contract.schema.as_str() {
        "chrono-cargo-inputs/v2" => {
            contract.configuration_ancestors.is_none()
                && value.get("cargo_input").is_none()
                && value.get("compiler").is_none()
        }
        "chrono-cargo-inputs/v3" => {
            contract.configuration_ancestors.is_some()
                && value.get("cargo_input").is_none()
                && value.get("compiler").is_none()
        }
        "chrono-cargo-inputs/v4" => {
            contract.configuration_ancestors.is_some()
                && contract.cargo_input.is_some()
                && contract.compiler.is_some()
        }
        _ => false,
    };
    if !version_valid
        || contract.target.is_empty()
        || contract.timeout_seconds == 0
        || contract.output_limit_bytes == 0
        || contract.output_limit_bytes > 64 * 1024 * 1024
    {
        return Err(error(
            "invalid chrono-cargo-inputs/v2, v3 or v4 contract schema/ancestor policy/tool bindings/target/process limits",
        ));
    }
    let root_package = contract
        .packages
        .iter()
        .find(|p| p.id == contract.root)
        .ok_or_else(|| error("missing root package"))?;
    let owner = root_package
        .project
        .as_deref()
        .ok_or_else(|| error("root must be a registered local project"))?;
    let root_project = contract
        .projects
        .iter()
        .find(|p| p.project == owner)
        .ok_or_else(|| error("root project missing"))?;
    let backing = contract
        .operations
        .get(operation)
        .ok_or_else(|| error("unregistered Cargo operation"))?;
    if contract.compiler.is_some()
        && !backing
            .argv
            .first()
            .is_some_and(|s| matches!(s.as_str(), "build" | "check" | "test" | "run" | "bench"))
    {
        return Err(error(
            "compiler binding does not cover additional doc/clippy tools",
        ));
    }
    if contract.metadata.tool.is_empty() || backing.tool != contract.metadata.tool {
        return Err(error(
            "metadata and operation must use the same registered tool",
        ));
    }
    let observing = command(
        &contract.metadata.argv,
        &root_project.manifest,
        &contract.target,
        true,
    )?;
    if command(
        &backing.argv,
        &root_project.manifest,
        &contract.target,
        false,
    )? != observing
    {
        return Err(error(
            "Cargo consumer target/features/ordered configuration differ from metadata",
        ));
    }
    let methods = chrono_judge_registration::execution::methods(r.projects())?;
    let methods = methods
        .get(operation)
        .ok_or_else(|| error("operation missing from projects registration"))?;
    if methods.len() != 1 {
        return Err(error("ambiguous registered operation"));
    }
    let method = &methods[0];
    let argv = &method.argv;
    if method.owner != format!("project:{owner}")
        || argv.len() != 9
        || argv[0] != "run"
        || argv[1] != "--host-root"
        || argv[3] != "--config"
        || argv[4] != config
        || argv[5] != "--policy"
        || argv[6] != policy
        || argv[7] != "--operation"
        || argv[8] != operation
        || fs::canonicalize(root.join(&argv[2])).map_err(error)? != root
    {
        return Err(error("operation must use the registered Cargo guard entry"));
    }
    let tool = r.config()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == method.tool)
        .ok_or_else(|| error("guard tool unregistered"))?;
    let program = tool["program"]
        .as_str()
        .ok_or_else(|| error("guard tool program missing"))?;
    let resolved = chrono_harness::resolve_program(
        root,
        program,
        Some(environment.get("PATH").map(String::as_str).unwrap_or("")),
    )?;
    if fs::canonicalize(resolved).map_err(error)?
        != fs::canonicalize(std::env::current_exe().map_err(error)?).map_err(error)?
    {
        return Err(error(
            "registered operation does not invoke this guard binary",
        ));
    }
    let declared_edges = edges(r)
        .into_iter()
        .filter(|e| {
            matches!(
                e.kind,
                EdgeKind::Compile | EdgeKind::BuildInput | EdgeKind::RuntimeInput
            )
        })
        .map(|e| graph::Edge {
            from: e.to,
            kind: e.kind,
            to: e.from,
        })
        .collect();
    let graph = graph::union(&BTreeSet::new(), &declared_edges);
    let node = format!("project:{owner}");
    let ancestors = graph::closure(
        &graph,
        &[Seed {
            id: node.clone(),
            node,
            reference: policy.into(),
            reason: "explicit Cargo consumer".into(),
        }],
    );
    let connected = |node: &str| ancestors.reached.contains_key(node);
    let files = r.filemap()["files"].as_array().unwrap();
    if !files.iter().any(|f| f["path"] == policy) || !connected(&format!("file:{policy}")) {
        return Err(error(
            "Cargo policy is not registered/connected to its consumer",
        ));
    }
    let retained = RefCell::new(BTreeMap::new());
    let retain = |path: PathBuf, expected: Option<&str>| -> Result<PathBuf> {
        let canonical = fs::canonicalize(&path).map_err(error)?;
        let (digest, _) = file_identity(&path).map_err(error)?;
        if expected.is_some_and(|s| s != digest) {
            return Err(error(format!("input identity changed: {}", path.display())));
        }
        retained
            .borrow_mut()
            .insert(path, (canonical.clone(), digest));
        Ok(canonical)
    };
    for path in facts::registry_paths(r.config(), config)? {
        retain(root.join(path), None)?;
    }
    retain(policy_path, None)?;
    for f in files
        .iter()
        .filter(|f| connected(&format!("file:{}", f["path"].as_str().unwrap())))
    {
        retain(no_symlink_parents(root, f["path"].as_str().unwrap())?, None)?;
    }
    let declarations = r.config()["environment"]["inputs"].as_array().unwrap();
    let strict_inputs = RefCell::new(Vec::new());
    let input = |id: &str| -> Result<PathBuf> {
        let matches: Vec<_> = declarations.iter().filter(|d| d["id"] == id).collect();
        if matches.len() != 1 || !connected(&format!("input:{id}")) {
            return Err(error(format!(
                "input {id} is not uniquely registered/connected"
            )));
        }
        let row = matches[0];
        let digest = row["sha256"]
            .as_str()
            .filter(|_| row["presence"] != "absent")
            .ok_or_else(|| error(format!("input {id} requires bound present bytes")))?;
        let path = root.join(row["location"].as_str().unwrap());
        if r.config()["schema_version"] == 2 {
            if chrono_judge_registration::inputs::observe_file(&path)
                .map_err(error)?
                .is_none()
            {
                return Err(error(format!("input {id} requires bound present bytes")));
            }
            strict_inputs.borrow_mut().push(path.clone());
        }
        retain(path, Some(digest))
    };
    let mut absent_inputs = Vec::new();
    let mut tool_inputs = BTreeMap::new();
    if let Some(compiler) = &contract.compiler {
        for key in [
            "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        ] {
            if environment.contains_key(key) {
                return Err(error(format!("compiler selection cannot also use {key}")));
            }
        }
        for (tool, id) in [
            (
                &contract.metadata.tool,
                contract.cargo_input.as_ref().unwrap(),
            ),
            (&compiler.tool, &compiler.input),
        ] {
            input(id)?;
            let row = declarations.iter().find(|d| d["id"] == *id).unwrap();
            let expected = ToolInput {
                path: root.join(row["location"].as_str().unwrap()),
                sha256: row["sha256"].as_str().unwrap().into(),
            };
            if tool.is_empty() || tool_inputs.insert(tool.clone(), expected).is_some() {
                return Err(error(
                    "compiler and Cargo need distinct registered tool IDs",
                ));
            }
        }
        let compiler_path = tool_inputs[&compiler.tool]
            .path
            .to_str()
            .ok_or_else(|| error("compiler path UTF-8"))?;
        if environment.get("RUSTC").map(String::as_str) != Some(compiler_path) {
            return Err(error(
                "RUSTC must select the explicitly bound compiler path",
            ));
        }
        for key in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            if environment.get(key).map(String::as_str) != Some("") {
                return Err(error(format!(
                    "compiler binding requires explicit empty {key} to disable Cargo wrappers"
                )));
            }
        }
    }
    for row in declarations
        .iter()
        .filter(|d| connected(&format!("input:{}", d["id"].as_str().unwrap())))
    {
        if row["presence"] == "absent" {
            let path = root.join(row["location"].as_str().unwrap());
            if chrono_judge_registration::inputs::observe_file(&path)
                .map_err(error)?
                .is_some()
            {
                return Err(error(format!("input {} must be absent", row["id"])));
            }
            absent_inputs.push(path);
        } else {
            input(row["id"].as_str().unwrap())?;
        }
    }
    let configuration = crate::configuration::check(
        root,
        environment,
        &contract.configuration_files,
        contract.configuration_ancestors,
        &observing.configurations,
        contract.compiler.is_some(),
        input,
    )?;
    let selected: BTreeSet<_> = contract
        .packages
        .iter()
        .filter_map(|p| p.project.clone())
        .collect();
    crate::check_policy(
        root,
        r,
        &Policy {
            schema: "chrono-cargo-projects/v1".into(),
            projects: contract.projects.clone(),
        },
        &selected,
        true,
    )?;
    let lock = table(&no_symlink_parents(root, &root_project.lockfile)?)?;
    let locked = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| error("missing Cargo lock packages"))?;
    let mut manifests = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for p in &contract.packages {
        if p.id.is_empty()
            || p.name.is_empty()
            || p.version.is_empty()
            || !identities.insert((p.name.clone(), p.version.clone(), p.source.clone()))
        {
            return Err(error("empty/duplicate package identity"));
        }
        strings(&p.features)?;
        strings(&p.inputs)?;
        dependency_set(&p.dependencies)?;
        let (manifest, source_files) = if let Some(local) = &p.project {
            if p.source.is_some()
                || p.manifest_input.is_some()
                || !p.inputs.is_empty()
                || p.checksum.is_some()
            {
                return Err(error("conflicting local/external package fields"));
            }
            let row = contract
                .projects
                .iter()
                .find(|row| row.project == *local)
                .ok_or_else(|| error("local Cargo project inventory missing"))?;
            let manifest = retain(no_symlink_parents(root, &row.manifest)?, None)?;
            let files = files
                .iter()
                .filter(|f| f["owner"] == *local)
                .map(|f| retain(no_symlink_parents(root, f["path"].as_str().unwrap())?, None))
                .collect::<Result<BTreeSet<_>>>()?;
            (manifest, files)
        } else {
            let source = p
                .source
                .as_deref()
                .ok_or_else(|| error("external package source missing"))?;
            if !(source.starts_with("registry+") || source.starts_with("git+"))
                || p.checksum.as_ref().is_some_and(|s| !wire::is_digest(s))
                || source.starts_with("registry+") && p.checksum.is_none()
            {
                return Err(error("invalid Cargo source/checksum"));
            }
            let id = p
                .manifest_input
                .as_deref()
                .ok_or_else(|| error("external manifest input missing"))?;
            if !p.inputs.iter().any(|s| s == id) {
                return Err(error("manifest absent from package allowlist"));
            }
            let manifest = input(id)?;
            let files = p
                .inputs
                .iter()
                .map(|id| input(id))
                .collect::<Result<BTreeSet<_>>>()?;
            if manifest.file_name().and_then(|s| s.to_str()) != Some("Cargo.toml")
                || files
                    .iter()
                    .any(|f| !f.starts_with(manifest.parent().unwrap()))
            {
                return Err(error("package inputs outside declared manifest root"));
            }
            if inventory(manifest.parent().unwrap())? != files {
                return Err(error(format!(
                    "package file inventory differs from explicit inputs: {}",
                    p.id
                )));
            }
            (manifest, files)
        };
        let matches: Vec<_> = locked
            .iter()
            .filter(|v| {
                v.get("name").and_then(toml::Value::as_str) == Some(&p.name)
                    && v.get("version").and_then(toml::Value::as_str) == Some(&p.version)
                    && v.get("source").and_then(toml::Value::as_str) == p.source.as_deref()
            })
            .collect();
        if matches.len() != 1
            || matches[0].get("checksum").and_then(toml::Value::as_str) != p.checksum.as_deref()
        {
            return Err(error(format!("lock identity/checksum mismatch: {}", p.id)));
        }
        let package = table(&manifest)?;
        if package
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
            != Some(&p.name)
            || package
                .get("package")
                .and_then(|v| v.get("version"))
                .and_then(toml::Value::as_str)
                != Some(&p.version)
        {
            return Err(error(format!("manifest identity mismatch: {}", p.id)));
        }
        if manifests.insert(p.id.clone(), manifest).is_some() {
            return Err(error("duplicate package ID"));
        }
        sources.insert(p.id.clone(), source_files);
    }
    for p in &contract.packages {
        for d in &p.dependencies {
            if !manifests.contains_key(&d.package) {
                return Err(error("undeclared dependency package"));
            }
        }
    }
    let target_directory = root.join(&root_project.output);
    Ok(Check {
        contract,
        configuration,
        tool_inputs,
        manifests,
        sources,
        target_directory,
        inputs: retained.into_inner(),
        absent_inputs,
        strict_inputs: strict_inputs.into_inner(),
    })
}
impl Check {
    pub(crate) fn unchanged(&self) -> Result {
        self.configuration.unchanged()?;
        for path in &self.absent_inputs {
            if chrono_judge_registration::inputs::observe_file(path)
                .map_err(error)?
                .is_some()
            {
                return Err(error(format!(
                    "absent input changed during guarded operation: {}",
                    path.display()
                )));
            }
        }
        for path in &self.strict_inputs {
            chrono_judge_registration::inputs::observe_file(path).map_err(error)?;
        }
        for (path, (canonical, digest)) in &self.inputs {
            if fs::canonicalize(path).map_err(error)? != *canonical
                || file_identity(path).map_err(error)?.0 != *digest
            {
                return Err(error(format!(
                    "input changed during guarded operation: {}",
                    path.display()
                )));
            }
        }
        for p in self
            .contract
            .packages
            .iter()
            .filter(|p| p.project.is_none())
        {
            if inventory(self.manifests[&p.id].parent().unwrap())? != self.sources[&p.id] {
                return Err(error(format!("package file inventory changed: {}", p.id)));
            }
        }
        Ok(())
    }
    pub(crate) fn validate(&self, v: &Value) -> Result {
        if v["version"] != 1 {
            return Err(error("Cargo metadata version must be 1"));
        }
        let packages = v["packages"]
            .as_array()
            .ok_or_else(|| error("metadata packages missing"))?;
        let nodes = v["resolve"]["nodes"]
            .as_array()
            .ok_or_else(|| error("metadata resolve missing"))?;
        if packages.len() != self.contract.packages.len() || nodes.len() != packages.len() {
            return Err(error("metadata package set differs from allowlist"));
        }
        let mut ids = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for actual in packages {
            let matches: Vec<_> = self
                .contract
                .packages
                .iter()
                .filter(|p| {
                    actual["name"] == p.name
                        && actual["version"] == p.version
                        && actual.get("source") == Some(&serde_json::to_value(&p.source).unwrap())
                })
                .collect();
            if matches.len() != 1 || !seen.insert(matches[0].id.clone()) {
                return Err(error("metadata package identity mismatch"));
            }
            let p = matches[0];
            let manifest = actual["manifest_path"]
                .as_str()
                .ok_or_else(|| error("metadata manifest missing"))?;
            if fs::canonicalize(manifest).map_err(error)? != self.manifests[&p.id] {
                return Err(error(format!(
                    "metadata source location mismatch: {}",
                    p.id
                )));
            }
            for target in actual["targets"]
                .as_array()
                .ok_or_else(|| error("metadata targets missing"))?
            {
                let path = target["src_path"]
                    .as_str()
                    .ok_or_else(|| error("metadata source target missing"))?;
                if !self.sources[&p.id].contains(&fs::canonicalize(path).map_err(error)?) {
                    return Err(error(format!("unregistered target source: {}", p.id)));
                }
            }
            let id = actual["id"]
                .as_str()
                .ok_or_else(|| error("metadata package ID missing"))?;
            if ids.insert(id.to_string(), p.id.clone()).is_some() {
                return Err(error("duplicate metadata package ID"));
            }
        }
        let lookup = |v: &Value| -> Result<String> {
            let s = v.as_str().ok_or_else(|| error("invalid metadata ID"))?;
            ids.get(s)
                .cloned()
                .ok_or_else(|| error("undeclared metadata ID"))
        };
        let root = lookup(&v["resolve"]["root"])?;
        let members = v["workspace_members"]
            .as_array()
            .ok_or_else(|| error("workspace members missing"))?;
        if root != self.contract.root || members.len() != 1 || lookup(&members[0])? != root {
            return Err(error(
                "Cargo workspace/root differs from independent project",
            ));
        }
        if Path::new(
            v["workspace_root"]
                .as_str()
                .ok_or_else(|| error("workspace root missing"))?,
        ) != self.manifests[&root].parent().unwrap()
            || Path::new(
                v["target_directory"]
                    .as_str()
                    .ok_or_else(|| error("target directory missing"))?,
            ) != self.target_directory
        {
            return Err(error("Cargo workspace/target directory binding"));
        }
        let mut seen_nodes = BTreeSet::new();
        for node in nodes {
            let id = lookup(&node["id"])?;
            if !seen_nodes.insert(id.clone()) {
                return Err(error("duplicate resolution node"));
            }
            let expected = self.contract.packages.iter().find(|p| p.id == id).unwrap();
            let features: Vec<String> =
                serde_json::from_value(node["features"].clone()).map_err(error)?;
            if strings(&features)? != strings(&expected.features)? {
                return Err(error(format!("resolved features differ: {id}")));
            }
            let mut dependencies = Vec::new();
            for dep in node["deps"]
                .as_array()
                .ok_or_else(|| error("metadata dependencies missing"))?
            {
                dependencies.push(Dependency {
                    package: lookup(&dep["pkg"])?,
                    name: dep["name"]
                        .as_str()
                        .ok_or_else(|| error("dependency alias missing"))?
                        .into(),
                    kinds: serde_json::from_value(dep["dep_kinds"].clone()).map_err(error)?,
                });
            }
            if dependency_set(&dependencies)? != dependency_set(&expected.dependencies)? {
                return Err(error(format!("resolved dependencies differ: {id}")));
            }
            let raw: Vec<_> = node["dependencies"]
                .as_array()
                .ok_or_else(|| error("dependency IDs missing"))?
                .iter()
                .map(lookup)
                .collect::<Result<_>>()?;
            if strings(&raw)? != dependencies.iter().map(|d| d.package.clone()).collect() {
                return Err(error("inconsistent Cargo dependency representations"));
            }
        }
        Ok(())
    }
}
