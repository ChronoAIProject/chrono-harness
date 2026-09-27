//! Immutable execution declarations. This module neither selects nor launches work.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub operations: Vec<String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
pub fn plans(filemap: &Value) -> Result<BTreeMap<String, Plan>, String> {
    let plans: BTreeMap<String, Plan> = serde_json::from_value(filemap["execution_plans"].clone())
        .map_err(|e| format!("E_EXECUTION_PLAN: {e}"))?;
    for (test, plan) in &plans {
        if !test.starts_with("test:")
            || test.len() == 5
            || plan.operations.is_empty()
            || plan.operations.iter().any(|s| s.is_empty())
            || plan.operations.iter().collect::<BTreeSet<_>>().len() != plan.operations.len()
            || plan.timeout_seconds == 0
            || plan.output_limit_bytes == 0
            || plan.output_limit_bytes > 64 * 1024 * 1024
        {
            return Err(format!("E_EXECUTION_PLAN: invalid plan {test}"));
        }
    }
    Ok(plans)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
}
/// Retain duplicates for the routes policy to reject; never overwrite a method.
pub fn methods(projects: &Value) -> Result<BTreeMap<String, Vec<Method>>, String> {
    let mut out: BTreeMap<String, Vec<Method>> = BTreeMap::new();
    for (key, prefix) in [("projects", "project"), ("scripts", "script")] {
        for row in projects[key]
            .as_array()
            .ok_or("projects collection missing")?
        {
            for action in row["actions"]
                .as_object()
                .ok_or("actions missing")?
                .values()
            {
                let operation = action["operation"].as_str().ok_or("operation missing")?;
                let method = Method {
                    owner: format!("{prefix}:{}", row["id"].as_str().ok_or("owner missing")?),
                    operation: operation.into(),
                    tool: action["tool"].as_str().ok_or("tool missing")?.into(),
                    argv: serde_json::from_value(action["argv"].clone())
                        .map_err(|e| e.to_string())?,
                };
                out.entry(operation.into()).or_default().push(method);
            }
        }
    }
    Ok(out)
}
