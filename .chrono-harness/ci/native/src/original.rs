//! 原始构建报告的候选校验器；只读取 producer 声明和原字节，不启动其工具或选择可写环境。
use crate::{
    bindings::{Declarations, NativePlatform},
    construction::Report,
    execution::{Method, methods},
    profile::Profile,
    source::SourceObservation,
    verified_bindings::{self as policy, BuildStatus, Executable},
};
use chrono_harness::{json, sha256};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// 所有者升级读取本宿主已安装工具的固定 producer；不执行其源码或 SDK。
pub fn validate_fixed_execution(
    root: &Path,
    environment: &crate::bindings::Environment,
    native: &NativePlatform,
    raw: &[u8],
) -> Result<Report, String> {
    let report: Report = chrono_harness::decode(raw)?;
    if Path::new(&report.host_root) != root {
        return Err("fixed original producer belongs to another host".into());
    }
    let identity = report
        .source
        .as_ref()
        .ok_or("fixed original producer source is missing")?;
    let fixed = crate::source::Committed::new(
        root,
        environment,
        identity
            .commit
            .as_deref()
            .ok_or("fixed original producer commit is missing")?,
        identity
            .tree
            .as_deref()
            .ok_or("fixed original producer tree is missing")?,
    )?;
    let producer = Producer::read(&report.config, native, &|path| fixed.read(path))?;
    producer.validate(raw, &fixed.observation(), &|path| {
        crate::profile::read(root, path, b".chrono-harness/state/")
    })
}

pub struct Producer {
    pub profile: Profile,
    config: String,
    declarations: Declarations,
    registrations: BTreeMap<String, String>,
    methods: BTreeMap<String, Method>,
}

fn declared(path: &str, prefix: &[u8]) -> Result<(), String> {
    if !policy::host_path(path.as_bytes(), prefix) {
        return Err(format!("invalid original producer input path: {path}"));
    }
    Ok(())
}

impl Producer {
    pub fn read(
        config: &str,
        native: &NativePlatform,
        input: &impl Fn(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Self, String> {
        declared(config, b".chrono-harness/ci/")?;
        let raw = input(config)?;
        let profile = Profile::parse(&raw)?;
        declared(&profile.projects, b".chrono-harness/")?;
        declared(&profile.tool_bindings_registry, b".chrono-harness/ci/")?;
        let registry = input(&profile.tool_bindings_registry)?;
        let declarations = Declarations::parse(&raw, native, Some(&registry))?
            .ok_or("original producer requires executable bindings")?;
        let projects = input(&profile.projects)?;
        let mut registered = methods(&json(&projects)?)?;
        let mut selected = BTreeMap::new();
        for name in profile.prerequisites.iter().chain(&profile.operations) {
            let mut rows = registered
                .remove(name)
                .ok_or_else(|| format!("unknown original producer operation: {name}"))?;
            if !policy::unique_method(rows.len()) {
                return Err(format!("ambiguous original producer operation: {name}"));
            }
            selected.insert(
                name.clone(),
                rows.pop().ok_or("missing original producer method")?,
            );
        }
        let mut paths = profile.registration_paths(config);
        // 这些是当前已安装 owner 的必要固定数据。新 Rust producer 不登记已退役模块。
        if policy::python_owner_inputs(
            &profile.entrypoint.script,
            profile.owner_upgrade.is_some() || profile.owner_stage.is_some(),
        ) {
            paths.extend([
                ".chrono-harness/ci/bootstrap_owner.py",
                ".chrono-harness/ci/bootstrap_shared.py",
            ]);
        }
        let mut registrations = BTreeMap::new();
        for path in paths {
            declared(path, b".chrono-harness/")?;
            let hash = if path == config {
                sha256(&raw)
            } else if path == profile.projects {
                sha256(&projects)
            } else if path == profile.tool_bindings_registry {
                sha256(&registry)
            } else {
                sha256(&input(path)?)
            };
            if registrations.insert(path.to_owned(), hash).is_some() {
                return Err(format!("duplicate original producer input: {path}"));
            }
        }
        Ok(Self {
            profile,
            config: config.into(),
            declarations,
            registrations,
            methods: selected,
        })
    }

    /// 核原始源码、执行、SDK 选择与两流。安装集、owner stage 和发布事务由各自调用方另核。
    pub fn validate(
        &self,
        raw: &[u8],
        observed: &SourceObservation,
        stream: &impl Fn(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Report, String> {
        let value = json(raw)?;
        let imported = value.get("provenance").is_some();
        let report: Report = serde_json::from_value(value).map_err(|error| error.to_string())?;
        let root = Path::new(&report.host_root);
        if !policy::original_header(
            &report.schema,
            matches!(report.status, BuildStatus::Passed),
            &report.config,
            &self.config,
            root.is_absolute(),
            imported,
            report.error.is_some(),
        ) {
            return Err("original producer header differs".into());
        }
        let source = report
            .source
            .as_ref()
            .ok_or("original producer source is missing")?;
        if !policy::original_source(source, observed)
            || !policy::immutable_producer(
                observed.commit.as_deref().unwrap_or("").as_bytes(),
                observed.tree.as_deref().unwrap_or("").as_bytes(),
            )
        {
            return Err("original producer clean source differs".into());
        }
        if report.registrations != self.registrations {
            return Err("original producer registrations differ".into());
        }
        let bindings = &self.declarations.tools;
        let executables: Vec<Executable> = serde_json::from_value(
            report
                .executables
                .clone()
                .ok_or("original producer executable bindings are missing")?,
        )
        .map_err(|error| error.to_string())?;
        if executables.len() != 2 {
            return Err("original producer executable bindings differ".into());
        }
        let mut selected = BTreeMap::new();
        let mut versions = BTreeMap::new();
        for ((name, binding), actual) in [("cargo", &bindings.cargo), ("rustc", &bindings.rustc)]
            .into_iter()
            .zip(&executables)
        {
            let expected = Executable {
                path: binding
                    .path
                    .as_ref()
                    .map(|path| root.join(path))
                    .unwrap_or_default()
                    .to_str()
                    .ok_or("non UTF-8 declared producer executable")?
                    .into(),
                sha256: binding.sha256.clone(),
                version: binding.version.clone(),
            };
            if !Path::new(&actual.path).is_absolute()
                || !policy::selected_matches(actual, &expected, binding.allow_relocation)
            {
                return Err(format!("original producer {name} binding differs"));
            }
            selected.insert(name, actual);
            versions.insert(name.to_owned(), binding.version.clone());
        }
        if report.versions != versions
            || report.selection
                != Some(json!({
                    "toolchain": self.profile.rust_toolchain,
                    "cargo": selected["cargo"].path, "rustc": selected["rustc"].path,
                    "compiler": selected["rustc"].path,
                }))
        {
            return Err("original producer SDK selection differs".into());
        }
        let environment = |key: &str, expected: &str| -> Result<(), String> {
            if report
                .environment
                .get(key)
                .and_then(|value| value.as_deref())
                != Some(expected)
            {
                return Err(format!("original producer environment differs: {key}"));
            }
            Ok(())
        };
        for (key, value) in [
            ("RUSTUP_TOOLCHAIN", self.profile.rust_toolchain.as_str()),
            ("RUSTC", &selected["rustc"].path),
            ("RUSTC_WRAPPER", ""),
            ("RUSTC_WORKSPACE_WRAPPER", ""),
        ] {
            environment(key, value)?;
        }
        if let Some(incremental) = self.profile.rust_incremental {
            environment("CARGO_INCREMENTAL", if incremental { "1" } else { "0" })?;
        }
        if report.host_environment != self.profile.host_environment {
            return Err("original producer host environment declaration differs".into());
        }
        if let Some(values) = &self.profile.host_environment {
            for (key, value) in values {
                if !policy::host_variable(key) {
                    return Err(format!("invalid original producer host variable: {key}"));
                }
                declared(value, b".chrono-harness/cache/")?;
                environment(
                    key,
                    root.join(value).to_str().ok_or("non UTF-8 producer home")?,
                )?;
            }
        }
        let expected = self
            .profile
            .prerequisites
            .iter()
            .chain(&self.profile.operations)
            .cloned()
            .collect();
        let actual = report
            .operations
            .iter()
            .map(|row| row.operation.clone())
            .collect();
        if !policy::text_list_equal(&actual, &expected) {
            return Err("original producer operations differ".into());
        }
        let mut paths = BTreeSet::new();
        for row in &report.operations {
            let method = &self.methods[&row.operation];
            let executable = if let Some(binding) = selected.get(method.tool.as_str()) {
                if row.executable_sha256 != binding.sha256 {
                    return Err("original producer operation executable differs".into());
                }
                binding.path.clone()
            } else {
                let configured =
                    self.profile.tools.get(&method.tool).ok_or_else(|| {
                        format!("missing original producer tool: {}", method.tool)
                    })?;
                if !Path::new(configured).is_absolute() && configured.contains('/') {
                    root.join(configured)
                        .to_str()
                        .ok_or("non UTF-8 producer tool")?
                        .into()
                } else {
                    configured.clone()
                }
            };
            let mut argv = vec![executable];
            argv.extend(method.argv.iter().cloned());
            if !policy::operation_record_matches(
                &row.tool,
                &method.tool,
                &row.argv,
                &argv,
                &row.cwd,
                &report.host_root,
                row.pid,
                row.exit_code,
            ) {
                return Err(format!(
                    "original producer operation execution differs: {}",
                    row.operation
                ));
            }
            for descriptor in [&row.stdout, &row.stderr] {
                declared(&descriptor.path, b".chrono-harness/state/")?;
                if !paths.insert(&descriptor.path) {
                    return Err("duplicate original producer process stream".into());
                }
                let bytes = stream(&descriptor.path)?;
                if !policy::stream_matches(
                    &sha256(&bytes),
                    &descriptor.sha256,
                    bytes.len() as u64,
                    descriptor.size,
                ) {
                    return Err("original producer process stream differs".into());
                }
            }
        }
        Ok(report)
    }
}
