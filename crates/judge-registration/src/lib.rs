//! Registration owns full-format schema, snapshot and affected-reference admissibility.
pub mod execution;
pub mod inputs;
mod registrations;
mod transition;
pub use transition::{
    ambiguity_repaired, downstream_validator, interpret, replacements, retirement_requests,
    reused_tools, views,
};
mod schema;
use chrono_harness::{
    facts, json, no_symlink_parents, sha256,
    wire::{self, Finding, Request, Response, Status},
};
pub use registrations::{NodeDefinition, NodeKind, NodeView, Registrations};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
type Result<T> = std::result::Result<T, String>;
fn read(path: &Path) -> Result<Value> {
    json(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
}
fn issue(
    r: &mut Response,
    code: &str,
    message: impl Into<String>,
    reference: impl Into<String>,
    error: bool,
) {
    r.status = r
        .status
        .clone()
        .max(if error { Status::Error } else { Status::Fail });
    r.findings.push(Finding {
        code: code.into(),
        level: "error".into(),
        message: message.into(),
        delta_refs: vec![reference.into()],
        causes: vec![],
    });
}
fn rows(v: &Value, key: &str, id: &str) -> BTreeMap<String, Value> {
    v[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r[id].as_str().map(|s| (s.into(), r.clone())))
        .collect()
}
fn strings(v: &Value) -> BTreeSet<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
pub fn judge(req: &Request) -> Response {
    let mut r = req.response(Status::Pass);
    if let Err(e) = evaluate(req, &mut r) {
        issue(&mut r, "E_INPUT", e, "/request", true);
    }
    r
}
fn evaluate(req: &Request, r: &mut Response) -> Result<()> {
    req.validate()?;
    let root = &req.candidate.root;
    let actual_root =
        fs::canonicalize(facts::utf8(facts::git(root, &["rev-parse", "--show-toplevel"])?)?.trim())
            .map_err(|e| e.to_string())?;
    if actual_root != *root {
        issue(
            r,
            "E_SNAPSHOT_DIRTY",
            "candidate.root must be actual checkout root",
            "/candidate/root",
            false,
        );
        return Ok(());
    }
    if facts::verify_oid(root, &req.base.commit)? != req.base.tree
        || facts::verify_oid(root, &req.candidate.commit)? != req.candidate.tree
    {
        issue(
            r,
            "E_IDENTITY",
            "tree/commit disagreement",
            "/candidate",
            true,
        );
        return Ok(());
    }
    let bt = facts::tree(root, &req.base.commit)?;
    let ct = facts::tree(root, &req.candidate.commit)?;
    if req.delta != facts::delta(&bt, &ct) {
        issue(
            r,
            "E_IDENTITY",
            "DELTA does not match fixed endpoints",
            "/delta",
            true,
        );
        return Ok(());
    }
    let observed = facts::checkout(root, &req.candidate.commit)?;
    // Newly written state artifacts may differ since acquisition; compare all governed dirt below.
    let bv = facts::registry_values(root, &req.base.commit, &req.config_path)?;
    let cv = facts::registry_values(root, &req.candidate.commit, &req.config_path)?;
    if facts::registry_digest(&bv, &cv)? != req.registries.digest
        || req.registries.base != req.base.root.join(".chrono-harness")
        || req.registries.candidate != root.join(".chrono-harness")
    {
        issue(
            r,
            "E_IDENTITY",
            "registry digest/root mismatch",
            "/registries",
            true,
        );
        return Ok(());
    }
    for (p, v) in &bv {
        let p = no_symlink_parents(&req.base.root, p)?;
        if read(&p)? != *v {
            issue(
                r,
                "E_IDENTITY",
                "base snapshot differs from fixed Git data",
                "/base",
                true,
            );
            return Ok(());
        }
    }
    for p in cv.keys() {
        let disk = fs::read(no_symlink_parents(root, p)?).map_err(|e| e.to_string())?;
        if disk != facts::blob(root, &req.candidate.commit, p)? {
            issue(
                r,
                "E_CONFIG_MISMATCH",
                format!("checkout registry differs from candidate: {p}"),
                "/registries",
                true,
            );
            return Ok(());
        }
    }
    let context = read(&req.context.path)?;
    if wire::digest(&context)? != req.context.sha256 {
        issue(
            r,
            "E_CONTEXT_MISMATCH",
            "context digest mismatch",
            "/context",
            true,
        );
        return Ok(());
    }
    let context_v2 = context["schema_version"] == 2;
    schema::object(
        &context,
        &[
            "schema_version",
            "base",
            "candidate",
            "dev_tip",
            "branch_ref",
            "fork_point",
            "branch_started_at",
            "observed_at",
            "operation",
            "integration_evidence",
        ],
        if context_v2 {
            &["retained_inputs", "run_kind"]
        } else {
            &["retained_inputs"]
        },
    )?;
    if (context["schema_version"] != 1 && !context_v2)
        || context["base"] != req.base.commit
        || context["candidate"] != req.candidate.commit
        || context["dev_tip"] != req.base.commit
        || context["operation"] != "validate.delta"
    {
        issue(
            r,
            "E_CONTEXT_MISMATCH",
            "context/CLI identities disagree",
            "/context",
            true,
        );
        return Ok(());
    }
    if context_v2
        && !matches!(
            context["run_kind"].as_str(),
            Some("integration" | "delivery")
        )
    {
        return Err("context v2 requires integration/delivery run_kind".into());
    }
    schema::string(&context["branch_ref"])?;
    facts::verify_oid(root, schema::string(&context["fork_point"])?)?;
    for k in ["branch_started_at", "observed_at"] {
        let dt = time::OffsetDateTime::parse(
            schema::string(&context[k])?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
        if dt.offset() != time::UtcOffset::UTC {
            return Err("context time must be UTC RFC3339".into());
        }
    }
    if !context["integration_evidence"].is_null()
        && !context["integration_evidence"]
            .as_str()
            .is_some_and(wire::is_digest)
    {
        return Err("invalid integration evidence digest".into());
    }
    let (old, new, view) = match views(req) {
        Ok(v) => v,
        Err(e) => {
            issue(r, "E_SCHEMA", e, "/registries", true);
            return Ok(());
        }
    };
    let artifacts: Vec<_> = new.config["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].as_str().unwrap())
        .collect();
    let nonartifact = |paths: &Vec<String>| -> Vec<String> {
        paths
            .iter()
            .filter(|p| !artifacts.iter().any(|a| p.starts_with(a)))
            .cloned()
            .collect()
    };
    if observed.head != req.checkout.head
        || observed.tracked != req.checkout.tracked
        || nonartifact(&observed.untracked) != nonartifact(&req.checkout.untracked)
        || observed.index_flags != req.checkout.index_flags
    {
        issue(
            r,
            "E_IDENTITY",
            "checkout observations changed since request",
            "/checkout",
            true,
        );
    }
    if observed.head != req.candidate.commit
        || !observed.tracked.is_empty()
        || !nonartifact(&observed.untracked).is_empty()
    {
        issue(
            r,
            "E_SNAPSHOT_DIRTY",
            format!(
                "head={}, tracked={:?}, nonartifact={:?}",
                observed.head,
                observed.tracked,
                nonartifact(&observed.untracked)
            ),
            "/checkout",
            false,
        );
    }
    for flag in &observed.index_flags {
        if flag.tag.as_bytes()[0].is_ascii_lowercase() || flag.tag.eq_ignore_ascii_case("S") {
            issue(
                r,
                "E_INPUT_UNSUPPORTED",
                format!("unsupported index flag {}: {}", flag.tag, flag.path),
                "/checkout/index_flags",
                true,
            );
        }
    }
    let unsupported: Vec<_> = ct
        .iter()
        .filter(|(_, e)| {
            e.kind != "blob" || !matches!(e.mode.as_str(), "100644" | "100755" | "120000")
        })
        .map(|(path, _)| path)
        .collect();
    if !unsupported.is_empty() {
        issue(
            r,
            "E_INPUT_UNSUPPORTED",
            format!("unsupported entries: {:?}", unsupported),
            "/candidate/tree",
            true,
        );
    }
    readiness(req, &old, &new, r)?;
    references(req, &old, &new, &bt, &ct, r)?;
    if r.status == Status::Pass {
        r.outputs.insert("registration_view".into(), view);
        r.outputs.insert("registration".into(),value!({"scope":"registration-only","base":req.base.commit,"candidate":req.candidate.commit,"registry_digest":req.registries.digest,"input_closure":"declared-complete","completeness_proven":false,"previous_enforcement":if old.config["status"]=="proposed"{"none"}else{"enabled"}}));
    }
    Ok(())
}
fn readiness(
    req: &Request,
    old: &Registrations,
    n: &Registrations,
    r: &mut Response,
) -> Result<()> {
    for (name, v) in [
        ("config", &n.config),
        ("judges", &n.judges),
        ("projects", &n.projects),
        ("filemap", &n.filemap),
        ("workflow", &n.workflow),
    ] {
        if v["status"] != "active" {
            issue(
                r,
                "E_ACTIVATION",
                format!("{name} is proposed"),
                format!("/registries/{name}/status"),
                true,
            );
        }
    }
    if n.config["enforcement"] != "enabled"
        || n.config["input_closure"]["status"] != "declared-complete"
        || !n.config["input_closure"]["unresolved"]
            .as_array()
            .unwrap()
            .is_empty()
    {
        issue(
            r,
            "E_EVIDENCE_UNRESOLVED",
            "activation requires enabled enforcement and declared-complete input closure without unresolved inputs",
            "/config/input_closure",
            true,
        );
    }
    let root = &req.candidate.root;
    let runner = &n.config["runner"];
    if fs::read(&req.runner.path)
        .ok()
        .map(|bytes| sha256(&bytes))
        .as_deref()
        != Some(&req.runner.sha256)
        || runner["sha256"] != req.runner.sha256
        || runner["version"] != req.runner.version
        || fs::canonicalize(root.join(runner["path"].as_str().unwrap())).ok()
            != fs::canonicalize(&req.runner.path).ok()
    {
        issue(
            r,
            "E_EXECUTABLE_BINDING",
            "runner binding differs from actual process",
            "/config/runner",
            true,
        );
    }
    for j in n.judges["judges"].as_array().unwrap() {
        let path = no_symlink_parents(root, j["executable"].as_str().unwrap())?;
        if j["sha256"].is_null()
            || fs::read(&path).ok().map(|v| sha256(&v)).as_deref() != j["sha256"].as_str()
        {
            issue(
                r,
                "E_EXECUTABLE_BINDING",
                format!("missing/mismatched candidate executable {}", j["id"]),
                "/judges",
                true,
            );
        }
    }
    for t in n.config["tools"].as_array().unwrap() {
        if t["expected_version"].is_null() {
            issue(
                r,
                "E_INPUT_UNDECLARED",
                format!("tool {} has no expected version", t["id"]),
                "/config/tools",
                true,
            );
        }
    }
    match inputs::validate(req, old, n) {
        Ok(evidence) => {
            if r.status == Status::Pass && !evidence.is_null() {
                r.outputs.insert("effective_inputs".into(), evidence);
            }
        }
        Err(e) => issue(
            r,
            "E_EVIDENCE_UNRESOLVED",
            e,
            "/config/environment/inputs",
            true,
        ),
    }
    Ok(())
}
fn references(
    req: &Request,
    old: &Registrations,
    new: &Registrations,
    _bt: &facts::Tree,
    ct: &facts::Tree,
    r: &mut Response,
) -> Result<()> {
    let a = rows(&old.filemap, "files", "path");
    let b = rows(&new.filemap, "files", "path");
    let nodes = new.nodes();
    let old_nodes = old.nodes();
    let changed_paths: BTreeSet<_> = req.delta.iter().map(|d| d.path.clone()).collect();
    for d in &req.delta {
        if (d.kind == "D" && !a.contains_key(&d.path))
            || (d.kind != "D" && !b.contains_key(&d.path))
        {
            issue(
                r,
                "E_UNREGISTERED",
                format!("{} lacks endpoint registration", d.path),
                d.path.clone(),
                false,
            );
        }
        if d.kind != "A" && !a.contains_key(&d.path) && a.get(&d.path) == b.get(&d.path) {
            issue(
                r,
                "E_UNREGISTERED",
                "modified input lacks base registration or explicit registration addition",
                d.path.clone(),
                false,
            );
        }
    }
    let owners = strings(&new.projects["owners"]);
    let edge_check = |edge: &Value, reference: &str, r: &mut Response| {
        for k in ["from", "to"] {
            if let Some(id) = edge[k].as_str() {
                if !nodes.contains(id) {
                    issue(
                        r,
                        "E_DANGLING_EDGE",
                        format!("missing node {id}"),
                        reference,
                        false,
                    );
                }
            }
        }
    };
    for (p, f) in &b {
        let removed_target = f["edges"].as_array().unwrap().iter().any(|e| {
            e["to"]
                .as_str()
                .is_some_and(|s| old_nodes.contains(s) && !nodes.contains(s))
        });
        let owner_removed = !owners.contains(f["owner"].as_str().unwrap())
            && strings(&old.projects["owners"]).contains(f["owner"].as_str().unwrap());
        let cost_removed = new.filemap["cost_models"]
            .get(f["cost"].as_str().unwrap())
            .is_none()
            && old.filemap["cost_models"]
                .get(f["cost"].as_str().unwrap())
                .is_some();
        let projection_removed = f.get("projection").is_some_and(|pr| {
            pr["sources"].as_array().unwrap().iter().any(|v| {
                a.contains_key(v.as_str().unwrap()) && !b.contains_key(v.as_str().unwrap())
            }) || (old_nodes.contains(pr["producer"].as_str().unwrap())
                && !nodes.contains(pr["producer"].as_str().unwrap()))
        });
        let affected = changed_paths.contains(p)
            || a.get(p) != Some(f)
            || removed_target
            || owner_removed
            || cost_removed
            || projection_removed;
        if !affected {
            continue;
        }
        let reference = format!(
            "/filemap/files/{}",
            new.filemap["files"]
                .as_array()
                .unwrap()
                .iter()
                .position(|row| row["path"] == *p)
                .unwrap()
        );
        if !owners.contains(f["owner"].as_str().unwrap()) {
            issue(
                r,
                "E_REFERENCE",
                format!("unknown owner {}", f["owner"]),
                &reference,
                false,
            );
        }
        if new.filemap["cost_models"]
            .get(f["cost"].as_str().unwrap())
            .is_none()
        {
            issue(r, "E_REFERENCE", "unknown cost", &reference, false);
        }
        if !ct.contains_key(p) {
            issue(
                r,
                "E_REFERENCE",
                format!("registered file missing from candidate: {p}"),
                &reference,
                false,
            );
        }
        for e in f["edges"].as_array().unwrap() {
            edge_check(e, &reference, r);
        }
        if let Some(projection) = f.get("projection") {
            for src in projection["sources"].as_array().unwrap() {
                if !b.contains_key(src.as_str().unwrap()) {
                    issue(
                        r,
                        "E_REFERENCE",
                        "unregistered projection source",
                        &reference,
                        false,
                    );
                }
            }
            if !nodes.contains(projection["producer"].as_str().unwrap()) {
                issue(
                    r,
                    "E_REFERENCE",
                    "unknown projection producer",
                    &reference,
                    false,
                );
            }
        }
        if let Some(e) = ct.get(p) {
            if e.mode == "120000" {
                let actual =
                    facts::utf8(facts::blob(&req.candidate.root, &req.candidate.commit, p)?)?;
                if f["symlink"].as_str() != Some(actual.as_str()) {
                    issue(
                        r,
                        "E_REFERENCE",
                        "symlink literal differs from registration",
                        &reference,
                        false,
                    );
                }
            } else if f.get("symlink").is_some() {
                issue(
                    r,
                    "E_REFERENCE",
                    "registered symlink is ordinary file",
                    &reference,
                    false,
                );
            }
        }
    }
    for e in new.filemap["project_edges"].as_array().unwrap() {
        let added = !old.filemap["project_edges"].as_array().unwrap().contains(e);
        let removed_target = ["from", "to"].iter().any(|k| {
            e[k].as_str()
                .is_some_and(|s| old_nodes.contains(s) && !nodes.contains(s))
        });
        if added || removed_target {
            edge_check(e, "/filemap/project_edges", r);
        }
    }
    for key in ["projects", "scripts"] {
        let before = rows(&old.projects, key, "id");
        let after = rows(&new.projects, key, "id");
        for (id, p) in &after {
            let tools = strings_ids(&new.config["tools"]);
            let old_tools = strings_ids(&old.config["tools"]);
            let removed_tool = p["actions"].as_object().unwrap().values().any(|a| {
                a["tool"]
                    .as_str()
                    .is_some_and(|s| old_tools.contains(s) && !tools.contains(s))
            });
            let removed_pair = ["test_project", "tests_for", "test_script"]
                .iter()
                .any(|k| {
                    p[k].as_str()
                        .is_some_and(|s| before.contains_key(s) && !after.contains_key(s))
                });
            let affected_input = ["manifest", "lockfile", "path"].iter().any(|k| {
                p[k].as_str().is_some_and(|s| {
                    changed_paths.contains(s) || (a.contains_key(s) && !b.contains_key(s))
                })
            });
            if before.get(id) == Some(p)
                && !removed_tool
                && !removed_pair
                && !affected_input
                && !(strings(&old.projects["owners"]).contains(id) && !owners.contains(id))
            {
                continue;
            }
            if !owners.contains(id) {
                issue(
                    r,
                    "E_REFERENCE",
                    format!("project/script owner {id} is not declared"),
                    "/projects/owners",
                    false,
                );
            }
            let reference = format!(
                "/projects/{key}/{}",
                new.projects[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .position(|row| row["id"] == *id)
                    .unwrap()
            );
            for field in if key == "projects" {
                vec!["manifest", "lockfile"]
            } else {
                vec!["path"]
            } {
                let path = p[field].as_str().unwrap();
                if !ct.contains_key(path) || !b.contains_key(path) {
                    issue(
                        r,
                        "E_REFERENCE",
                        format!("missing/unregistered {field}: {path}"),
                        &reference,
                        false,
                    );
                }
            }
            for field in ["test_project", "tests_for", "test_script"] {
                if let Some(target) = p[field].as_str() {
                    if !after.contains_key(target) {
                        issue(
                            r,
                            "E_REFERENCE",
                            format!("missing {field}: {target}"),
                            &reference,
                            false,
                        );
                    }
                }
            }
            for action in p["actions"].as_object().unwrap().values() {
                if !tools.contains(action["tool"].as_str().unwrap()) {
                    issue(
                        r,
                        "E_REFERENCE",
                        format!(
                            "undeclared tool {} for {}",
                            action["tool"], action["operation"]
                        ),
                        &reference,
                        false,
                    );
                }
            }
        }
    }
    if let Some(plans) = new
        .filemap
        .get("execution_plans")
        .and_then(Value::as_object)
    {
        let methods = execution::methods(new.projects())?;
        let old_methods = execution::methods(old.projects())?;
        for (test, plan) in plans {
            let removed_method = plan["operations"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .any(|op| old_methods.contains_key(op) && !methods.contains_key(op));
            if old.filemap.get("execution_plans").and_then(|p| p.get(test)) == Some(plan)
                && !removed_method
            {
                continue;
            }
            if !nodes.contains(test) {
                issue(
                    r,
                    "E_REFERENCE",
                    format!("execution plan test missing: {test}"),
                    "/filemap/execution_plans",
                    false,
                );
            }
            for op in plan["operations"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
            {
                if !methods.contains_key(op) {
                    issue(
                        r,
                        "E_REFERENCE",
                        format!("execution plan operation missing: {op}"),
                        "/filemap/execution_plans",
                        false,
                    );
                }
            }
        }
    }
    let judges = rows(&new.judges, "judges", "id");
    if !judges.contains_key(new.judges["migration_validator"].as_str().unwrap()) {
        issue(
            r,
            "E_REFERENCE",
            "missing migration validator",
            "/judges/migration_validator",
            false,
        );
    }
    for cost in new.filemap["test_costs"].as_array().unwrap() {
        let test = format!("test:{}", cost["test"].as_str().unwrap());
        let name = cost["cost"].as_str().unwrap();
        let affected = !old.filemap["test_costs"].as_array().unwrap().contains(cost)
            || (old_nodes.contains(&test) && !nodes.contains(&test))
            || (old.filemap["cost_models"].get(name).is_some()
                && new.filemap["cost_models"].get(name).is_none());
        if affected && (!nodes.contains(&test) || new.filemap["cost_models"].get(name).is_none()) {
            issue(
                r,
                "E_REFERENCE",
                "test_costs target/cost missing",
                "/filemap/test_costs",
                false,
            );
        }
    }
    for artifact in new.config["artifacts"].as_array().unwrap() {
        let owner = artifact["owner"].as_str().unwrap();
        let affected = !old.config["artifacts"]
            .as_array()
            .unwrap()
            .contains(artifact)
            || (strings(&old.projects["owners"]).contains(owner) && !owners.contains(owner));
        if affected && !owners.contains(owner) {
            issue(
                r,
                "E_REFERENCE",
                "unknown artifact owner",
                "/config/artifacts",
                false,
            );
        }
    }
    for field in new.config["semantic_fields"].as_array().unwrap() {
        let path = field["path"].as_str().unwrap();
        let affected = !old.config["semantic_fields"]
            .as_array()
            .unwrap()
            .contains(field)
            || (a.contains_key(path) && !b.contains_key(path));
        if affected && !b.contains_key(path) {
            issue(
                r,
                "E_REFERENCE",
                "semantic_fields path is not registered",
                "/config/semantic_fields",
                false,
            );
        }
    }
    let old_stability = rows(&old.workflow, "stability", "id");
    for rule in new.workflow["stability"].as_array().unwrap() {
        let changed = old_stability.get(rule["id"].as_str().unwrap()) != Some(rule);
        for path in rule["paths"].as_array().unwrap() {
            let p = path.as_str().unwrap();
            if !b.contains_key(p) && (changed || a.contains_key(p)) {
                issue(
                    r,
                    "E_REFERENCE",
                    format!("missing stability path {p}"),
                    "/workflow/stability",
                    false,
                );
            }
        }
        for test in strings(&rule["tests"]) {
            let id = format!("test:{test}");
            if !nodes.contains(&id) && (changed || old_nodes.contains(&id)) {
                issue(
                    r,
                    "E_REFERENCE",
                    format!("missing stability test {test}"),
                    "/workflow/stability",
                    false,
                );
            }
        }
    }
    let old_tests = strings(&old.workflow["integration"]["tests"]);
    for test in strings(&new.workflow["integration"]["tests"]) {
        let id = format!("test:{test}");
        if !nodes.contains(&id) && (!old_tests.contains(&test) || old_nodes.contains(&id)) {
            issue(
                r,
                "E_REFERENCE",
                format!("missing workflow test {test}"),
                "/workflow/integration/tests",
                false,
            );
        }
    }
    for m in new.workflow["migrations"].as_array().unwrap() {
        for k in ["script", "test"] {
            let id = format!("script:{}", m[k].as_str().unwrap());
            if !nodes.contains(&id)
                && (!old.workflow["migrations"].as_array().unwrap().contains(m)
                    || old_nodes.contains(&id))
            {
                issue(
                    r,
                    "E_REFERENCE",
                    format!("missing migration {k}"),
                    "/workflow/migrations",
                    false,
                );
            }
        }
    }

    Ok(())
}
fn strings_ids(v: &Value) -> BTreeSet<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["id"].as_str())
        .map(str::to_owned)
        .collect()
}
