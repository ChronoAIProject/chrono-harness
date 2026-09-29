//! Strict SPEC §3 structure. Reference/activation policy is evaluated separately.
use chrono_harness::{relative_path, wire};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
type Result<T = ()> = std::result::Result<T, String>;
pub fn object<'a>(
    v: &'a Value,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>> {
    let o = v.as_object().ok_or("expected object")?;
    for k in required {
        if !o.contains_key(*k) {
            return Err(format!("missing field {k}"));
        }
    }
    for k in o.keys() {
        if !required.contains(&k.as_str()) && !optional.contains(&k.as_str()) {
            return Err(format!("unknown field {k}"));
        }
    }
    Ok(o)
}
pub fn string(v: &Value) -> Result<&str> {
    v.as_str()
        .filter(|s| !s.is_empty())
        .ok_or("expected nonempty string".into())
}
pub fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("expected array".into())
}
fn strings(v: &Value) -> Result {
    for s in array(v)? {
        if !s.is_string() {
            return Err("expected string array".into());
        }
    }
    Ok(())
}
fn unique(v: &Value, key: Option<&str>) -> Result {
    let mut seen = BTreeSet::new();
    for row in array(v)? {
        let id = string(if let Some(k) = key { &row[k] } else { row })?;
        if !seen.insert(id) {
            return Err(format!("duplicate ID {id}"));
        }
    }
    Ok(())
}
fn choice(v: &Value, choices: &[&str]) -> Result {
    let s = string(v)?;
    if !choices.contains(&s) {
        return Err(format!("unsupported value {s}"));
    }
    Ok(())
}
fn path(v: &Value) -> Result {
    let p = string(v)?;
    relative_path(p)?;
    if p.split('/').any(|c| c.is_empty() || c == ".") {
        return Err(format!("noncanonical path {p}"));
    }
    Ok(())
}
fn paths(v: &Value) -> Result {
    for p in array(v)? {
        path(p)?;
    }
    Ok(())
}
fn hash(v: &Value) -> Result {
    if v.is_null() || v.as_str().is_some_and(wire::is_digest) {
        Ok(())
    } else {
        Err("invalid SHA-256".into())
    }
}
fn number(v: &Value) -> Result {
    if v.as_f64().is_some_and(|n| n >= 0.0) {
        Ok(())
    } else {
        Err("expected nonnegative number".into())
    }
}
fn boolean(v: &Value) -> Result {
    if v.is_boolean() {
        Ok(())
    } else {
        Err("expected bool".into())
    }
}
fn common(v: &Value, fields: &[&str]) -> Result {
    common_versions(v, fields, &[1])
}
fn common_versions(v: &Value, fields: &[&str], versions: &[u64]) -> Result {
    let mut required = vec!["schema_version", "status"];
    required.extend(fields);
    object(v, &required, &[])?;
    if !v["schema_version"]
        .as_u64()
        .is_some_and(|n| versions.contains(&n))
    {
        return Err("unsupported schema_version; migration evidence required".into());
    }
    choice(&v["status"], &["active", "proposed"])
}
fn action(v: &Value) -> Result {
    object(v, &["operation", "tool", "argv"], &[])?;
    string(&v["operation"])?;
    string(&v["tool"])?;
    strings(&v["argv"])
}
fn actions(v: &Value) -> Result {
    let o = v.as_object().ok_or("actions must be an object")?;
    if o.is_empty() {
        return Err("empty actions".into());
    }
    for (name, a) in o {
        if name.is_empty() {
            return Err("empty action name".into());
        }
        action(a)?;
    }
    Ok(())
}
fn edge(v: &Value, from: bool) -> Result {
    object(
        v,
        if from {
            &["from", "kind", "to"]
        } else {
            &["kind", "to"]
        },
        &[],
    )?;
    choice(
        &v["kind"],
        &[
            "compile",
            "build-input",
            "runtime-input",
            "test-execution",
            "judge-trigger",
        ],
    )?;
    if from {
        string(&v["from"])?;
    }
    string(&v["to"])?;
    Ok(())
}
pub fn config(v: &Value) -> Result {
    let mut fields = vec![
        "enforcement",
        "runner",
        "registries",
        "canonical_check",
        "tools",
        "protocol",
        "semantic_fields",
        "artifacts",
        "environment",
        "input_closure",
    ];
    if v["schema_version"] == 3 {
        fields.push("facts_git");
        chrono_harness::facts::git_declaration(v)?;
    }
    common_versions(v, &fields, &[1, 2, 3])?;
    choice(&v["enforcement"], &["enabled", "not-implemented"])?;
    object(&v["runner"], &["path", "version", "sha256"], &[])?;
    path(&v["runner"]["path"])?;
    string(&v["runner"]["version"])?;
    hash(&v["runner"]["sha256"])?;
    let registries = object(
        &v["registries"],
        &["judges", "projects", "filemap", "workflow"],
        &[],
    )?;
    let mut seen = BTreeSet::new();
    for p in registries.values() {
        path(p)?;
        if !seen.insert(string(p)?) {
            return Err("duplicate registry path".into());
        }
    }
    object(&v["canonical_check"], &["operation", "argv"], &[])?;
    choice(&v["canonical_check"]["operation"], &["validate.delta"])?;
    strings(&v["canonical_check"]["argv"])?;
    unique(&v["tools"], Some("id"))?;
    for t in array(&v["tools"])? {
        object(
            t,
            &[
                "id",
                "program",
                "resolution",
                "version_argv",
                "expected_version",
            ],
            &[],
        )?;
        string(&t["program"])?;
        choice(&t["resolution"], &["PATH-once"])?;
        strings(&t["version_argv"])?;
        if !t["expected_version"].is_null() {
            string(&t["expected_version"])?;
        }
    }
    object(
        &v["protocol"],
        &["id", "timeout_seconds", "stdout_limit_bytes", "encoding"],
        &[],
    )?;
    choice(&v["protocol"]["id"], &[wire::PROTOCOL])?;
    choice(&v["protocol"]["encoding"], &["UTF-8"])?;
    for k in ["timeout_seconds", "stdout_limit_bytes"] {
        if !v["protocol"][k].as_u64().is_some_and(|n| n > 0) {
            return Err(format!("invalid protocol {k}"));
        }
    }
    for s in array(&v["semantic_fields"])? {
        object(s, &["path", "pointers", "on"], &[])?;
        path(&s["path"])?;
        strings(&s["pointers"])?;
        choice(&s["on"], &["add-modify-delete", "modify-delete-existing"])?;
        for p in array(&s["pointers"])? {
            if !string(p)?.starts_with('/') {
                return Err("invalid semantic pointer".into());
            }
        }
    }
    unique(&v["artifacts"], Some("path"))?;
    for a in array(&v["artifacts"])? {
        object(a, &["path", "owner", "kind", "tracked"], &[])?;
        let p = string(&a["path"])?;
        if !p.ends_with('/') {
            return Err("artifact must be directory ending /".into());
        }
        path(&Value::from(p.trim_end_matches('/')))?;
        string(&a["owner"])?;
        string(&a["kind"])?;
        if a["tracked"] != false {
            return Err("artifact tracked must be false".into());
        }
    }
    object(&v["environment"], &["inherit", "values", "inputs"], &[])?;
    unique(&v["environment"]["inherit"], None)?;
    for (k, val) in v["environment"]["values"]
        .as_object()
        .ok_or("environment values must be object")?
    {
        if k.is_empty() || k.contains(['=', '\0']) || !val.is_string() {
            return Err("invalid environment value".into());
        }
    }
    unique(&v["environment"]["inputs"], Some("id"))?;
    for i in array(&v["environment"]["inputs"])? {
        if matches!(v["schema_version"].as_u64(), Some(2 | 3)) {
            choice(&i["presence"], &["present", "absent"])?;
            if i["presence"] == "absent" {
                object(i, &["id", "location", "presence"], &[])?;
            } else {
                object(i, &["id", "location", "presence", "sha256"], &[])?;
                hash(&i["sha256"])?;
            }
        } else {
            object(i, &["id", "location", "sha256"], &[])?;
            hash(&i["sha256"])?;
        }
        let s = string(&i["location"])?;
        if !std::path::Path::new(s).is_absolute() {
            path(&i["location"])?;
        }
    }
    object(&v["input_closure"], &["status", "unresolved"], &[])?;
    choice(
        &v["input_closure"]["status"],
        &["incomplete", "declared-complete"],
    )?;
    strings(&v["input_closure"]["unresolved"])?;
    Ok(())
}
pub fn projects(v: &Value) -> Result {
    common(v, &["owners", "projects", "scripts"])?;
    unique(&v["owners"], None)?;
    unique(&v["projects"], Some("id"))?;
    unique(&v["scripts"], Some("id"))?;
    for p in array(&v["projects"])? {
        object(
            p,
            &["id", "kind", "actions"],
            &["test_project", "tests_for", "manifest", "lockfile", "root"],
        )?;
        choice(&p["kind"], &["production", "test"])?;
        if p.get("test_project").is_some() == p.get("tests_for").is_some() {
            return Err("project requires exactly test_project or tests_for".into());
        }
        for k in ["manifest", "lockfile", "root"] {
            if let Some(value) = p.get(k) {
                path(value)?;
            }
        }
        actions(&p["actions"])?;
        string(
            &p[if p["kind"] == "production" {
                "test_project"
            } else {
                "tests_for"
            }],
        )?;
    }
    for s in array(&v["scripts"])? {
        object(s, &["id", "path", "actions"], &["test_script", "tests_for"])?;
        path(&s["path"])?;
        actions(&s["actions"])?;
        if s.get("test_script").is_some() == s.get("tests_for").is_some() {
            return Err("script requires exactly test_script or tests_for".into());
        }
        string(
            &s[if s.get("test_script").is_some() {
                "test_script"
            } else {
                "tests_for"
            }],
        )?;
    }
    Ok(())
}
pub fn filemap(v: &Value) -> Result {
    if v["schema_version"] == 2 {
        object(
            v,
            &[
                "schema_version",
                "status",
                "cost_models",
                "files",
                "project_edges",
                "test_costs",
                "execution_plans",
            ],
            &[],
        )?;
        choice(&v["status"], &["active", "proposed"])?;
        crate::execution::plans(v)?;
    } else {
        common(v, &["cost_models", "files", "project_edges", "test_costs"])?;
    }
    for c in v["cost_models"]
        .as_object()
        .ok_or("cost_models object")?
        .values()
    {
        object(
            c,
            &["cpu_ms", "wall_ms", "peak_rss_bytes", "io_bytes", "basis"],
            &[],
        )?;
        for k in ["cpu_ms", "wall_ms", "peak_rss_bytes", "io_bytes"] {
            if !c[k].is_null() {
                number(&c[k])?;
            }
        }
        string(&c["basis"])?;
    }
    unique(&v["files"], Some("path"))?;
    for f in array(&v["files"])? {
        object(
            f,
            &["path", "owner", "surface", "cost", "edges"],
            &["symlink", "projection"],
        )?;
        path(&f["path"])?;
        string(&f["owner"])?;
        string(&f["cost"])?;
        choice(
            &f["surface"],
            &[
                "product",
                "test",
                "documentation",
                "membership",
                "instruction-policy",
                "judge-policy",
                "judge-implementation",
            ],
        )?;
        for e in array(&f["edges"])? {
            edge(e, false)?;
        }
        if let Some(s) = f.get("symlink") {
            let s = string(s)?;
            if std::path::Path::new(s).is_absolute() {
                return Err("absolute symlink target".into());
            }
        }
        if let Some(p) = f.get("projection") {
            object(p, &["sources", "producer", "scope"], &[])?;
            paths(&p["sources"])?;
            string(&p["producer"])?;
            choice(&p["scope"], &["managed-block", "whole-file"])?;
        }
    }
    for e in array(&v["project_edges"])? {
        edge(e, true)?;
    }
    unique(&v["test_costs"], Some("test"))?;
    for c in array(&v["test_costs"])? {
        object(c, &["test", "cost"], &[])?;
        string(&c["cost"])?;
    }
    Ok(())
}
pub fn judges(v: &Value) -> Result {
    common(v, &["migration_validator", "judges"])?;
    string(&v["migration_validator"])?;
    unique(&v["judges"], Some("id"))?;
    let b: Vec<wire::Binding> =
        serde_json::from_value(v["judges"].clone()).map_err(|e| e.to_string())?;
    for j in &b {
        path(&Value::from(j.executable.clone()))?;
        if j.version.is_empty() {
            return Err("empty judge version".into());
        }
        hash(&serde_json::to_value(&j.sha256).map_err(|e| e.to_string())?)?;
    }
    chrono_harness::full::schedule(&b)?;
    Ok(())
}
pub fn workflow(v: &Value) -> Result {
    let fields = [
        "target_branch",
        "feature_prefix",
        "integration_prefix",
        "staleness",
        "stability",
        "semantic_changes_require_integration",
        "mixed_change",
        "integration",
        "retirements",
        "migrations",
    ];
    if v["schema_version"] == 2 || v["schema_version"] == 3 {
        let mut required = vec!["schema_version", "status", "historical_profiles"];
        required.extend(fields);
        object(v, &required, &[])?;
        choice(&v["status"], &["active", "proposed"])?;
        unique(&v["historical_profiles"], Some("id"))?;
        for profile in array(&v["historical_profiles"])? {
            let mut fields = vec![
                "id",
                "script",
                "test",
                "mappings",
                "legacy_records",
                "ambiguities",
            ];
            if v["schema_version"] == 3 && profile.get("from_versions").is_some() {
                fields.extend(["from_versions", "to_versions"]);
                object(profile, &fields, &["method_replacements"])?;
                for side in ["from_versions", "to_versions"] {
                    let versions = object(
                        &profile[side],
                        &["config", "filemap", "projects", "judges", "workflow"],
                        &[],
                    )?;
                    if versions
                        .values()
                        .any(|v| !v.as_u64().is_some_and(|n| n > 0))
                    {
                        return Err("historical schema versions must be positive integers".into());
                    }
                }
                if profile["from_versions"] == profile["to_versions"] {
                    return Err("historical version selector must describe a transition".into());
                }
            } else {
                fields.extend(["filemap_version", "profile_path"]);
                object(profile, &fields, &["method_replacements"])?;
                path(&profile["profile_path"])?;
                if profile["filemap_version"] != 1 {
                    return Err(
                        "only explicitly declared historical FILEMAP v1 profiles are supported"
                            .into(),
                    );
                }
            }
            for k in ["script", "test"] {
                string(&profile[k])?;
            }
            for mapping in array(&profile["mappings"])? {
                object(mapping, &["from", "to"], &[])?;
                string(&mapping["from"])?;
                string(&mapping["to"])?;
            }
            if let Some(replacements) = profile.get("method_replacements") {
                let mut seen = BTreeSet::new();
                for replacement in array(replacements)? {
                    object(replacement, &["from", "to", "reason"], &[])?;
                    string(&replacement["reason"])?;
                    for side in ["from", "to"] {
                        let method = &replacement[side];
                        object(method, &["owner", "operation", "tool", "argv"], &[])?;
                        for key in ["owner", "operation", "tool"] {
                            string(&method[key])?;
                        }
                        strings(&method["argv"])?;
                    }
                    let operation = string(&replacement["from"]["operation"])?;
                    if !seen.insert(operation)
                        || replacement["to"]["operation"] != operation
                        || replacement["from"] == replacement["to"]
                    {
                        return Err("invalid/duplicate historical method replacement".into());
                    }
                }
            }
            for record in array(&profile["legacy_records"])? {
                object(record, &["collection", "id", "definition", "nodes"], &[])?;
                choice(&record["collection"], &["scripts"])?;
                string(&record["id"])?;
                strings(&record["nodes"])?;
            }
            for repair in array(&profile["ambiguities"])? {
                object(repair, &["node", "definitions", "replacement"], &[])?;
                string(&repair["node"])?;
                strings(&repair["definitions"])?;
                string(&repair["replacement"])?;
            }
        }
    } else {
        common(v, &fields)?;
    }
    for k in ["target_branch", "feature_prefix", "integration_prefix"] {
        string(&v[k])?;
    }
    object(
        &v["staleness"],
        &["max_behind_commits", "max_age_hours", "combine"],
        &[],
    )?;
    for k in ["max_behind_commits", "max_age_hours"] {
        number(&v["staleness"][k])?;
    }
    choice(&v["staleness"]["combine"], &["any-exceeded"])?;
    unique(&v["stability"], Some("id"))?;
    for s in array(&v["stability"])? {
        object(s, &["id", "paths", "tests", "reason"], &[])?;
        paths(&s["paths"])?;
        strings(&s["tests"])?;
        string(&s["reason"])?;
    }
    boolean(&v["semantic_changes_require_integration"])?;
    object(
        &v["mixed_change"],
        &["level", "code", "requires_acknowledgement"],
        &[],
    )?;
    choice(&v["mixed_change"]["level"], &["warning"])?;
    string(&v["mixed_change"]["code"])?;
    if v["mixed_change"]["requires_acknowledgement"] != false {
        return Err("mixed warning cannot require acknowledgement".into());
    }
    object(&v["integration"], &["tests", "bind", "evidence"], &[])?;
    strings(&v["integration"]["tests"])?;
    strings(&v["integration"]["bind"])?;
    path(&v["integration"]["evidence"])?;
    for r in array(&v["retirements"])? {
        object(r, &["kind", "id", "replacement", "reason"], &[])?;
        choice(&r["kind"], &["project", "test", "script", "judge"])?;
        string(&r["id"])?;
        string(&r["reason"])?;
        if !r["replacement"].is_null() {
            string(&r["replacement"])?;
        }
    }
    for m in array(&v["migrations"])? {
        object(
            m,
            &[
                "from_version",
                "to_version",
                "script",
                "test",
                "mappings",
                "reason",
            ],
            &[],
        )?;
        for k in ["from_version", "to_version"] {
            if m[k].as_u64().is_none() {
                return Err("migration version must be integer".into());
            }
        }
        for k in ["script", "test", "reason"] {
            string(&m[k])?;
        }
        for pair in array(&m["mappings"])? {
            object(pair, &["from", "to"], &[])?;
            if pair["from"].is_null() && pair["to"].is_null() {
                return Err("empty migration mapping".into());
            }
            for k in ["from", "to"] {
                if !pair[k].is_null() {
                    string(&pair[k])?;
                }
            }
        }
    }
    Ok(())
}
