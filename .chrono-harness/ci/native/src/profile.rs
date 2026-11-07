//! 当前宿主启动配置与构建操作的预检；只读取显式输入，不选择或启动进程。
use crate::{
    bindings::{Declarations, Environment, NativePlatform},
    environment::{Prepared, Provision},
    execution::{Method, methods},
    verified_bindings as policy,
};
use chrono_harness::{json, no_symlink_parents, sha256};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participation {
    pub coordinator_root: String,
    pub program: String,
    pub git: String,
    pub config: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entrypoint {
    pub tool: String,
    pub script: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerStage {
    pub source: String,
    pub selected: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub projects: String,
    pub rust_toolchain: String,
    pub tools: BTreeMap<String, String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    pub operations: Vec<String>,
    pub install: Vec<Installation>,
    pub participation: Participation,
    pub entrypoint: Entrypoint,
    pub require_tool_bindings: bool,
    pub tool_bindings_registry: String,
    pub report_path: Option<String>,
    pub rust_incremental: Option<bool>,
    pub toolchain_provision: Option<Provision>,
    pub host_environment: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    pub owner_upgrade: Option<String>,
    pub owner_stage: Option<OwnerStage>,
    #[serde(default)]
    pub reuse_owner_operations: Vec<String>,
}

pub fn path(root: &Path, declared: &str, prefix: &[u8]) -> Result<PathBuf, String> {
    if !policy::host_path(declared.as_bytes(), prefix) {
        return Err(format!("invalid bootstrap path: {declared}"));
    }
    no_symlink_parents(root, declared)
}

pub fn read(root: &Path, declared: &str, prefix: &[u8]) -> Result<Vec<u8>, String> {
    let file = path(root, declared, prefix)?;
    if !file.is_file() {
        return Err(format!("bootstrap input is not a regular file: {declared}"));
    }
    fs::read(file).map_err(|error| error.to_string())
}

impl Profile {
    /// 解析固定 producer 数据时不查询当前工作树，也不选择其可写环境。
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let profile: Self =
            serde_json::from_value(json(bytes)?).map_err(|error| error.to_string())?;
        if !policy::profile_options(
            &profile.schema,
            profile.require_tool_bindings,
            profile.owner_upgrade.is_some(),
            profile.owner_stage.is_some(),
            profile.reuse_owner_operations.is_empty(),
        ) {
            return Err("invalid bootstrap schema, bindings or owner mode".into());
        }
        let mut sequence = profile.prerequisites.clone();
        sequence.extend(profile.operations.iter().cloned());
        if !policy::valid_names(&sequence)
            || !policy::valid_names(&profile.reuse_owner_operations)
            || !policy::names_subset(&profile.reuse_owner_operations, &profile.operations)
        {
            return Err("invalid bootstrap operation sequence or owner reuse".into());
        }
        Ok(profile)
    }

    pub fn decode(root: &Path, bytes: &[u8]) -> Result<Self, String> {
        let profile = Self::parse(bytes)?;
        path(root, &profile.projects, b".chrono-harness/")?;
        path(
            root,
            &profile.tool_bindings_registry,
            b".chrono-harness/ci/",
        )?;
        path(root, &profile.entrypoint.script, b".chrono-harness/ci/")?;
        profile.report_destination(root)?;
        let participation = &profile.participation;
        if !policy::participation_mode(
            &participation.coordinator_root,
            Path::new(&participation.git).is_absolute(),
        ) {
            return Err("unsupported bootstrap participation declaration".into());
        }
        path(root, &participation.program, b".chrono-harness/bin/")?;
        path(root, &participation.config, b".chrono-harness/")?;
        if let Some(upgrade) = &profile.owner_upgrade {
            path(root, upgrade, b".chrono-harness/ci/")?;
        }
        if let Some(stage) = &profile.owner_stage {
            path(root, &stage.source, b"")?;
            path(root, &stage.selected, b".chrono-harness/bin/")?;
        }
        let destinations: Vec<_> = profile
            .install
            .iter()
            .map(|item| item.destination.clone())
            .collect();
        if !policy::valid_names(&destinations) {
            return Err("duplicate bootstrap installation destination".into());
        }
        for item in &profile.install {
            path(root, &item.source, b"")?;
            path(root, &item.destination, b".chrono-harness/bin/")?;
        }
        Ok(profile)
    }

    pub fn report_destination(&self, root: &Path) -> Result<PathBuf, String> {
        path(
            root,
            self.report_path
                .as_deref()
                .unwrap_or(".chrono-harness/state/bootstrap.json"),
            b".chrono-harness/state/",
        )
    }

    /// 原生实现由完整 producer commit/tree 绑定；配置输入分别保留原摘要。
    pub fn registration_paths<'a>(&'a self, config: &'a str) -> Vec<&'a str> {
        let mut paths = vec![
            config,
            self.projects.as_str(),
            self.entrypoint.script.as_str(),
            self.tool_bindings_registry.as_str(),
        ];
        if let Some(upgrade) = &self.owner_upgrade {
            paths.push(upgrade);
        }
        paths
    }
}

pub struct Loaded {
    pub profile: Profile,
    pub config: String,
    pub operations: BTreeMap<String, Method>,
    pub registrations: BTreeMap<String, String>,
    pub prepared: Prepared,
    pub declarations: Declarations,
}

impl Loaded {
    pub fn read(
        root: &Path,
        config: &str,
        native: &NativePlatform,
        environment: &Environment,
    ) -> Result<Self, String> {
        let bytes = read(root, config, b".chrono-harness/ci/")?;
        let profile = Profile::decode(root, &bytes)?;
        let prepared = Prepared::parse(root, &bytes, environment)?;
        let registry = read(
            root,
            &profile.tool_bindings_registry,
            b".chrono-harness/ci/",
        )?;
        let declarations = Declarations::parse(&bytes, native, Some(&registry))?
            .ok_or("native bootstrap requires tool bindings")?;
        let projects = read(root, &profile.projects, b".chrono-harness/")?;
        let mut registered = methods(&json(&projects)?)?;
        let mut operations = BTreeMap::new();
        for (names, prerequisite) in [(&profile.prerequisites, true), (&profile.operations, false)]
        {
            for name in names {
                let mut rows = registered
                    .remove(name)
                    .ok_or_else(|| format!("unknown bootstrap operation: {name}"))?;
                if !policy::unique_method(rows.len()) {
                    return Err(format!("ambiguous bootstrap operation: {name}"));
                }
                let method = rows.pop().ok_or("missing registered method")?;
                if policy::needs_configured_tool(prerequisite, &method.tool)
                    && !profile.tools.contains_key(&method.tool)
                {
                    return Err(format!(
                        "bootstrap operation requires a configured tool: {name}: {}",
                        method.tool
                    ));
                }
                operations.insert(name.clone(), method);
            }
        }
        let mut registrations = BTreeMap::new();
        for declared in profile.registration_paths(config) {
            let raw = if declared == config {
                bytes.clone()
            } else if declared == profile.projects {
                projects.clone()
            } else if declared == profile.tool_bindings_registry {
                registry.clone()
            } else {
                read(root, declared, b".chrono-harness/ci/")?
            };
            registrations.insert(declared.into(), sha256(&raw));
        }
        Ok(Self {
            profile,
            config: config.into(),
            operations,
            registrations,
            prepared,
            declarations,
        })
    }
}
