//! 当前 v5 发布登记；先核完整关系，再为所选单元解析实际工具。
use crate::verified_recipe::{self as policy, Invocation};
use chrono_harness::{no_symlink_parents, resolve_program};
use chrono_judge_registration::execution::{Method, methods};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const CONFIG: &str = ".chrono-harness/release/build.json";

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_RECIPE: {message}")
}

pub fn read_json(root: &Path, relative: &str) -> Result<Value, String> {
    let path = no_symlink_parents(root, relative)?;
    chrono_harness::json(&fs::read(&path).map_err(|e| error(format!("{}: {e}", path.display())))?)
}

pub fn overlaps(a: &str, b: &str) -> bool {
    chrono_harness::units::overlap(a, b)
}

fn names(values: &[String], nonempty: bool) -> bool {
    (!nonempty || !values.is_empty())
        && values.iter().all(|s| !s.is_empty() && !s.contains('\0'))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn same_members<'a>(
    actual: impl Iterator<Item = &'a str>,
    expected: impl Iterator<Item = &'a str>,
) -> bool {
    let mut actual: Vec<_> = actual.collect();
    let mut expected: Vec<_> = expected.collect();
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Unit {
    #[serde(rename = "build")]
    Build {
        id: String,
        manifest: String,
        asset: String,
        needs: Vec<String>,
        handoff: String,
    },
    #[serde(rename = "verify")]
    Verify {
        id: String,
        operation: String,
        rust_toolchain: bool,
        needs: Vec<String>,
        handoff: String,
    },
}

impl Unit {
    pub fn id(&self) -> &str {
        match self {
            Self::Build { id, .. } | Self::Verify { id, .. } => id,
        }
    }
    pub fn needs(&self) -> &[String] {
        match self {
            Self::Build { needs, .. } | Self::Verify { needs, .. } => needs,
        }
    }
    pub fn handoff(&self) -> &str {
        match self {
            Self::Build { handoff, .. } | Self::Verify { handoff, .. } => handoff,
        }
    }
    pub fn invocation(&self) -> Invocation {
        match self {
            Self::Build { .. } => Invocation::Build,
            Self::Verify { rust_toolchain, .. } => Invocation::Verify {
                rust_toolchain: *rust_toolchain,
            },
        }
    }
    pub fn asset(&self) -> Option<&str> {
        match self {
            Self::Build { asset, .. } => Some(asset),
            _ => None,
        }
    }
    pub fn operation(&self) -> Option<&str> {
        match self {
            Self::Verify { operation, .. } => Some(operation),
            _ => None,
        }
    }
    pub fn value(&self) -> Value {
        serde_json::to_value(self).expect("serializable release unit")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Collector {
    pub id: String,
    pub needs: Vec<String>,
    pub package_asset: String,
    pub dependency_metadata: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub unit_seconds: u64,
    pub platform_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub need: String,
    pub release_assets: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub asset: String,
    pub destinations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Staging {
    pub release_plan: String,
    pub host_config: String,
    pub bindings: Vec<Binding>,
}

fn present_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
    bool::deserialize(d).map(Some)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema: String,
    pub projects: String,
    pub tools: BTreeMap<String, String>,
    pub rust_toolchain: String,
    pub rust_components: Vec<String>,
    #[serde(
        default,
        deserialize_with = "present_bool",
        skip_serializing_if = "Option::is_none"
    )]
    pub rust_incremental: Option<bool>,
    pub manifests: Vec<String>,
    pub verification_operations: Vec<String>,
    pub consumer_staging: Staging,
    pub units: Vec<Unit>,
    pub collector: Collector,
    pub local_workers: usize,
    pub native_jobs: BTreeMap<String, BTreeMap<String, String>>,
    pub unit_costs: BTreeMap<String, String>,
    pub limits: Limits,
    #[serde(default)]
    pub failure_evidence: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub verification_consumers: BTreeMap<String, Consumer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasePlan {
    pub schema: String,
    pub version: String,
    pub assets: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StageRow {
    pub asset: String,
    pub source: String,
    pub destination: String,
}

#[derive(Debug)]
pub struct Loaded {
    pub recipe: Recipe,
    pub plan: ReleasePlan,
    pub actions: BTreeMap<String, Method>,
    pub staging: Vec<StageRow>,
    pub contracts: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug)]
pub enum Scope<'a> {
    Unit(&'a str),
    Collect,
}

impl Loaded {
    pub fn load(root: &Path) -> Result<Self, String> {
        let recipe: Recipe = serde_json::from_value(read_json(root, CONFIG)?).map_err(error)?;
        if recipe.schema != "chrono-release-build/v5" {
            return Err(error("unsupported release build schema"));
        }
        if !names(&recipe.manifests, true)
            || !names(&recipe.verification_operations, true)
            || !names(&recipe.rust_components, false)
            || recipe.rust_toolchain.is_empty()
            || recipe
                .tools
                .iter()
                .any(|(n, p)| n.is_empty() || p.is_empty() || p.contains('\0'))
        {
            return Err(error("invalid manifests, operations, components or tools"));
        }
        for manifest in &recipe.manifests {
            no_symlink_parents(root, manifest)?;
        }
        let staging = &recipe.consumer_staging;
        let plan: ReleasePlan =
            serde_json::from_value(read_json(root, &staging.release_plan)?).map_err(error)?;
        if plan.schema != "chrono-release-plan/v1"
            || plan.assets.is_empty()
            || plan.version.is_empty()
        {
            return Err(error("unsupported or empty consumer release plan"));
        }
        let sources: Vec<_> = plan.assets.values().cloned().collect();
        for source in &sources {
            no_symlink_parents(root, source)?;
        }
        let host = read_json(root, &staging.host_config)?;
        let artifacts = host["artifacts"]
            .as_array()
            .ok_or_else(|| error("missing host artifacts"))?;
        let mut rows = Vec::new();
        let mut bound_assets = BTreeSet::new();
        let mut destinations = Vec::<String>::new();
        for binding in &staging.bindings {
            let source = plan
                .assets
                .get(&binding.asset)
                .ok_or_else(|| error("unknown consumer asset"))?;
            if !bound_assets.insert(binding.asset.as_str()) || !names(&binding.destinations, true) {
                return Err(error("duplicate consumer asset or invalid destinations"));
            }
            for destination in &binding.destinations {
                no_symlink_parents(root, destination)?;
                let owners: Vec<_> = artifacts
                    .iter()
                    .filter(|a| {
                        a["path"].as_str().is_some_and(|p| {
                            if p.ends_with('/') {
                                destination.starts_with(p)
                            } else {
                                destination == p
                            }
                        })
                    })
                    .collect();
                if owners.len() != 1 || owners[0]["tracked"] != false {
                    return Err(error("consumer destination needs one untracked artifact"));
                }
                no_symlink_parents(
                    root,
                    owners[0]["path"].as_str().unwrap().trim_end_matches('/'),
                )?;
                if destinations
                    .iter()
                    .chain(&sources)
                    .any(|p| overlaps(destination, p))
                {
                    return Err(error("overlapping consumer destination"));
                }
                destinations.push(destination.clone());
                rows.push(StageRow {
                    asset: binding.asset.clone(),
                    source: source.clone(),
                    destination: destination.clone(),
                });
            }
        }
        if bound_assets != plan.assets.keys().map(String::as_str).collect() {
            return Err(error("release staging must cover every asset"));
        }
        let registered = methods(&read_json(root, &recipe.projects)?)?;
        if registered.values().any(|v| v.len() != 1) {
            return Err(error("ambiguous registered operation"));
        }
        let mut actions = BTreeMap::new();
        for operation in &recipe.verification_operations {
            let action = registered
                .get(operation)
                .and_then(|v| v.first())
                .ok_or_else(|| error(format!("unknown registered operation: {operation}")))?;
            if !recipe.tools.contains_key(&action.tool)
                || action.argv.iter().any(|a| a.contains('\0'))
            {
                return Err(error("invalid operation tool or argv"));
            }
            actions.insert(operation.clone(), action.clone());
        }
        let mut ids = BTreeSet::new();
        let mut builds = BTreeMap::new();
        let mut tests = BTreeMap::new();
        let mut handoffs = Vec::<String>::new();
        for unit in &recipe.units {
            if !chrono_identifiers::valid_id(unit.id().as_bytes())
                || unit.id().contains('.')
                || !ids.insert(unit.id().to_owned())
            {
                return Err(error("invalid or duplicate release unit"));
            }
            match unit {
                Unit::Build {
                    manifest,
                    asset,
                    needs,
                    ..
                } => {
                    if !needs.is_empty()
                        || !recipe.manifests.contains(manifest)
                        || !plan.assets.contains_key(asset)
                    {
                        return Err(error(
                            "unknown build manifest/asset or nonempty build needs",
                        ));
                    }
                    builds.insert(unit.id(), unit);
                }
                Unit::Verify { operation, .. } => {
                    if !recipe.verification_operations.contains(operation) {
                        return Err(error("unknown verification operation"));
                    }
                    tests.insert(unit.id(), unit);
                }
            }
            no_symlink_parents(root, unit.handoff())?;
            if handoffs
                .iter()
                .chain(&sources)
                .chain(&destinations)
                .any(|p| overlaps(unit.handoff(), p))
            {
                return Err(error("overlapping release handoff directory"));
            }
            handoffs.push(unit.handoff().to_owned());
        }
        if !same_members(
            builds.values().map(|u| match u {
                Unit::Build { manifest, .. } => manifest.as_str(),
                _ => unreachable!(),
            }),
            recipe.manifests.iter().map(String::as_str),
        ) || !same_members(
            builds.values().map(|u| u.asset().unwrap()),
            plan.assets.keys().map(String::as_str),
        ) || !same_members(
            tests.values().map(|u| u.operation().unwrap()),
            recipe.verification_operations.iter().map(String::as_str),
        ) {
            return Err(error(
                "units must cover each manifest, asset and unfiltered operation exactly once",
            ));
        }
        for unit in tests.values() {
            if !names(unit.needs(), false)
                || unit
                    .needs()
                    .iter()
                    .any(|n| !builds.contains_key(n.as_str()))
            {
                return Err(error(
                    "verification needs must be unique declared build units",
                ));
            }
        }
        let collector = &recipe.collector;
        if !chrono_identifiers::valid_id(collector.id.as_bytes())
            || collector.id.contains('.')
            || ids.contains(&collector.id)
            || !same_members(
                collector.needs.iter().map(String::as_str),
                ids.iter().map(String::as_str),
            )
            || !plan.assets.contains_key(&collector.package_asset)
        {
            return Err(error(
                "collector needs exact build/test membership and package asset",
            ));
        }
        no_symlink_parents(root, &collector.dependency_metadata)?;
        if handoffs
            .iter()
            .any(|p| overlaps(&collector.dependency_metadata, p))
        {
            return Err(error("collector metadata overlaps handoff"));
        }
        ids.insert(collector.id.clone());
        if ids != recipe.unit_costs.keys().cloned().collect()
            || recipe.unit_costs.values().any(String::is_empty)
            || recipe.units.is_empty()
            || !(1..=recipe.units.len()).contains(&recipe.local_workers)
            || recipe.limits.unit_seconds == 0
            || recipe.limits.platform_seconds == 0
        {
            return Err(error(
                "release units need exact costs and positive limits/workers",
            ));
        }
        for bindings in recipe.native_jobs.values() {
            if ids != bindings.keys().cloned().collect()
                || !names(&bindings.values().cloned().collect::<Vec<_>>(), true)
            {
                return Err(error(
                    "native jobs need explicit exact unique unit bindings",
                ));
            }
        }
        for (operation, directories) in &recipe.failure_evidence {
            if !actions.contains_key(operation) || !names(directories, true) {
                return Err(error("invalid failure evidence operation or directories"));
            }
            let mut selected: Vec<&str> = Vec::new();
            for directory in directories {
                let relative = directory
                    .strip_suffix('/')
                    .ok_or_else(|| error("failure evidence needs directories ending in /"))?;
                no_symlink_parents(root, relative)?;
                let owners: Vec<_> = artifacts
                    .iter()
                    .filter(|a| {
                        a["path"]
                            .as_str()
                            .is_some_and(|p| p.ends_with('/') && directory.starts_with(p))
                    })
                    .collect();
                if owners.len() != 1
                    || owners[0]["tracked"] != false
                    || selected.iter().any(|p| overlaps(relative, p))
                {
                    return Err(error(
                        "failure evidence needs nonoverlapping untracked artifact owners",
                    ));
                }
                selected.push(relative);
            }
        }
        if recipe
            .verification_consumers
            .keys()
            .any(|n| !tests.contains_key(n.as_str()))
        {
            return Err("E_RELEASE_CONSUMPTION: unknown verification unit".into());
        }
        let mut contracts = BTreeMap::new();
        for (name, unit) in &tests {
            let mut assets: Vec<_> = unit
                .needs()
                .iter()
                .map(|n| builds[n.as_str()].asset().unwrap())
                .collect();
            assets.sort_unstable();
            let contract = recipe.verification_consumers.get(*name);
            let (status, need, findings) = if let Some(contract) = contract {
                if contract.need.trim().is_empty()
                    || !names(&contract.release_assets, false)
                    || !same_members(
                        contract.release_assets.iter().map(String::as_str),
                        assets.iter().copied(),
                    )
                {
                    return Err(format!(
                        "E_RELEASE_CONSUMPTION: execution dependencies differ from consumer assets: {name}"
                    ));
                }
                ("declared", Some(contract.need.as_str()), vec![])
            } else {
                (
                    "unverified",
                    None,
                    if assets.is_empty() {
                        vec![]
                    } else {
                        vec!["W_RELEASE_CONSUMPTION_UNVERIFIED"]
                    },
                )
            };
            contracts.insert(name.to_string(),json!({"operation":unit.operation(),"needs":unit.needs(),"release_assets":assets,"need":need,"status":status,"findings":findings}));
        }
        Ok(Self {
            recipe,
            plan,
            actions,
            staging: rows,
            contracts,
        })
    }

    pub fn unit(&self, id: &str) -> Result<&Unit, String> {
        self.recipe
            .units
            .iter()
            .find(|u| u.id() == id)
            .ok_or_else(|| error(format!("unknown explicit release unit: {id}")))
    }

    /// 只观察当前 scope 实际需要的工具与 manifest；汇总不解析 Rust 绑定。
    pub fn tools(
        &self,
        root: &Path,
        scope: Scope<'_>,
    ) -> Result<BTreeMap<String, PathBuf>, String> {
        let mut required = BTreeSet::from(["git"]);
        let invocation = match scope {
            Scope::Collect => Invocation::Collect,
            Scope::Unit(id) => {
                let unit = self.unit(id)?;
                match unit {
                    Unit::Build { manifest, .. } => {
                        if !no_symlink_parents(root, manifest)?.is_file() {
                            return Err(error(format!("missing release manifest: {manifest}")));
                        }
                    }
                    Unit::Verify { operation, .. } => {
                        required.insert(&self.actions[operation].tool);
                    }
                }
                unit.invocation()
            }
        };
        if policy::requires_rust(&invocation) {
            required.extend(["rustup", "cargo", "rustc"]);
        }
        required
            .into_iter()
            .map(|name| {
                let program = self
                    .recipe
                    .tools
                    .get(name)
                    .ok_or_else(|| error(format!("missing tool binding: {name}")))?;
                Ok((name.to_owned(), resolve_program(root, program, None)?))
            })
            .collect()
    }
}
