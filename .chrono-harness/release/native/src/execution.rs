//! 发布单元的真实执行：固定源码、原始操作、消费向量和原结果由本生产者发布。
use crate::{
    evidence, frontend, process,
    receipt::{self, Kind, Lineage, Verified},
    recipe::{self, Loaded, Scope, Unit},
    staging::Staging,
    transport, verified_recipe as policy,
};
use chrono_harness::{file_identity, no_symlink_parents};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub type Environment = BTreeMap<OsString, OsString>;

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_EXECUTION: {message}")
}

fn timestamp() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(error)?
        .as_nanos()
        .try_into()
        .map_err(error)
}

fn env<'a>(environment: &'a Environment, name: &str) -> Result<&'a str, String> {
    environment
        .get(std::ffi::OsStr::new(name))
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error(format!("missing original {name} binding")))
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| error("non UTF-8 execution path"))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(error)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(error)?;
    serde_json::to_writer_pretty(&mut file, value).map_err(error)?;
    file.write_all(b"\n")
        .and_then(|()| file.flush())
        .map_err(error)
}

fn executable(path: &Path) -> Result<Value, String> {
    let metadata = fs::metadata(path).map_err(error)?;
    let mode = metadata.permissions().mode() & 0o7777;
    if !path.is_absolute()
        || !policy::executable_file(metadata.is_file(), mode)
        || metadata.len() == 0
    {
        return Err(error("not a regular executable"));
    }
    let (sha256, size) = file_identity(path)?;
    Ok(json!({"path":path_text(path)?,"sha256":sha256,"size":size,"mode":mode}))
}

struct Execution<'a> {
    root: PathBuf,
    output: PathBuf,
    loaded: &'a Loaded,
    scope: Scope<'a>,
    tools: BTreeMap<String, PathBuf>,
    environment: Environment,
    report: Value,
    phase: String,
    child_failure: Option<(String, i32, String)>,
    started: Instant,
}

impl<'a> Execution<'a> {
    fn begin(
        root: &Path,
        output: &Path,
        loaded: &'a Loaded,
        scope: Scope<'a>,
        mut environment: Environment,
    ) -> Result<Self, String> {
        let started = Instant::now();
        let root = fs::canonicalize(root).map_err(error)?;
        let output = if output.is_absolute() {
            output.to_owned()
        } else {
            std::env::current_dir().map_err(error)?.join(output)
        };
        match fs::symlink_metadata(&output) {
            Ok(_) => return Err(error("release output already exists")),
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {}
            Err(failure) => return Err(error(failure)),
        }
        let tools = loaded.tools(&root, scope)?;
        let (unit, requires_rust) = match scope {
            Scope::Collect => (
                serde_json::to_value(&loaded.recipe.collector).map_err(error)?,
                false,
            ),
            Scope::Unit(id) => {
                let unit = loaded.unit(id)?;
                (unit.value(), policy::requires_rust(&unit.invocation()))
            }
        };
        if requires_rust {
            environment.insert(
                "RUSTUP_TOOLCHAIN".into(),
                loaded.recipe.rust_toolchain.clone().into(),
            );
            environment.insert("CARGO_PROFILE_RELEASE_STRIP".into(), "symbols".into());
            if let Some(incremental) = loaded.recipe.rust_incremental {
                environment.insert(
                    "CARGO_INCREMENTAL".into(),
                    if incremental { "1" } else { "0" }.into(),
                );
            }
        }
        let observations: BTreeMap<_, _> = [
            "PATH",
            "RUSTUP_TOOLCHAIN",
            "CARGO_INCREMENTAL",
            "CARGO_PROFILE_RELEASE_STRIP",
            "SDKROOT",
            "MACOSX_DEPLOYMENT_TARGET",
            "CARGO_TARGET_DIR",
            "RUSTFLAGS",
        ]
        .into_iter()
        .map(|name| {
            (
                name,
                environment
                    .get(std::ffi::OsStr::new(name))
                    .map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
        let mut report = json!({"schema":"chrono-release-unit/v1","unit":unit,"status":"failed","started_ns":timestamp()?,"finished_ns":null,"context":null,"lineage":null,"environment":observations,"processes":[],"failure":null,"selected":{},"assets":{}});
        let contracts = match scope {
            Scope::Collect => json!(loaded.contracts),
            Scope::Unit(id) => loaded
                .contracts
                .get(id)
                .map(|contract| json!({id:contract}))
                .unwrap_or_else(|| json!({})),
        };
        for (id, contract) in contracts
            .as_object()
            .ok_or_else(|| error("invalid dependency contracts"))?
        {
            for finding in contract["findings"]
                .as_array()
                .ok_or_else(|| error("invalid consumer findings"))?
            {
                eprintln!(
                    "{}: {id} selects release assets without a consumer explanation",
                    finding
                        .as_str()
                        .unwrap_or("W_RELEASE_CONSUMPTION_UNVERIFIED")
                );
            }
        }
        report["dependency_contracts"] = contracts;
        if let Scope::Unit(id) = scope {
            if let Some(operation) = loaded.unit(id)?.operation() {
                report["action_tool"] = json!(loaded.actions[operation].tool);
            }
        }
        Ok(Self {
            root,
            output,
            loaded,
            scope,
            tools,
            environment,
            report,
            phase: "source-identity".into(),
            child_failure: None,
            started,
        })
    }

    fn secondary(&mut self, component: &str, message: impl std::fmt::Display) {
        let message = message.to_string();
        let record = json!({"phase":self.phase,"component":component,"message":message});
        if self.report.get("secondary_failures").is_none() {
            self.report["secondary_failures"] = json!([]);
        }
        self.report["secondary_failures"]
            .as_array_mut()
            .expect("producer-owned secondary failures")
            .push(record);
        let _ = writeln!(
            std::io::stderr(),
            "release unit {component} failed: {message}"
        );
    }

    fn run(
        &mut self,
        phase: &str,
        argv: Vec<String>,
        operation: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        self.phase = phase.into();
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| error("empty release command"))?;
        let timeout = Duration::from_secs(
            self.loaded
                .recipe
                .limits
                .unit_seconds
                .min(self.loaded.recipe.limits.platform_seconds),
        )
        .checked_sub(self.started.elapsed())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| error("registered release unit span exhausted"))?;
        let environment = self
            .environment
            .iter()
            .filter(|(key, _)| {
                key.as_os_str() != std::ffi::OsStr::new(chrono_harness::process_fds::ENV)
            })
            .map(|(key, value)| {
                Ok((
                    key.to_str()
                        .ok_or_else(|| error("non UTF-8 release environment key"))?
                        .to_owned(),
                    value
                        .to_str()
                        .ok_or_else(|| error("non UTF-8 release environment value"))?
                        .to_owned(),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let command = chrono_harness::CommandSpec {
            program: program.clone(),
            args: args.to_vec(),
            env: environment,
            timeout_seconds: timeout
                .as_secs()
                .saturating_add(u64::from(timeout.subsec_nanos() != 0)),
            output_limit_bytes: chrono_harness::verified_limits::MAX_OUTPUT_BYTES,
        };
        let index = self.report["processes"]
            .as_array()
            .expect("producer-owned processes")
            .len();
        let captured = process::execute(
            &self.root,
            &command,
            &self.output,
            index,
            timeout,
            std::io::stdout(),
            std::io::stderr(),
        )?;
        let mut record = captured.receipt(phase, operation);
        if let Some(code) = captured.child_exit.filter(|code| *code != 0) {
            let message = error(format!("original {phase} process exited {code}: {argv:?}"));
            self.child_failure = Some((phase.into(), code, message));
            let directories = operation
                .and_then(|op| self.loaded.recipe.failure_evidence.get(op))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let retained = evidence::retain(
                &self.root,
                &self.output,
                phase,
                directories,
                &captured.stdout.bytes,
                &captured.stderr.bytes,
            );
            if let Some(value) = retained.nested {
                record["failure_evidence"] = value;
            }
            if let Some(value) = retained.registered {
                record["registered_failure_evidence"] = value;
            }
            for failure in retained.failures {
                self.secondary("failure evidence", failure);
            }
        }
        self.report["processes"]
            .as_array_mut()
            .expect("producer-owned processes")
            .push(record);
        for failure in captured
            .failures
            .iter()
            .chain(&captured.stdout.failures)
            .chain(&captured.stderr.failures)
        {
            self.secondary("original process transport", failure);
        }
        if !captured.passed() {
            return Err(self
                .child_failure
                .as_ref()
                .map(|(_, _, message)| message.clone())
                .unwrap_or_else(|| error("original process or output transport failed")));
        }
        Ok(captured.stdout.bytes)
    }

    fn git(&mut self, phase: &str, args: &[&str]) -> Result<Vec<u8>, String> {
        let argv = std::iter::once(path_text(&self.tools["git"])?)
            .chain(args.iter().map(|s| (*s).to_owned()))
            .collect();
        self.run(phase, argv, None)
    }

    fn text_run(&mut self, phase: &str, argv: Vec<String>) -> Result<String, String> {
        String::from_utf8(self.run(phase, argv, None)?)
            .map(|s| s.trim().to_owned())
            .map_err(error)
    }

    fn context(&mut self) -> Result<Lineage, String> {
        let source = String::from_utf8(self.git("source-identity", &["rev-parse", "HEAD"])?)
            .map_err(error)?
            .trim()
            .to_owned();
        let tree = String::from_utf8(self.git("source-tree", &["rev-parse", "HEAD^{tree}"])?)
            .map_err(error)?
            .trim()
            .to_owned();
        if !chrono_identifiers::valid_oid(source.as_bytes())
            || !chrono_identifiers::valid_oid(tree.as_bytes())
        {
            return Err(error("invalid source identity"));
        }
        if !self
            .git(
                "source-clean",
                &["status", "--porcelain", "--untracked-files=normal"],
            )?
            .is_empty()
        {
            return Err(error("release source must be a fixed clean candidate"));
        }
        let mut inputs = BTreeMap::new();
        for name in [
            recipe::CONFIG,
            &self.loaded.recipe.projects,
            &self.loaded.recipe.consumer_staging.release_plan,
            &self.loaded.recipe.consumer_staging.host_config,
        ] {
            inputs.insert(
                name,
                file_identity(&no_symlink_parents(&self.root, name)?)?.0,
            );
        }
        let tools: BTreeMap<_, _> = self
            .tools
            .iter()
            .map(|(name, path)| Ok((name, executable(path)?)))
            .collect::<Result<_, String>>()?;
        let mut producer = executable(&std::env::current_exe().map_err(error)?)?;
        producer
            .as_object_mut()
            .expect("executable identity")
            .remove("path");
        let system = match std::env::consts::OS {
            "macos" => "Darwin",
            "linux" => "Linux",
            other => other,
        };
        self.report["execution_tools"] = json!(tools);
        self.report["context"] = json!({"source_commit":source,"source_tree":tree,"system":system,"machine":std::env::consts::ARCH,"recipe":inputs,"producer":producer,"rust_toolchain":self.loaded.recipe.rust_toolchain,"rust_components":self.loaded.recipe.rust_components});
        let local = self
            .environment
            .get(std::ffi::OsStr::new("CHRONO_RELEASE_LOCAL"))
            .is_some_and(|s| s == "1");
        let lineage = Lineage {
            kind: if local { Kind::Local } else { Kind::Github },
            run_id: env(&self.environment, "CHRONO_RELEASE_RUN")?.into(),
            attempt: env(&self.environment, "CHRONO_RELEASE_ATTEMPT")?.into(),
            job: env(&self.environment, "CHRONO_RELEASE_JOB")?.into(),
        };
        lineage.validate()?;
        let raw = env(&self.environment, "CHRONO_RELEASE_DEPENDENCIES")?
            .as_bytes()
            .to_vec();
        let selected = receipt::select(self.loaded, self.scope, &lineage, &raw)?;
        fs::create_dir_all(&self.output).map_err(error)?;
        let path = self.output.join("dependency-context.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(error)?;
        file.write_all(&raw)
            .and_then(|()| file.flush())
            .map_err(error)?;
        self.report["dependency_input"] = json!({"path":"dependency-context.json","size":raw.len(),"sha256":chrono_harness::sha256(&raw),"source":if local {"local-outcomes"} else {"native-needs"}});
        self.report["lineage"] = json!(lineage);
        self.report["selected"] = json!(selected.units);
        self.report["frontend_selection"] = json!(selected.frontend);
        if let Some(selected) = &selected.frontend {
            self.phase = "frontend-source".into();
            let declared = frontend::declaration(self.loaded,selected)?;
            let directory = no_symlink_parents(&self.root,&declared.directory)?;
            let source = chrono_host_bootstrap::source::SourceObservation {
                commit:Some(source),tree:Some(tree),dirty:Some(false),error:None,
            };
            frontend::validate(&self.root,&directory,declared,&chrono_host_bootstrap::bindings::native_platform()?,
                &source,selected,Some(&std::env::current_exe().map_err(error)?))?;
            let files=frontend::retain_metadata(&directory,&self.output.join("frontend"))?;
            let mut reference=files["frontend.json"].clone();
            reference["path"]=json!("frontend/frontend.json");
            self.report["frontend_input"]=reference;
        }
        Ok(lineage)
    }

    fn toolchain(&mut self) -> Result<(), String> {
        let cfg = &self.loaded.recipe;
        let mut argv = vec![
            path_text(&self.tools["rustup"])?,
            "toolchain".into(),
            "install".into(),
            cfg.rust_toolchain.clone(),
            "--profile".into(),
            "minimal".into(),
        ];
        for component in &cfg.rust_components {
            argv.extend(["--component".into(), component.clone()]);
        }
        self.run("toolchain-install", argv, None)?;
        let mut versions = BTreeMap::new();
        for name in ["cargo", "rustc"] {
            versions.insert(
                name,
                self.text_run(
                    "compiler-version",
                    vec![path_text(&self.tools[name])?, "--version".into()],
                )?,
            );
        }
        if !versions["rustc"].starts_with(&format!("rustc {} ", self.loaded.recipe.rust_toolchain))
        {
            return Err(error("unexpected compiler"));
        }
        let mut active = BTreeMap::new();
        for name in ["cargo", "rustc"] {
            let path = self.text_run(
                "compiler-executable",
                vec![
                    path_text(&self.tools["rustup"])?,
                    "which".into(),
                    "--toolchain".into(),
                    self.loaded.recipe.rust_toolchain.clone(),
                    name.into(),
                ],
            )?;
            active.insert(name, executable(Path::new(&path))?);
        }
        let tools: BTreeMap<_, _> = self
            .tools
            .iter()
            .map(|(name, path)| Ok((name, executable(path)?)))
            .collect::<Result<_, String>>()?;
        self.report["toolchain"] =
            json!({"versions":versions,"tools":tools,"active_compilers":active});
        Ok(())
    }

    fn final_source(&mut self) -> Result<(), String> {
        let clean = self.git(
            "source-clean-final",
            &["status", "--porcelain", "--untracked-files=normal"],
        )?;
        let source =
            String::from_utf8(self.git("source-final", &["rev-parse", "HEAD"])?).map_err(error)?;
        let tree = String::from_utf8(self.git("tree-final", &["rev-parse", "HEAD^{tree}"])?)
            .map_err(error)?;
        if !clean.is_empty()
            || self.report["context"]["source_commit"] != source.trim()
            || self.report["context"]["source_tree"] != tree.trim()
        {
            return Err(error("fixed release source changed during unit"));
        }
        Ok(())
    }

    fn stage(&mut self) -> Result<Staging, String> {
        let mut staging = Staging::selected(self.loaded, &self.report["assets"])?;
        let destinations = staging.destinations();
        if !destinations.is_empty() {
            let args: Vec<_> = ["--literal-pathspecs", "ls-files", "-z", "--"]
                .into_iter()
                .chain(destinations)
                .collect();
            if !self.git("consumer-tracking", &args)?.is_empty() {
                return Err(error("release consumer destination is tracked by Git"));
            }
        }
        let copied = staging.copy(&self.root);
        let intact = staging.observe(&self.root, "staged", None);
        self.report["consumer_staging"] = staging.receipt();
        copied?;
        if !intact {
            return Err(error("release staging identity mismatch"));
        }
        Ok(staging)
    }

    fn finish(mut self, outcome: Result<(), String>) -> i32 {
        let mut failure = outcome.err();
        let mut child = None;
        if let Some((phase, code, original)) = self.child_failure.take() {
            if let Some(extra) = failure.as_ref().filter(|extra| **extra != original) {
                self.secondary("post-child operation", extra);
            }
            self.phase = phase;
            failure = Some(original);
            child = Some(code);
        }
        let finished = match timestamp() {
            Ok(value) => Some(value),
            Err(error) => {
                self.secondary("finish timestamp", &error);
                failure.get_or_insert(error);
                None
            }
        };
        if let Some(finished) = finished {
            let start = self.report["started_ns"].as_u64().expect("producer start");
            if failure.is_none()
                && (finished < start
                    || u128::from(finished - start)
                        > u128::from(self.loaded.recipe.limits.unit_seconds) * 1_000_000_000)
            {
                self.phase = "unit-span".into();
                failure = Some(error("registered release unit span exceeded"));
            }
            if let Some(started) = self
                .report
                .get("platform_started_ns")
                .and_then(Value::as_u64)
            {
                self.report["platform_span_ns"] = json!(finished.checked_sub(started));
                if failure.is_none()
                    && (finished < started
                        || u128::from(finished - started)
                            > u128::from(self.loaded.recipe.limits.platform_seconds)
                                * 1_000_000_000)
                {
                    self.phase = "platform-span".into();
                    failure = Some(error("registered release platform span exceeded"));
                }
            }
        }
        self.report["finished_ns"] = json!(finished);
        self.report["status"] = json!(if failure.is_some() {
            "failed"
        } else {
            "passed"
        });
        if let Some(message) = &failure {
            self.report["failure"] =
                json!({"phase":self.phase,"message":message,"exit_code":child});
            let _ = writeln!(std::io::stderr(), "{message}");
        }
        let mut code = match child {
            Some(code) if code > 0 => code,
            Some(code) => 128 - code,
            None => {
                if failure.is_some() {
                    2
                } else {
                    0
                }
            }
        };
        if let Err(failure) = write_json(&self.output.join("receipt.json"), &self.report) {
            self.secondary("receipt publication", &failure);
            let _ = writeln!(
                std::io::stderr(),
                "cannot retain release unit evidence: {failure}"
            );
            if code == 0 {
                code = 2;
            }
        }
        code
    }
}

fn retain_original(output: &Path, id: &str, verified: &Verified) -> Result<Value, String> {
    let target = output.join("originals").join(id);
    fs::create_dir_all(&target).map_err(error)?;
    let source = no_symlink_parents(&verified.directory, "receipt.json")?;
    let mut declarations = BTreeMap::from([(
        "receipt.json".to_owned(),
        json!({"path":"receipt.json","sha256":verified.sha256,"size":fs::metadata(&source).map_err(error)?.len()}),
    )]);
    declarations.insert(
        verified.receipt["dependency_input"]["path"]
            .as_str()
            .ok_or_else(|| error("missing original dependency path"))?
            .to_owned(),
        verified.receipt["dependency_input"].clone(),
    );
    for process in verified.receipt["processes"]
        .as_array()
        .ok_or_else(|| error("missing original processes"))?
    {
        for stream in ["stdout", "stderr"] {
            let declaration = &process[stream];
            declarations.insert(
                declaration["path"]
                    .as_str()
                    .ok_or_else(|| error("missing original stream path"))?
                    .to_owned(),
                declaration.clone(),
            );
        }
    }
    if verified.receipt["frontend_input"].is_object() {
        for (name, mut declaration) in frontend::metadata_files(&verified.directory.join("frontend"))? {
            let name=format!("frontend/{name}");
            declaration["path"]=json!(name);
            if declarations.insert(name,declaration).is_some() {
                return Err(error("duplicate original frontend evidence path"));
            }
        }
    }
    for (name, declaration) in &declarations {
        let source = receipt::original(&verified.directory, declaration)?;
        let destination = chrono_harness::prepare_publication(&target, name)?;
        let mut input = fs::File::open(source).map_err(error)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(error)?;
        let (sha, size) = chrono_harness::copy_hashed(&mut input, &mut output)?;
        output.flush().map_err(error)?;
        if declaration["sha256"] != sha || declaration["size"] != size {
            return Err(error("original selected evidence changed during retention"));
        }
    }
    Ok(json!(declarations))
}

fn package_identity(output: &Path, loaded: &Loaded, report: &Value) -> Result<Value, String> {
    let release = recipe::read_json(output, "release.json")?;
    let context = &report["context"];
    let platform = format!(
        "{}-{}",
        match std::env::consts::OS {
            "macos" => "macos",
            "linux" => "linux",
            other => other,
        },
        std::env::consts::ARCH
    );
    if release["schema"] != "chrono-release/v1"
        || release["version"] != loaded.plan.version
        || release["source_commit"] != context["source_commit"]
        || release["source_tree"] != context["source_tree"]
        || release["platforms"]
            .as_object()
            .is_none_or(|p| p.len() != 1 || !p.contains_key(&platform))
    {
        return Err(error("final package source/version/platform mismatch"));
    }
    let members = release["platforms"][&platform]
        .as_object()
        .ok_or_else(|| error("missing final package assets"))?;
    if members.keys().collect::<std::collections::BTreeSet<_>>()
        != loaded.plan.assets.keys().collect()
    {
        return Err(error("final package asset membership mismatch"));
    }
    let expected: std::collections::BTreeSet<_> = std::iter::once("release.json".to_owned())
        .chain(members.keys().map(|name| format!("{name}-{platform}")))
        .collect();
    let actual = fs::read_dir(output)
        .map_err(error)?
        .map(|entry| {
            entry.map_err(error).and_then(|entry| {
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| error("non UTF-8 package member"))
            })
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    if actual != expected {
        return Err(error("final package file membership mismatch"));
    }
    let mut observations = BTreeMap::new();
    for (asset, declaration) in members {
        let name = format!("{asset}-{platform}");
        if declaration["file"] != name {
            return Err(error("final package asset filename mismatch"));
        }
        let actual = transport::observe(output, &name)?;
        let expected =
            transport::ExecutableIdentity::from_receipt(&report["assets"][asset]["identity"])?;
        if actual != expected
            || declaration["sha256"] != actual.sha256
            || declaration["size"] != actual.size
        {
            return Err(error("final package asset byte/mode mismatch"));
        }
        observations.insert(asset, actual.receipt(&name));
    }
    Ok(
        json!({"release_sha256":file_identity(&output.join("release.json"))?.0,"assets":observations}),
    )
}

fn units(execution: &mut Execution<'_>, lineage: &Lineage) -> Result<(), String> {
    let loaded = execution.loaded;
    if let Scope::Unit(id) = execution.scope {
        let unit = loaded.unit(id)?;
        if policy::requires_rust(&unit.invocation()) {
            execution.toolchain()?;
        }
        if let Unit::Build {
            manifest, asset, ..
        } = unit
        {
            execution.run(
                "build",
                vec![
                    path_text(&execution.tools["cargo"])?,
                    "build".into(),
                    "--release".into(),
                    "--locked".into(),
                    "--manifest-path".into(),
                    manifest.clone(),
                ],
                None,
            )?;
            let source = &loaded.plan.assets[asset];
            let packed = transport::pack(
                &execution.root,
                source,
                &execution.output.join("executable.tar"),
            )?;
            execution.report["assets"] = json!({asset:packed.executable.receipt(source)});
            execution.report["transport"] = packed.archive.receipt();
            return execution.final_source();
        }
    }
    let selected: BTreeMap<String, receipt::Selection> =
        serde_json::from_value(execution.report["selected"].clone()).map_err(error)?;
    let mut originals = BTreeMap::new();
    let mut fingerprint = None;
    for unit in &loaded.recipe.units {
        if unit.asset().is_none() || !selected.contains_key(unit.id()) {
            continue;
        }
        execution.phase = format!("selected-build:{}", unit.id());
        let imported = receipt::import_build(
            loaded,
            &execution.root,
            &execution.report["context"],
            lineage,
            unit,
            &selected[unit.id()],
        )?;
        let current = imported
            .original
            .compiler_fingerprint()
            .ok_or_else(|| error("missing producer compiler fingerprint"))?;
        if fingerprint
            .as_ref()
            .is_some_and(|expected| *expected != current)
        {
            return Err(error("selected producer toolchain versions differ"));
        }
        fingerprint = Some(current);
        execution.report["assets"][&imported.asset] = imported.lineage;
        originals.insert(unit.id().to_owned(), imported.original);
    }
    if let Scope::Unit(id) = execution.scope {
        let unit = loaded.unit(id)?;
        if policy::requires_rust(&unit.invocation())
            && fingerprint.as_ref().is_some_and(|expected| {
                Verified {
                    directory: execution.output.clone(),
                    receipt: execution.report.clone(),
                    sha256: String::new(),
                }
                .compiler_fingerprint()
                .as_ref()
                    != Some(expected)
            })
        {
            return Err(error(
                "verification toolchain differs from selected producers",
            ));
        }
    } else if fingerprint.is_none() {
        return Err(error("collector has no selected producers"));
    }
    let mut staging = execution.stage()?;
    match execution.scope {
        Scope::Unit(id) => {
            let operation = loaded
                .unit(id)?
                .operation()
                .ok_or_else(|| error("expected verification unit"))?;
            let action = &loaded.actions[operation];
            let argv = std::iter::once(path_text(&execution.tools[&action.tool])?)
                .chain(action.argv.clone())
                .collect();
            if !staging.observe(&execution.root, "before-verification", Some(operation)) {
                execution.report["consumer_staging"] = staging.receipt();
                return Err(error("changed consumer before verification"));
            }
            let result = execution.run("verification", argv, Some(operation));
            let intact = staging.observe(&execution.root, "after-verification", Some(operation));
            execution.report["consumer_staging"] = staging.receipt();
            result?;
            if !intact {
                return Err(error("changed release consumer after verification"));
            }
        }
        Scope::Collect => {
            let metadata = &loaded.recipe.collector.dependency_metadata;
            let raw = fs::read(no_symlink_parents(&execution.root, metadata)?).map_err(error)?;
            if chrono_harness::json(&raw)?
                != chrono_harness::json(
                    env(&execution.environment, "CHRONO_RELEASE_DEPENDENCIES")?.as_bytes(),
                )?
            {
                return Err(error(
                    "native current dependency metadata differs from selected downloads",
                ));
            }
            let file = execution.output.join("dependency-file.json");
            let mut retained = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file)
                .map_err(error)?;
            retained
                .write_all(&raw)
                .and_then(|()| retained.flush())
                .map_err(error)?;
            execution.report["dependency_file"] = json!({"path":"dependency-file.json","size":raw.len(),"sha256":chrono_harness::sha256(&raw),"source_path":metadata});
            for unit in &loaded.recipe.units {
                if unit.operation().is_none() {
                    continue;
                }
                execution.phase = format!("selected-verification:{}", unit.id());
                let verified = receipt::read(
                    loaded,
                    &execution.root,
                    &execution.report["context"],
                    lineage,
                    unit,
                    &selected[unit.id()],
                )?;
                if policy::requires_rust(&unit.invocation())
                    && verified.compiler_fingerprint() != fingerprint
                {
                    return Err(error(
                        "selected verification toolchain differs from producers",
                    ));
                }
                receipt::verify_consumption(loaded, &verified, &execution.report["assets"])?;
                if execution.report.get("verifications").is_none() {
                    execution.report["verifications"] = json!({});
                }
                execution.report["verifications"][unit.id()] =
                    json!({"receipt_sha256":verified.sha256,"selection":selected[unit.id()]});
                originals.insert(unit.id().to_owned(), verified);
            }
            let mut retained = BTreeMap::new();
            let mut started = u64::MAX;
            for (id, original) in &originals {
                started = started.min(
                    original.receipt["started_ns"]
                        .as_u64()
                        .ok_or_else(|| error("missing original start"))?,
                );
                retained.insert(id, retain_original(&execution.output, id, original)?);
            }
            execution.report["originals"] = json!(retained);
            execution.report["platform_started_ns"] = json!(started);
            if !staging.observe(&execution.root, "before-package", None) {
                execution.report["consumer_staging"] = staging.receipt();
                return Err(error("changed consumer before package"));
            }
            let program = no_symlink_parents(
                &execution.root,
                &loaded.plan.assets[&loaded.recipe.collector.package_asset],
            )?;
            let package = execution.output.join("package");
            let result = execution.run(
                "package",
                vec![
                    path_text(&program)?,
                    "pack".into(),
                    "--root".into(),
                    path_text(&execution.root)?,
                    "--plan".into(),
                    path_text(
                        &execution
                            .root
                            .join(&loaded.recipe.consumer_staging.release_plan),
                    )?,
                    "--output".into(),
                    path_text(&package)?,
                ],
                None,
            );
            let intact = staging.observe(&execution.root, "after-package", None);
            execution.report["consumer_staging"] = staging.receipt();
            result?;
            if !intact {
                return Err(error("changed consumer after package"));
            }
            execution.phase = "package-identity".into();
            let identity = package_identity(&package, loaded, &execution.report)?;
            let assets = identity["assets"]
                .as_object()
                .ok_or_else(|| error("missing package asset vector"))?;
            let names: Vec<_> = std::iter::once("release.json".to_owned())
                .chain(
                    assets
                        .values()
                        .map(|v| v["path"].as_str().expect("observed asset path").to_owned()),
                )
                .collect();
            for name in names {
                let target = no_symlink_parents(&execution.output, &name)?;
                if fs::symlink_metadata(&target).is_ok() {
                    return Err(error("package output collision"));
                }
                fs::rename(no_symlink_parents(&package, &name)?, target).map_err(error)?;
            }
            fs::remove_dir(package).map_err(error)?;
            execution.report["package"] = identity;
        }
    }
    execution.final_source()
}

pub fn scoped(
    root: &Path,
    output: &Path,
    scope: Scope<'_>,
    environment: Environment,
) -> Result<i32, String> {
    let loaded = Loaded::load(root)?;
    let mut execution = Execution::begin(root, output, &loaded, scope, environment)?;
    let outcome = execution
        .context()
        .and_then(|lineage| units(&mut execution, &lineage));
    Ok(execution.finish(outcome))
}
