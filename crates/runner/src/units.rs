//! Typed scoped check selection and explicit report inputs. No test discovery.
use crate::{CheckConfig, relative_path};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "chrono-ci-check/v3";
pub const PROTOCOL: &str = "chrono-ci-judge/v2";
pub const REPORT_REFERENCE: &str = "chrono-check-reference/v1";

/// A fixed publication slot names one immutable original; it carries no verdict.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportReference {
    pub schema: String,
    pub original: crate::prepared::Original,
}

pub fn reference_publication(config: &CheckConfig) -> Result<bool, String> {
    match config.policy.get("report_publication") {
        None => Ok(false),
        Some(value)
            if config.schema == PROFILE
                && matches!(
                    value.as_str(),
                    Some("retained-reference/v1" | "retained-reference/v2")
                ) =>
        {
            Ok(true)
        }
        Some(_) => Err("report_publication requires scoped v3 retained-reference/v1 or v2".into()),
    }
}

pub fn stream_publication(config: &CheckConfig) -> bool {
    config.policy["report_publication"] == "retained-reference/v2"
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub tests: Vec<String>,
    pub report_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub reports: Vec<ReportInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportInput {
    pub unit: String,
    pub path: String,
    pub sha256: String,
    pub runner_sha256: String,
    pub judge_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<crate::prepared::ArtifactTransport>,
}

/// Full collection derives executable pins from the bound candidate registry,
/// never from caller claims copied out of a report.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullManifest {
    pub schema: String,
    pub reports: Vec<FullReportInput>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullReportInput {
    pub unit: String,
    pub path: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<crate::prepared::ArtifactTransport>,
}

/// Validate and select the explicitly registered full execution units.  This
/// is deliberately a pure map operation shared by the full route and the
/// scoped adapter; it never discovers plans or derives ownership from paths.
pub fn assignments(
    units: &BTreeMap<String, Unit>,
    plans: &BTreeSet<String>,
    shared: &BTreeMap<String, Vec<String>>,
    operation_sets: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let mut owners = BTreeMap::<String, String>::new();
    for (unit, definition) in units {
        id(unit)?;
        for test in &definition.tests {
            if !plans.contains(test) {
                return Err(format!("unknown CI unit plan: {test}"));
            }
            if owners.insert(test.clone(), unit.clone()).is_some() {
                return Err(format!("duplicate CI unit plan assignment: {test}"));
            }
        }
    }
    if owners.keys().collect::<BTreeSet<_>>() != plans.iter().collect::<BTreeSet<_>>() {
        return Err("missing CI unit plan assignment".into());
    }
    let mut expected = BTreeMap::<String, BTreeSet<String>>::new();
    for (operation, names) in shared {
        let set: BTreeSet<_> = names.iter().cloned().collect();
        if set.len() != names.len() || set.len() < 2 || names.iter().any(|u| !units.contains_key(u))
        {
            return Err(format!("invalid shared operation assignment: {operation}"));
        }
        expected.insert(operation.clone(), set);
    }
    let actual: BTreeMap<_, _> = operation_sets
        .iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(operation, names)| (operation.clone(), names.clone()))
        .collect();
    if actual != expected {
        return Err(
            "shared operations must explicitly name exactly the units that repeat them".into(),
        );
    }
    Ok(())
}

pub fn select(
    units: &BTreeMap<String, Unit>,
    selected: &BTreeSet<String>,
    unit: &str,
) -> Result<BTreeSet<String>, String> {
    id(unit)?;
    Ok(units
        .get(unit)
        .ok_or("unregistered CI unit")?
        .tests
        .iter()
        .filter(|test| selected.contains(*test))
        .cloned()
        .collect())
}

pub fn required(units: &BTreeMap<String, Unit>, selected: &BTreeSet<String>) -> Vec<String> {
    units
        .iter()
        .filter(|(_, definition)| definition.tests.iter().any(|test| selected.contains(test)))
        .map(|(id, _)| id.clone())
        .collect()
}

/// Read the opt-in full-v3 execution unit block.  Older full registrations do
/// not acquire selector semantics merely because a caller supplied `--unit`.
pub fn overlap(a: &str, b: &str) -> bool {
    let (a, b) = (std::path::Path::new(a), std::path::Path::new(b));
    a.starts_with(b) || b.starts_with(a)
}
/// Validate a caller-supplied collection manifest against every registered
/// publication destination. This is the single path owner used by the scoped
/// adapter, full runner and offline collector.
pub fn validate_collection_paths(
    manifest: &str,
    report_path: &str,
    unit_paths: &[String],
) -> Result<(), String> {
    artifact_path(manifest)?;
    artifact_path(report_path)?;
    let mut paths = vec![manifest.to_owned(), report_path.to_owned()];
    paths.extend(unit_paths.iter().cloned());
    for (index, path) in paths.iter().enumerate() {
        if paths[..index].iter().any(|prior| overlap(path, prior)) {
            return Err("collection manifest overlaps publication path".into());
        }
    }
    Ok(())
}
pub fn report_paths(profile: &serde_json::Value, evidence: Option<&str>) -> Result<(), String> {
    let Some(block) = profile.get("execution_units") else {
        return Ok(());
    };
    let mut reserved = vec![
        ".chrono-harness/state/report.json".to_owned(),
        ".chrono-harness/state/integration.json".to_owned(),
        ".chrono-harness/state/context.json".to_owned(),
        ".chrono-harness/state/inputs.json".to_owned(),
    ];
    if let Some(e) = evidence {
        reserved.push(e.into());
    }
    if let Some(argv) = profile["canonical_check"]["argv"].as_array() {
        for pair in argv.windows(2) {
            if pair[0] == "--context" {
                if let Some(path) = pair[1].as_str() {
                    reserved.push(path.into());
                }
            }
        }
    }
    let mut paths = vec![
        block["report_path"]
            .as_str()
            .ok_or("collection report path")?,
    ];
    for unit in block["units"]
        .as_object()
        .ok_or("unit definitions")?
        .values()
    {
        paths.push(unit["report_path"].as_str().ok_or("unit report path")?);
    }
    for (i, path) in paths.iter().enumerate() {
        if reserved.iter().any(|r| overlap(path, r))
            || paths[..i].iter().any(|p| overlap(path, p))
            || path.starts_with(".chrono-harness/state/run-")
            || path.starts_with(".chrono-harness/state/import-")
            || path.starts_with(".chrono-harness/state/base-")
        {
            return Err(
                "execution report paths overlap reserved evidence or another report".into(),
            );
        }
    }
    Ok(())
}
pub fn full_execution_units(
    profile: &serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !matches!(
        profile
            .get("schema_version")
            .and_then(serde_json::Value::as_u64),
        Some(3 | 4)
    ) {
        return Ok(None);
    }
    let Some(block) = profile.get("execution_units") else {
        return Ok(None);
    };
    report_paths(profile, None)?;
    let object = block
        .as_object()
        .ok_or("execution_units must be an object")?;
    let units_value = object.get("units").ok_or("execution_units.units missing")?;
    let units: BTreeMap<String, Unit> = serde_json::from_value(units_value.clone())
        .map_err(|e| format!("invalid execution_units.units: {e}"))?;
    if units.is_empty() {
        return Err("execution_units.units must be nonempty".into());
    }
    let shared: BTreeMap<String, Vec<String>> = serde_json::from_value(
        object
            .get("shared_operations")
            .cloned()
            .ok_or("execution_units.shared_operations missing")?,
    )
    .map_err(|e| format!("invalid execution_units.shared_operations: {e}"))?;
    let report_path = object
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .ok_or("execution_units.report_path missing")?;
    artifact_path(report_path)?;
    if report_path == ".chrono-harness/state/report.json" {
        return Err("execution_units.report_path collides with canonical full report".into());
    }
    let collection_limits = object
        .get("collection_limits")
        .cloned()
        .ok_or("execution_units.collection_limits missing")?;
    let limits: BTreeMap<String, u64> = serde_json::from_value(collection_limits.clone())
        .map_err(|e| format!("invalid execution_units.collection_limits: {e}"))?;
    for key in ["manifest_bytes", "report_bytes"] {
        if limits
            .get(key)
            .copied()
            .is_none_or(|n| n == 0 || n > 64 * 1024 * 1024)
        {
            return Err(format!("invalid execution_units limit {key}"));
        }
    }
    for (unit_id, unit) in &units {
        id(unit_id)?;
        if unit.tests.is_empty() {
            return Err(format!("execution unit has no tests: {unit_id}"));
        }
        artifact_path(&unit.report_path)?;
        if unit.report_path == report_path {
            return Err(format!(
                "execution unit report collides with full report: {unit_id}"
            ));
        }
        if units.iter().any(|(other_id, other)| {
            other_id != unit_id
                && (unit.report_path == other.report_path
                    || unit
                        .report_path
                        .starts_with(&(other.report_path.clone() + "/"))
                    || other
                        .report_path
                        .starts_with(&(unit.report_path.clone() + "/")))
        }) {
            return Err(format!("execution unit report paths overlap: {unit_id}"));
        }
    }
    Ok(Some(serde_json::json!({
        "units": units,
        "shared_operations": shared,
        "collection_limits": collection_limits,
        "report_path": report_path,
    })))
}

pub fn id(s: &str) -> Result<(), String> {
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(format!("invalid CI unit ID: {s:?}"));
    }
    Ok(())
}

pub fn artifact_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err(
            "unit reports and collection manifests must reside in .chrono-harness/state/".into(),
        );
    }
    Ok(())
}

impl Scope {
    pub fn short_argv(&self) -> Vec<String> {
        match self {
            Self::Unit { unit } => vec!["--unit".into(), unit.clone()],
            Self::Collect { .. } => vec!["--collect".into()],
        }
    }
    pub fn contract_argv(&self, profile: &serde_json::Value) -> Vec<String> {
        if profile["schema_version"] == 4 {
            self.short_argv()
        } else {
            self.argv().to_vec()
        }
    }

    pub fn argv(&self) -> [String; 2] {
        match self {
            Self::Unit { unit } => ["--unit".into(), unit.clone()],
            Self::Collect { manifest } => ["--collect".into(), manifest.clone()],
        }
    }

    pub fn report_path(&self, c: &CheckConfig) -> Result<String, String> {
        if c.schema != PROFILE {
            return Err("unit/collect selection requires chrono-ci-check/v3".into());
        }
        match self {
            Self::Unit { unit } => {
                id(unit)?;
                let units: BTreeMap<String, Unit> =
                    serde_json::from_value(c.policy["units"].clone()).map_err(|e| e.to_string())?;
                let path = &units.get(unit).ok_or("unregistered CI unit")?.report_path;
                artifact_path(path)?;
                Ok(path.clone())
            }
            Self::Collect { manifest } => {
                let units: Vec<String> = c.policy["units"]
                    .as_object()
                    .ok_or("unit definitions")?
                    .values()
                    .map(|unit| {
                        unit["report_path"]
                            .as_str()
                            .ok_or("unit report path")
                            .map(str::to_owned)
                    })
                    .collect::<Result<_, _>>()?;
                validate_collection_paths(manifest, &c.report_path, &units)?;
                Ok(c.report_path.clone())
            }
        }
    }
}

/// Explicit bounded state input reader shared by registration bootstrap and projects admission.
pub fn read_bounded(root: &std::path::Path, path: &str, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    artifact_path(path)?;
    let resolved = crate::no_symlink_parents(root, path)?;
    let meta = std::fs::symlink_metadata(&resolved).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limit {
        return Err(format!("bounded collection input rejected: {path}"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(resolved)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("bounded collection input exceeds limit: {path}"));
    }
    Ok(bytes)
}
