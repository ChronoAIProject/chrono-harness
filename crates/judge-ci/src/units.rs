use super::*;
use chrono_harness::units::{Scope, Unit};

pub(super) fn validate(config: &CheckConfig, policy: &Policy) -> Result<(), String> {
    if config.schema != chrono_harness::units::PROFILE {
        if config.policy.get("units").is_some()
            || config.policy.get("shared_operations").is_some()
            || config.policy.get("collection_limits").is_some()
        {
            return Err("CI units require chrono-ci-check/v3".into());
        }
        return Ok(());
    }
    if config.policy.get("collection_limits").is_some() && policy.collection_limits.is_none() {
        return Err("collection_limits must be an object when supplied".into());
    }
    if let Some(limits) = &policy.collection_limits {
        for (name, value) in [
            ("manifest_bytes", limits.manifest_bytes),
            ("report_bytes", limits.report_bytes),
        ] {
            if value == 0 || value >= usize::MAX as u64 {
                return Err(format!("invalid collection limit {name}: {value}"));
            }
        }
    }
    let units = policy.units.as_ref().ok_or("v3 requires explicit units")?;
    policy
        .shared_operations
        .as_ref()
        .ok_or("v3 requires explicit shared_operations")?;
    if units.is_empty() {
        return Err("CI units must be nonempty".into());
    }
    let mut paths = BTreeSet::from([config.report_path.clone()]);
    for (key, unit) in units {
        chrono_harness::units::id(key)?;
        chrono_harness::units::artifact_path(&unit.report_path)?;
        if !policy
            .artifacts
            .iter()
            .any(|a| unit.report_path.starts_with(a))
            || !paths.insert(unit.report_path.clone())
            || unit.tests.is_empty()
        {
            return Err(
                "CI unit report collision, undeclared artifact or empty test assignment".into(),
            );
        }
    }
    for a in &paths {
        for b in &paths {
            if a != b && b.starts_with(&format!("{a}/")) {
                return Err("CI report paths overlap".into());
            }
        }
    }
    Ok(())
}

pub(super) fn assignments(snapshot: &Snapshot) -> Result<(), String> {
    let Some(units) = &snapshot.policy.units else {
        return Ok(());
    };
    let mut operations: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (unit, definition) in units {
        for test in &definition.tests {
            let plan = snapshot
                .plans
                .get(test)
                .ok_or_else(|| format!("unknown CI unit plan: {test}"))?;
            for op in &plan.operations {
                operations
                    .entry(op.clone())
                    .or_default()
                    .insert(unit.clone());
            }
        }
    }
    chrono_harness::units::assignments(
        units,
        &snapshot.plans.keys().cloned().collect(),
        snapshot
            .policy
            .shared_operations
            .as_ref()
            .ok_or("missing shared operations")?,
        &operations,
    )
}
pub(super) fn selection(
    units: &BTreeMap<String, Unit>,
    selected: &BTreeSet<String>,
    unit: &str,
) -> Result<BTreeSet<String>, String> {
    chrono_harness::units::select(units, selected, unit)
}
pub(super) fn required(units: &BTreeMap<String, Unit>, selected: &BTreeSet<String>) -> Vec<String> {
    chrono_harness::units::required(units, selected)
}

pub(super) fn scope_protocol(req: &Request) -> &str {
    if req.scope.is_some() {
        chrono_harness::units::PROTOCOL
    } else {
        PROTOCOL
    }
}

pub(super) fn decorate(
    req: &Request,
    snapshot: &Snapshot,
    selected: &BTreeSet<String>,
    evidence: &mut Value,
) {
    if let Some(units) = &snapshot.policy.units {
        evidence["global_selected"] = object!(selected);
        evidence["required_units"] = object!(required(units, selected));
        evidence["execution_scope"] = object!(req.scope);
        let own: BTreeSet<String> = serde_json::from_value(evidence["selected"].clone()).unwrap();
        evidence["assigned_elsewhere"] = object!(selected.difference(&own).collect::<Vec<_>>());
        evidence["acceptance"] = object!(if matches!(req.scope, Some(Scope::Unit { .. })) {
            "unit-only; global completion requires collection"
        } else {
            "global"
        });
    }
}
