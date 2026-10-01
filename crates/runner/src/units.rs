//! Typed scoped check selection and explicit report inputs. No test discovery.
use crate::{CheckConfig, relative_path};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROFILE: &str = "chrono-ci-check/v3";
pub const PROTOCOL: &str = "chrono-ci-judge/v2";

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
                artifact_path(manifest)?;
                if manifest == &c.report_path {
                    return Err("collection manifest collides with report".into());
                }
                Ok(c.report_path.clone())
            }
        }
    }
}
