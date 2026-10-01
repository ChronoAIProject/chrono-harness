//! Offline composition of explicitly addressed original endpoint pairs.
use super::*;
use chrono_harness::{
    prepared::{ArtifactTransport, Original},
    retained_artifacts, sha256,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub pair: Original,
    #[serde(default)]
    pub transport: Option<ArtifactTransport>,
    #[serde(default)]
    pub artifacts: Option<Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub sources: Vec<Source>,
}
fn bounded(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let p = no_symlink_parents(root, path)?;
    if fs::metadata(&p).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
        return Err("E_COMPOSE_BOUND: JSON original exceeds bound".into());
    }
    fs::read(p).map_err(|e| e.to_string())
}
fn source_bytes(root: &Path, source: &Source) -> Result<Vec<u8>, String> {
    let raw = if let Some(a) = &source.artifacts {
        retained_artifacts::bytes(root, a, &source.pair.path, source.transport.as_ref())?
    } else {
        chrono_harness::prepared::read_original(root, &source.pair, source.transport.as_ref())?
    };
    if sha256(&raw) != source.pair.sha256 {
        return Err("E_COMPOSE_ORIGINAL: pair digest differs".into());
    }
    Ok(raw)
}
fn import_blob(root: &Path, f: &Value, source: &Source) -> Result<Value, String> {
    let Retained::Blob {
        path,
        digest,
        length,
    } = retained_shape(f)?
    else {
        return match retained_shape(f)? {
            Retained::Absent => Ok(f.clone()),
            _ => Err("E_COMPOSE_ORIGINAL: capture blob reference required".into()),
        };
    };
    let Some(a) = &source.artifacts else {
        let original = Original {
            path: path.into(),
            sha256: digest.into(),
        };
        let p = chrono_harness::prepared::original_path(&original, source.transport.as_ref())?;
        let observed = store(root, &no_symlink_parents(root, &p)?)?;
        if observed["sha256"] != digest || observed["length"] != length {
            return Err("E_COMPOSE_BLOB: original identity differs".into());
        }
        return Ok(observed);
    };
    let dir = state(root, ".chrono-harness/state/inputs/blobs")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    if retained_artifacts::copy_to(root, a, path, source.transport.as_ref(), &mut tmp)?
        != (digest.into(), length)
    {
        return Err("E_COMPOSE_BLOB: original identity differs".into());
    }
    store(root, tmp.path())
}
/// No live business inputs, SDK probes or inferred dependencies are consulted.
pub fn compose(
    root: &Path,
    config: &str,
    base: &str,
    candidate: &str,
    manifest_path: &str,
    output: &str,
    receipt: &str,
) -> Result<Value, String> {
    state(root, output)?;
    state(root, receipt)?;
    if output == receipt {
        return Err("E_COMPOSE_PATH: output/receipt collision".into());
    }
    let raw = bounded(root, manifest_path)?;
    let manifest: Manifest = chrono_harness::decode(&raw)?;
    if manifest.schema != "chrono-input-composition/v1"
        || manifest.sources.is_empty()
        || manifest.sources.len() > 4096
    {
        return Err("E_COMPOSE_MANIFEST: schema/count".into());
    }
    let reader = facts::Reader::for_config(root, config)?;
    let old = Registrations::load(&reader.registry_values(root, base, config)?, config)?;
    let new = Registrations::load(&reader.registry_values(root, candidate, config)?, config)?;
    let mut pair = value!({});
    let mut originals = Vec::new();
    let mut seen = BTreeSet::new();
    for source in &manifest.sources {
        if !seen.insert(wire::digest(
            &serde_json::to_value(source).map_err(|e| e.to_string())?,
        )?) {
            return Err("E_COMPOSE_ORIGINAL: duplicate source".into());
        }
        let bytes = source_bytes(root, source)?;
        let v = json(&bytes)?;
        if v.as_object()
            .is_none_or(|m| m.len() != 2 || !m.contains_key("base") || !m.contains_key("candidate"))
        {
            return Err("E_COMPOSE_PAIR: exact endpoints required".into());
        }
        let retained = chrono_harness::prepared::retain_original(
            root,
            ".chrono-harness/state/inputs/originals/",
            "pair",
            &bytes,
        )?;
        originals
            .push(value!({"source":source.pair,"retained":retained,"transport":source.transport}));
        for (name, commit, r) in [("base", base, &old), ("candidate", candidate, &new)] {
            let snapshot = &v[name];
            snapshot_shape(snapshot)?;
            if snapshot["schema"] != snapshot_schema_for(r)
                || snapshot["commit"] != commit
                || snapshot["config_path"] != config
                || snapshot["config_digest"] != wire::digest(r.config())?
                || (r.selection().is_some()
                    && (snapshot["selection"] != r.selection().unwrap().clone()
                        || snapshot["effective_config_path"] != r.effective_config_path()))
            {
                return Err(format!(
                    "E_COMPOSE_HEADER: {name} configuration/endpoint/selector mismatch"
                ));
            }
            chrono_judge_registration::inputs::environment(r.config(), &snapshot["environment"])?;
            if pair.get(name).is_none() {
                pair[name] = snapshot.clone();
                pair[name]["files"] = value!({});
            }
            if pair[name]["environment"] != snapshot["environment"] {
                return Err(format!(
                    "E_COMPOSE_ENVIRONMENT: {name} conflicting originals"
                ));
            }
            for (id, file) in snapshot["files"].as_object().ok_or("snapshot files")? {
                let declared = r.config()["environment"]["inputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|i| i["id"] == *id)
                    .ok_or_else(|| format!("E_COMPOSE_INPUT: {name} unregistered {id}"))?;
                let observed = import_blob(root, file, source)?;
                if observed["absent"] == true {
                    if declared["presence"] != "absent" {
                        return Err(format!("E_COMPOSE_INPUT: {name} unexpected absence {id}"));
                    }
                } else if declared["presence"] == "absent"
                    || observed["sha256"] != declared["sha256"]
                {
                    return Err(format!(
                        "E_COMPOSE_INPUT: {name} digest/presence mismatch {id}"
                    ));
                }
                if pair[name]["files"]
                    .get(id)
                    .is_some_and(|prior| prior != &observed)
                {
                    return Err(format!("E_COMPOSE_CONFLICT: {name} overlapping input {id}"));
                }
                pair[name]["files"][id] = observed;
            }
        }
    }
    for (name, r) in [("base", &old), ("candidate", &new)] {
        for id in chrono_judge_registration::inputs::required_files(r, None)? {
            if pair[name]["files"].get(&id).is_none() {
                return Err(format!("E_COMPOSE_COVERAGE: {name} missing input {id}"));
            }
        }
    }
    publish(root, output, &pair)?;
    let provenance = value!({"schema":"chrono-input-composition-result/v1","manifest":{"path":manifest_path,"sha256":sha256(&raw)},"base":base,"candidate":candidate,"config_path":config,"output":{"path":output,"sha256":sha256(&wire::canonical(&pair)?)},"originals":originals,"completeness_proven":false});
    publish(root, receipt, &provenance)?;
    Ok(pair)
}
