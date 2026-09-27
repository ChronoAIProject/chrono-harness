//! Shared strict registration data; loading does not assert readiness or semantic validity.
use crate::{Result, schema};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub struct Registrations {
    pub(crate) config: Value,
    pub(crate) judges: Value,
    pub(crate) projects: Value,
    pub(crate) filemap: Value,
    pub(crate) workflow: Value,
}
impl Registrations {
    pub fn load(v: &BTreeMap<String, Value>, config_path: &str) -> Result<Self> {
        let config = v.get(config_path).ok_or("missing config")?.clone();
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
            judges: get("judges")?,
            projects: get("projects")?,
            filemap: get("filemap")?,
            workflow: get("workflow")?,
            config,
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
