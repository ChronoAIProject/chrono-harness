//! Shared strict registration data; loading does not assert readiness or semantic validity.
use crate::{Result, schema};
use chrono_harness::facts;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub struct Registrations {
    entry_path: String,
    effective_path: String,
    selection: Option<serde_json::Value>,
    entry: Value,
    pub(crate) config: Value,
    pub(crate) judges: Value,
    pub(crate) projects: Value,
    pub(crate) filemap: Value,
    pub(crate) workflow: Value,
    pub(crate) historical: BTreeMap<String, Vec<(String, Value)>>,
    pub(crate) original_schemas: BTreeMap<String, (String, u64)>,
}
impl Registrations {
    pub fn load(v: &BTreeMap<String, Value>, config_path: &str) -> Result<Self> {
        let identity = facts::registry_identity(v, config_path)
            .map_err(|e| format!("configuration selection: {e}"))?;
        let config = v
            .get(&identity.effective_path)
            .ok_or("missing effective config")?
            .clone();
        schema::config(&config).map_err(|e| format!("config: {e}"))?;
        let get = |k: &str| -> Result<Value> {
            Ok(v.get(
                config["registries"][k]
                    .as_str()
                    .ok_or("missing registry path")?,
            )
            .ok_or("missing registry bytes")?
            .clone())
        };
        let s = Self {
            entry: v[config_path].clone(),
            entry_path: identity.entry_path,
            effective_path: identity.effective_path,
            selection: identity.selection,
            judges: get("judges")?,
            projects: get("projects")?,
            filemap: get("filemap")?,
            workflow: get("workflow")?,
            config,
            historical: BTreeMap::new(),
            original_schemas: Self::schemas(v, config_path)?,
        };
        for (name, v, check) in [
            (
                "judges",
                &s.judges,
                schema::judges as fn(&Value) -> Result<()>,
            ),
            ("projects", &s.projects, schema::projects),
            ("filemap", &s.filemap, schema::filemap),
            ("workflow", &s.workflow, schema::workflow),
        ] {
            check(v).map_err(|e| format!("{name}: {e}"))?;
        }
        Ok(s)
    }
    /// Structurally validated values; semantic admissibility is still registration policy.
    pub fn config(&self) -> &Value {
        &self.config
    }
    /// Stable CLI/profile entry identity supplied by the caller.
    pub fn entry_path(&self) -> &str {
        &self.entry_path
    }
    pub fn entry(&self) -> &Value {
        &self.entry
    }
    /// Direct policy path selected for this endpoint.
    pub fn effective_config_path(&self) -> &str {
        &self.effective_path
    }
    /// Selector binding metadata, when the entry was a platform map.
    pub fn selection(&self) -> Option<&Value> {
        self.selection.as_ref()
    }
    pub(crate) fn schemas(
        v: &BTreeMap<String, Value>,
        config_path: &str,
    ) -> Result<BTreeMap<String, (String, u64)>> {
        let effective = facts::registry_identity(v, config_path)?.effective_path;
        let config = v.get(&effective).ok_or("missing effective config")?;
        let mut out = BTreeMap::new();
        for key in ["config", "projects", "filemap", "judges", "workflow"] {
            let path = if key == "config" {
                effective.as_str()
            } else {
                config["registries"][key]
                    .as_str()
                    .ok_or("original registry path")?
            };
            let version = v
                .get(path)
                .and_then(|v| v["schema_version"].as_u64())
                .ok_or("original schema version")?;
            out.insert(key.into(), (path.into(), version));
        }
        Ok(out)
    }
    /// Fixed original schema identities, before an explicit historical decoder.
    pub fn original_schemas(&self) -> &BTreeMap<String, (String, u64)> {
        &self.original_schemas
    }
    pub fn judges(&self) -> &Value {
        &self.judges
    }
    pub fn projects(&self) -> &Value {
        &self.projects
    }
    pub fn filemap(&self) -> &Value {
        &self.filemap
    }
    pub fn workflow(&self) -> &Value {
        &self.workflow
    }
    pub fn nodes(&self) -> BTreeSet<String> {
        self.node_data().into_keys().collect()
    }
    /// The authoritative node inventory. Every definition is retained, including alias collisions.
    /// Values borrow immutable validated data; only `NodeView::unique` resolves a single definition.
    pub fn node_data(&self) -> BTreeMap<String, NodeView<'_>> {
        let mut nodes: BTreeMap<String, NodeView<'_>> = BTreeMap::new();
        for (prefix, kind, value, key, id) in [
            ("file", NodeKind::File, &self.filemap, "files", "path"),
            (
                "project",
                NodeKind::Project,
                &self.projects,
                "projects",
                "id",
            ),
            ("script", NodeKind::Script, &self.projects, "scripts", "id"),
            (
                "input",
                NodeKind::Input,
                &self.config["environment"],
                "inputs",
                "id",
            ),
            ("judge", NodeKind::Judge, &self.judges, "judges", "id"),
            ("tool", NodeKind::Tool, &self.config, "tools", "id"),
        ] {
            for row in value[key].as_array().unwrap() {
                let identity = format!("{prefix}:{}", row[id].as_str().unwrap());
                let definition = NodeDefinition {
                    identity: identity.clone(),
                    value: row,
                };
                nodes.insert(
                    identity,
                    NodeView {
                        kind,
                        definitions: vec![definition.clone()],
                    },
                );
                if (kind == NodeKind::Project && row["kind"] == "test"
                    || kind == NodeKind::Script && row.get("tests_for").is_some())
                    && row["actions"].get("execute").is_some()
                {
                    nodes
                        .entry(format!("test:{}", row[id].as_str().unwrap()))
                        .or_insert_with(|| NodeView {
                            kind: NodeKind::Test,
                            definitions: vec![],
                        })
                        .definitions
                        .push(definition);
                }
            }
        }
        for key in self.config["environment"]["inherit"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .chain(
                self.config["environment"]["values"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str),
            )
        {
            let identity = format!("environment:{key}");
            nodes.entry(identity.clone()).or_insert_with(|| NodeView {
                kind: NodeKind::Environment,
                definitions: vec![NodeDefinition {
                    identity,
                    value: &self.config["environment"],
                }],
            });
        }
        for (identity, definitions) in &self.historical {
            let kind = if identity.starts_with("test:") {
                NodeKind::Test
            } else {
                NodeKind::Script
            };
            let view = nodes.entry(identity.clone()).or_insert_with(|| NodeView {
                kind,
                definitions: vec![],
            });
            for (source, value) in definitions {
                view.definitions.push(NodeDefinition {
                    identity: source.clone(),
                    value,
                });
            }
        }
        nodes
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Project,
    Script,
    Input,
    Test,
    Judge,
    Tool,
    Environment,
}
#[derive(Clone, Debug)]
pub struct NodeDefinition<'a> {
    /// Identity of the defining record, e.g. project:t or script:t for the alias test:t.
    pub identity: String,
    pub value: &'a Value,
}
#[derive(Clone, Debug)]
pub struct NodeView<'a> {
    pub kind: NodeKind,
    pub definitions: Vec<NodeDefinition<'a>>,
}
impl<'a> NodeView<'a> {
    pub fn unique(&self) -> Option<&NodeDefinition<'a>> {
        match self.definitions.as_slice() {
            [definition] => Some(definition),
            _ => None,
        }
    }
}
