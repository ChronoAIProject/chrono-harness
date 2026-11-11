//! Explicit Git input guards. Never discover files or register dependencies.
use crate::{input_file::observe_file, relative_path, verified_git_binding};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) fn declaration(config: &Value) -> Result<Option<Vec<Value>>, String> {
    let Some(guard) = config["facts_git"].get("guard") else {
        return Ok(None);
    };
    verified_git_binding::admit_git_guard_object(
        guard.as_object().map(|object| object.len()),
        guard["schema"].as_str(),
        guard.get("inputs").is_some(),
    )?;
    let ids = verified_git_binding::git_guard_ids(
        guard["inputs"].as_array().map(|ids| (ids, ids.len())),
    )?;
    let inputs = verified_git_binding::git_guard_files(config["environment"]["inputs"].as_array())?;
    let mut seen = BTreeSet::new();
    let mut selected = vec![];
    for id in ids {
        let id = verified_git_binding::git_guard_id(id.as_str())?;
        let index = verified_git_binding::git_guard_reference(
            &JsonGuardReferences(inputs),
            id,
            seen.insert(id),
        )
        .map_err(guard_error)?;
        let input = &inputs[index];
        let location = verified_git_binding::git_guard_location(input["location"].as_str())?;
        if !Path::new(location).is_absolute() {
            relative_path(location)?;
        }
        verified_git_binding::admit_git_guard_presence(
            id,
            input["presence"].as_str(),
            input.get("sha256").is_some(),
            input["sha256"].as_str(),
        )
        .map_err(guard_error)?;
        selected.push(input.clone());
    }
    Ok(Some(selected))
}

struct JsonGuardReferences<'a>(&'a [Value]);

impl verified_git_binding::BindingReferenceRows for JsonGuardReferences<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn id(&self, index: usize) -> Option<&str> {
        self.0[index]["id"].as_str()
    }
}

fn guard_error(error: verified_git_binding::GitGuardError<'_>) -> String {
    format!("{}{}{}", error.prefix, error.id, error.suffix)
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
