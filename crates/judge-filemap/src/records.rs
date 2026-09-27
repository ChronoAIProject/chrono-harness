//! Stable registration record identities and set normalization, owned by FILEMAP impact.
use chrono_judge_registration::{NodeView, Registrations};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub path: String,
    pub targets: BTreeSet<String>,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordPair {
    pub base: Option<Record>,
    pub candidate: Option<Record>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub record: String,
    pub fields: Vec<String>,
}
pub fn escape(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}
/// All schema arrays are identity sets except argv/version_argv, whose order is semantic.
fn normalize(v: &Value, key: &str) -> Value {
    match v {
        Value::Array(a) => {
            let mut a: Vec<_> = a.iter().map(|v| normalize(v, "")).collect();
            if !matches!(key, "argv" | "version_argv" | "operations") {
                a.sort_by_cached_key(|v| serde_json::to_string(v).unwrap());
            }
            Value::Array(a)
        }
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), normalize(v, k)))
                .collect(),
        ),
        _ => v.clone(),
    }
}
pub fn inventory(
    r: &Registrations,
    config_path: &str,
    nodes: &BTreeMap<String, NodeView<'_>>,
    effective_environment: Option<&BTreeMap<String, String>>,
) -> BTreeMap<String, Record> {
    let mut out = BTreeMap::new();
    let mut add = |scope: &str,
                   collection: &str,
                   key: &str,
                   path: &str,
                   value: &Value,
                   targets: BTreeSet<String>| {
        out.insert(
            format!("/records/{scope}/{collection}/{}", escape(key)),
            Record {
                path: path.into(),
                value: normalize(value, collection),
                targets,
            },
        );
    };
    for (scope, value, path) in [
        ("config", r.config(), config_path),
        (
            "projects",
            r.projects(),
            r.config()["registries"]["projects"].as_str().unwrap(),
        ),
        (
            "filemap",
            r.filemap(),
            r.config()["registries"]["filemap"].as_str().unwrap(),
        ),
        (
            "judges",
            r.judges(),
            r.config()["registries"]["judges"].as_str().unwrap(),
        ),
        (
            "workflow",
            r.workflow(),
            r.config()["registries"]["workflow"].as_str().unwrap(),
        ),
    ] {
        for (key, v) in value.as_object().unwrap() {
            let identity = match (scope, key.as_str()) {
                ("filemap", "files") => Some(("path", "file")),
                ("projects", "projects") => Some(("id", "project")),
                ("projects", "scripts") => Some(("id", "script")),
                ("judges", "judges") => Some(("id", "judge")),
                ("filemap", "test_costs") => Some(("test", "")),
                ("config", "tools") => Some(("id", "tool")),
                ("workflow", "stability") => Some(("id", "")),
                ("config", "artifacts") => Some(("path", "")),
                _ => None,
            };
            if let Some((id, prefix)) = identity {
                for row in v.as_array().unwrap() {
                    let id = row[id].as_str().unwrap();
                    let target = if !prefix.is_empty() {
                        format!("{prefix}:{id}")
                    } else if key == "test_costs" {
                        format!("test:{id}")
                    } else {
                        format!("file:{path}")
                    };
                    let mut targets = BTreeSet::from([target]);
                    // Artifact ownership is an explicit declaration. Changing/removing
                    // it affects that owner at each endpoint, without deriving any
                    // source directory, language or dependency from the output path.
                    if scope == "config" && key == "artifacts" {
                        let owner = row["owner"].as_str().unwrap();
                        for prefix in ["project", "script"] {
                            let node = format!("{prefix}:{owner}");
                            if nodes.contains_key(&node) {
                                targets.insert(node);
                            }
                        }
                    }
                    // Registration owns aliases such as project/script records' executable test nodes.
                    let test = format!("test:{id}");
                    if nodes.get(&test).is_some_and(|view| {
                        view.definitions.iter().any(|d| std::ptr::eq(d.value, row))
                    }) {
                        targets.insert(test);
                    }
                    add(scope, key, id, path, row, targets);
                }
            } else if scope == "filemap" && key == "execution_plans" {
                for (test, plan) in v.as_object().unwrap() {
                    let mut targets = BTreeSet::new();
                    if let Some(view) = nodes.get(test) {
                        targets.extend(view.definitions.iter().map(|d| d.identity.clone()));
                    }
                    for operation in plan["operations"].as_array().unwrap() {
                        for (node, view) in nodes {
                            if matches!(
                                view.kind,
                                chrono_judge_registration::NodeKind::Project
                                    | chrono_judge_registration::NodeKind::Script
                            ) && view.definitions.iter().any(|d| {
                                d.value["actions"].as_object().is_some_and(|a| {
                                    a.values().any(|a| a["operation"] == *operation)
                                })
                            }) {
                                targets.insert(node.clone());
                            }
                        }
                    }
                    add(scope, key, test, path, plan, targets);
                }
            } else if scope == "filemap" && key == "project_edges" {
                for edge in v.as_array().unwrap() {
                    let id =
                        serde_json::to_string(&json!([edge["from"], edge["kind"], edge["to"]]))
                            .unwrap();
                    add(
                        scope,
                        key,
                        &id,
                        path,
                        edge,
                        BTreeSet::from([
                            edge["from"].as_str().unwrap().into(),
                            edge["to"].as_str().unwrap().into(),
                        ]),
                    );
                }
            } else if scope == "filemap" && key == "cost_models" {
                for (id, cost) in v.as_object().unwrap() {
                    let mut targets = BTreeSet::new();
                    for f in r.filemap()["files"].as_array().unwrap() {
                        if f["cost"] == *id {
                            targets.insert(format!("file:{}", f["path"].as_str().unwrap()));
                        }
                    }
                    for t in r.filemap()["test_costs"].as_array().unwrap() {
                        if t["cost"] == *id {
                            targets.insert(format!("test:{}", t["test"].as_str().unwrap()));
                        }
                    }
                    add(scope, key, id, path, cost, targets);
                }
            } else if scope == "projects" && key == "owners" {
                for owner in v.as_array().unwrap() {
                    let id = owner.as_str().unwrap();
                    let targets = r.filemap()["files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|f| f["owner"] == id)
                        .map(|f| format!("file:{}", f["path"].as_str().unwrap()))
                        .collect();
                    add(scope, key, id, path, owner, targets);
                }
            } else if scope == "config" && key == "environment" {
                for input in v["inputs"].as_array().unwrap() {
                    let id = input["id"].as_str().unwrap();
                    add(
                        scope,
                        "inputs",
                        id,
                        path,
                        input,
                        BTreeSet::from([format!("input:{id}")]),
                    );
                }
                let keys: BTreeSet<_> = v["inherit"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_str)
                    .chain(v["values"].as_object().unwrap().keys().map(String::as_str))
                    .collect();
                for name in keys {
                    let inherited = v["inherit"].as_array().unwrap().contains(&json!(name));
                    let mut value = json!({"inherit":inherited,"value":v["values"].get(name)});
                    if let Some(effective) = effective_environment {
                        value["effective"] = json!(effective.get(name));
                    }
                    add(
                        scope,
                        "environment",
                        name,
                        path,
                        &value,
                        BTreeSet::from([format!("environment:{name}")]),
                    );
                }
            } else {
                add(
                    scope,
                    "fields",
                    key,
                    path,
                    v,
                    BTreeSet::from([format!("file:{path}")]),
                );
            }
        }
    }
    out
}
pub fn changed_fields(a: Option<&Value>, b: Option<&Value>, prefix: &str, out: &mut Vec<String>) {
    if a == b {
        return;
    }
    if let (Some(Value::Object(a)), Some(Value::Object(b))) = (a, b) {
        for k in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
            changed_fields(a.get(k), b.get(k), &format!("{prefix}/{}", escape(k)), out);
        }
    } else {
        out.push(prefix.into());
    }
}
