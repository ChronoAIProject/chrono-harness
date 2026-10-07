//! Explicit CI policy: fixed Git snapshots, registered old/new impact graph, candidate operations.
use chrono_harness::{
    CheckConfig, CheckResult, PROTOCOL, ProcessResult, Request, Response, Status, decode, json,
    relative_path, sha256,
};
use chrono_judge_filemap::graph::{self, Edge, EdgeKind, Seed};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as object};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
mod collect;
mod units;

/// The receipt is the canonical owner of complete process evidence.  The
/// adjacent `process` field in an executed row is a compatibility projection
/// used for status/identity consumers; large streams are kept only once, in
/// `receipt.process`, so a large DELTA cannot exhaust the judge transport
/// merely by repeating the same bytes.
const INLINE_PROCESS_STREAM_BYTES: usize = 4096;

pub(crate) fn process_projection(process: &ProcessResult) -> Value {
    let mut value = serde_json::to_value(process).expect("process result is serializable");
    if process
        .stdout_bytes
        .len()
        .saturating_add(process.stderr_bytes.len())
        > INLINE_PROCESS_STREAM_BYTES
    {
        let object = value
            .as_object_mut()
            .expect("serialized process result is an object");
        object.remove("stdout_bytes");
        object.remove("stderr_bytes");
        object.remove("stdout");
        object.remove("stderr");
        object.insert("stdout_omitted".into(), Value::Bool(true));
        object.insert("stderr_omitted".into(), Value::Bool(true));
    }
    value
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CollectionLimits {
    pub manifest_bytes: u64,
    pub report_bytes: u64,
}

impl Default for CollectionLimits {
    fn default() -> Self {
        Self {
            manifest_bytes: 64 * 1024 * 1024,
            report_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_publication: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_limits: Option<CollectionLimits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<BTreeMap<String, chrono_harness::units::Unit>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_operations: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts_config: Option<String>,
    #[serde(default)]
    pub registration_config: Option<String>,
    pub filemap: String,
    pub projects: String,
    pub tools: BTreeMap<String, String>,
    #[serde(default)]
    pub bindings: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    pub artifacts: Vec<String>,
    pub required_inputs: Vec<String>,
    pub adoption_base: Option<String>,
    pub operation_timeout_seconds: u64,
    pub operation_output_limit_bytes: usize,
}
#[derive(Clone, Debug)]
struct Snapshot {
    registry: Value,
    owner_nodes: BTreeMap<String, String>,
    files: BTreeMap<String, Value>,
    projects: BTreeMap<String, Value>,
    operations: BTreeMap<String, (String, String, Vec<String>)>,
    edges: BTreeSet<Edge>,
    tree: BTreeMap<String, (String, String)>,
    policy: Policy,
    plans: BTreeMap<String, chrono_judge_registration::execution::Plan>,
    execute: BTreeMap<String, String>,
    scheduling: Option<chrono_judge_registration::execution::Scheduling>,
}
use chrono_harness::facts::{Reader, utf8};
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing/string field {k}"))
}
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>, String> {
    v.get(k)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing/array field {k}"))
}
fn strings(v: &Value) -> Result<Vec<String>, String> {
    v.as_array()
        .ok_or("expected string array")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or("expected string".into())
        })
        .collect()
}
fn id(s: &str) -> Result<(), String> {
    if s.is_empty()
        || !s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(format!("invalid ID {s:?}"));
    }
    Ok(())
}
pub fn full_oid(s: &str) -> Result<(), String> {
    if !matches!(s.len(), 40 | 64)
        || s.bytes()
            .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
        || s.bytes().all(|b| b == b'0')
    {
        return Err(format!("expected full nonzero lowercase OID: {s}"));
    }
    Ok(())
}
fn verify_oid(reader: &Reader, root: &Path, oid: &str) -> Result<(), String> {
    full_oid(oid)?;
    reader.verify_oid(root, oid)?;
    Ok(())
}
fn at(reader: &Reader, root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
    relative_path(path)?;
    reader.blob(root, oid, path)
}
fn tree(
    reader: &Reader,
    root: &Path,
    oid: &str,
) -> Result<BTreeMap<String, (String, String)>, String> {
    reader
        .tree(root, oid)?
        .into_iter()
        .map(|(path, entry)| {
            if entry.kind != "blob"
                || !matches!(entry.mode.as_str(), "100644" | "100755" | "120000")
            {
                return Err(format!(
                    "unsupported tree entry {path}: {} {} {}",
                    entry.mode, entry.kind, entry.oid
                ));
            }
            Ok((path, (entry.mode, entry.oid)))
        })
        .collect()
}
fn policy(c: &CheckConfig) -> Result<Policy, String> {
    chrono_harness::units::reference_publication(c)?;
    if !matches!(
        c.schema.as_str(),
        "chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3"
    ) {
        return Err("unsupported CI profile".into());
    }
    if c.schema == "chrono-ci-check/v1" && c.policy.get("facts_config").is_some() {
        return Err("v1 cannot declare facts_config, including null".into());
    }
    let p: Policy = serde_json::from_value(c.policy.clone()).map_err(|e| e.to_string())?;
    if c.schema == "chrono-ci-check/v2" {
        let path = p
            .facts_config
            .as_deref()
            .ok_or("v2 requires facts_config")?;
        relative_path(path)?;
        if !path.starts_with(".chrono-harness/") {
            return Err("facts_config must reside in .chrono-harness/".into());
        }
    }
    relative_path(&p.filemap)?;
    relative_path(&p.projects)?;
    if p.artifacts.is_empty() {
        return Err("artifact declarations required".into());
    }
    for a in &p.artifacts {
        if !a.ends_with('/') {
            return Err("artifact must be a directory prefix ending /".into());
        }
        relative_path(a.trim_end_matches('/'))?;
    }
    if !p.artifacts.iter().any(|a| c.report_path.starts_with(a)) {
        return Err("report must reside in declared artifact directory".into());
    }
    for v in &p.required_inputs {
        relative_path(v)?
    }
    for (k, v) in &p.tools {
        id(k)?;
        if v.is_empty() {
            return Err("empty tool program".into());
        }
    }
    if let Some(b) = &p.adoption_base {
        full_oid(b)?
    }
    if p.operation_timeout_seconds == 0
        || p.operation_output_limit_bytes == 0
        || p.operation_output_limit_bytes > 64 * 1024 * 1024
    {
        return Err("invalid operation bounds".into());
    }
    units::validate(c, &p)?;
    Ok(p)
}
fn snapshot(reader: &Reader, root: &Path, oid: &str, mut p: Policy) -> Result<Snapshot, String> {
    let fm = json(&at(reader, root, oid, &p.filemap)?)?;
    let pr = json(&at(reader, root, oid, &p.projects)?)?;
    if ![object!(1), object!(2)].contains(&fm["schema_version"])
        || ![object!(1), object!(2)].contains(&pr["schema_version"])
    {
        return Err("unsupported registry version".into());
    }
    // Legacy scoped v1 retains its original loose shape; v2 adopts the shared schema.
    if pr["schema_version"] == 2 {
        chrono_judge_registration::validate_projects(&pr)?;
    }
    let plans = if fm["schema_version"] == 2 {
        if !p.bindings.is_empty() {
            return Err("current FILEMAP v2 cannot duplicate plans in scoped bindings".into());
        }
        let plans = chrono_judge_registration::execution::plans(&fm)?;
        p.bindings = plans
            .iter()
            .map(|(id, p)| (id.clone(), p.operations.clone()))
            .collect();
        plans
    } else {
        p.bindings
            .iter()
            .map(|(id, ops)| {
                (
                    id.clone(),
                    chrono_judge_registration::execution::Plan {
                        operations: ops.clone(),
                        timeout_seconds: p.operation_timeout_seconds,
                        output_limit_bytes: p.operation_output_limit_bytes,
                    },
                )
            })
            .collect()
    };
    let execute: BTreeMap<_, _> = chrono_judge_registration::execution::test_bindings(&pr)?
        .into_iter()
        .map(|(test, bindings)| match bindings.as_slice() {
            [binding] => Ok((test, binding.operation.clone())),
            _ => Err(format!("E_TEST_BINDING: duplicate test identity {test}")),
        })
        .collect::<Result<_, String>>()?;
    let owners: BTreeSet<_> = strings(&pr["owners"])?.into_iter().collect();
    if owners.is_empty() {
        return Err("empty owner registry".into());
    }
    let mut projects = BTreeMap::new();
    let mut owner_nodes = BTreeMap::new();
    let mut ops = BTreeMap::new();
    let mut nodes = BTreeSet::new();
    if let Some(config) = &p.registration_config {
        let snapshot = reader.registry_snapshot(root, oid, config)?;
        let config = snapshot
            .values
            .get(&snapshot.effective_path)
            .ok_or("missing effective registration config")?;
        for tool in config["tools"].as_array().ok_or("tools missing")? {
            nodes.insert(format!("tool:{}", text(tool, "id")?));
        }
        for input in config["environment"]["inputs"]
            .as_array()
            .ok_or("inputs missing")?
        {
            nodes.insert(format!("input:{}", text(input, "id")?));
        }
        for key in config["environment"]["inherit"]
            .as_array()
            .ok_or("inherit missing")?
            .iter()
            .filter_map(Value::as_str)
            .chain(
                config["environment"]["values"]
                    .as_object()
                    .ok_or("values missing")?
                    .keys()
                    .map(String::as_str),
            )
        {
            nodes.insert(format!("environment:{key}"));
        }
    }

    for collection in ["projects", "scripts"] {
        for entry in array(&pr, collection)? {
            let name = text(entry, "id")?;
            id(name)?;
            if !owners.contains(name) {
                return Err(format!("undeclared owner {name}"));
            }
            if projects.insert(name.to_owned(), entry.clone()).is_some() {
                return Err(format!("duplicate project/script {name}"));
            }
            let prefix = if pr["schema_version"] == 2 && collection == "scripts" {
                "script"
            } else {
                "project"
            };
            owner_nodes.insert(name.to_owned(), format!("{prefix}:{name}"));
            nodes.insert(format!("script:{name}"));
            nodes.insert(format!("project:{name}"));
            for a in entry
                .get("actions")
                .and_then(Value::as_object)
                .ok_or("actions object required")?
                .values()
            {
                let operation = text(a, "operation")?;
                id(operation)?;
                let tool = text(a, "tool")?;
                if !p.tools.contains_key(tool) {
                    return Err(format!("unknown tool {tool}"));
                }
                let argv = strings(&a["argv"])?;
                if a.as_object().is_none_or(|o| o.len() != 3)
                    || argv.iter().any(|s| s.contains('\0'))
                {
                    return Err(format!("invalid action {operation}"));
                }
                if ops
                    .insert(
                        operation.to_string(),
                        (name.to_string(), tool.to_string(), argv),
                    )
                    .is_some()
                {
                    return Err(format!("duplicate operation {operation}"));
                }
            }
        }
    }
    nodes.extend(execute.keys().cloned());
    for (test, binding) in &p.bindings {
        if !test.starts_with("test:") || !nodes.contains(test) || binding.is_empty() {
            return Err(format!("invalid check binding {test}"));
        }
        let mut seen = BTreeSet::new();
        for op in binding {
            if !ops.contains_key(op) || !seen.insert(op) {
                return Err(format!("invalid bound operation {op}"));
            }
        }
    }
    let tree = tree(reader, root, oid)?;
    let mut files = BTreeMap::new();
    for f in array(&fm, "files")? {
        let path = text(f, "path")?;
        relative_path(path)?;
        if !owners.contains(text(f, "owner")?) {
            return Err(format!("unknown file owner: {path}"));
        }
        if !matches!(
            text(f, "surface")?,
            "product"
                | "test"
                | "documentation"
                | "membership"
                | "instruction-policy"
                | "judge-policy"
                | "judge-implementation"
        ) {
            return Err(format!("unknown surface: {path}"));
        }
        text(f, "cost")?;
        if files.insert(path.to_string(), f.clone()).is_some() {
            return Err(format!("duplicate FILEMAP path {path}"));
        }
        nodes.insert(format!("file:{path}"));
    }
    for path in tree.keys() {
        if !files.contains_key(path) {
            return Err(format!("unregistered tracked path at {oid}: {path}"));
        }
        if p.artifacts.iter().any(|a| path.starts_with(a)) {
            return Err(format!("tracked path overlaps artifact exemption: {path}"));
        }
    }
    for (path, f) in &files {
        let (mode, _) = tree
            .get(path)
            .ok_or_else(|| format!("registered path absent from {oid}: {path}"))?;
        match (mode.as_str(), f.get("symlink")) {
            ("120000", Some(Value::String(target))) => {
                relative_path(target)?;
                if utf8(at(reader, root, oid, path)?)? != *target {
                    return Err(format!("symlink target mismatch: {path}"));
                }
                let dest = Path::new(path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(target);
                let dest = dest.to_str().ok_or("symlink target UTF-8")?;
                if !tree.get(dest).is_some_and(|(m, _)| m != "120000") {
                    return Err(format!("symlink target not regular: {path}"));
                }
            }
            ("120000", _) => return Err(format!("undeclared symlink: {path}")),
            (_, Some(_)) => return Err(format!("declared symlink is regular: {path}")),
            _ => {}
        }
    }
    for required in p.required_inputs.iter().chain([&p.filemap, &p.projects]) {
        if !tree.get(required).is_some_and(|(m, _)| m != "120000") {
            return Err(format!("required regular input absent: {required}"));
        }
    }
    let mut edges = BTreeSet::new();
    let mut add = |from: &str, e: &Value| -> Result<(), String> {
        let kind = text(e, "kind")?;
        let to = text(e, "to")?;
        if !matches!(
            kind,
            "compile" | "build-input" | "runtime-input" | "test-execution" | "judge-trigger"
        ) {
            return Err(format!("unsupported edge kind {kind}"));
        }
        if kind == "judge-trigger" {
            if !to.starts_with("judge:") {
                return Err("judge-trigger target requires judge namespace".into());
            }
            return Ok(());
        }
        if !nodes.contains(from) || !nodes.contains(to) {
            return Err(format!("unresolved edge {from} -> {to}"));
        }
        if kind == "test-execution" && (!to.starts_with("test:") || !p.bindings.contains_key(to)) {
            return Err(format!("test edge lacks active binding: {to}"));
        }
        if kind != "test-execution" && to.starts_with("test:") {
            return Err("test target requires test-execution edge".into());
        }
        if !edges.insert(Edge {
            from: from.into(),
            kind: serde_json::from_value(Value::String(kind.into())).map_err(|e| e.to_string())?,
            to: to.into(),
        }) {
            return Err(format!("duplicate edge {from} -> {to}"));
        }
        Ok(())
    };
    for (path, f) in &files {
        for e in array(f, "edges")? {
            add(&format!("file:{path}"), e)?
        }
    }
    for e in array(&fm, "project_edges")? {
        add(text(e, "from")?, e)?
    }
    let artifacts = if let Some(config_path) = &p.registration_config {
        let values = reader.registry_snapshot(root, oid, config_path)?;
        values.values[&values.effective_path]["artifacts"].clone()
    } else {
        object!(
            p.artifacts
                .iter()
                .map(|path| object!({"path":path}))
                .collect::<Vec<_>>()
        )
    };
    let scheduling = chrono_judge_registration::execution::scheduling(&fm, &pr, &artifacts)?;
    let snapshot = Snapshot {
        registry: pr,
        owner_nodes,
        files,
        projects,
        operations: ops,
        edges,
        tree,
        policy: p,
        plans,
        execute,
        scheduling,
    };
    units::assignments(&snapshot)?;
    Ok(snapshot)
}
fn clean(reader: &Reader, root: &Path, candidate: &str, p: &Policy) -> Result<(), String> {
    if utf8(reader.git(root, &["rev-parse", "HEAD"])?)?.trim() != candidate {
        return Err("checkout HEAD does not equal candidate".into());
    }
    // -v lowercases tags for assume-unchanged; S/s marks skip-worktree.
    // Paths follow the two-byte tag prefix and end at NUL, never whitespace.
    for entry in reader.index_flags(root)? {
        let assume_unchanged = entry.tag.as_bytes()[0].is_ascii_lowercase();
        let skip_worktree = entry.tag.eq_ignore_ascii_case("S");
        if assume_unchanged || skip_worktree {
            let path = entry.path;
            let flag = match (assume_unchanged, skip_worktree) {
                (true, true) => "assume-unchanged and skip-worktree",
                (true, false) => "assume-unchanged",
                _ => "skip-worktree",
            };
            return Err(format!("unsupported index flags ({flag}): {path}"));
        }
    }
    if !reader.tracked_changes(root, candidate)?.is_empty() {
        return Err("dirty tracked candidate inputs/index".into());
    }
    let artifacts: Vec<_> = p.artifacts.iter().map(String::as_str).collect();
    for path in reader.untracked_excluding(root, &artifacts)? {
        if !p.artifacts.iter().any(|a| path.starts_with(a)) {
            return Err(format!("untracked nonartifact input: {path}"));
        }
    }
    Ok(())
}
fn changed<T: PartialEq>(a: &BTreeMap<String, T>, b: &BTreeMap<String, T>) -> BTreeSet<String> {
    a.keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}
// Scoped CI owns legacy seeds and selections; it consumes only shared union/closure mechanics.
fn ci_impact(
    old: Option<&Snapshot>,
    new: &Snapshot,
) -> (BTreeSet<String>, BTreeSet<String>, Value, BTreeSet<String>) {
    let paths = if let Some(old) = old {
        changed(&old.tree, &new.tree)
    } else {
        new.tree.keys().cloned().collect()
    };
    let mut seeds: BTreeSet<_> = paths.iter().map(|p| format!("file:{p}")).collect();
    let mut extras: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(old) = old {
        // Adopting a new document contract affects every declared owner, even
        // when legacy rows already contain the newly meaningful metadata.
        if old.registry["schema_version"] != new.registry["schema_version"] {
            seeds.extend(
                old.owner_nodes
                    .values()
                    .chain(new.owner_nodes.values())
                    .cloned(),
            );
        }
        for path in changed(&old.files, &new.files) {
            seeds.insert(format!("file:{path}"));
        }
        for p in changed(&old.projects, &new.projects) {
            for snapshot in [old, new] {
                if let Some(node) = snapshot.owner_nodes.get(&p) {
                    seeds.insert(node.clone());
                }
            }
        }
        for test in changed(&old.execute, &new.execute) {
            extras
                .entry(test)
                .or_default()
                .insert("changed test action binding".into());
        }
        for op in changed(&old.operations, &new.operations) {
            for s in [old, new] {
                if let Some((owner, _, _)) = s.operations.get(&op) {
                    seeds.insert(s.owner_nodes[owner].clone());
                }
                for (test, ops) in &s.policy.bindings {
                    if ops.contains(&op) {
                        extras
                            .entry(test.clone())
                            .or_default()
                            .insert(format!("changed operation: {op}"));
                    }
                }
            }
        }
        for e in old.edges.symmetric_difference(&new.edges) {
            seeds.insert(e.from.clone());
            if e.kind != EdgeKind::TestExecution {
                seeds.insert(e.to.clone());
            }
        }
        // Effective plans retain explicit v2 bounds and the projected legacy v1 bounds.
        for test in changed(&old.plans, &new.plans) {
            let before = old.plans.get(&test);
            let after = new.plans.get(&test);
            let reasons = extras.entry(test).or_default();
            if before.map(|p| &p.operations) != after.map(|p| &p.operations) {
                reasons.insert("changed binding".into());
            }
            if let (Some(before), Some(after)) = (before, after) {
                if before.timeout_seconds != after.timeout_seconds {
                    reasons.insert("changed operation timeout".into());
                }
                if before.output_limit_bytes != after.output_limit_bytes {
                    reasons.insert("changed operation output limit".into());
                }
            }
        }
        if old.scheduling != new.scheduling {
            for s in [old, new] {
                for (test, plan) in &s.plans {
                    let global = old
                        .scheduling
                        .as_ref()
                        .map(|p| (p.max_running, &p.resources))
                        != new
                            .scheduling
                            .as_ref()
                            .map(|p| (p.max_running, &p.resources));
                    if global
                        || plan.operations.iter().any(|op| {
                            old.scheduling.as_ref().and_then(|p| p.claims.get(op))
                                != new.scheduling.as_ref().and_then(|p| p.claims.get(op))
                                || [old.scheduling.as_ref(), new.scheduling.as_ref()]
                                    .into_iter()
                                    .flatten()
                                    .any(|policy| {
                                        policy.claims.keys().any(|changed| {
                                            old.scheduling
                                                .as_ref()
                                                .and_then(|p| p.claims.get(changed))
                                                != new
                                                    .scheduling
                                                    .as_ref()
                                                    .and_then(|p| p.claims.get(changed))
                                                && policy.claims.contains_key(op)
                                                && policy.conflicts(op, changed)
                                        })
                                    })
                        })
                    {
                        extras
                            .entry(test.clone())
                            .or_default()
                            .insert("changed execution scheduling declaration".into());
                    }
                }
            }
        }
        let empty_units = BTreeMap::new();
        let old_units = old.policy.units.as_ref().unwrap_or(&empty_units);
        let new_units = new.policy.units.as_ref().unwrap_or(&empty_units);
        for id in changed(old_units, new_units) {
            for unit in [old_units.get(&id), new_units.get(&id)]
                .into_iter()
                .flatten()
            {
                for test in &unit.tests {
                    extras
                        .entry(test.clone())
                        .or_default()
                        .insert(format!("changed unit assignment: {id}"));
                }
            }
        }
        let changed_tools = changed(&old.policy.tools, &new.policy.tools);
        if !changed_tools.is_empty() || old.policy.environment != new.policy.environment {
            for s in [old, new] {
                for (test, ops) in &s.policy.bindings {
                    if old.policy.environment != new.policy.environment
                        || ops.iter().any(|op| {
                            s.operations
                                .get(op)
                                .is_some_and(|(_, tool, _)| changed_tools.contains(tool))
                        })
                    {
                        let reasons = extras.entry(test.clone()).or_default();
                        if old.policy.environment != new.policy.environment {
                            reasons.insert("changed environment".into());
                        }
                        for tool in &changed_tools {
                            if ops
                                .iter()
                                .any(|op| s.operations.get(op).is_some_and(|(_, t, _)| t == tool))
                            {
                                reasons.insert(format!("changed bound tool: {tool}"));
                            }
                        }
                    }
                }
            }
        }
    }
    let empty = BTreeSet::new();
    let edges = graph::union(old.map(|s| &s.edges).unwrap_or(&empty), &new.edges);
    let seeds: Vec<_> = seeds
        .into_iter()
        .map(|node| Seed {
            id: node.clone(),
            reference: node.clone(),
            node,
            reason: "scoped CI path, registration or operation seed".into(),
        })
        .collect();
    let closure = graph::closure(&edges, &seeds);
    let mut selected = closure.selected_tests();
    let legacy_only: BTreeSet<_> = extras
        .keys()
        .filter(|t| !selected.contains(*t))
        .cloned()
        .collect();
    selected.extend(extras.keys().cloned());
    let seed_nodes: Vec<_> = seeds.iter().map(|s| &s.node).collect();
    let mut affected: BTreeSet<String> = closure.reached.keys().cloned().collect();
    // A large DELTA can make the predecessor map quadratic in the number of
    // seeds.  Keep the complete explanation for ordinary changes, but publish
    // a digest/count for oversized derived paths.  The registered graph,
    // seeds, traversed edges and selected tests remain available in the same
    // response; omission is explicit and fail-closed consumers can require a
    // smaller scope when they need every witness path inline.
    let graph::Closure {
        reached,
        predecessors,
        traversed,
    } = closure;
    // Estimate the derived map from registered node/seed cardinalities only.
    // No second JSON traversal is allowed here: the graph itself may be large
    // enough that serialising it merely to decide whether to omit it would
    // exhaust the judge before the bounded response is written.
    let predecessor_count = predecessors.values().map(BTreeMap::len).sum::<usize>();
    let estimated_predecessor_entries = seeds.len().saturating_mul(reached.len());
    let predecessors_omitted = estimated_predecessor_entries > 16_384
        || predecessor_count > 16_384
        || reached.len() > 8_192
        || traversed.len() > 16_384;
    // Once the predecessor map exceeds the inline budget, retain only bounded
    // structural counts for the derived closure as well. Re-emitting the
    // reached/traversed maps would recreate the same quadratic transport
    // pressure even after removing predecessors.
    let paths_omitted = predecessors_omitted;
    let closure_value = if paths_omitted {
        object!({
            "predecessors_omitted": true,
            "predecessor_count": predecessor_count,
            "reached_omitted": true,
            "reached_count": reached.len(),
            "traversed_omitted": true,
            "traversed_count": traversed.len()
        })
    } else if predecessors_omitted {
        object!({
            "reached": reached,
            "traversed": traversed,
            "predecessors_omitted": true,
            "predecessor_count": predecessor_count
        })
    } else {
        object!({
            "reached": reached,
            "predecessors": predecessors,
            "traversed": traversed
        })
    };
    let mut explanation = object!({
        "scope":"chrono-ci-check/v1-adapter",
        "edges": if paths_omitted {
            object!({
                "omitted": true,
                "count": edges.len()
            })
        } else {
            object!(edges)
        },
        "seeds":seed_nodes,
        "seed_causes":seeds,
        "closure":closure_value,
        "extra_selections":extras,
        "legacy_only_selections":legacy_only
    });
    if predecessors_omitted {
        explanation["predecessor_transport"] = object!({
            "omitted": true,
            "count": predecessor_count,
            "source": "registered FILEMAP graph plus explicit seeds"
        });
    }
    if paths_omitted {
        explanation["closure_transport"] = object!({"omitted":true});
    }
    if old.is_none() {
        affected.extend(new.owner_nodes.values().cloned());
    }
    // Direct plan/tool selections also carry their explicitly registered test owner.
    for (test, bindings) in chrono_judge_registration::execution::test_bindings(&new.registry)
        .expect("snapshot validated test bindings")
    {
        if selected.contains(&test) {
            affected.extend(bindings.into_iter().map(|binding| binding.owner));
        }
    }
    (paths, selected, explanation, affected)
}

pub fn judge(req: &Request) -> Response {
    let mut reader = Reader::legacy();
    let mut opening = Value::Null;
    let mut scope = "chrono-ci-check/v1".to_owned();
    let result = evaluate(req, &mut reader, &mut opening, &mut scope);
    let mut response = match result {
        Ok(r) => r,
        Err(e) => Response {
            protocol: units::scope_protocol(req).into(),
            request_id: req.request_id.clone(),
            status: Status::Failed,
            results: vec![CheckResult {
                id: "ci.inventory".into(),
                status: Status::Failed,
                cause: e,
                exit_code: None,
            }],
            evidence: object!({"scope":scope,"candidate":req.candidate,"base":req.base,"selected":[],"executed":[],"blocked":["selection or snapshot validation failed"],"parity":"unestablished"}),
        },
    };
    if reader.is_bound() {
        response.evidence["git_facts"] = reader.observation();
    } else if !opening.is_null() {
        response.evidence["git_facts"] = opening;
    }
    response
}
// Shared DELTA planning. Collection acquisition consumes requirements, never an admission result.
struct Inventory {
    config: CheckConfig,
    p: Policy,
    new: Snapshot,
    previous: String,
    paths: BTreeSet<String>,
    selected: BTreeSet<String>,
    global_selected: BTreeSet<String>,
    selection_explanation: Value,
    blocked: Vec<String>,
    removed: BTreeMap<String, String>,
    conversion: Value,
    reused: BTreeMap<String, chrono_harness::observation::Tool>,
    environment: BTreeMap<String, String>,
    environment_policy: Value,
    declarations: Value,
    methods: BTreeMap<String, Vec<chrono_judge_registration::execution::Method>>,
}

/// Resolve only DELTA-required collection units through the adjudicator's current inventory/assignment owner.
/// This does not validate a canonical entry, execute operations or grant admission.
/// Full preparation shares FILEMAP impact and routes' global obligation owner.
/// No judge or business/version process is launched here.
pub fn full_collection_requirements(
    root: &Path,
    profile: &str,
    prepared: &chrono_harness::prepared::PreparedCheck,
    originals: &chrono_harness::units::FullManifest,
) -> Result<Value, String> {
    full_collection_requirements_with_inputs(root, profile, prepared, &|_| Ok(originals.clone()))
}
/// The acquisition callback is used only for necessary historical conversion.
/// Registration names the decoder test units; the collector reads DELTA-required
/// reports separately after the shared selection owner returns its requirements.
pub fn full_collection_requirements_with_inputs(
    root: &Path,
    profile: &str,
    prepared: &chrono_harness::prepared::PreparedCheck,
    originals: &dyn Fn(&[String]) -> Result<chrono_harness::units::FullManifest, String>,
) -> Result<Value, String> {
    let context = prepared
        .context
        .as_ref()
        .ok_or("full collection context missing")?;
    let (mut req, _, reader, cfg) = chrono_harness::full::prepare_request(
        root,
        profile,
        prepared.base.as_deref().ok_or("full base missing")?,
        &prepared.candidate,
        Path::new(&context.path),
        Value::Null,
        prepared.scope.clone(),
        None,
    )?;
    req.judge_id = "routes".into();
    let result = (|| {
        // prepare_request observes its current process, which here is chrono-ci.
        // Bootstrap instead carries the independently validated registered runner;
        // no retained report can assign an executable identity to this request.
        let runner = chrono_harness::no_symlink_parents(
            root,
            cfg["runner"]["path"]
                .as_str()
                .ok_or("registered runner path")?,
        )?;
        let runner = fs::canonicalize(runner).map_err(|e| e.to_string())?;
        let digest = chrono_harness::file_identity(&runner)?.0;
        if cfg["runner"]["sha256"] != digest || cfg["runner"]["version"] != req.runner.version {
            return Err("E_EXECUTABLE_BINDING: registered collection runner differs".into());
        }
        req.runner.path = runner.to_str().ok_or("runner path UTF8")?.into();
        req.runner.sha256 = digest;
        let (old, r, _) =
            chrono_judge_registration::views_for_collection_inputs_with(&req, &reader, originals)?;
        let inputs = chrono_judge_registration::inputs::validate(&req, &old, &r)?;
        let (impact, findings) =
            chrono_judge_filemap::produce_for_request(&req, &old, &r, &reader, &inputs)?;
        if findings.iter().any(|f| f.level == "error") {
            return Err(format!(
                "blocked full obligations: {}",
                serde_json::to_string(&findings).map_err(|e| e.to_string())?
            ));
        }
        let selected = chrono_judge_routes::global_selection(
            &old,
            &r,
            &impact,
            chrono_judge_registration::downstream_validator(&r, "routes"),
        )?;
        let units = serde_json::from_value(r.config()["execution_units"]["units"].clone())
            .map_err(|e| e.to_string())?;
        Ok(
            object!({"required_units":chrono_harness::units::required(&units,&selected),"global_selected":selected,"git_facts":reader.observation()}),
        )
    })();
    fs::remove_dir_all(&req.base.root).map_err(|e| e.to_string())?;
    result
}
/// Full scheduling reads complete trees and registry/semantic blobs, never exports host sources,
/// captures SDK inputs, or launches a historical decoder. Unsupported historical registries fail.
/// External effective-input changes remain the final full collection owner's obligation.
pub fn full_scheduling_requirements(
    root: &Path,
    profile: &str,
    base: &str,
    candidate: &str,
) -> Result<Value, String> {
    let reader = Reader::for_config(root, profile)?;
    reader.verify_config(root, candidate)?;
    reader.verify_oid(root, base)?;
    reader.verify_oid(root, candidate)?;
    let (old, current, _) = chrono_judge_registration::interpret_scheduling_with_reader(
        &reader,
        root,
        base,
        candidate,
        profile,
        reader.registry_values(root, base, profile)?,
        reader.registry_values(root, candidate, profile)?,
    )
    .map_err(|e| {
        format!("full scheduling requires directly readable historical registries: {e}")
    })?;
    let delta =
        chrono_harness::facts::delta(&reader.tree(root, base)?, &reader.tree(root, candidate)?);
    let (impact, findings) = chrono_judge_filemap::produce_for_endpoints(
        root, base, candidate, profile, &delta, &old, &current, &reader,
    )?;
    if findings.iter().any(|f| f.level == "error") {
        return Err(format!(
            "blocked full scheduling obligations: {}",
            serde_json::to_string(&findings).map_err(|e| e.to_string())?
        ));
    }
    let selected = chrono_judge_routes::global_selection(
        &old,
        &current,
        &impact,
        chrono_judge_registration::downstream_validator(&current, "routes"),
    )?;
    let plans = chrono_judge_registration::execution::plans(current.filemap())?;
    let block = chrono_harness::units::full_execution_units(current.config())?
        .ok_or("full scheduling units missing")?;
    let units = chrono_judge_routes::validate_unit_assignments(&block, &plans)?;
    chrono_judge_routes::order(
        &selected,
        &plans,
        &chrono_judge_registration::execution::methods(current.projects())?,
        &chrono_judge_routes::execute_actions(&current),
    )?;
    Ok(
        object!({"required_units":chrono_harness::units::required(&units, &selected),"global_selected":selected,"changed_paths":delta.iter().map(|d| &d.path).collect::<Vec<_>>(),"git_facts":reader.observation(),"input_scope":"git-and-declarations; final-effective-input-admission-required"}),
    )
}

/// Scheduling uses the same scoped inventory without requiring a materialized host checkout.
/// Registry bytes are still bound to the fixed candidate; this grants no check admission.
pub fn scheduling_requirements(
    root: &Path,
    profile: &str,
    base: Option<String>,
    candidate: String,
    initial: bool,
) -> Result<Value, String> {
    let req = Request {
        protocol: PROTOCOL.into(),
        request_id: "scheduling-requirements".into(),
        host_root: root.into(),
        config_path: profile.into(),
        config_sha256: sha256(&fs::read(root.join(profile)).map_err(|e| e.to_string())?),
        base,
        candidate,
        initial,
        scope: None,
        observations: object!({}),
    };
    let mut reader = Reader::legacy();
    let i = inventory(
        &req,
        &mut reader,
        &mut Value::Null,
        &mut String::new(),
        false,
        false,
    )?;
    if !i.blocked.is_empty() {
        return Err(format!("blocked global obligations: {:?}", i.blocked));
    }
    Ok(
        object!({"required_units":units::required(i.p.units.as_ref().ok_or("scheduling requires registered units")?, &i.global_selected),
        "global_selected":i.global_selected,"changed_paths":i.paths,"selection_explanation":i.selection_explanation,"git_facts":reader.observation()}),
    )
}

pub fn collection_requirements(
    root: &Path,
    profile: &str,
    manifest: &str,
    base: Option<String>,
    candidate: String,
    initial: bool,
) -> Result<Value, String> {
    let req = Request {
        protocol: chrono_harness::units::PROTOCOL.into(),
        request_id: "collection-requirements".into(),
        host_root: root.into(),
        config_path: profile.into(),
        config_sha256: sha256(&fs::read(root.join(profile)).map_err(|e| e.to_string())?),
        base,
        candidate,
        initial,
        scope: Some(chrono_harness::units::Scope::Collect {
            manifest: manifest.into(),
        }),
        observations: object!({}),
    };
    let mut reader = Reader::legacy();
    let i = inventory(
        &req,
        &mut reader,
        &mut Value::Null,
        &mut String::new(),
        false,
        true,
    )?;
    if !i.blocked.is_empty() {
        return Err(format!("blocked global obligations: {:?}", i.blocked));
    }
    let required = units::required(
        i.p.units
            .as_ref()
            .ok_or("collection requires registered units")?,
        &i.global_selected,
    );
    clean(&reader, root, &req.candidate, &i.p)?;
    Ok(
        object!({"required_units":required,"global_selected":i.global_selected,"selection_explanation":i.selection_explanation,"git_facts":reader.observation()}),
    )
}

fn inventory(
    req: &Request,
    reader: &mut Reader,
    opening: &mut Value,
    scope: &mut String,
    admit_entry: bool,
    verify_checkout: bool,
) -> Result<Inventory, String> {
    if req.protocol != units::scope_protocol(req) || req.request_id.is_empty() {
        return Err("invalid request protocol/identity".into());
    }
    relative_path(&req.config_path)?;
    let root = &req.host_root;
    let bytes = fs::read(root.join(&req.config_path)).map_err(|e| e.to_string())?;
    if sha256(&bytes) != req.config_sha256 {
        return Err("config does not match candidate/request".into());
    }
    let config: CheckConfig = decode(&bytes)?;
    *scope = config.schema.clone();
    let p = policy(&config)?;
    if let Some(scope) = &req.scope {
        scope.report_path(&config)?;
    }
    full_oid(&req.candidate)?;
    if let Some(path) = &p.facts_config {
        *reader = Reader::for_config_observed(root, path).map_err(|failure| {
            *opening = failure.observation;
            failure.message
        })?;
        if !reader.is_bound() {
            return Err("scoped v2 requires a bound full-v3 facts_config".into());
        }
        reader.verify_config(root, &req.candidate)?;
    }
    verify_oid(reader, root, &req.candidate)?;
    if at(reader, root, &req.candidate, &req.config_path)? != bytes {
        return Err("config does not match candidate/request".into());
    }
    if verify_checkout {
        clean(reader, root, &req.candidate, &p)?;
    }
    let previous;
    let old = if req.initial {
        if req.base.is_some() {
            return Err("initial cannot have base".into());
        }
        if !reader.parents(root, &req.candidate)?.is_empty() {
            return Err("initial requires parentless commit".into());
        }
        previous = "none".to_owned();
        None
    } else {
        let base = req.base.as_deref().ok_or("base required")?;
        verify_oid(reader, root, base)?;
        let base_tree = tree(reader, root, base)?;
        let op = if base_tree.contains_key(&req.config_path) {
            let c: CheckConfig = decode(&at(reader, root, base, &req.config_path)?)?;
            let op = policy(&c)?;
            previous = c.schema;
            if op.filemap != p.filemap || op.projects != p.projects {
                return Err("registry path migration unsupported".into());
            }
            op
        } else {
            if p.adoption_base.as_deref() != Some(base) {
                return Err("base profile absent; explicit exact adoption_base required".into());
            }
            previous = "none".to_owned();
            // Adoption projects the explicitly supplied bindings onto the real base registry;
            // it never fabricates a base tree or executes historical operations.
            let registry = json(&at(reader, root, base, &p.projects)?)?;
            let names: BTreeSet<_> =
                chrono_judge_registration::execution::test_bindings(&registry)?
                    .into_keys()
                    .collect();
            let mut old_ops = BTreeSet::new();
            for collection in ["projects", "scripts"] {
                for entry in array(&registry, collection)? {
                    for action in entry["actions"]
                        .as_object()
                        .ok_or("actions missing")?
                        .values()
                    {
                        old_ops.insert(text(action, "operation")?.to_string());
                    }
                }
            }
            let mut adopted = p.clone();
            adopted
                .required_inputs
                .retain(|v| base_tree.contains_key(v));
            adopted
                .bindings
                .retain(|k, v| names.contains(k) && v.iter().all(|op| old_ops.contains(op)));
            adopted
        };
        Some(snapshot(reader, root, base, op)?)
    };
    let new = snapshot(reader, root, &req.candidate, p.clone())?;
    let (paths, mut selected, selection_explanation, affected) = ci_impact(old.as_ref(), &new);
    if new.registry["schema_version"] == 2 {
        chrono_judge_projects::pair_declarations(&new.registry, &affected)?;
    }
    let mut blocked = vec![];
    let mut removed = BTreeMap::new();
    let mut conversion = Value::Null;
    let mut reused = BTreeMap::new();
    let mut environment: BTreeMap<String, String> = std::env::vars().collect();
    environment.extend(p.environment.clone());
    #[cfg(unix)]
    environment.remove(chrono_harness::process_fds::ENV);
    let mut environment_policy = object!({"values":p.environment,"inherit":null});
    let mut declarations=object!(p.tools.iter().map(|(id,program)|object!({"id":id,"program":program,"version_argv":["--version"],"expected_version":null})).collect::<Vec<_>>());
    if req.base.is_none() {
        if let Some(config_path) = &p.registration_config {
            let values = reader.registry_values(root, &req.candidate, config_path)?;
            let identity = chrono_harness::facts::registry_identity(&values, config_path)?;
            let cfg = &values[&identity.effective_path];
            if cfg["schema_version"] == 4 {
                chrono_judge_registration::Registrations::load(&values, config_path)?;
                let c = chrono_harness::prepared::declaration(cfg)?;
                let mut argv = c.argv;
                if let Some(scope) = &req.scope {
                    match scope {
                        chrono_harness::units::Scope::Unit { unit } => {
                            argv.extend(["--unit".into(), unit.clone()])
                        }
                        chrono_harness::units::Scope::Collect { .. } => {
                            argv.push("--collect".into())
                        }
                    }
                }
                if admit_entry {
                    chrono_judge_routes::validate_invocation(
                        root,
                        &req.observations["entry"],
                        &argv,
                        &object!({}),
                    )?;
                }
                if admit_entry {
                    chrono_harness::prepared::validate_binding(
                        root,
                        cfg,
                        &req.config_path,
                        None,
                        &req.candidate,
                        req.initial,
                        &req.scope,
                        None,
                        &req.observations["preparation"],
                    )?;
                }
                declarations = cfg["tools"].clone();
                environment.clear();
                for key in cfg["environment"]["inherit"]
                    .as_array()
                    .ok_or("environment inherit missing")?
                {
                    let key = key.as_str().ok_or("environment key")?;
                    if let Ok(v) = std::env::var(key) {
                        environment.insert(key.into(), v);
                    }
                }
                for (key, v) in cfg["environment"]["values"]
                    .as_object()
                    .ok_or("environment values missing")?
                {
                    environment.insert(key.clone(), v.as_str().ok_or("environment value")?.into());
                }
                environment.extend(p.environment.clone());
                environment_policy = cfg["environment"].clone();
                for (key, v) in &p.environment {
                    environment_policy["values"][key] = object!(v);
                }
            }
        }
    }
    if let (Some(config_path), Some(base)) = (&p.registration_config, &req.base) {
        let a = reader.registry_values(root, base, config_path)?;
        let b = reader.registry_values(root, &req.candidate, config_path)?;
        let _a_identity = chrono_harness::facts::registry_identity(&a, config_path)?;
        let b_identity = chrono_harness::facts::registry_identity(&b, config_path)?;
        let b_config = b
            .get(&b_identity.effective_path)
            .ok_or("missing effective candidate registration config")?;
        if b_config["schema_version"] == 4 {
            chrono_judge_registration::Registrations::load(&b, config_path)?;
        }
        let mut template: Vec<String> =
            serde_json::from_value(b_config["canonical_check"]["argv"].clone())
                .map_err(|e| e.to_string())?;
        if let Some(scope) = &req.scope {
            if b_config["schema_version"] == 4 {
                match scope {
                    chrono_harness::units::Scope::Unit { unit } => {
                        template.extend(["--unit".into(), unit.clone()])
                    }
                    chrono_harness::units::Scope::Collect { .. } => {
                        template.push("--collect".into())
                    }
                }
            } else {
                template.extend(scope.argv());
            }
        }
        if admit_entry {
            chrono_judge_routes::validate_invocation(
                root,
                &req.observations["entry"],
                &template,
                &object!({"base":req.base,"candidate":req.candidate}),
            )?;
        }
        if admit_entry && b_config["schema_version"] == 4 {
            chrono_harness::prepared::validate_binding(
                root,
                b_config,
                &req.config_path,
                req.base.as_deref(),
                &req.candidate,
                req.initial,
                &req.scope,
                None,
                &req.observations["preparation"],
            )?;
        }
        environment.clear();
        for key in b_config["environment"]["inherit"]
            .as_array()
            .ok_or("environment inherit missing")?
        {
            let key = key.as_str().ok_or("environment key")?;
            if let Ok(value) = std::env::var(key) {
                environment.insert(key.into(), value);
            }
        }
        for (key, value) in b_config["environment"]["values"]
            .as_object()
            .ok_or("environment values missing")?
        {
            environment.insert(
                key.clone(),
                value.as_str().ok_or("environment value")?.into(),
            );
        }
        environment.extend(p.environment.clone());
        environment_policy = b_config["environment"].clone();
        for (key, value) in &p.environment {
            environment_policy["values"][key] = object!(value);
        }
        let (_, r, view) = if !verify_checkout {
            chrono_judge_registration::interpret_scheduling_with_reader(
                reader,
                root,
                base,
                &req.candidate,
                config_path,
                a,
                b,
            )?
        } else if reader.is_bound() {
            chrono_judge_registration::interpret_with_reader(
                reader,
                root,
                base,
                &req.candidate,
                config_path,
                a,
                b,
                &environment,
            )?
        } else {
            chrono_judge_registration::interpret(
                root,
                base,
                &req.candidate,
                config_path,
                a,
                b,
                &environment,
            )?
        };
        declarations = r.config()["tools"].clone();
        reused = chrono_judge_registration::reused_tools(&view)?;
        conversion = view["conversion"].clone();
        let replacements = chrono_judge_registration::replacements(&r)?;
        for test in selected.clone() {
            if !new.plans.contains_key(&test) {
                if let Some(replacement) = replacements.get(&test) {
                    removed.insert(test.clone(), replacement.clone());
                    selected.remove(&test);
                    selected.insert(replacement.clone());
                }
            }
        }
    }
    for test in &selected {
        if !new.plans.contains_key(test) {
            blocked.push(format!("affected binding removed: {test}"));
        }
        if let Some(old) = &old {
            if let Some(ops) = old.policy.bindings.get(test) {
                for op in ops {
                    if !new.operations.contains_key(op) {
                        blocked.push(format!("affected bound operation removed: {op}"));
                    }
                }
            }
        }
    }
    // Replaced obligations must preserve every formerly bound method in the replacement plan.
    for (test, replacement) in &removed {
        for op in old
            .as_ref()
            .and_then(|s| s.policy.bindings.get(test))
            .ok_or("historical obligation missing")?
        {
            if !new
                .plans
                .get(replacement)
                .is_some_and(|p| p.operations.contains(op))
            {
                blocked.push(format!("replacement omits old obligation {op}"));
            }
        }
    }
    let methods: BTreeMap<_, _> = new
        .operations
        .iter()
        .map(|(id, (owner, tool, argv))| {
            (
                id.clone(),
                vec![chrono_judge_registration::execution::Method {
                    owner: format!("project:{owner}"),
                    operation: id.clone(),
                    tool: tool.clone(),
                    argv: argv.clone(),
                }],
            )
        })
        .collect();
    let global_selected = selected.clone();
    if blocked.is_empty() && p.units.is_some() {
        // Validate the complete obligation graph before selecting any unit.
        chrono_judge_routes::order(&global_selected, &new.plans, &methods, &new.execute)?;
    }
    Ok(Inventory {
        config,
        p,
        new,
        previous,
        paths,
        selected,
        global_selected,
        selection_explanation,
        blocked,
        removed,
        conversion,
        reused,
        environment,
        environment_policy,
        declarations,
        methods,
    })
}
fn evaluate(
    req: &Request,
    reader: &mut Reader,
    opening: &mut Value,
    scope: &mut String,
) -> Result<Response, String> {
    let Inventory {
        config,
        p,
        new,
        previous,
        paths,
        mut selected,
        global_selected,
        selection_explanation,
        blocked,
        removed,
        conversion,
        reused,
        environment,
        environment_policy,
        declarations,
        methods,
    } = inventory(req, reader, opening, scope, true, true)?;
    let root = &req.host_root;
    if let Some(chrono_harness::units::Scope::Collect { manifest }) = &req.scope {
        if !blocked.is_empty() {
            return Err(format!("blocked global obligations: {blocked:?}"));
        }
        let mut response = collect::collect(
            req,
            manifest,
            &config,
            &new,
            &global_selected,
            &methods,
            &declarations,
            &environment_policy,
        )?;
        clean(reader, root, &req.candidate, &p)?;
        response.evidence["changed_paths"] = object!(paths);
        response.evidence["selection_explanation"] = selection_explanation;
        response.evidence["removed"] = object!(removed);
        return Ok(response);
    }
    if let Some(chrono_harness::units::Scope::Unit { unit }) = &req.scope {
        selected = units::selection(
            p.units.as_ref().ok_or("unit profile missing assignments")?,
            &global_selected,
            unit,
        )?;
    }
    let plan = if blocked.is_empty() {
        Some(chrono_judge_routes::prepare_scheduled(
            root,
            object!({"base":req.base,"candidate":req.candidate,"config":req.config_sha256,"run":req.request_id,"entry":req.observations["entry"]}),
            &selected,
            &new.plans,
            &methods,
            &new.execute,
            &declarations,
            environment,
            &reused,
            new.scheduling.clone(),
            false,
        )?)
    } else {
        None
    };
    let operations: Vec<_> = plan
        .as_ref()
        .into_iter()
        .flat_map(|p| p.operations.iter().map(|o| o.method.operation.clone()))
        .collect();
    let mut results = vec![CheckResult {
        id: "ci.inventory".into(),
        status: Status::Passed,
        cause: "fixed candidate and registered endpoint inventory validated".into(),
        exit_code: None,
    }];
    let mut executed = vec![];
    for reason in &blocked {
        results.push(CheckResult {
            id: format!("ci.blocked.{}", results.len()),
            status: Status::Blocked,
            cause: reason.clone(),
            exit_code: None,
        })
    }
    if let Some(plan) = &plan {
        let outcome = chrono_judge_projects::execute(plan)?;
        for result in outcome.executed.iter().chain(&outcome.blocked) {
            results.push(CheckResult {
                id: result.operation.clone(),
                status: match result.status.as_str() {
                    "passed" => Status::Passed,
                    "blocked" => Status::Blocked,
                    _ => Status::Failed,
                },
                cause: result
                    .error
                    .clone()
                    .unwrap_or_else(|| "registered command completed with matching receipt".into()),
                // Infrastructure/receipt failures can follow a successful child.
                // The summary has no failing subprocess exit in that case; the
                // original zero exit and diagnostic remain in the receipt below.
                exit_code: result.receipt.as_ref().and_then(|r| {
                    (result.status == "passed" || r.process.exit_code != 0)
                        .then_some(r.process.exit_code)
                }),
            });
            if let Some(receipt) = &result.receipt {
                executed.push(object!({
                    "operation": result.operation,
                    "process": process_projection(&receipt.process),
                    "receipt": receipt
                }));
            }
        }
        for (old, replacement) in &removed {
            if selected.contains(replacement)
                && outcome.tests.get(replacement).map(String::as_str) != Some("passed")
            {
                results.push(CheckResult {
                    id: format!("replacement:{old}"),
                    status: Status::Failed,
                    cause: "required replacement did not execute successfully".into(),
                    exit_code: None,
                });
            }
        }
    }
    let not_required: Vec<_> = new
        .policy
        .bindings
        .keys()
        .filter(|t| !global_selected.contains(*t))
        .cloned()
        .collect();
    for t in &not_required {
        results.push(CheckResult {
            id: t.clone(),
            status: Status::NotRequired,
            cause: "outside explicit DELTA execution closure".into(),
            exit_code: None,
        });
    }
    if let Err(e) = clean(reader, root, &req.candidate, &p) {
        results.push(CheckResult {
            id: "ci.post-snapshot".into(),
            status: Status::Failed,
            cause: e,
            exit_code: None,
        })
    }
    let failed = results
        .iter()
        .any(|r| matches!(r.status, Status::Failed | Status::Blocked));
    let mut response = Response {
        protocol: units::scope_protocol(req).into(),
        request_id: req.request_id.clone(),
        status: if failed {
            Status::Failed
        } else {
            Status::Passed
        },
        results,
        evidence: object!({"scope":config.schema,"mode":if req.initial{"initial-inventory"}else{"delta"},"base":req.base,"candidate":req.candidate,"previous_enforcement":previous,"changed_paths":paths,"selection_explanation":selection_explanation,"selected":selected,"operations":operations,"plan":plan,"conversion":conversion,"removed":removed,"executed":executed,"not_required":not_required,"blocked":blocked,"input_closure":"incomplete: ambient toolchain, SDK, environment and external inputs are not fully enumerated","parity":"unestablished"}),
    };
    units::decorate(req, &new, &global_selected, &mut response.evidence);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(stdout_bytes: Vec<u8>, stderr_bytes: Vec<u8>) -> ProcessResult {
        ProcessResult {
            argv: vec!["tool".into()],
            cwd: "/tmp/host".into(),
            environment: BTreeMap::new(),
            environment_digest: "environment".into(),
            ownership_fds: None,
            stdin_sha256: "stdin".into(),
            stdout_sha256: "stdout".into(),
            stderr_sha256: "stderr".into(),
            stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
            stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
            stdout_bytes,
            stderr_bytes,
            failure: None,
            exit_code: 0,
            executable: "/bin/tool".into(),
            sha256: "tool".into(),
        }
    }

    #[test]
    fn process_projection_retains_small_streams() {
        let projected = process_projection(&process(vec![b'a'; 3], vec![b'b'; 2]));
        assert_eq!(projected["stdout_bytes"], serde_json::json!([97, 97, 97]));
        assert!(projected.get("stdout_omitted").is_none());
    }

    #[test]
    fn process_projection_keeps_large_streams_in_receipt_only() {
        let projected = process_projection(&process(vec![b'a'; 4097], vec![]));
        assert!(projected.get("stdout_bytes").is_none());
        assert!(projected.get("stderr_bytes").is_none());
        assert_eq!(projected["stdout_omitted"], serde_json::json!(true));
        assert_eq!(projected["stderr_omitted"], serde_json::json!(true));
        assert_eq!(projected["stdout_sha256"], serde_json::json!("stdout"));
    }
}
