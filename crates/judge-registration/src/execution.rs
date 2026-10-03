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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub resources: Vec<String>,
    pub outputs: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scheduling {
    pub max_running: usize,
    pub resources: Vec<String>,
    pub claims: BTreeMap<String, Claims>,
}
impl Scheduling {
    pub fn validate(&self) -> Result<(), String> {
        let unique = |items: &[String]| {
            items.iter().all(|v| !v.is_empty() && !v.contains('\0'))
                && items.iter().collect::<BTreeSet<_>>().len() == items.len()
        };
        if self.max_running == 0 || !unique(&self.resources) {
            return Err(
                "E_SCHEDULING: positive max_running and unique resource names required".into(),
            );
        }
        for (op, claims) in &self.claims {
            if op.is_empty()
                || !unique(&claims.resources)
                || !unique(&claims.outputs)
                || claims
                    .outputs
                    .iter()
                    .map(|p| std::path::Path::new(p))
                    .collect::<BTreeSet<_>>()
                    .len()
                    != claims.outputs.len()
                || claims.resources.iter().any(|r| !self.resources.contains(r))
                || claims
                    .outputs
                    .iter()
                    .any(|p| chrono_harness::relative_path(p.trim_end_matches('/')).is_err())
            {
                return Err(format!("E_SCHEDULING: invalid claims for {op}"));
            }
        }
        Ok(())
    }
    pub fn conflicts(&self, a: &str, b: &str) -> bool {
        let a = &self.claims[a];
        let b = &self.claims[b];
        a.resources.iter().any(|r| b.resources.contains(r))
            || a.outputs.iter().any(|x| {
                b.outputs
                    .iter()
                    .any(|y| chrono_harness::units::overlap(x, y))
            })
    }
}
/// Interpret the complete host policy before tool observation or operation effects.
pub fn scheduling(
    filemap: &Value,
    projects: &Value,
    artifacts: &Value,
) -> Result<Option<Scheduling>, String> {
    let Some(value) = filemap.get("execution_scheduling") else {
        return Ok(None);
    };
    let policy: Scheduling =
        serde_json::from_value(value.clone()).map_err(|e| format!("E_SCHEDULING: {e}"))?;
    policy.validate()?;
    let participating: BTreeSet<_> = plans(filemap)?
        .values()
        .flat_map(|p| p.operations.iter().cloned())
        .collect();
    if policy.claims.keys().cloned().collect::<BTreeSet<_>>() != participating {
        return Err("E_SCHEDULING: claims must name every participating operation exactly once, including empty claims".into());
    }
    let methods = methods(projects)?;
    for (operation, claims) in &policy.claims {
        if methods.get(operation).is_none_or(|m| m.len() != 1) {
            return Err(format!(
                "E_SCHEDULING: unknown or ambiguous operation {operation}"
            ));
        }
        for output in &claims.outputs {
            let matches: Vec<_> = artifacts
                .as_array()
                .ok_or("E_SCHEDULING: artifact declarations missing")?
                .iter()
                .filter(|a| {
                    a["path"]
                        .as_str()
                        .is_some_and(|p| std::path::Path::new(p) == std::path::Path::new(output))
                })
                .collect();
            if matches.len() != 1 || matches[0]["path"] != *output {
                return Err(format!(
                    "E_SCHEDULING: output must name one registered artifact path: {operation}: {output}"
                ));
            }
        }
    }
    Ok(Some(policy))
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
/// A test identity is opaque; its owner and terminal action are explicit registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestBinding {
    pub owner: String,
    pub action: String,
    pub operation: String,
}

/// Shared by full and scoped consumers. Legacy scripts retain their single execute alias.
pub fn test_bindings(projects: &Value) -> Result<BTreeMap<String, Vec<TestBinding>>, String> {
    let mut out: BTreeMap<String, Vec<TestBinding>> = BTreeMap::new();
    let mut explicit = BTreeSet::new();
    for (collection, prefix) in [("projects", "project"), ("scripts", "script")] {
        for row in projects[collection]
            .as_array()
            .ok_or("projects collection missing")?
        {
            let id = row["id"].as_str().ok_or("project ID missing")?;
            let owner = format!("{prefix}:{id}");
            let actions = row["actions"].as_object().ok_or("actions missing")?;
            let groups = if let Some(groups) = row.get("test_groups") {
                if collection != "projects" || row["kind"] != "test" {
                    return Err(format!(
                        "E_TEST_BINDING: test_groups requires a test project: {owner}"
                    ));
                }
                let groups = groups
                    .as_object()
                    .ok_or("E_TEST_BINDING: test_groups must be a mapping")?;
                if groups.is_empty() || !groups.contains_key(id) {
                    return Err(format!(
                        "E_TEST_BINDING: test_groups must include legacy identity {id}"
                    ));
                }
                groups
                    .iter()
                    .map(|(test, action)| {
                        Ok((
                            test.as_str(),
                            action
                                .as_str()
                                .ok_or("E_TEST_BINDING: action key must be a string")?,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?
            } else if actions.contains_key("execute")
                && (collection == "projects"
                    && (row["kind"] == "test" || row.get("kind").is_none())
                    || collection == "scripts"
                        && (row.get("tests_for").is_some() || row.get("test_script").is_none()))
            {
                vec![(id, "execute")]
            } else {
                vec![]
            };
            let mut selected = BTreeSet::new();
            for (test, key) in groups {
                if test.is_empty() || key.is_empty() || !selected.insert(key) {
                    return Err(format!(
                        "E_TEST_BINDING: empty or repeated action binding: {owner}"
                    ));
                }
                let action = actions
                    .get(key)
                    .ok_or_else(|| format!("E_TEST_BINDING: {owner} has no action {key}"))?;
                let operation = action["operation"]
                    .as_str()
                    .ok_or("E_TEST_BINDING: operation missing")?;
                let binding = TestBinding {
                    owner: owner.clone(),
                    action: key.into(),
                    operation: operation.into(),
                };
                let identity = format!("test:{test}");
                if out.contains_key(&identity)
                    && (row.get("test_groups").is_some() || explicit.contains(&identity))
                {
                    return Err(format!(
                        "E_TEST_BINDING: duplicate test identity {identity}"
                    ));
                }
                if row.get("test_groups").is_some() {
                    explicit.insert(identity.clone());
                }
                // Legacy ambiguity stays visible for DELTA-local adjudication, never a winner.
                out.entry(identity).or_default().push(binding);
            }
        }
    }
    Ok(out)
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
