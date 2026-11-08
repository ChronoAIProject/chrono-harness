//! 源码构建报告的实际生产者；所有者保护、升级和共享交接由外层现役入口编排。
use crate::{
    bindings::{Declarations, Environment},
    environment::{Failure, Prepared, Selected},
    execution::{Method, Operation, run_operation},
    profile::{Loaded, Profile, path},
    source::{self, SourceIdentity, SourceObservation},
    verified_bindings::{self as policy, BuildStatus},
};
use chrono_harness::{file_identity, resolve_program};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct BuildFailure {
    pub message: String,
    pub exit_code: Option<i32>,
}
impl From<String> for BuildFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            exit_code: None,
        }
    }
}
impl From<&str> for BuildFailure {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}
impl From<Failure> for BuildFailure {
    fn from(failure: Failure) -> Self {
        let exit_code = match &failure {
            Failure::Installer(status) => policy::process_exit(status.code(), status.signal()),
            Failure::Selection(_) => None,
        };
        Self {
            message: failure.to_string(),
            exit_code,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Installed {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub config: String,
    pub host_root: String,
    pub status: BuildStatus,
    pub registrations: BTreeMap<String, String>,
    pub versions: BTreeMap<String, String>,
    pub environment: BTreeMap<String, Option<String>>,
    pub operations: Vec<Operation>,
    pub installed: Vec<Installed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executables: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_environment: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_stage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct Construction<'a> {
    pub root: &'a Path,
    pub profile: Profile,
    pub directory: PathBuf,
    pub report: Report,
    operations: BTreeMap<String, Method>,
    declarations: Declarations,
    prepared: Option<Prepared>,
    selected: Option<Selected>,
    before: SourceObservation,
    inherited: Environment,
    owner_source: Option<PathBuf>,
}

pub struct Published {
    pub original: PathBuf,
    pub report: Report,
    pub outcome: Result<(), BuildFailure>,
}

fn environment_text(environment: &Environment, key: &str) -> Result<Option<String>, String> {
    environment
        .get(OsStr::new(key))
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("non UTF-8 {key}"))
        })
        .transpose()
}

fn configured(
    root: &Path,
    name: &str,
    tools: &BTreeMap<String, String>,
    environment: &Environment,
) -> Result<PathBuf, String> {
    let program = tools
        .get(name)
        .ok_or_else(|| format!("missing configured bootstrap tool: {name}"))?;
    let search = environment
        .get(OsStr::new("PATH"))
        .map(|value| value.to_str().ok_or("non UTF-8 PATH"))
        .transpose()?;
    resolve_program(root, program, search)
}

fn execute(
    root: &Path,
    directory: &Path,
    report: &mut Report,
    method: &Method,
    executable: &Path,
    environment: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
) -> Result<(), BuildFailure> {
    let record = run_operation(
        root,
        directory,
        report.operations.len(),
        method,
        executable,
        environment,
        stdout,
        stderr,
    )?;
    let exit_code = record.exit_code;
    let failure = if exit_code != 0 {
        Some(BuildFailure {
            message: format!(
                "bootstrap operation {} failed with exit {exit_code}: {:?}",
                method.operation, record.argv
            ),
            exit_code: Some(exit_code),
        })
    } else {
        None
    };
    report.operations.push(record);
    if let Some(failure) = failure {
        return Err(failure);
    }
    Ok(())
}

impl<'a> Construction<'a> {
    pub fn begin(
        root: &'a Path,
        loaded: Loaded,
        inherited: &Environment,
    ) -> Result<Self, BuildFailure> {
        let host_root = root.to_str().ok_or("non UTF-8 host root")?.to_owned();
        let before = source::observe(root, inherited);
        let destination = loaded.profile.report_destination(root)?;
        let parent = destination
            .parent()
            .ok_or("missing bootstrap report parent")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        // 保留原目录供原始报告和两流消费者使用；回收仍归宿主生命周期所有者。
        let directory = tempfile::Builder::new()
            .prefix("bootstrap-")
            .tempdir_in(parent)
            .map_err(|error| error.to_string())?
            .keep();
        let report = Report {
            schema: "chrono-bootstrap-result/v1".into(),
            config: loaded.config,
            host_root,
            status: BuildStatus::Failed,
            registrations: loaded.registrations,
            versions: BTreeMap::new(),
            environment: BTreeMap::new(),
            operations: vec![],
            installed: vec![],
            executables: None,
            selection: None,
            host_environment: None,
            owner_stage: None,
            source: None,
            error: None,
        };
        Ok(Self {
            root,
            profile: loaded.profile,
            directory,
            report,
            operations: loaded.operations,
            declarations: loaded.declarations,
            prepared: Some(loaded.prepared),
            selected: None,
            before,
            inherited: inherited.clone(),
            owner_source: None,
        })
    }

    pub fn prerequisites(
        &mut self,
        stdout: &mut (impl Write + Send),
        stderr: &mut (impl Write + Send),
    ) -> Result<(), BuildFailure> {
        if !self.report.operations.is_empty() {
            return Err("bootstrap prerequisites already started".into());
        }
        for name in &self.profile.prerequisites {
            let method = &self.operations[name];
            let executable = configured(
                self.root,
                &method.tool,
                &self.profile.tools,
                &self.inherited,
            )?;
            execute(
                self.root,
                &self.directory,
                &mut self.report,
                method,
                &executable,
                &self.inherited,
                stdout,
                stderr,
            )?;
        }
        Ok(())
    }

    pub fn select(&mut self) -> Result<(), BuildFailure> {
        let names = self
            .report
            .operations
            .iter()
            .map(|row| row.operation.clone())
            .collect();
        let codes = self
            .report
            .operations
            .iter()
            .map(|row| row.exit_code)
            .collect();
        if !policy::text_list_equal(&self.profile.prerequisites, &names)
            || !policy::all_successful(&codes)
        {
            return Err("bootstrap prerequisites have not succeeded".into());
        }
        let prepared = self
            .prepared
            .take()
            .ok_or("bootstrap SDK selection already attempted")?;
        for directory in &prepared.host_directories {
            fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        }
        let selected =
            prepared.activate(self.root, Some(&self.declarations), &self.profile.tools)?;
        self.report
            .versions
            .insert("cargo".into(), selected.versions.cargo.clone());
        self.report
            .versions
            .insert("rustc".into(), selected.versions.rustc.clone());
        for key in [
            "RUSTUP_TOOLCHAIN",
            "CARGO_INCREMENTAL",
            "RUSTC",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
        ] {
            self.report
                .environment
                .insert(key.into(), environment_text(&selected.environment, key)?);
        }
        if let Some(bindings) = &selected.bindings {
            self.report.executables = Some(json!([bindings.cargo, bindings.rustc]));
            self.report.selection = Some(
                json!({"toolchain":self.profile.rust_toolchain,"cargo":bindings.cargo.path,
                "rustc":bindings.rustc.path,"compiler":environment_text(&selected.environment,"RUSTC")?}),
            );
        }
        if let Some(values) = &self.profile.host_environment {
            self.report.host_environment = Some(values.clone());
            for key in values.keys() {
                self.report
                    .environment
                    .insert(key.clone(), environment_text(&selected.environment, key)?);
            }
        }
        self.selected = Some(selected);
        Ok(())
    }

    pub fn operations(
        &mut self,
        stdout: &mut (impl Write + Send),
        stderr: &mut (impl Write + Send),
    ) -> Result<(), BuildFailure> {
        self.operations_with_reuse(&BTreeMap::new(), stdout, stderr)
    }

    pub fn operations_reusing_owner(
        &mut self,
        native: &crate::bindings::NativePlatform,
        stdout: &mut (impl Write + Send),
        stderr: &mut (impl Write + Send),
    ) -> Result<(), BuildFailure> {
        let reuse = crate::owner::reuse_construction(
            self.root,
            &self.inherited,
            native,
            &self.profile,
            &self.report,
            &self.before,
        )?;
        self.owner_source = reuse.source;
        self.operations_with_reuse(&reuse.operations, stdout, stderr)
    }

    fn operations_with_reuse(
        &mut self,
        reused: &BTreeMap<String, Operation>,
        stdout: &mut (impl Write + Send),
        stderr: &mut (impl Write + Send),
    ) -> Result<(), BuildFailure> {
        let selected = self
            .selected
            .as_ref()
            .ok_or("bootstrap SDK has not been selected")?;
        for name in &self.profile.operations {
            if let Some(original) = reused.get(name) {
                self.report.operations.push(original.clone());
                continue;
            }
            let method = &self.operations[name];
            let bindings = selected
                .bindings
                .as_ref()
                .ok_or("bootstrap executable bindings missing")?;
            let executable = match method.tool.as_str() {
                "cargo" => PathBuf::from(&bindings.cargo.path),
                "rustc" => PathBuf::from(&bindings.rustc.path),
                _ => configured(
                    self.root,
                    &method.tool,
                    &self.profile.tools,
                    &selected.environment,
                )?,
            };
            execute(
                self.root,
                &self.directory,
                &mut self.report,
                method,
                &executable,
                &selected.environment,
                stdout,
                stderr,
            )?;
        }
        let current = self.declarations.select(
            self.root,
            &resolve_program(
                self.root,
                "rustup",
                environment_text(&selected.environment, "PATH")?.as_deref(),
            )?,
            &self.profile.rust_toolchain,
            &self.profile.tools,
            &selected.environment,
        )?;
        let original = selected
            .bindings
            .as_ref()
            .ok_or("bootstrap executable bindings missing")?;
        if !policy::selected_matches(&current.cargo, &original.cargo, false)
            || !policy::selected_matches(&current.rustc, &original.rustc, false)
        {
            return Err("bound executable changed during bootstrap".into());
        }
        Ok(())
    }

    /// 调用方先执行现役所有者的安装判定，再传入实际采用的 source。
    pub fn install_tool(&mut self, source: &Path, destination: &str) -> Result<(), BuildFailure> {
        let target = path(self.root, destination, b".chrono-harness/bin/")?;
        let source = if destination == self.profile.participation.program {
            self.owner_source.as_deref().unwrap_or(source)
        } else {
            source
        };
        if !source.is_file() {
            return Err(format!("missing built tool: {}", source.display()).into());
        }
        if destination == self.profile.participation.program {
            crate::owner::check_shared_install(
                self.root,
                &self.inherited,
                &self.profile,
                source,
                &target,
            )?;
            if target.exists() && file_identity(source)?.0 == file_identity(&target)?.0 {
                self.report.installed.push(Installed {
                    path: destination.into(),
                    sha256: file_identity(&target)?.0,
                });
                return Ok(());
            }
        }
        fs::create_dir_all(target.parent().ok_or("missing install parent")?)
            .map_err(|error| error.to_string())?;
        fs::copy(source, &target).map_err(|error| error.to_string())?;
        self.report.installed.push(Installed {
            path: destination.into(),
            sha256: file_identity(&target)?.0,
        });
        Ok(())
    }

    /// 成功还须具备全部原操作、安装和 owner stage，调用方的 Ok 不能代替完成事实。
    pub fn stage_owner(
        &mut self,
        previous: &policy::owner_publication::Binding,
    ) -> Result<(), BuildFailure> {
        let stage = crate::owner::stage(self.root, &self.inherited, &self.profile, previous)?;
        self.report.owner_stage =
            Some(serde_json::to_value(stage).map_err(|error| error.to_string())?);
        Ok(())
    }

    pub fn finish(mut self, outcome: Result<(), BuildFailure>) -> Result<Published, BuildFailure> {
        let mut expected = self.profile.prerequisites.clone();
        expected.extend(self.profile.operations.iter().cloned());
        let actual = self
            .report
            .operations
            .iter()
            .map(|row| row.operation.clone())
            .collect();
        let codes = self
            .report
            .operations
            .iter()
            .map(|row| row.exit_code)
            .collect();
        let installations = self
            .report
            .installed
            .iter()
            .map(|row| row.path.clone())
            .collect();
        let expected_installations = self
            .profile
            .install
            .iter()
            .map(|item| item.destination.clone())
            .collect();
        let complete = policy::construction_complete(
            &expected,
            &actual,
            &codes,
            &expected_installations,
            &installations,
            self.selected.is_some(),
            self.profile.owner_stage.is_some(),
            self.report.owner_stage.is_some(),
        );
        let outcome = if outcome.is_ok() && !complete {
            Err("bootstrap construction is incomplete".into())
        } else {
            outcome
        };
        self.report.status = policy::construction_status(outcome.is_err(), complete);
        self.report.error = outcome.as_ref().err().map(|error| error.message.clone());
        self.report.source = Some(source::source_identity(
            self.before,
            source::observe(self.root, &self.inherited),
        ));
        let raw = serde_json::to_vec_pretty(&self.report).map_err(|error| error.to_string())?;
        let original = self.directory.join("report.json");
        let publish = (|| -> Result<(), String> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&original)
                .map_err(|error| error.to_string())?;
            file.write_all(&raw)
                .and_then(|_| file.write_all(b"\n"))
                .map_err(|error| error.to_string())?;
            if outcome.is_ok() {
                let destination = self.profile.report_destination(self.root)?;
                fs::write(destination, [raw.as_slice(), b"\n"].concat())
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })();
        if let Err(error) = publish {
            return Err(BuildFailure {
                message: format!(
                    "{}cannot publish bootstrap report {}: {error}",
                    outcome
                        .as_ref()
                        .err()
                        .map(|error| format!("{}; ", error.message))
                        .unwrap_or_default(),
                    original.display()
                ),
                exit_code: outcome.as_ref().err().and_then(|error| error.exit_code),
            });
        }
        Ok(Published {
            original,
            report: self.report,
            outcome,
        })
    }
}
