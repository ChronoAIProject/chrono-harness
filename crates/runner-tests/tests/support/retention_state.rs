//! Test-fixture inspection of the current addressed declarations. This view is
//! never persisted or consumed by a production executor.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn object_path(root: &Path, id: &str) -> PathBuf {
    root.join(".chrono-harness/state/output-retention-v1/objects")
        .join(id)
        .join("record.json")
}
pub fn role_path(root: &Path, role: &str) -> PathBuf {
    root.join(".chrono-harness/state/output-retention-v1/roles")
        .join(chrono_harness::sha256(role.as_bytes()))
        .join("record.json")
}
fn records(root: &Path, namespace: &str) -> Vec<Value> {
    let directory = root
        .join(".chrono-harness/state/output-retention-v1")
        .join(namespace);
    let mut paths: Vec<_> = match fs::read_dir(directory) {
        Ok(entries) => entries
            .map(|e| e.unwrap().path().join("record.json"))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(e) => panic!("fixture declaration namespace: {e}"),
    };
    paths.sort();
    paths
        .into_iter()
        .map(|p| serde_json::from_slice(&fs::read(p).unwrap()).unwrap())
        .collect()
}
pub fn snapshot(root: &Path) -> Value {
    let objects = records(root, "objects");
    let roles = records(root, "roles");
    let mut outputs = Vec::new();
    let mut summaries = Vec::new();
    let mut retired = Vec::new();
    for record in &objects {
        if !record["output"].is_null() {
            outputs.push(record["output"].clone());
        }
        summaries.extend(record["summaries"].as_array().unwrap().iter().cloned());
        retired.extend(record["retired_leases"].as_array().unwrap().iter().cloned());
    }
    let mut roots = serde_json::Map::new();
    for role in roles {
        if !role["ids"].as_array().unwrap().is_empty() {
            roots.insert(role["role"].as_str().unwrap().into(), role["ids"].clone());
        }
        summaries.extend(role["summaries"].as_array().unwrap().iter().cloned());
    }
    let policy =
        chrono_harness::sha256(&fs::read(root.join(".chrono-harness/retention.json")).unwrap());
    json!({"policy_sha256":objects.first().map(|r|r["policy_sha256"].clone()).unwrap_or(json!(policy)),
        "outputs":outputs,"roots":roots,"summaries":summaries,"retired_leases":retired})
}
pub fn replace_output(root: &Path, output: &Value) {
    let path = object_path(root, output["id"].as_str().unwrap());
    let mut record: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    record["output"] = output.clone();
    fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
}
