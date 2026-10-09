//! Explicit domain coverage; it validates declarations, never discovers reads.
use crate::{NodeKind, Registrations, issue, schema};
use chrono_harness::wire::Response;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn shape(coverage: &Value, closure: &Value) -> Result<(), String> {
    schema::object(coverage, &["schema", "required", "rows"], &[])?;
    if coverage["schema"] != "chrono-input-coverage/v1" {
        return Err("unsupported input coverage schema".into());
    }
    let mut required = BTreeSet::new();
    let mut consumers = BTreeSet::new();
    for scope in schema::array(&coverage["required"])? {
        schema::object(scope, &["consumer", "domains"], &[])?;
        let consumer = schema::string(&scope["consumer"])?;
        if !consumers.insert(consumer) {
            return Err(format!("duplicate coverage consumer {consumer}"));
        }
        let domains = schema::array(&scope["domains"])?;
        if domains.is_empty() {
            return Err(format!("empty coverage domains for {consumer}"));
        }
        for domain in domains {
            let domain = schema::string(domain)?;
            if !required.insert((consumer, domain)) {
                return Err(format!("duplicate coverage domain {consumer}/{domain}"));
            }
        }
    }
    if required.is_empty() {
        return Err("input coverage requires an explicit nonempty domain scope".into());
    }
    let bindings: BTreeMap<_, _> = schema::array(&closure["bindings"])?
        .iter()
        .map(|binding| (binding["id"].as_str().unwrap(), binding))
        .collect();
    let mut seen = BTreeSet::new();
    let mut covered = BTreeSet::new();
    for row in schema::array(&coverage["rows"])? {
        schema::object(
            row,
            &["consumer", "domain", "state", "bindings", "reason"],
            &[],
        )?;
        let consumer = schema::string(&row["consumer"])?;
        let domain = schema::string(&row["domain"])?;
        let key = (consumer, domain);
        if !required.contains(&key) || !seen.insert(key) {
            return Err(format!(
                "unknown or duplicate coverage row {consumer}/{domain}"
            ));
        }
        schema::string(&row["reason"])?;
        let state = schema::string(&row["state"])?;
        let ids = schema::array(&row["bindings"])?;
        match state {
            "bound" | "absent" if ids.is_empty() => {
                return Err(format!(
                    "coverage {state} requires bindings: {consumer}/{domain}"
                ));
            }
            "bound" | "absent" => {}
            "not-applicable" | "unresolved" => {
                if !ids.is_empty() {
                    return Err(format!("coverage {state} cannot carry bindings"));
                }
                if state == "unresolved" && closure["status"] == "declared-complete" {
                    return Err(format!("unresolved coverage domain {consumer}/{domain}"));
                }
            }
            _ => return Err(format!("unsupported coverage state {state}")),
        }
        let mut row_ids = BTreeSet::new();
        for id in ids {
            let id = schema::string(id)?;
            let binding = bindings
                .get(id)
                .ok_or_else(|| format!("unknown coverage binding {id}"))?;
            if binding["consumer"] != consumer || !row_ids.insert(id) {
                return Err(format!("wrong consumer or duplicate coverage binding {id}"));
            }
            if schema::array(&binding["inputs"])?.is_empty() {
                return Err(format!("coverage binding {id} has no inputs"));
            }
            covered.insert(id);
        }
    }
    if required != seen {
        return Err(format!(
            "missing coverage rows: {:?}",
            required.difference(&seen).collect::<Vec<_>>()
        ));
    }
    if closure["status"] == "declared-complete" {
        for id in bindings.keys() {
            if !covered.contains(id) {
                return Err(format!("declared-complete coverage omits binding {id}"));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate(registrations: &Registrations, response: &mut Response) {
    let closure = &registrations.config()["input_closure"];
    let Some(coverage) = closure.get("coverage") else {
        return;
    };
    let nodes = registrations.node_data();
    for (index, scope) in coverage["required"].as_array().unwrap().iter().enumerate() {
        let consumer = scope["consumer"].as_str().unwrap();
        if !nodes.get(consumer).is_some_and(|node| {
            matches!(
                node.kind,
                NodeKind::Project | NodeKind::Script | NodeKind::Test | NodeKind::Judge
            )
        }) {
            issue(
                response,
                "E_REFERENCE",
                format!("unknown or non-executable coverage consumer {consumer}"),
                format!("/config/input_closure/coverage/required/{index}/consumer"),
                false,
            );
        }
    }
    let bindings: BTreeMap<_, _> = closure["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|binding| (binding["id"].as_str().unwrap(), binding))
        .collect();
    let inputs: BTreeMap<_, _> = registrations.config()["environment"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| (input["id"].as_str().unwrap(), input))
        .collect();
    for (index, row) in coverage["rows"].as_array().unwrap().iter().enumerate() {
        if row["state"] != "absent" {
            continue;
        }
        for id in row["bindings"].as_array().unwrap() {
            for input in bindings[id.as_str().unwrap()]["inputs"].as_array().unwrap() {
                let absent = input
                    .as_str()
                    .and_then(|node| node.strip_prefix("input:"))
                    .and_then(|id| inputs.get(id))
                    .is_some_and(|value| value["presence"] == "absent");
                if !absent {
                    issue(
                        response,
                        "E_INPUT_UNDECLARED",
                        format!(
                            "absent coverage requires an explicitly absent file input: {input}"
                        ),
                        format!("/config/input_closure/coverage/rows/{index}/bindings"),
                        false,
                    );
                }
            }
        }
    }
}
