//! Explicit Git input guards. Never discover files or register dependencies.
use crate::{input_file::observe_file, relative_path, wire};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) fn declaration(config: &Value) -> Result<Option<Vec<Value>>, String> {
    let Some(guard) = config["facts_git"].get("guard") else {
        return Ok(None);
    };
    let object = guard
        .as_object()
        .ok_or("Git input guard must be an object")?;
    if object.len() != 2
        || guard["schema"] != "chrono-git-inputs/v1"
        || !object.contains_key("inputs")
    {
        return Err("Git input guard requires only schema chrono-git-inputs/v1 and inputs".into());
    }
    let ids = guard["inputs"]
        .as_array()
        .filter(|ids| !ids.is_empty())
        .ok_or("Git input guard requires nonempty inputs")?;
    let inputs = config["environment"]["inputs"]
        .as_array()
        .ok_or("Git input guard requires file inputs")?;
    let mut seen = BTreeSet::new();
    let mut selected = vec![];
    for id in ids {
        let id = id
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("invalid Git input ID")?;
        if !seen.insert(id) {
            return Err(format!("duplicate Git input {id}"));
        }
        let matches: Vec<_> = inputs.iter().filter(|input| input["id"] == id).collect();
        if matches.len() != 1 {
            return Err(format!("missing/ambiguous Git input {id}"));
        }
        let input = matches[0];
        let location = input["location"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Git input location required")?;
        if !Path::new(location).is_absolute() {
            relative_path(location)?;
        }
        match input["presence"].as_str() {
            Some("absent") if input.get("sha256").is_none() => {}
            Some("present") if input["sha256"].as_str().is_some_and(wire::is_digest) => {}
            _ => {
                return Err(format!(
                    "Git input {id} requires explicit absence or a present digest"
                ));
            }
        }
        selected.push(input.clone());
    }
    Ok(Some(selected))
}

pub(crate) struct Guard {
    entries: Vec<Value>,
    pub observation: Value,
}
impl Guard {
    pub fn open(root: &Path, config: &Value) -> Result<Option<Self>, String> {
        declaration(config)?
            .map(|entries| {
                let observation = observe(root, &entries)?;
                Ok(Self {
                    entries,
                    observation,
                })
            })
            .transpose()
    }
    pub fn unchanged(&self, root: &Path) -> Result<(), String> {
        if observe(root, &self.entries)? != self.observation {
            return Err("Git input observation changed".into());
        }
        Ok(())
    }
}
fn observe(root: &Path, entries: &[Value]) -> Result<Value, String> {
    let mut files = BTreeMap::new();
    for input in entries {
        let id = input["id"].as_str().ok_or("Git input ID")?;
        let path = Path::new(input["location"].as_str().ok_or("Git input location")?);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        let identity = observe_file(&path).map_err(|e| format!("Git input {id}: {e}"))?;
        let observation = match identity {
            None if input["presence"] == "absent" => json!({"presence":"absent","location":path}),
            Some((digest, length))
                if input["presence"] == "present" && input["sha256"] == digest =>
            {
                json!({"presence":"present","location":path,"sha256":digest,"length":length})
            }
            _ => return Err(format!("Git input {id} digest/presence mismatch")),
        };
        files.insert(id, observation);
    }
    Ok(json!({"schema":"chrono-git-inputs-result/v1","files":files,"completeness_proven":false}))
}
