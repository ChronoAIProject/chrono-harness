//! Completeness declarations and retained endpoint bytes; a digest is not a closure proof.
use crate::Registrations;
use chrono_harness::{
    sha256,
    wire::{self, Request},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
pub fn environment(config: &Value, snapshot: &Value) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    if let Some(values) = snapshot.as_object() {
        for key in values.keys() {
            if !config["environment"]["inherit"]
                .as_array()
                .ok_or("inherit missing")?
                .contains(&json!(key))
                && config["environment"]["values"].get(key).is_none()
            {
                return Err(format!("unregistered retained environment variable {key}"));
            }
        }
    }
    for key in config["environment"]["inherit"]
        .as_array()
        .ok_or("inherit missing")?
    {
        let key = key.as_str().ok_or("inherit key")?;
        match snapshot
            .get(key)
            .ok_or_else(|| format!("missing retained environment variable {key}"))?
        {
            Value::Null => {}
            Value::String(value) => {
                out.insert(key.into(), value.clone());
            }
            _ => {
                return Err(
                    "environment must distinguish absent/null and string (including empty)".into(),
                );
            }
        }
    }
    for (key, value) in config["environment"]["values"]
        .as_object()
        .ok_or("environment values")?
    {
        out.insert(
            key.clone(),
            value.as_str().ok_or("environment value")?.into(),
        );
    }
    Ok(out)
}
pub fn validate(req: &Request, old: &Registrations, new: &Registrations) -> Result<Value, String> {
    if new.filemap()["schema_version"] == 1
        && old.config()["environment"]["inputs"]
            .as_array()
            .unwrap()
            .is_empty()
        && new.config()["environment"]["inputs"]
            .as_array()
            .unwrap()
            .is_empty()
        && req.observations["retained"].is_null()
    {
        // Historical registration-only profiles have no retained-input success claim.
        return Ok(Value::Null);
    }
    let mut endpoints = serde_json::Map::new();
    for (name, endpoint, r) in [("base", &req.base, old), ("candidate", &req.candidate, new)] {
        let cfg = r.config();
        let retained = &req.observations["retained"][name];
        let files = cfg["environment"]["inputs"]
            .as_array()
            .ok_or("inputs missing")?;
        let strict_environment = new.filemap()["schema_version"] == 2;
        if (!files.is_empty()
            || strict_environment && !cfg["environment"]["inherit"].as_array().unwrap().is_empty())
            && retained["commit"] != endpoint.commit
        {
            return Err(format!(
                "{name}: retained input snapshot absent or wrong endpoint"
            ));
        }
        let inherited = if retained.is_object() {
            &retained["environment"]
        } else if name == "candidate" {
            &req.observations["environment"]["inherited"]
        } else {
            &Value::Null
        };
        let effective = environment(cfg, inherited);
        if strict_environment && effective.is_err() {
            return Err(format!("{name}: {}", effective.unwrap_err()));
        }
        if name == "candidate"
            && strict_environment
            && serde_json::to_value(effective.as_ref().unwrap()).unwrap()
                != req.observations["environment"]["effective"]
        {
            return Err("candidate environment differs from entry observation".into());
        }
        let mut evidence = BTreeMap::new();
        for input in files {
            let id = input["id"].as_str().unwrap();
            let bytes: Vec<u8> = serde_json::from_value(retained["files"][id]["bytes"].clone())
                .map_err(|_| format!("{name}: missing retained input {id}"))?;
            let digest = sha256(&bytes);
            if input["sha256"].as_str() != Some(&digest) {
                return Err(format!("{name}: retained input digest mismatch: {id}"));
            }
            if name == "candidate" {
                let path = Path::new(input["location"].as_str().unwrap());
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    req.candidate.root.join(path)
                };
                if fs::read(&path).map_err(|e| format!("candidate input {id}: {e}"))? != bytes {
                    return Err(format!("candidate input changed: {id}"));
                }
            }
            evidence.insert(
                id.to_string(),
                json!({"sha256":digest,"length":bytes.len(),"location":input["location"]}),
            );
        }
        endpoints.insert(name.into(),json!({"commit":endpoint.commit,"environment":effective.ok(),"inherited":inherited,"files":evidence}));
    }
    let endpoints = Value::Object(endpoints);
    Ok(
        json!({"schema":"chrono-effective-inputs/v1","identity":wire::digest(&endpoints)?,"endpoints":endpoints,"completeness_proven":false}),
    )
}

/// Consume retained facts through the same environment semantics as validation.
/// Null is supported only by the historical registration-only v1 contract.
pub fn effective_environments(
    evidence: &Value,
    old: &Registrations,
    new: &Registrations,
) -> Result<Option<[BTreeMap<String, String>; 2]>, String> {
    if evidence.is_null() && new.filemap()["schema_version"] == 1 {
        return Ok(None);
    }
    if evidence["schema"] != "chrono-effective-inputs/v1"
        || evidence["identity"] != wire::digest(&evidence["endpoints"])?
    {
        return Err("effective input identity missing/mismatched".into());
    }
    let mut pair = [BTreeMap::new(), BTreeMap::new()];
    for (index, name, r) in [(0, "base", old), (1, "candidate", new)] {
        let endpoint = &evidence["endpoints"][name];
        let effective = environment(r.config(), &endpoint["inherited"])?;
        if serde_json::to_value(&effective).unwrap() != endpoint["environment"] {
            return Err(format!(
                "{name}: effective environment differs from declared retained inputs"
            ));
        }
        pair[index] = effective;
    }
    Ok(Some(pair))
}
