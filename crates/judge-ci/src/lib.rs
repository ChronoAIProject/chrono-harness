//! Explicit CI policy: fixed Git snapshots, registered old/new impact graph, candidate operations.
use chrono_harness::{
    CheckConfig, CheckResult, PROTOCOL, Request, Response, Status, decode, json, relative_path,
    sha256,
};
use chrono_judge_filemap::graph::{self, Edge, EdgeKind, Seed};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as object};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
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
    files: BTreeMap<String, Value>,
    projects: BTreeMap<String, Value>,
    operations: BTreeMap<String, (String, String, Vec<String>)>,
    edges: BTreeSet<Edge>,
    tree: BTreeMap<String, (String, String)>,
    policy: Policy,
    plans: BTreeMap<String, chrono_judge_registration::execution::Plan>,
    execute: BTreeMap<String, String>,
}
use chrono_harness::facts::{git, utf8};
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
fn verify_oid(root: &Path, oid: &str) -> Result<(), String> {
    full_oid(oid)?;
    if utf8(git(
        root,
        &["rev-parse", "--verify", &format!("{oid}^{{commit}}")],
    )?)?
    .trim()
        != oid
    {
        return Err("OID is not a commit".into());
    }
    Ok(())
}
fn at(root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
    relative_path(path)?;
    git(root, &["show", &format!("{oid}:{path}")])
}
fn tree(root: &Path, oid: &str) -> Result<BTreeMap<String, (String, String)>, String> {
    let raw = git(root, &["ls-tree", "-rz", "--full-tree", oid])?;
    let mut out = BTreeMap::new();
    for row in raw.split(|b| *b == 0).filter(|v| !v.is_empty()) {
        let s = std::str::from_utf8(row).map_err(|_| "non UTF-8 tree path")?;
        let (meta, path) = s.split_once('\t').ok_or("bad tree record")?;
        let fields: Vec<_> = meta.split(' ').collect();
        if fields.len() != 3
            || fields[1] != "blob"
            || !matches!(fields[0], "100644" | "100755" | "120000")
        {
            return Err(format!("unsupported tree entry {path}: {meta}"));
        }
        relative_path(path)?;
        out.insert(path.into(), (fields[0].into(), fields[2].into()));
    }
    Ok(out)
}
fn policy(c: &CheckConfig) -> Result<Policy, String> {
    if c.schema != "chrono-ci-check/v1" {
        return Err("unsupported CI profile".into());
    }
    let p: Policy = serde_json::from_value(c.policy.clone()).map_err(|e| e.to_string())?;
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
    Ok(p)
}
fn snapshot(root: &Path, oid: &str, mut p: Policy) -> Result<Snapshot, String> {
    let fm = json(&at(root, oid, &p.filemap)?)?;
    let pr = json(&at(root, oid, &p.projects)?)?;
    if ![object!(1), object!(2)].contains(&fm["schema_version"]) || pr["schema_version"] != 1 {
        return Err("unsupported registry version".into());
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
    let mut execute = BTreeMap::new();
    let owners: BTreeSet<_> = strings(&pr["owners"])?.into_iter().collect();
    if owners.is_empty() {
        return Err("empty owner registry".into());
    }
    let mut projects = BTreeMap::new();
    let mut ops = BTreeMap::new();
    let mut nodes = BTreeSet::new();
    if let Some(config) = &p.registration_config {
        let config = json(&at(root, oid, config)?)?;
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
            if let Some(op) = entry["actions"]["execute"]["operation"].as_str() {
                execute.insert(format!("test:{name}"), op.into());
            }
            nodes.insert(format!("script:{name}"));
            nodes.insert(format!("project:{name}"));
            nodes.insert(format!("test:{name}"));
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
    let tree = tree(root, oid)?;
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
                if utf8(at(root, oid, path)?)? != *target {
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
    Ok(Snapshot {
        files,
        projects,
        operations: ops,
        edges,
        tree,
        policy: p,
        plans,
        execute,
    })
}
fn clean(root: &Path, candidate: &str, p: &Policy) -> Result<(), String> {
    if utf8(git(root, &["rev-parse", "HEAD"])?)?.trim() != candidate {
        return Err("checkout HEAD does not equal candidate".into());
    }
    // -v lowercases tags for assume-unchanged; S/s marks skip-worktree.
    // Paths follow the two-byte tag prefix and end at NUL, never whitespace.
    for entry in chrono_harness::facts::index_flags(root)? {
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
    if !git(root, &["diff", "--cached", "--raw", "-z", candidate, "--"])?.is_empty()
        || !git(root, &["diff", "--raw", "-z", "--"])?.is_empty()
    {
        return Err("dirty tracked candidate inputs/index".into());
    }
    for path in git(root, &["ls-files", "--others", "-z"])?
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
    {
        let path = std::str::from_utf8(path).map_err(|_| "non UTF-8 untracked path")?;
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
) -> (BTreeSet<String>, BTreeSet<String>, Value) {
    let paths = if let Some(old) = old {
        changed(&old.tree, &new.tree)
    } else {
        new.tree.keys().cloned().collect()
    };
    let mut seeds: BTreeSet<_> = paths.iter().map(|p| format!("file:{p}")).collect();
    let mut extras: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(old) = old {
        for path in changed(&old.files, &new.files) {
            seeds.insert(format!("file:{path}"));
        }
        for p in changed(&old.projects, &new.projects) {
            seeds.insert(format!("project:{p}"));
        }
        for op in changed(&old.operations, &new.operations) {
            for s in [old, new] {
                if let Some((owner, _, _)) = s.operations.get(&op) {
                    seeds.insert(format!("project:{owner}"));
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
        for t in changed(&old.policy.bindings, &new.policy.bindings) {
            extras
                .entry(t)
                .or_default()
                .insert("changed binding".into());
        }
        let changed_tools = changed(&old.policy.tools, &new.policy.tools);
        if !changed_tools.is_empty()
            || old.policy.environment != new.policy.environment
            || old.policy.operation_timeout_seconds != new.policy.operation_timeout_seconds
            || old.policy.operation_output_limit_bytes != new.policy.operation_output_limit_bytes
        {
            for s in [old, new] {
                for (test, ops) in &s.policy.bindings {
                    if old.policy.environment != new.policy.environment
                        || old.policy.operation_timeout_seconds
                            != new.policy.operation_timeout_seconds
                        || old.policy.operation_output_limit_bytes
                            != new.policy.operation_output_limit_bytes
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
                        if old.policy.operation_timeout_seconds
                            != new.policy.operation_timeout_seconds
                        {
                            reasons.insert("changed operation timeout".into());
                        }
                        if old.policy.operation_output_limit_bytes
                            != new.policy.operation_output_limit_bytes
                        {
                            reasons.insert("changed operation output limit".into());
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
    let explanation = object!({"scope":"chrono-ci-check/v1-adapter", "edges":edges, "seeds":seed_nodes, "seed_causes":seeds, "closure":closure, "extra_selections":extras, "legacy_only_selections":legacy_only});
    (paths, selected, explanation)
}

pub fn judge(req: &Request) -> Response {
    match evaluate(req) {
        Ok(r) => r,
        Err(e) => Response {
            protocol: PROTOCOL.into(),
            request_id: req.request_id.clone(),
            status: Status::Failed,
            results: vec![CheckResult {
                id: "ci.inventory".into(),
                status: Status::Failed,
                cause: e,
                exit_code: None,
            }],
            evidence: object!({"scope":"chrono-ci-check/v1","candidate":req.candidate,"base":req.base,"selected":[],"executed":[],"blocked":["selection or snapshot validation failed"],"parity":"unestablished"}),
        },
    }
}
fn evaluate(req: &Request) -> Result<Response, String> {
    if req.protocol != PROTOCOL || req.request_id.is_empty() {
        return Err("invalid request protocol/identity".into());
    }
    relative_path(&req.config_path)?;
    let root = &req.host_root;
    verify_oid(root, &req.candidate)?;
    let bytes = fs::read(root.join(&req.config_path)).map_err(|e| e.to_string())?;
    if sha256(&bytes) != req.config_sha256 || at(root, &req.candidate, &req.config_path)? != bytes {
        return Err("config does not match candidate/request".into());
    }
    let config: CheckConfig = decode(&bytes)?;
    let p = policy(&config)?;
    clean(root, &req.candidate, &p)?;
    let mut previous = "chrono-ci-check/v1";
    let old = if req.initial {
        if req.base.is_some() {
            return Err("initial cannot have base".into());
        }
        if !chrono_harness::facts::parents(root, &req.candidate)?.is_empty() {
            return Err("initial requires parentless commit".into());
        }
        previous = "none";
        None
    } else {
        let base = req.base.as_deref().ok_or("base required")?;
        verify_oid(root, base)?;
        let base_tree = tree(root, base)?;
        let op = if base_tree.contains_key(&req.config_path) {
            let c: CheckConfig = decode(&at(root, base, &req.config_path)?)?;
            let op = policy(&c)?;
            if op.filemap != p.filemap || op.projects != p.projects {
                return Err("registry path migration unsupported".into());
            }
            op
        } else {
            if p.adoption_base.as_deref() != Some(base) {
                return Err("base profile absent; explicit exact adoption_base required".into());
            }
            previous = "none";
            // Adoption projects the explicitly supplied bindings onto the real base registry;
            // it never fabricates a base tree or executes historical operations.
            let registry = json(&at(root, base, &p.projects)?)?;
            let mut names = BTreeSet::new();
            let mut old_ops = BTreeSet::new();
            for collection in ["projects", "scripts"] {
                for entry in array(&registry, collection)? {
                    names.insert(format!("test:{}", text(entry, "id")?));
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
        Some(snapshot(root, base, op)?)
    };
    let new = snapshot(root, &req.candidate, p.clone())?;
    let (paths, mut selected, selection_explanation) = ci_impact(old.as_ref(), &new);
    let mut blocked = vec![];
    let mut removed = BTreeMap::new();
    let mut conversion = Value::Null;
    let mut reused = BTreeMap::new();
    let mut environment: BTreeMap<String, String> = std::env::vars().collect();
    environment.extend(p.environment.clone());
    let mut declarations=object!(p.tools.iter().map(|(id,program)|object!({"id":id,"program":program,"version_argv":["--version"],"expected_version":null})).collect::<Vec<_>>());
    if let (Some(config_path), Some(base)) = (&p.registration_config, &req.base) {
        let a = chrono_harness::facts::registry_values(root, base, config_path)?;
        let b = chrono_harness::facts::registry_values(root, &req.candidate, config_path)?;
        let template: Vec<String> =
            serde_json::from_value(b[config_path]["canonical_check"]["argv"].clone())
                .map_err(|e| e.to_string())?;
        chrono_judge_routes::validate_invocation(
            root,
            &req.observations["entry"],
            &template,
            &object!({"base":req.base,"candidate":req.candidate}),
        )?;
        environment.clear();
        for key in b[config_path]["environment"]["inherit"]
            .as_array()
            .ok_or("environment inherit missing")?
        {
            let key = key.as_str().ok_or("environment key")?;
            if let Ok(value) = std::env::var(key) {
                environment.insert(key.into(), value);
            }
        }
        for (key, value) in b[config_path]["environment"]["values"]
            .as_object()
            .ok_or("environment values missing")?
        {
            environment.insert(
                key.clone(),
                value.as_str().ok_or("environment value")?.into(),
            );
        }
        environment.extend(p.environment.clone());
        let (_, r, view) = chrono_judge_registration::interpret(
            root,
            base,
            &req.candidate,
            config_path,
            a,
            b,
            &environment,
        )?;
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
    let plan = if blocked.is_empty() {
        Some(chrono_judge_routes::prepare_scoped(
            root,
            object!({"base":req.base,"candidate":req.candidate,"config":req.config_sha256,"run":req.request_id,"entry":req.observations["entry"]}),
            &selected,
            &new.plans,
            &methods,
            &new.execute,
            &declarations,
            environment,
            &reused,
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
                exit_code: result.receipt.as_ref().map(|r| r.process.exit_code),
            });
            if let Some(receipt) = &result.receipt {
                executed.push(object!({"operation":result.operation,"process":receipt.process,"receipt":receipt}));
            }
        }
        for (old, replacement) in &removed {
            if outcome.tests.get(replacement).map(String::as_str) != Some("passed") {
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
        .filter(|t| !selected.contains(*t))
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
    if let Err(e) = clean(root, &req.candidate, &p) {
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
    Ok(Response {
        protocol: PROTOCOL.into(),
        request_id: req.request_id.clone(),
        status: if failed {
            Status::Failed
        } else {
            Status::Passed
        },
        results,
        evidence: object!({"scope":"chrono-ci-check/v1","mode":if req.initial{"initial-inventory"}else{"delta"},"base":req.base,"candidate":req.candidate,"previous_enforcement":previous,"changed_paths":paths,"selection_explanation":selection_explanation,"selected":selected,"operations":operations,"plan":plan,"conversion":conversion,"removed":removed,"executed":executed,"not_required":not_required,"blocked":blocked,"input_closure":"incomplete: ambient toolchain, SDK, environment and external inputs are not fully enumerated","parity":"unestablished"}),
    })
}
