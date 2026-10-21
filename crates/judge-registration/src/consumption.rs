//! Opted-in internal consumption references. These checks establish consistency,
//! not the engineering judgment that a need or an output is useful.
use crate::{execution, schema};
use chrono_harness::{facts::Reader, json, relative_path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub schema: String,
    pub shared_startup: Vec<Chain>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    pub id: String,
    pub units: String,
    pub startup: String,
    pub consumer: String,
    pub producer: String,
    pub caller: String,
    pub evidence: Evidence,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub test: String,
    pub operation: String,
}

pub fn contract(value: Option<&Value>) -> Result<BTreeMap<String, Chain>, String> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let c: Contract =
        serde_json::from_value(value.clone()).map_err(|e| format!("E_CONSUMPTION_SCHEMA: {e}"))?;
    if c.schema != "chrono-internal-causal-consumption/v1" || c.shared_startup.is_empty() {
        return Err("E_CONSUMPTION_SCHEMA: unsupported or empty contract".into());
    }
    let mut rows = BTreeMap::new();
    for chain in c.shared_startup {
        for s in [
            &chain.id,
            &chain.consumer,
            &chain.producer,
            &chain.caller,
            &chain.evidence.operation,
        ] {
            if s.is_empty() {
                return Err("E_CONSUMPTION_SCHEMA: empty identity".into());
            }
        }
        for path in [&chain.units, &chain.startup] {
            relative_path(path)?;
            if !path.starts_with(".chrono-harness/") {
                return Err("E_CONSUMPTION_SCHEMA: host path required".into());
            }
        }
        if !chain.evidence.test.starts_with("test:") || chain.evidence.test.len() == 5 {
            return Err("E_CONSUMPTION_SCHEMA: evidence test identity".into());
        }
        if rows.insert(chain.id.clone(), chain).is_some() {
            return Err("E_CONSUMPTION_SCHEMA: duplicate chain".into());
        }
    }
    Ok(rows)
}

/// Fixed endpoint data. Only paths explicitly referenced by the adopted chain
/// are loaded; unavailable bytes remain unavailable until that chain is checked.
pub struct Endpoint {
    pub adoption: Option<Value>,
    pub filemap: Value,
    pub projects: Value,
    pub config: Value,
    pub config_paths: BTreeSet<String>,
    pub documents: BTreeMap<String, Value>,
    pub unavailable: BTreeMap<String, String>,
}
impl Endpoint {
    fn document(&self, path: &str) -> Result<&Value, String> {
        if !self.filemap["files"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|f| f["path"] == path)
        {
            return Err(format!("unregistered input {path}"));
        }
        self.documents.get(path).ok_or_else(|| {
            format!(
                "missing input {path}: {}",
                self.unavailable
                    .get(path)
                    .map(String::as_str)
                    .unwrap_or("unavailable")
            )
        })
    }
    fn paths(&self, c: &Chain) -> BTreeSet<String> {
        let mut paths = self.config_paths.clone();
        paths.extend([c.units.clone(), c.startup.clone()]);
        if let Some(u) = self.documents.get(&c.units) {
            if let Some(p) = u["collection"]["check_config"].as_str() {
                paths.insert(p.into());
            }
        }
        if let Some(s) = self.documents.get(&c.startup) {
            for profile in s["consumers"][&c.consumer]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if let Some(p) = s["profiles"][profile]["config"].as_str() {
                    paths.insert(p.into());
                }
            }
        }
        paths
    }
    fn operations(&self, c: &Chain) -> BTreeSet<String> {
        let mut operations = BTreeSet::from([
            c.producer.clone(),
            c.caller.clone(),
            c.evidence.operation.clone(),
        ]);
        for path in self.paths(c) {
            if let Some(v) = self.documents.get(&path) {
                operations.extend(
                    v["operations"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
            }
        }
        operations
    }
    fn affected(
        &self,
        c: &Chain,
        paths: &BTreeSet<String>,
        nodes: &BTreeSet<String>,
    ) -> Result<bool, String> {
        let methods = execution::methods(&self.projects)?;
        Ok(self
            .paths(c)
            .iter()
            .any(|p| paths.contains(p) || nodes.contains(&format!("file:{p}")))
            || self.operations(c).iter().any(|op| {
                methods
                    .get(op)
                    .is_some_and(|m| m.iter().any(|m| nodes.contains(&m.owner)))
            })
            || nodes.contains(&c.evidence.test))
    }
}

#[derive(Default, Debug, Serialize)]
pub struct Checked {
    pub checked: BTreeSet<String>,
    pub historical: BTreeSet<String>,
    pub required_tests: BTreeSet<String>,
}

pub fn validate_delta(
    old: Option<&Endpoint>,
    new: &Endpoint,
    changed: &BTreeSet<String>,
    affected: &BTreeSet<String>,
) -> Result<Checked, String> {
    let before = old
        .map(|s| contract(s.adoption.as_ref()))
        .transpose()?
        .unwrap_or_default();
    let after = contract(new.adoption.as_ref())?;
    let mut result = Checked::default();
    for id in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        let a = before.get(id);
        let b = after.get(id);
        let selected = old.is_none()
            || a != b
            || a.zip(old)
                .map(|(c, s)| s.affected(c, changed, affected))
                .transpose()?
                .unwrap_or(false)
            || b.map(|c| new.affected(c, changed, affected))
                .transpose()?
                .unwrap_or(false);
        if !selected {
            result.historical.insert(id.clone());
            continue;
        }
        let c = b.ok_or_else(|| {
            format!(
                "E_CONSUMPTION_RETIRED: {id}: adopted chain removed; retirement is not certified"
            )
        })?;
        validate_chain(new, c).map_err(|e| format!("E_CONSUMPTION: {id}: {e}"))?;
        result.checked.insert(id.clone());
        result.required_tests.insert(c.evidence.test.clone());
    }
    Ok(result)
}

fn nonempty(v: &Value) -> Result<&str, String> {
    schema::string(v)
}
fn argv(v: &Value) -> Result<Vec<String>, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("argv: {e}"))
}
fn validate_chain(s: &Endpoint, c: &Chain) -> Result<(), String> {
    let units = s.document(&c.units)?;
    let startup = s.document(&c.startup)?;
    if units["schema"] != "chrono-github-units/v1"
        || units["job_gating"]["startup"]["schema"] != "chrono-shared-startup/v1"
        || startup["schema"] != "chrono-host-startup/v1"
    {
        return Err("unsupported startup/units schema".into());
    }
    let consumer = &units["job_gating"]["startup"]["consumers"][&c.consumer];
    nonempty(&consumer["need"])?;
    nonempty(&consumer["output_use"])?;
    if startup["directory"] != units["job_gating"]["startup"]["directory"] {
        return Err("startup output directory mismatch".into());
    }
    let unit = c
        .consumer
        .strip_prefix("unit_")
        .ok_or("internal unit caller required")?;
    if !units["units"][unit].is_object() {
        return Err("dangling unit caller".into());
    }
    let methods = execution::methods(&s.projects)?;
    let method = |id: &str| -> Result<&execution::Method, String> {
        match methods.get(id).map(Vec::as_slice) {
            Some([m]) => Ok(m),
            _ => Err(format!("missing/ambiguous operation {id}")),
        }
    };
    let command = |id: &str| -> Result<Vec<String>, String> {
        let m = method(id)?;
        let tools: Vec<_> = s.config["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|t| t["id"] == m.tool)
            .collect();
        let [tool] = tools.as_slice() else {
            return Err(format!("missing/ambiguous caller tool {}", m.tool));
        };
        // Python's registered -B flag is interpreter configuration, not an installer argument.
        let mut args = m.argv.clone();
        if args.first().is_some_and(|a| a == "-B") {
            args.remove(0);
        }
        let mut cmd = vec![nonempty(&tool["program"])?.into()];
        cmd.extend(args);
        Ok(cmd)
    };
    let produce = command(&c.producer)?;
    let install = command(&c.caller)?;
    if produce != argv(&units["job_gating"]["detector"]["bootstrap"])?
        || install != argv(&consumer["install"])?
    {
        return Err("producer/caller invocation mismatch".into());
    }
    if produce.len() != 5
        || produce[2] != "."
        || produce[3] != c.startup
        || produce[4] != "produce"
        || install.len() != 6
        || install[..4] != produce[..4]
        || install[4] != "install"
        || install[5] != c.consumer
    {
        return Err("producer/caller not bound to startup/consumer".into());
    }
    let profiles = startup["consumers"][&c.consumer]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or("missing consumer profiles")?;
    let check_path = nonempty(&units["collection"]["check_config"])?;
    let check = s.document(check_path)?;
    let verify = argv(&units["units"][unit]["bootstrap"])?;
    if verify.len() != 6
        || verify[..4] != produce[..4]
        || verify[4] != "verify"
        || !profiles.iter().any(|p| p == &verify[5])
    {
        return Err("unit startup invocation/profile mismatch".into());
    }
    let mut installed = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for profile in profiles {
        let name = nonempty(profile)?;
        let path = nonempty(&startup["profiles"][name]["config"])?;
        let config = s.document(path)?;
        if config["schema"] != "chrono-bootstrap/v1" {
            return Err("unsupported bootstrap profile".into());
        }
        let operations = config["operations"]
            .as_array()
            .filter(|v| !v.is_empty())
            .ok_or("missing producing operations")?;
        for operation in operations {
            method(nonempty(operation)?)?;
        }
        for output in config["install"]
            .as_array()
            .filter(|v| !v.is_empty())
            .ok_or("missing installed outputs")?
        {
            let source = nonempty(&output["source"])?;
            let destination = nonempty(&output["destination"])?;
            relative_path(source)?;
            relative_path(destination)?;
            if !installed.insert(destination.to_owned()) || !sources.insert(source.to_owned()) {
                return Err("ambiguous installed output".into());
            }
            let producers = operations
                .iter()
                .filter(|op| {
                    s.filemap["execution_scheduling"]["claims"][op.as_str().unwrap()]["outputs"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .any(|p| p.ends_with('/') && source.starts_with(p))
                })
                .count();
            if producers != 1 {
                return Err(format!("output {source} lacks one declared producer claim"));
            }
            if ![
                &s.config["runner"]["path"],
                &units["collection"]["generator"],
                &check["judge"]["program"],
                &units["persistent_cache"]["program"],
            ]
            .iter()
            .any(|p| p.as_str() == Some(destination))
                && !s.config["tools"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|t| t["program"] == destination)
            {
                return Err(format!(
                    "output {destination} is not a current registered tool"
                ));
            }
        }
    }
    let runner = nonempty(&s.config["runner"]["path"])?;
    for role in [
        &units["collection"]["generator"],
        &check["judge"]["program"],
        &units["persistent_cache"]["program"],
    ] {
        let path = nonempty(role)?;
        if !installed.contains(path) {
            return Err(format!("missing installed current tool {path}"));
        }
    }
    let participant = nonempty(&s.config["canonical_check"]["participation"]["tool"])?;
    let tool = s.config["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["id"] == participant)
        .ok_or("missing canonical participation tool")?;
    if !installed.contains(nonempty(&tool["program"])?) {
        return Err("missing installed canonical participation tool".into());
    }
    if !installed.contains(runner)
        || units["collection"]["runner"] != runner
        || argv(&s.config["canonical_check"]["argv"])? != [runner, "check"]
    {
        return Err("installed runner/current canonical check mismatch".into());
    }
    if s.config["canonical_check"]["profile"] != check_path {
        return Err("current check profile mismatch".into());
    }
    if check["policy"]["units"][unit]["tests"]
        .as_array()
        .is_none_or(|tests| tests.is_empty())
    {
        return Err("missing current unit check".into());
    }
    for test in check["policy"]["units"][unit]["tests"].as_array().unwrap() {
        let test = nonempty(test)?;
        if !execution::plans(&s.filemap)?.contains_key(test) {
            return Err(format!("dangling check test {test}"));
        }
    }
    let evidence = method(&c.evidence.operation)?;
    let bindings = execution::test_bindings(&s.projects)?;
    if !bindings
        .get(&c.evidence.test)
        .is_some_and(|v| v.len() == 1 && v[0].owner == evidence.owner)
        || !execution::plans(&s.filemap)?
            .get(&c.evidence.test)
            .is_some_and(|p| p.operations.contains(&c.evidence.operation))
        || s.filemap["execution_scheduling"]["claims"][&c.evidence.operation]["outputs"]
            .as_array()
            .is_none_or(|v| v.is_empty())
    {
        return Err(
            "missing/misbound required real-consumer evidence operation/plan/output".into(),
        );
    }
    Ok(())
}

/// The scoped adapter delegates admission and evidence obligations to this owner.
/// Both endpoints are immutable candidate-readable data; no old executable runs.
pub fn check_scoped(
    reader: &Reader,
    root: &Path,
    base: Option<&str>,
    candidate: &str,
    filemap: &str,
    projects: &str,
    config: Option<&str>,
    profile: &str,
    changed: &BTreeSet<String>,
    affected: &BTreeSet<String>,
) -> Result<Checked, String> {
    let load = |oid: &str| -> Result<Endpoint, String> {
        let read = |p: &str| -> Result<Value, String> {
            relative_path(p)?;
            json(&reader.blob(root, oid, p)?)
        };
        let check = read(profile)?;
        let mut s = Endpoint {
            adoption: check["policy"].get("causal_consumption").cloned(),
            filemap: read(filemap)?,
            projects: read(projects)?,
            config: Value::Null,
            config_paths: BTreeSet::new(),
            documents: BTreeMap::new(),
            unavailable: BTreeMap::new(),
        };
        let chains = contract(s.adoption.as_ref())?;
        if chains.is_empty() {
            return Ok(s);
        }
        let cfg =
            config.ok_or("E_CONSUMPTION: registration_config required for adopted contract")?;
        let snapshot = reader.registry_snapshot(root, oid, cfg)?;
        s.config = snapshot.values[&snapshot.effective_path].clone();
        s.config_paths
            .extend([cfg.to_owned(), snapshot.effective_path, profile.to_owned()]);
        let mut paths: BTreeSet<_> = chains
            .values()
            .flat_map(|c| [c.units.clone(), c.startup.clone()])
            .collect();
        for _ in 0..2 {
            for path in paths.clone() {
                if s.documents.contains_key(&path) || s.unavailable.contains_key(&path) {
                    continue;
                }
                match read(&path) {
                    Ok(v) => {
                        s.documents.insert(path, v);
                    }
                    Err(e) => {
                        s.unavailable.insert(path, e);
                    }
                }
            }
            paths.extend(chains.values().flat_map(|c| s.paths(c)));
        }
        Ok(s)
    };
    let old = base.map(load).transpose()?;
    let new = load(candidate)?;
    validate_delta(old.as_ref(), &new, changed, affected)
}
