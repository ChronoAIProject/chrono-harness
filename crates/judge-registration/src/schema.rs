#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::source_verifier::identifiers;
#[cfg(verus_keep_ghost)]
use super::source_verifier::sources::{self as source_policy, continuity as source_continuity};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{
    verified_identifiers as identifiers,
    verified_verifier::sources::{self as source_policy, continuity as source_continuity},
    verus,
};
#[cfg(verus_keep_ghost)]
use vstd::{assert_seqs_equal, prelude::*};

verus! {

pub enum ObjectFieldError<'a> {
    ExpectedObject,
    Missing(&'a str),
    Unknown(&'a str),
}

pub open spec fn field_named(fields: Seq<&str>, name: Seq<char>) -> bool {
    exists|i: int| 0 <= i < fields.len() && #[trigger] fields[i]@ == name
}

pub open spec fn required_fields_present(required: Seq<&str>, keys: Seq<&str>) -> bool {
    forall|i: int| 0 <= i < required.len() ==> field_named(keys, #[trigger] required[i]@)
}

pub open spec fn known_fields_only(required: Seq<&str>, optional: Seq<&str>, keys: Seq<&str>) -> bool {
    forall|i: int| 0 <= i < keys.len() ==> (
        field_named(required, #[trigger] keys[i]@) || field_named(optional, keys[i]@)
    )
}

pub open spec fn first_missing_field(required: Seq<&str>, keys: Seq<&str>, name: Seq<char>) -> bool {
    exists|i: int| 0 <= i < required.len() && required[i]@ == name
        && !field_named(keys, required[i]@)
        && forall|j: int| 0 <= j < i ==> field_named(keys, #[trigger] required[j]@)
}

pub open spec fn first_unknown_field(
    required: Seq<&str>, optional: Seq<&str>, keys: Seq<&str>, name: Seq<char>,
) -> bool {
    exists|i: int| 0 <= i < keys.len() && keys[i]@ == name
        && !field_named(required, keys[i]@) && !field_named(optional, keys[i]@)
        && forall|j: int| 0 <= j < i ==> (
            field_named(required, #[trigger] keys[j]@) || field_named(optional, keys[j]@)
        )
}

fn field_list_contains(fields: &[&str], name: &str) -> (found: bool)
    ensures found == field_named(fields@, name@),
{
    let mut i = 0;
    while i < fields.len()
        invariant 0 <= i <= fields.len(),
            forall|j: int| 0 <= j < i ==> fields[j]@ != name@,
        decreases fields.len() - i,
    {
        if identifiers::equal_text(fields[i], name) {
            return true;
        }
        i += 1;
    }
    false
}

/// 返回同一个对象；缺字段按 required 顺序优先，未知字段按实际键顺序优先。
pub fn validate_object_fields<'a, T>(
    object: Option<T>, required: &[&'a str], optional: &[&str], keys: &[&'a str],
) -> (out: std::result::Result<T, ObjectFieldError<'a>>)
    ensures out.is_ok() == (object.is_some()
        && required_fields_present(required@, keys@)
        && known_fields_only(required@, optional@, keys@)),
        match out {
            Ok(value) => object == Some(value),
            Err(ObjectFieldError::ExpectedObject) => object.is_none(),
            Err(ObjectFieldError::Missing(name)) => object.is_some()
                && first_missing_field(required@, keys@, name@),
            Err(ObjectFieldError::Unknown(name)) => object.is_some()
                && required_fields_present(required@, keys@)
                && first_unknown_field(required@, optional@, keys@, name@),
        },
{
    let value = match object {
        Some(value) => value,
        None => return Err(ObjectFieldError::ExpectedObject),
    };
    let mut i = 0;
    while i < required.len()
        invariant 0 <= i <= required.len(), object == Some(value),
            forall|j: int| 0 <= j < i ==> field_named(keys@, #[trigger] required[j]@),
        decreases required.len() - i,
    {
        if !field_list_contains(keys, required[i]) {
            proof { assert(first_missing_field(required@, keys@, required[i as int]@)); }
            return Err(ObjectFieldError::Missing(required[i]));
        }
        i += 1;
    }
    let mut j = 0;
    while j < keys.len()
        invariant 0 <= j <= keys.len(), object == Some(value),
            required_fields_present(required@, keys@),
            forall|k: int| 0 <= k < j ==> (
                field_named(required@, #[trigger] keys[k]@) || field_named(optional@, keys[k]@)
            ),
        decreases keys.len() - j,
    {
        if !field_list_contains(required, keys[j]) && !field_list_contains(optional, keys[j]) {
            proof { assert(first_unknown_field(required@, optional@, keys@, keys[j as int]@)); }
            return Err(ObjectFieldError::Unknown(keys[j]));
        }
        j += 1;
    }
    Ok(value)
}

pub open spec fn object_error_message(error: ObjectFieldError) -> Seq<char> {
    match error {
        ObjectFieldError::ExpectedObject => "expected object"@,
        ObjectFieldError::Missing(name) => "missing field "@ + name@,
        ObjectFieldError::Unknown(name) => "unknown field "@ + name@,
    }
}

/// 标准 concat 只连接两片；错误类别、前缀及完整字段名在此产生。
pub fn object_error_parts<'a>(error: ObjectFieldError<'a>) -> (out: [&'a str; 2])
    ensures out[0]@ + out[1]@ == object_error_message(error),
        match error {
            ObjectFieldError::ExpectedObject => out[0]@ == "expected object"@ && out[1]@ == ""@,
            ObjectFieldError::Missing(name) => out[0]@ == "missing field "@ && out[1]@ == name@,
            ObjectFieldError::Unknown(name) => out[0]@ == "unknown field "@ && out[1]@ == name@,
        },
{
    match error {
        ObjectFieldError::ExpectedObject => {
            proof {
                reveal_strlit("");
                assert_seqs_equal!("expected object"@ + ""@, "expected object"@);
            }
            ["expected object", ""]
        },
        ObjectFieldError::Missing(name) => ["missing field ", name],
        ObjectFieldError::Unknown(name) => ["unknown field ", name],
    }
}

pub open spec fn declared_reference_valid(form: &source_continuity::ReferenceSyntax) -> bool {
    source_policy::canonical_source_path(vstd::utf8::encode_utf8(form.file@))
        && vstd::utf8::encode_utf8(form.before@).len() > 0
        && vstd::utf8::encode_utf8(form.after@).len() > 0
        && (vstd::utf8::encode_utf8(form.before@).last() == 34
            || (form.base == 2 && vstd::utf8::encode_utf8(form.before@).last() == 47))
        && (vstd::utf8::encode_utf8(form.after@).last() == 34
            || (form.base == 2 && vstd::utf8::encode_utf8(form.after@).last() == 47))
        && form.base <= 2
}

pub open spec fn reference_declarations_valid(forms: Seq<source_continuity::ReferenceSyntax>) -> bool {
    forms.len() <= 250 && forall|i: int| 0 <= i < forms.len() ==> declared_reference_valid(#[trigger] &forms[i])
}

/// 声明数量、文件身份、开引号和基准必须完整有效；缺失不推测 shell 变量含义。
pub fn validate_reference_syntax(forms: &Vec<source_continuity::ReferenceSyntax>) -> (out: std::result::Result<(), String>)
    ensures out.is_ok() == reference_declarations_valid(forms@),
        match out { Err(error) => error@ == "E_SOURCE_CONTINUITY: invalid reference syntax declaration"@, Ok(()) => true },
{
    if forms.len() > 250 {
        return Err("E_SOURCE_CONTINUITY: invalid reference syntax declaration".to_string());
    }
    let mut i = 0;
    while i < forms.len()
        invariant 0 <= i <= forms.len() <= 250,
            forall|j: int| 0 <= j < i ==> declared_reference_valid(#[trigger] &forms[j]),
        decreases forms.len() - i,
    {
        let form = &forms[i];
        let before = form.before.as_str().as_bytes();
        let after = form.after.as_str().as_bytes();
        if !source_policy::canonical_source_name(form.file.as_str())
            || before.len() == 0 || after.len() == 0
            || (before[before.len() - 1] != 34 && !(form.base == 2 && before[before.len() - 1] == 47))
            || (after[after.len() - 1] != 34 && !(form.base == 2 && after[after.len() - 1] == 47))
            || form.base > 2 {
            return Err("E_SOURCE_CONTINUITY: invalid reference syntax declaration".to_string());
        }
        i += 1;
    }
    Ok(())
}

} // verus!

// Strict SPEC §3 structure. Reference/activation policy is evaluated separately.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{relative_path, wire};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Map, Value};
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeSet;
#[cfg(not(verus_keep_ghost))]
type Result<T = ()> = std::result::Result<T, String>;
#[cfg(not(verus_keep_ghost))]
pub fn object<'a>(
    v: &'a Value,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>> {
    let object = v.as_object();
    let keys: Vec<&str> = object
        .into_iter()
        .flat_map(|object| object.keys().map(String::as_str))
        .collect();
    validate_object_fields(object, required, optional, &keys)
        .map_err(|error| object_error_parts(error).concat())
}
#[cfg(not(verus_keep_ghost))]
pub fn string(v: &Value) -> Result<&str> {
    v.as_str()
        .filter(|s| !s.is_empty())
        .ok_or("expected nonempty string".into())
}
#[cfg(not(verus_keep_ghost))]
pub fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("expected array".into())
}
#[cfg(not(verus_keep_ghost))]
fn strings(v: &Value) -> Result {
    for s in array(v)? {
        if !s.is_string() {
            return Err("expected string array".into());
        }
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn nonempty_strings(v: &Value) -> Result {
    for s in array(v)? {
        if string(s).is_err() {
            return Err("expected nonempty string array".into());
        }
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
fn choice(v: &Value, choices: &[&str]) -> Result {
    let s = string(v)?;
    if !choices.contains(&s) {
        return Err(format!("unsupported value {s}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn path(v: &Value) -> Result {
    let p = string(v)?;
    relative_path(p)?;
    canonical_path(p)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn state_path(v: &Value, context: &str) -> Result {
    let p = string(v)?;
    if !chrono_harness::verified_paths::valid_host_state_path(p.as_bytes()) {
        relative_path(p)?;
        return Err(context.into());
    }
    canonical_path(p)
}
#[cfg(not(verus_keep_ghost))]
fn canonical_path(p: &str) -> Result {
    if p.split('/').any(|c| c.is_empty() || c == ".") {
        return Err(format!("noncanonical path {p}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn paths(v: &Value) -> Result {
    for p in array(v)? {
        path(p)?;
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn hash(v: &Value) -> Result {
    if v.is_null() || v.as_str().is_some_and(wire::is_digest) {
        Ok(())
    } else {
        Err("invalid SHA-256".into())
    }
}
#[cfg(not(verus_keep_ghost))]
fn number(v: &Value) -> Result {
    if v.as_f64().is_some_and(|n| n >= 0.0) {
        Ok(())
    } else {
        Err("expected nonnegative number".into())
    }
}
#[cfg(not(verus_keep_ghost))]
fn boolean(v: &Value) -> Result {
    if v.is_boolean() {
        Ok(())
    } else {
        Err("expected bool".into())
    }
}
#[cfg(not(verus_keep_ghost))]
fn common(v: &Value, fields: &[&str]) -> Result {
    common_versions(v, fields, &[1])
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
fn action(v: &Value) -> Result {
    object(v, &["operation", "tool", "argv"], &["output_contract"])?;
    string(&v["operation"])?;
    string(&v["tool"])?;
    strings(&v["argv"])?;
    output_contract(&v["output_contract"])
}
#[cfg(not(verus_keep_ghost))]
fn output_contract(v: &Value) -> Result {
    let contract = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    chrono_harness::verified_verifier::validate_contract(&contract)
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
    if matches!(v["schema_version"].as_u64(), Some(3 | 4)) {
        fields.push("facts_git");
        chrono_harness::facts::git_declaration(v)?;
        if let Some(units) = v.get("execution_units") {
            fields.push("execution_units");
            validate_execution_units(units)?;
        }
    }
    common_versions(v, &fields, &[1, 2, 3, 4])?;
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
    if v["schema_version"] == 4 {
        chrono_harness::prepared::declaration(v)?;
    } else {
        object(&v["canonical_check"], &["operation", "argv"], &[])?;
    }
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
    object(
        &v["environment"],
        &["inherit", "values", "inputs"],
        if v["schema_version"] == 4 {
            &["credential_environment"]
        } else {
            &[]
        },
    )?;
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
        if matches!(v["schema_version"].as_u64(), Some(2 | 3 | 4)) {
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
    let closure_optional = if matches!(v["schema_version"].as_u64(), Some(3 | 4)) {
        vec!["bindings", "coverage"]
    } else {
        vec![]
    };
    object(
        &v["input_closure"],
        &["status", "unresolved"],
        &closure_optional,
    )?;
    choice(
        &v["input_closure"]["status"],
        &["incomplete", "declared-complete"],
    )?;
    strings(&v["input_closure"]["unresolved"])?;
    if matches!(v["schema_version"].as_u64(), Some(3 | 4))
        && v["input_closure"]["status"] == "declared-complete"
        && v["input_closure"].get("bindings").is_none()
    {
        return Err("schema v3 declared-complete input closure requires bindings".into());
    }
    if let Some(bindings) = v["input_closure"].get("bindings") {
        unique(bindings, Some("id"))?;
        for binding in array(bindings)? {
            object(binding, &["id", "consumer", "kind", "inputs"], &[])?;
            string(&binding["id"])?;
            string(&binding["consumer"])?;
            string(&binding["kind"])?;
            nonempty_strings(&binding["inputs"])?;
        }
    }
    if let Some(coverage) = v["input_closure"].get("coverage") {
        crate::input_coverage::shape(coverage, &v["input_closure"])?;
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn validate_execution_units(v: &Value) -> Result {
    chrono_harness::units::report_paths(&serde_json::json!({"execution_units":v}), None)?;
    let fields = object(
        v,
        &[
            "units",
            "shared_operations",
            "collection_limits",
            "report_path",
        ],
        &[],
    )?;
    state_path(
        &fields["report_path"],
        "execution_units.report_path must reside in .chrono-harness/state/",
    )?;
    let units = fields["units"].as_object().ok_or("execution_units.units")?;
    if units.is_empty() {
        return Err("execution_units.units must be nonempty".into());
    }
    for (id, unit) in units {
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(format!("invalid execution unit ID {id}"));
        }
        object(unit, &["tests", "report_path"], &[])?;
        nonempty_strings(&unit["tests"])?;
        state_path(
            &unit["report_path"],
            "execution unit report must reside in .chrono-harness/state/",
        )?;
    }
    let limits = object(
        &fields["collection_limits"],
        &["manifest_bytes", "report_bytes"],
        &[],
    )?;
    for key in ["manifest_bytes", "report_bytes"] {
        if !limits[key]
            .as_u64()
            .is_some_and(|n| n > 0 && n <= 64 * 1024 * 1024)
        {
            return Err(format!("invalid execution unit limit {key}"));
        }
    }
    let shared = fields["shared_operations"]
        .as_object()
        .ok_or("execution_units.shared_operations")?;
    for (operation, owners) in shared {
        if operation.is_empty() {
            return Err("empty shared operation".into());
        }
        let owners = owners.as_array().ok_or("shared operation owners")?;
        if owners.len() < 2 || owners.iter().any(|v| v.as_str().is_none()) {
            return Err(format!("invalid shared operation assignment {operation}"));
        }
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn projects(v: &Value) -> Result {
    common_versions(v, &["owners", "projects", "scripts"], &[1, 2])?;
    let languages = v["schema_version"] == 2;
    unique(&v["owners"], None)?;
    unique(&v["projects"], Some("id"))?;
    unique(&v["scripts"], Some("id"))?;
    for p in array(&v["projects"])? {
        object(
            p,
            if languages {
                &["id", "kind", "actions", "language"]
            } else {
                &["id", "kind", "actions"]
            },
            &[
                "test_project",
                "tests_for",
                "manifest",
                "lockfile",
                "root",
                "test_groups",
            ],
        )?;
        choice(&p["kind"], &["production", "test"])?;
        if languages {
            string(&p["language"]).map_err(|e| format!("project language: {e}"))?;
        }
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
        object(
            s,
            if languages {
                &["id", "path", "actions", "language"]
            } else {
                &["id", "path", "actions"]
            },
            &["test_script", "tests_for"],
        )?;
        if languages {
            string(&s["language"]).map_err(|e| format!("script language: {e}"))?;
        }
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
    crate::execution::test_bindings(v)?;
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn filemap(v: &Value) -> Result {
    let mut plan_operations = Vec::new();
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
            &[
                "execution_scheduling",
                "configuration_edges",
                "require_new_file_classification",
                "reference_syntax",
            ],
        )?;
        choice(&v["status"], &["active", "proposed"])?;
        if let Some(required) = v.get("require_new_file_classification") {
            boolean(required)?;
        }
        if let Some(declaration) = v.get("reference_syntax") {
            let syntax: Vec<source_continuity::ReferenceSyntax> =
                serde_json::from_value(declaration.clone()).map_err(|e| e.to_string())?;
            validate_reference_syntax(&syntax)?;
        }
        let plans = crate::execution::plans(v)?;
        for plan in plans.values() {
            plan_operations.extend(plan.operations.iter().cloned());
        }
        if let Some(policy) = v.get("execution_scheduling") {
            let policy: crate::execution::Scheduling =
                serde_json::from_value(policy.clone()).map_err(|e| e.to_string())?;
            policy.validate()?;
        }
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
            &["symlink", "projection", "verification"],
        )?;
        let source: chrono_harness::verified_verifier::sources::SourceFile =
            serde_json::from_value(f.clone()).map_err(|e| e.to_string())?;
        chrono_harness::verified_verifier::sources::validate_source_declaration(&source)
            .map_err(|error| format!("{}: {error}", source.path))?;
        if !chrono_harness::verified_verifier::sources::proof_roots_have_registered_operations(
            &source,
            &plan_operations,
        ) {
            return Err(format!(
                "{}: E_VERUS_COVERAGE: proof root operation is not registered",
                source.path
            ));
        }
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
    if let Some(table) = v.get("configuration_edges") {
        object(table, &["schema", "configurations"], &[])?;
        choice(&table["schema"], &["chrono-filemap-configuration-edges/v1"])?;
        let configurations = array(&table["configurations"])?;
        if configurations.is_empty() {
            return Err("configuration edges require nonempty configurations".into());
        }
        unique(&table["configurations"], Some("config"))?;
        for row in configurations {
            object(row, &["config", "edges"], &[])?;
            path(&row["config"])?;
            if !array(&v["files"])?
                .iter()
                .any(|file| file["path"] == row["config"])
            {
                return Err("configuration edge policy is not a registered file".into());
            }
            let edges = array(&row["edges"])?;
            if edges.is_empty() {
                return Err("configuration edges require nonempty edges".into());
            }
            for e in edges {
                edge(e, true)?;
            }
        }
    }
    unique(&v["test_costs"], Some("test"))?;
    for c in array(&v["test_costs"])? {
        object(c, &["test", "cost"], &[])?;
        string(&c["cost"])?;
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
                        object(
                            method,
                            &["owner", "operation", "tool", "argv"],
                            &["output_contract"],
                        )?;
                        for key in ["owner", "operation", "tool"] {
                            string(&method[key])?;
                        }
                        strings(&method["argv"])?;
                        output_contract(&method["output_contract"])?;
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
            &["from_version", "to_version", "mappings", "reason"],
            &["script", "test"],
        )?;
        for k in ["from_version", "to_version"] {
            if m[k].as_u64().is_none() {
                return Err("migration version must be integer".into());
            }
        }
        string(&m["reason"])?;
        if m.get("script").is_some() != m.get("test").is_some() {
            return Err("data conversion script and test must be declared together".into());
        }
        for k in ["script", "test"] {
            if let Some(value) = m.get(k) {
                string(value)?;
            }
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
