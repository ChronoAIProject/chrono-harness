//! 收齐登记的平台原件后，复用 distribution assemble，再执行公开发布状态机。
use crate::{
    execution::Environment,
    identity,
    publication::{self, Commands, Reply},
    receipt,
    recipe::{self, Loaded},
};
use chrono_harness::{file_identity, no_symlink_parents};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub platform: String,
    pub directory: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub publication: publication::Config,
    pub packages: BTreeMap<String, Package>,
    pub frontend: String,
    pub tools: BTreeMap<String, String>,
    pub curl: String,
    pub installation: Installation,
    pub promotion: Option<promotion::Config>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub publisher: String,
    pub directory: String,
    pub shell: String,
}

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_DELIVERY: {message}")
}
fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| error(format!("missing {name}")))
}
fn env<'a>(environment: &'a Environment, name: &str) -> Result<&'a str, String> {
    environment
        .get(OsStr::new(name))
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| error(format!("missing {name}")))
}
fn positive(s: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .ok()
        .filter(|v| *v > 0 && v.to_string() == s)
        .ok_or_else(|| error("invalid run, attempt or artifact identity"))
}

pub fn select(
    job: &str,
    needs: &Value,
    run: &str,
    attempt: &str,
) -> Result<receipt::Selection, String> {
    positive(run)?;
    let limit = positive(attempt)?;
    let row = &needs[job];
    let outputs = &row["outputs"];
    let selected = receipt::Selection {
        job: job.into(),
        artifact_id: text(outputs, "artifact_id")?.into(),
        run_id: text(outputs, "run_id")?.into(),
        attempt: text(outputs, "attempt")?.into(),
    };
    if row["result"] != "success" || selected.run_id != run || positive(&selected.attempt)? > limit
    {
        return Err(error(format!("current dependency cannot select {job}")));
    }
    positive(&selected.artifact_id)?;
    Ok(selected)
}

pub fn verify_package(
    root: &Path,
    loaded: &Loaded,
    package: &Package,
    selected: &receipt::Selection,
    identity: &identity::Identity,
) -> Result<Value, String> {
    let directory = no_symlink_parents(root, &package.directory)?;
    let report = recipe::read_json(&directory, "receipt.json")?;
    let lineage: receipt::Lineage =
        serde_json::from_value(report["lineage"].clone()).map_err(error)?;
    let expected = receipt::Lineage {
        kind: receipt::Kind::Github,
        run_id: selected.run_id.clone(),
        attempt: selected.attempt.clone(),
        job: selected.job.clone(),
    };
    let start = report["started_ns"]
        .as_u64()
        .ok_or_else(|| error("missing collector start"))?;
    let finish = report["finished_ns"]
        .as_u64()
        .ok_or_else(|| error("missing collector finish"))?;
    if report["schema"] != "chrono-release-unit/v1"
        || report["status"] != "passed"
        || report["failure"] != Value::Null
        || lineage != expected
        || !crate::verified_recipe::delivery::original_spans(
            start,
            finish,
            report["platform_started_ns"].as_u64(),
            report["platform_span_ns"].as_u64(),
            loaded.recipe.limits.unit_seconds,
            loaded.recipe.limits.platform_seconds,
        )
        || report["unit"] != serde_json::to_value(&loaded.recipe.collector).map_err(error)?
        || report["context"]["release_identity"] != serde_json::to_value(identity).map_err(error)?
        || report["context"]["source_commit"] != identity.source_commit
        || report["context"]["source_tree"] != identity.source_tree
    {
        return Err(error(
            "collector source, identity, lineage or result mismatch",
        ));
    }
    let release = recipe::read_json(&directory, "release.json")?;
    let members = release["platforms"][&package.platform]
        .as_object()
        .ok_or_else(|| error("missing declared platform"))?;
    if release["schema"] != "chrono-release/v1"
        || release["version"] != identity.version
        || release["source_commit"] != identity.source_commit
        || release["source_tree"] != identity.source_tree
        || release["platforms"]
            .as_object()
            .is_none_or(|p| p.len() != 1)
        || members.keys().collect::<BTreeSet<_>>() != loaded.plan.assets.keys().collect()
        || report["package"]["release_sha256"] != file_identity(&directory.join("release.json"))?.0
    {
        return Err(error(
            "platform package differs from the selected collector",
        ));
    }
    for (asset, declared) in members {
        let name = format!("{asset}-{}", package.platform);
        let file = no_symlink_parents(&directory, &name)?;
        let (sha, size) = file_identity(&file)?;
        let observed = &report["package"]["assets"][asset];
        if declared["file"] != name
            || declared["sha256"] != sha
            || declared["size"] != size
            || observed["path"] != name
            || observed["sha256"] != sha
            || observed["size"] != size
        {
            return Err(error(format!("platform asset differs: {asset}")));
        }
    }
    let originals = report["originals"]
        .as_object()
        .ok_or_else(|| error("missing original receipts"))?;
    if originals.keys().collect::<BTreeSet<_>>() != loaded.recipe.collector.needs.iter().collect() {
        return Err(error("collector original membership differs"));
    }
    for (id, files) in originals {
        for declaration in files
            .as_object()
            .ok_or_else(|| error("invalid retained originals"))?
            .values()
        {
            receipt::original(&directory.join("originals").join(id), declaration)?;
        }
    }
    let processes = report["processes"]
        .as_array()
        .filter(|p| !p.is_empty())
        .ok_or_else(|| error("missing collector processes"))?;
    for process in processes {
        if process["exit_code"] != 0 {
            return Err(error("failed original collector process"));
        }
        for stream in ["stdout", "stderr"] {
            receipt::original(&directory, &process[stream])?;
        }
    }
    Ok(report)
}

struct Executor<'a> {
    root: &'a Path,
    output: &'a Path,
    environment: &'a Environment,
    tools: BTreeMap<String, PathBuf>,
    started: Instant,
    seconds: u64,
    report: Value,
    bodies: usize,
}

impl Commands for Executor<'_> {
    fn execute(&mut self, program: &str, args: &[String]) -> Result<Reply, String> {
        let tool = program;
        let program = if let Some(tool) = self.tools.get(program) {
            tool.clone()
        } else {
            let path = PathBuf::from(program);
            if !path.is_absolute() {
                return Err(error("undeclared delivery executable"));
            }
            path
        };
        let timeout = Duration::from_secs(self.seconds)
            .checked_sub(self.started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| error("delivery span exhausted"))?;
        let environment = self
            .environment
            .iter()
            .filter(|(key, _)| key.as_os_str() != OsStr::new(chrono_harness::process_fds::ENV))
            .map(|(k, v)| {
                Ok((
                    k.to_str().ok_or("non UTF-8 environment key")?.into(),
                    v.to_str().ok_or("non UTF-8 environment value")?.into(),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let spec = chrono_harness::CommandSpec {
            program: program.to_str().ok_or("invalid delivery tool path")?.into(),
            args: args.to_vec(),
            env: environment,
            timeout_seconds: timeout.as_secs().saturating_add(1),
            output_limit_bytes: chrono_harness::verified_limits::MAX_OUTPUT_BYTES,
        };
        let index = self.report["processes"]
            .as_array()
            .ok_or("invalid process list")?
            .len();
        let captured = crate::process::execute(
            self.root,
            &spec,
            self.output,
            index,
            timeout,
            std::io::stdout(),
            std::io::stderr(),
        )?;
        let mut original = captured.receipt("delivery", None);
        original["tool"] = json!(tool);
        self.report["processes"]
            .as_array_mut()
            .ok_or("invalid process list")?
            .push(original);
        if captured.auxiliary_failed() {
            return Err(error(
                "delivery process or original stream retention failed",
            ));
        }
        Ok(Reply {
            exit: captured
                .child_exit
                .ok_or("unknown delivery process result")?,
            stdout: captured.stdout.bytes,
        })
    }
    fn body(&mut self, value: &Value) -> Result<String, String> {
        let path = self.output.join(format!("request-{}.json", self.bodies));
        self.bodies += 1;
        fs::write(&path, serde_json::to_vec(value).map_err(error)?).map_err(error)?;
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| error("invalid request path"))
    }
}

/// frontend 在导出前使用现有发布执行器观察工作流，失败原流也归该 artifact 保管。
pub(crate) fn workflow_preflight(
    root: &Path,
    output: &Path,
    loaded: &Loaded,
    workflow_path: &str,
    source_commit: &str,
    environment: &Environment,
) -> Result<identity::WorkflowIdentity, String> {
    let program = loaded
        .recipe
        .tools
        .get("git")
        .ok_or("missing release Git")?;
    let git = chrono_harness::resolve_program(root, program, Some(env(environment, "PATH")?))?;
    let mut executor = Executor {
        root,
        output,
        environment,
        tools: BTreeMap::from([("git".into(), git)]),
        started: Instant::now(),
        seconds: loaded.recipe.limits.unit_seconds,
        report: json!({"schema":"chrono-release-workflow/v1","status":"failed","processes":[]}),
        bodies: 0,
    };
    let outcome =
        identity::verify_workflow(workflow_path, source_commit, environment, &mut executor);
    match &outcome {
        Ok(identity) => {
            executor.report["status"] = json!("passed");
            executor.report["identity"] = serde_json::to_value(identity).map_err(error)?;
        }
        Err(failure) => executor.report["failure"] = json!(failure),
    }
    fs::write(
        output.join("workflow.json"),
        serde_json::to_vec_pretty(&executor.report).map_err(error)?,
    )
    .map_err(error)?;
    outcome
}

fn checked(
    executor: &mut impl Commands,
    program: &str,
    args: &[String],
) -> Result<Vec<u8>, String> {
    let reply = executor.execute(program, args)?;
    if reply.exit != 0 {
        return Err(error(format!("original {program} exited {}", reply.exit)));
    }
    Ok(reply.stdout)
}

fn source(executor: &mut impl Commands) -> Result<(String, String), String> {
    let mut query = |args: &[&str]| -> Result<String, String> {
        String::from_utf8(checked(
            executor,
            "git",
            &args.iter().map(|v| (*v).into()).collect::<Vec<_>>(),
        )?)
        .map_err(error)
    };
    let commit = query(&["rev-parse", "HEAD"])?.trim().to_owned();
    let tree = query(&["rev-parse", "HEAD^{tree}"])?.trim().to_owned();
    if !query(&["status", "--porcelain", "--untracked-files=normal"])?.is_empty() {
        return Err(error("delivery requires fixed clean source"));
    }
    Ok((commit, tree))
}

fn perform(
    root: &Path,
    output: &Path,
    config: &Config,
    loaded: &Loaded,
    environment: &Environment,
    executor: &mut Executor<'_>,
) -> Result<Value, String> {
    config.publication.validate()?;
    if config.schema != "chrono-release-delivery/v1"
        || config.packages.keys().collect::<BTreeSet<_>>()
            != loaded.recipe.native_jobs.keys().collect()
        || config
            .tools
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != BTreeSet::from(["gh", "git"])
    {
        return Err(error("delivery platform or tool registration mismatch"));
    }
    let (commit, tree) = source(executor)?;
    let identity = identity::resolve(&loaded.plan.version, &commit, &tree, environment)?;
    executor.report["identity"] = serde_json::to_value(&identity).map_err(error)?;
    let needs = chrono_harness::json(env(environment, "CHRONO_RELEASE_DEPENDENCIES")?.as_bytes())?;
    let run = env(environment, "CHRONO_RELEASE_RUN")?;
    let attempt = env(environment, "CHRONO_RELEASE_ATTEMPT")?;
    executor.report["dependencies"] = needs.clone();
    let declared = loaded
        .recipe
        .native_frontends
        .get(&config.frontend)
        .ok_or("missing delivery frontend")?;
    let selected = select(&declared.job, &needs, run, attempt)?;
    crate::frontend::validate(
        root,
        &no_symlink_parents(root, &declared.directory)?,
        declared,
        &chrono_host_bootstrap::bindings::native_platform()?,
        &chrono_host_bootstrap::source::SourceObservation {
            commit: Some(commit.clone()),
            tree: Some(tree.clone()),
            dirty: Some(false),
            error: None,
        },
        &selected,
        Some(&std::env::current_exe().map_err(error)?),
    )?;
    let promoted = if identity.channel == identity::Channel::Stable {
        Some(promotion::prepare(
            root,
            output,
            config,
            loaded,
            &identity,
            environment,
            executor,
        )?)
    } else {
        None
    };
    if let Some(origin) = &promoted {
        executor.report["promotion"] = origin.observation.clone();
    }
    let package_identity = promoted.as_ref().map_or(&identity, |p| &p.identity);
    let packages = promoted.as_ref().map_or(&config.packages, |p| &p.packages);
    let mut inputs = Vec::new();
    let mut reports = BTreeMap::new();
    for (native, package) in packages {
        let job = &loaded.recipe.native_jobs[native][&loaded.recipe.collector.id];
        let selection = if let Some(origin) = &promoted {
            origin.selections[native].clone()
        } else {
            select(job, &needs, run, attempt)?
        };
        let report = verify_package(root, loaded, package, &selection, package_identity)?;
        inputs.push(no_symlink_parents(root, &package.directory)?);
        reports.insert(native, report);
    }
    executor.report["selected"] = json!(
        reports
            .iter()
            .map(|(native, report)| (
                *native,
                json!({"lineage":report["lineage"],"package":report["package"]})
            ))
            .collect::<BTreeMap<_, _>>()
    );
    let native = packages
        .get(&config.frontend)
        .ok_or("missing native assembly package")?;
    let directory = no_symlink_parents(root, &native.directory)?;
    let name = format!(
        "{}-{}",
        loaded.recipe.collector.package_asset, native.platform
    );
    let program = no_symlink_parents(&directory, &name)?;
    let mode = reports[&config.frontend]["package"]["assets"]
        [&loaded.recipe.collector.package_asset]["mode"]
        .as_u64()
        .filter(|m| *m <= 0o7777 && *m & 0o111 != 0)
        .ok_or("invalid distribution mode")?;
    fs::set_permissions(&program, fs::Permissions::from_mode(mode as u32)).map_err(error)?;
    let combined = output.join("release");
    let mut args = vec!["assemble".into()];
    if promoted.is_some() {
        args.extend([
            "--from-version".into(),
            package_identity.version.clone(),
            "--version".into(),
            identity.version.clone(),
        ]);
    }
    for input in inputs {
        args.extend([
            "--input".into(),
            input.to_str().ok_or("invalid package path")?.into(),
        ]);
    }
    args.extend([
        "--output".into(),
        combined.to_str().ok_or("invalid release output")?.into(),
        "--base-url".into(),
        format!(
            "https://github.com/{}/releases/download/{}",
            config.publication.repository, identity.version
        ),
        "--curl".into(),
        config.curl.clone(),
    ]);
    checked(
        executor,
        program.to_str().ok_or("invalid distribution executable")?,
        &args,
    )?;
    let release = recipe::read_json(&combined, "release.json")?;
    if release["version"] != identity.version
        || release["source_commit"] != commit
        || release["source_tree"] != tree
        || release["platforms"]
            .as_object()
            .ok_or("missing combined platforms")?
            .keys()
            .collect::<BTreeSet<_>>()
            != config.packages.values().map(|p| &p.platform).collect()
    {
        return Err(error("assembled release identity mismatch"));
    }
    let mut names = BTreeSet::from(["release.json".to_owned(), "install.sh".to_owned()]);
    for package in config.packages.values() {
        for asset in loaded.plan.assets.keys() {
            names.insert(format!("{asset}-{}", package.platform));
        }
    }
    let mut assets = BTreeMap::new();
    for name in names {
        let path = no_symlink_parents(&combined, &name)?;
        let (sha256, size) = file_identity(&path)?;
        assets.insert(
            name,
            publication::Asset {
                path: path.to_str().ok_or("invalid upload path")?.into(),
                sha256,
                size,
            },
        );
    }
    if source(executor)? != (commit, tree) {
        return Err(error("source changed before publication"));
    }
    if identity.channel == identity::Channel::Build {
        Ok(json!({"mode":"build","identity":identity,"assets":assets}))
    } else {
        publication::publish(&config.publication, &identity, &assets, executor)
    }
}

pub fn run(
    root: &Path,
    output: &Path,
    config_path: &str,
    environment: &Environment,
) -> Result<i32, String> {
    let root = fs::canonicalize(root).map_err(error)?;
    let loaded = Loaded::load(&root)?;
    let config: Config =
        serde_json::from_value(recipe::read_json(&root, config_path)?).map_err(error)?;
    let output = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir().map_err(error)?.join(output)
    };
    fs::create_dir_all(output.parent().ok_or("missing output parent")?).map_err(error)?;
    fs::create_dir(&output).map_err(error)?;
    let search = env(environment, "PATH")?;
    let tools = config
        .tools
        .iter()
        .map(|(name, program)| {
            Ok((
                name.clone(),
                chrono_harness::resolve_program(&root, program, Some(search))?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let mut executor = Executor {
        root: &root,
        output: &output,
        environment,
        tools,
        started: Instant::now(),
        seconds: loaded.recipe.limits.unit_seconds,
        report: json!({"schema":"chrono-release-publication/v1","status":"failed","processes":[],
            "lineage":{"kind":"github","run_id":env(environment,"CHRONO_RELEASE_RUN")?,
                "attempt":env(environment,"CHRONO_RELEASE_ATTEMPT")?,"job":env(environment,"CHRONO_RELEASE_JOB")?}}),
        bodies: 0,
    };
    let outcome = perform(&root, &output, &config, &loaded, environment, &mut executor);
    match &outcome {
        Ok(result) => {
            executor.report["status"] = json!("passed");
            executor.report["result"] = result.clone();
        }
        Err(error) => {
            executor.report["failure"] = json!(error);
        }
    }
    fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&executor.report).map_err(error)?,
    )
    .map_err(error)?;
    outcome.map(|_| 0)
}

pub(crate) fn verify_publication_processes(
    publication: &Path,
    published: &Value,
    identity: &identity::Identity,
    repository: &str,
) -> Result<(), String> {
    for process in published["processes"]
        .as_array()
        .filter(|p| !p.is_empty())
        .ok_or_else(|| error("missing original publication processes"))?
    {
        let mut stdout = None;
        for stream in ["stdout", "stderr"] {
            if process[stream].get("status").is_some() {
                return Err(error("incomplete original publication stream"));
            }
            let original = receipt::original(publication, &process[stream])?;
            if stream == "stdout" {
                stdout = Some(original);
            }
        }
        let exit = process["exit_code"]
            .as_i64()
            .and_then(|exit| i32::try_from(exit).ok())
            .ok_or_else(|| error("missing original publication exit"))?;
        let observation = &process["observation"];
        if observation.get("failure") != Some(&Value::Null)
            || observation.get("exit_signal") != Some(&Value::Null)
            || observation["engine_exit_code"] != exit
        {
            return Err(error("failed original publication process"));
        }
        if exit == 0 {
            continue;
        }
        let stdout =
            fs::read(stdout.ok_or("missing original publication stdout")?).map_err(error)?;
        if !publication::original_absence(process, &stdout, repository, identity) {
            return Err(error("failed original publication process"));
        }
    }
    Ok(())
}

/// 安装使用原发布产物；命令边界在测试中模拟，安装结果由实际安装器产生。
pub fn install_release(
    publication: &Path,
    host: &Path,
    identity: &identity::Identity,
    platform: &str,
    members: &BTreeSet<String>,
    shell: &str,
    curl: &str,
    repository: &str,
    public_launcher: &str,
    commands: &mut impl Commands,
) -> Result<Value, String> {
    let published = recipe::read_json(publication, "receipt.json")?;
    if published["schema"] != "chrono-release-publication/v1"
        || published["status"] != "passed"
        || published["identity"] != serde_json::to_value(identity).map_err(error)?
        || (identity.channel == identity::Channel::Build && published["result"]["mode"] != "build")
        || (identity.channel != identity::Channel::Build
            && (published["result"]["release"]["draft"] != false
                || published["result"]["release"]["tag_name"] != identity.version))
    {
        return Err(error(
            "installation needs the matching successful publication",
        ));
    }
    verify_publication_processes(publication, &published, identity, repository)?;
    let directory = no_symlink_parents(publication, "release")?;
    let release = recipe::read_json(&directory, "release.json")?;
    let tools = release["platforms"][platform]
        .as_object()
        .ok_or_else(|| error("installation platform missing"))?;
    if release["schema"] != "chrono-release/v1"
        || release["version"] != identity.version
        || release["source_commit"] != identity.source_commit
        || release["source_tree"] != identity.source_tree
        || tools.keys().cloned().collect::<BTreeSet<_>>() != *members
        || !members.contains("chrono-distribution")
    {
        return Err(error("installation manifest or membership mismatch"));
    }
    let mut names = vec!["release.json".to_owned(), "install.sh".to_owned()];
    for (name, asset) in tools {
        let file = format!("{name}-{platform}");
        if asset["file"] != file {
            return Err(error("installation asset filename mismatch"));
        }
        names.push(file);
    }
    for name in &names {
        let asset = &published["result"]["assets"][name];
        receipt::original(
            &directory,
            &json!({"path":name,"sha256":asset["sha256"],"size":asset["size"]}),
        )?;
    }
    // create_dir 拒绝已有宿主及链接；不覆盖其它宿主的采用或安装。
    fs::create_dir(host).map_err(error)?;
    if identity.channel == identity::Channel::Build {
        let program = no_symlink_parents(&directory, &format!("chrono-distribution-{platform}"))?;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).map_err(error)?;
        let mut args = vec![
            "adopt".into(),
            "--host-root".into(),
            host.to_str().ok_or("invalid install host")?.into(),
            "--manifest".into(),
            directory
                .join("release.json")
                .to_str()
                .ok_or("invalid manifest path")?
                .into(),
            "--source-dir".into(),
            directory.to_str().ok_or("invalid release path")?.into(),
        ];
        for name in members {
            args.extend(["--tool".into(), name.clone()]);
        }
        checked(
            commands,
            program.to_str().ok_or("invalid installer path")?,
            &args,
        )?;
    }
    let launcher = if identity.channel == identity::Channel::Build {
        host.join(".chrono-harness/install.sh")
    } else {
        let downloaded = host.join("install.sh");
        checked(
            commands,
            curl,
            &[
                "--disable".into(),
                "--proto".into(),
                "=https".into(),
                "--proto-redir".into(),
                "=https".into(),
                "--fail".into(),
                "--show-error".into(),
                "--location".into(),
                "--output".into(),
                downloaded
                    .to_str()
                    .ok_or("invalid launcher download")?
                    .into(),
                public_launcher.into(),
            ],
        )?;
        let asset = &published["result"]["assets"]["install.sh"];
        receipt::original(
            host,
            &json!({"path":"install.sh","sha256":asset["sha256"],"size":asset["size"]}),
        )?;
        downloaded
    };
    checked(
        commands,
        shell,
        &[
            launcher.to_str().ok_or("invalid launcher path")?.into(),
            "--host-root".into(),
            host.to_str().ok_or("invalid install host")?.into(),
        ],
    )?;
    let installed = recipe::read_json(host, ".chrono-harness/state/distribution.json")?;
    let rows = installed["installed"]
        .as_array()
        .ok_or("missing installed members")?;
    if installed["schema"] != "chrono-install-result/v1"
        || installed["version"] != identity.version
        || installed["source_commit"] != identity.source_commit
        || installed["source_tree"] != identity.source_tree
        || installed["platform"] != platform
        || rows.len() != members.len()
        || rows
            .iter()
            .filter_map(|row| row["name"].as_str())
            .collect::<BTreeSet<_>>()
            != members.iter().map(String::as_str).collect()
    {
        return Err(error("actual installation receipt differs"));
    }
    for row in rows {
        let name = text(row, "name")?;
        let path = format!(".chrono-harness/bin/{name}");
        let actual = no_symlink_parents(host, &path)?;
        let (sha, size) = file_identity(&actual)?;
        if row["path"] != path
            || row["sha256"] != sha
            || row["size"] != size
            || tools[name]["sha256"] != sha
            || tools[name]["size"] != size
            || fs::metadata(actual).map_err(error)?.permissions().mode() & 0o111 == 0
        {
            return Err(error(format!("installed member differs: {name}")));
        }
    }
    Ok(installed)
}

pub fn install(
    root: &Path,
    output: &Path,
    config_path: &str,
    native: &str,
    environment: &Environment,
) -> Result<i32, String> {
    let root = fs::canonicalize(root).map_err(error)?;
    let loaded = Loaded::load(&root)?;
    let config: Config =
        serde_json::from_value(recipe::read_json(&root, config_path)?).map_err(error)?;
    if config.schema != "chrono-release-delivery/v1" {
        return Err(error("unknown installation delivery configuration"));
    }
    let output = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir().map_err(error)?.join(output)
    };
    fs::create_dir_all(output.parent().ok_or("missing install output parent")?).map_err(error)?;
    fs::create_dir(&output).map_err(error)?;
    let report = output.join("report");
    fs::create_dir(&report).map_err(error)?;
    let search = env(environment, "PATH")?;
    let tools = config
        .tools
        .iter()
        .filter(|(name, _)| name.as_str() == "git")
        .map(|(name, program)| {
            Ok((
                name.clone(),
                chrono_harness::resolve_program(&root, program, Some(search))?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let mut executor = Executor {
        root: &root,
        output: &report,
        environment,
        tools,
        started: Instant::now(),
        seconds: loaded.recipe.limits.unit_seconds,
        bodies: 0,
        report: json!({"schema":"chrono-release-installation/v1","status":"failed","processes":[]}),
    };
    let outcome = (|| {
        let (commit, tree) = source(&mut executor)?;
        let identity = identity::resolve(&loaded.plan.version, &commit, &tree, environment)?;
        executor.report["identity"] = serde_json::to_value(&identity).map_err(error)?;
        let needs =
            chrono_harness::json(env(environment, "CHRONO_RELEASE_DEPENDENCIES")?.as_bytes())?;
        let run = env(environment, "CHRONO_RELEASE_RUN")?;
        let attempt = env(environment, "CHRONO_RELEASE_ATTEMPT")?;
        let selected = select(&config.installation.publisher, &needs, run, attempt)?;
        let publication = no_symlink_parents(&root, &config.installation.directory)?;
        let published = recipe::read_json(&publication, "receipt.json")?;
        let lineage: receipt::Lineage =
            serde_json::from_value(published["lineage"].clone()).map_err(error)?;
        if lineage
            != (receipt::Lineage {
                kind: receipt::Kind::Github,
                run_id: selected.run_id,
                attempt: selected.attempt,
                job: selected.job,
            })
        {
            return Err(error("selected publication lineage differs"));
        }
        let declared = loaded
            .recipe
            .native_frontends
            .get(native)
            .ok_or("unknown installation frontend")?;
        let selected = select(&declared.job, &needs, run, attempt)?;
        crate::frontend::validate(
            &root,
            &no_symlink_parents(&root, &declared.directory)?,
            declared,
            &chrono_host_bootstrap::bindings::native_platform()?,
            &chrono_host_bootstrap::source::SourceObservation {
                commit: Some(commit.clone()),
                tree: Some(tree.clone()),
                dirty: Some(false),
                error: None,
            },
            &selected,
            Some(&std::env::current_exe().map_err(error)?),
        )?;
        let platform = &config
            .packages
            .get(native)
            .ok_or("unknown installation platform")?
            .platform;
        let shell =
            chrono_harness::resolve_program(&root, &config.installation.shell, Some(search))?;
        let curl = chrono_harness::resolve_program(&root, &config.curl, Some(search))?;
        let public_launcher = format!(
            "https://github.com/{}/releases/download/{}/install.sh",
            config.publication.repository, identity.version
        );
        let installed = install_release(
            &publication,
            &output.join("host"),
            &identity,
            platform,
            &loaded.plan.assets.keys().cloned().collect(),
            shell.to_str().ok_or("invalid shell path")?,
            curl.to_str().ok_or("invalid curl path")?,
            &config.publication.repository,
            &public_launcher,
            &mut executor,
        )?;
        if source(&mut executor)? != (commit, tree) {
            return Err(error("source changed during installation"));
        }
        Ok(installed)
    })();
    match &outcome {
        Ok(result) => {
            executor.report["status"] = json!("passed");
            executor.report["result"] = result.clone();
        }
        Err(error) => {
            executor.report["failure"] = json!(error);
        }
    }
    fs::write(
        report.join("receipt.json"),
        serde_json::to_vec_pretty(&executor.report).map_err(error)?,
    )
    .map_err(error)?;
    outcome.map(|_| 0)
}

/// 晋升沿用本发布者；IO 编排不另建发布入口。
pub mod promotion {
    use crate::{
        delivery::{self, Package},
        execution::Environment,
        identity::{self, Channel, Identity},
        publication::{self, Asset, Commands},
        receipt::Selection,
        recipe::{self, Loaded},
        verified_recipe::delivery as policy,
    };
    use chrono_harness::{file_identity, no_symlink_parents};
    use serde::Deserialize;
    use serde_json::{Value, json};
    use std::{
        collections::{BTreeMap, BTreeSet},
        ffi::OsStr,
        fs,
        path::Path,
    };

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        pub workflow_id: u64,
        pub workflow_config: String,
    }

    pub struct Prepared {
        pub identity: Identity,
        pub packages: BTreeMap<String, Package>,
        pub selections: BTreeMap<String, Selection>,
        pub observation: Value,
    }

    fn error(message: impl std::fmt::Display) -> String {
        format!("E_RELEASE_PROMOTION: {message}")
    }
    fn positive(value: &str) -> Result<u64, String> {
        value
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0 && n.to_string() == value)
            .ok_or_else(|| error("explicit positive preview run and attempt required"))
    }
    fn env<'a>(environment: &'a Environment, key: &str) -> Result<&'a str, String> {
        environment
            .get(OsStr::new(key))
            .and_then(|v| v.to_str())
            .ok_or_else(|| error(format!("missing {key}")))
    }

    /// 固定 attempt 的成功终态包括原发布及两平台安装；不能只核 source SHA。
    fn verify_run(
        run: &Value,
        repository: &str,
        workflow_id: u64,
        path: &str,
        id: u64,
        attempt: u64,
    ) -> Result<(), String> {
        if !policy::promotion_run(
            &[
                (run["id"].as_u64(), id),
                (run["run_attempt"].as_u64(), attempt),
                (run["workflow_id"].as_u64(), workflow_id),
            ],
            &[
                (run["path"].as_str(), path),
                (run["repository"]["full_name"].as_str(), repository),
                (run["status"].as_str(), "completed"),
                (run["conclusion"].as_str(), "success"),
            ],
            run["event"].as_str(),
            run["head_sha"].as_str(),
        ) {
            return Err(error(
                "original preview workflow attempt is not successful or has another identity",
            ));
        }
        Ok(())
    }

    /// API 精确名称查询必须只有一个未过期、不可变的工件，再按其同一 run/name 下载。
    fn select_artifact(
        list: &Value,
        name: &str,
        run: u64,
        head: &str,
        expected: Option<&str>,
    ) -> Result<Value, String> {
        let rows = list["artifacts"]
            .as_array()
            .ok_or_else(|| error("missing artifacts"))?;
        if !policy::single_original(list["total_count"].as_u64(), rows.len()) {
            return Err(error("original artifact is missing or ambiguous"));
        }
        let artifact = &rows[0];
        let selected = expected.map(positive).transpose()?;
        if !policy::promotion_artifact(
            artifact["id"].as_u64(),
            selected,
            artifact["expired"].as_bool(),
            artifact["size_in_bytes"].as_u64(),
            &[(artifact["workflow_run"]["id"].as_u64(), run)],
            &[
                (artifact["name"].as_str(), name),
                (artifact["workflow_run"]["head_sha"].as_str(), head),
            ],
        ) {
            return Err(error(
                "original artifact identity, expiry or selected producer differs",
            ));
        }
        Ok(artifact.clone())
    }

    fn download(
        repository: &str,
        run: &Value,
        name: &str,
        destination: &Path,
        expected: Option<&str>,
        commands: &mut impl Commands,
    ) -> Result<Value, String> {
        if destination.exists() {
            return Err(error("original download destination already exists"));
        }
        let id = run["id"].as_u64().ok_or_else(|| error("missing run ID"))?;
        let list = publication::read_api(
            commands,
            format!("repos/{repository}/actions/runs/{id}/artifacts?name={name}&per_page=100"),
        )?;
        let artifact = select_artifact(
            &list,
            name,
            id,
            run["head_sha"]
                .as_str()
                .ok_or_else(|| error("missing workflow head"))?,
            expected,
        )?;
        let reply = commands.execute(
            "gh",
            &[
                "run".into(),
                "download".into(),
                id.to_string(),
                "--repo".into(),
                repository.into(),
                "--name".into(),
                name.into(),
                "--dir".into(),
                destination
                    .to_str()
                    .ok_or_else(|| error("invalid download path"))?
                    .into(),
            ],
        )?;
        if reply.exit != 0 {
            return Err(error(format!(
                "original artifact download exited {}",
                reply.exit
            )));
        }
        // CLI 的退出不能代替下载结果；后续逐个核原回执、原流及包字节。
        if !destination.is_dir() {
            return Err(error("original artifact download produced no directory"));
        }
        Ok(artifact)
    }

    fn job<'a>(workflow: &'a Value, id: &str) -> Result<&'a Value, String> {
        let rows = workflow["jobs"]
            .as_array()
            .ok_or_else(|| error("missing workflow jobs"))?;
        let found = rows
            .iter()
            .filter(|row| row["id"] == id)
            .collect::<Vec<_>>();
        if found.len() != 1 {
            return Err(error(format!("missing or ambiguous registered job {id}")));
        }
        Ok(found[0])
    }
    fn artifact_name(
        workflow: &Value,
        job_id: &str,
        run: &str,
        attempt: &str,
    ) -> Result<String, String> {
        let prefix = job(workflow, job_id)?["artifact_name"]
            .as_str()
            .ok_or_else(|| error("missing artifact name"))?;
        if !prefix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || prefix.is_empty()
        {
            return Err(error("artifact prefix must be an explicit literal name"));
        }
        Ok(format!("{prefix}-{run}-{attempt}"))
    }

    pub fn prepare(
        root: &Path,
        output: &Path,
        config: &delivery::Config,
        loaded: &Loaded,
        stable: &Identity,
        environment: &Environment,
        commands: &mut impl Commands,
    ) -> Result<Prepared, String> {
        if stable.channel != Channel::Stable {
            return Err(error("only stable releases promote previews"));
        }
        let promotion = config
            .promotion
            .as_ref()
            .ok_or_else(|| error("missing promotion registration"))?;
        if promotion.workflow_id == 0 {
            return Err(error("missing release workflow identity"));
        }
        let workflow = recipe::read_json(root, &promotion.workflow_config)?;
        let path = workflow["workflow_path"]
            .as_str()
            .ok_or_else(|| error("missing workflow path"))?;
        let publisher = workflow["delivery"]["publisher"]
            .as_str()
            .ok_or_else(|| error("missing publisher"))?;
        let run_id = env(environment, "CHRONO_RELEASE_PREVIEW_RUN")?;
        let attempt = env(environment, "CHRONO_RELEASE_PREVIEW_ATTEMPT")?;
        let (run_number, attempt_number) = (positive(run_id)?, positive(attempt)?);
        if run_id == env(environment, "CHRONO_RELEASE_RUN")? {
            return Err(error("preview must be a completed earlier run"));
        }
        let run = publication::read_api(
            commands,
            format!(
                "repos/{}/actions/runs/{run_id}/attempts/{attempt}",
                config.publication.repository
            ),
        )?;
        verify_run(
            &run,
            &config.publication.repository,
            promotion.workflow_id,
            path,
            run_number,
            attempt_number,
        )?;
        let workflow_identity = identity::verify_workflow_commits(
            path,
            &stable.source_commit,
            run["head_sha"].as_str().unwrap(),
            commands,
        )?;
        let mut preview_environment = environment.clone();
        preview_environment.insert("CHRONO_RELEASE_CHANNEL".into(), "preview".into());
        preview_environment.remove(OsStr::new("CHRONO_RELEASE_VERSION"));
        let preview = identity::resolve(
            &loaded.plan.version,
            &stable.source_commit,
            &stable.source_tree,
            &preview_environment,
        )?;
        let origin = output.join("origin");
        fs::create_dir(&origin).map_err(error)?;
        let original_publication = origin.join("publication");
        let name = artifact_name(&workflow, publisher, run_id, attempt)?;
        let publication_artifact = download(
            &config.publication.repository,
            &run,
            &name,
            &original_publication,
            None,
            commands,
        )?;
        let published = recipe::read_json(&original_publication, "receipt.json")?;
        if published["schema"] != "chrono-release-publication/v1"
            || published["status"] != "passed"
            || published["identity"] != serde_json::to_value(&preview).map_err(error)?
            || published["lineage"]
                != json!({"kind":"github","run_id":run_id,"attempt":attempt,"job":publisher})
            || published["result"]["release"]["draft"] != false
            || published["result"]["release"]["prerelease"] != true
            || published["result"]["release"]["tag_name"] != preview.version
        {
            return Err(error(
                "original publication is not the matching successful preview",
            ));
        }
        delivery::verify_publication_processes(
            &original_publication,
            &published,
            &preview,
            &config.publication.repository,
        )?;
        let mut packages = BTreeMap::new();
        let mut selections = BTreeMap::new();
        let mut package_artifacts = BTreeMap::new();
        fs::create_dir(origin.join("platforms")).map_err(error)?;
        for (native, package) in &config.packages {
            let job_id = &loaded.recipe.native_jobs[native][&loaded.recipe.collector.id];
            let selection = delivery::select(job_id, &published["dependencies"], run_id, attempt)?;
            let name = artifact_name(&workflow, job_id, run_id, &selection.attempt)?;
            let destination = origin.join("platforms").join(native);
            let artifact = download(
                &config.publication.repository,
                &run,
                &name,
                &destination,
                Some(&selection.artifact_id),
                commands,
            )?;
            packages.insert(
                native.clone(),
                Package {
                    platform: package.platform.clone(),
                    directory: destination
                        .strip_prefix(root)
                        .map_err(error)?
                        .to_str()
                        .ok_or_else(|| error("invalid origin path"))?
                        .into(),
                },
            );
            selections.insert(native.clone(), selection);
            package_artifacts.insert(native.clone(), artifact);
        }
        // 公开版本、完整资产、标签和原 CI 均沿现有发布者的只读判法复核。
        let asset_root = original_publication.join("release");
        let declared = published["result"]["assets"]
            .as_object()
            .ok_or_else(|| error("missing original publication assets"))?;
        let mut names = BTreeSet::from(["install.sh".to_owned(), "release.json".to_owned()]);
        for package in config.packages.values() {
            for asset in loaded.plan.assets.keys() {
                names.insert(format!("{asset}-{}", package.platform));
            }
        }
        if declared.keys().collect::<BTreeSet<_>>() != names.iter().collect() {
            return Err(error("original publication asset membership differs"));
        }
        let mut assets = BTreeMap::new();
        for (name, declaration) in declared {
            let path = no_symlink_parents(&asset_root, name)?;
            let (sha256, size) = file_identity(&path)?;
            if declaration["sha256"] != sha256 || declaration["size"] != size {
                return Err(error("original publication asset bytes differ"));
            }
            assets.insert(
                name.clone(),
                Asset {
                    path: path
                        .to_str()
                        .ok_or_else(|| error("invalid original path"))?
                        .into(),
                    sha256,
                    size,
                },
            );
        }
        let release = recipe::read_json(&asset_root, "release.json")?;
        if release["schema"] != "chrono-release/v1"
            || release["version"] != preview.version
            || release["source_commit"] != preview.source_commit
            || release["source_tree"] != preview.source_tree
            || release["platforms"].as_object().is_none_or(|p| {
                p.keys().collect::<BTreeSet<_>>()
                    != config.packages.values().map(|p| &p.platform).collect()
            })
        {
            return Err(error(
                "original public manifest identity or platforms differ",
            ));
        }
        for package in packages.values() {
            let package_release = recipe::read_json(
                &no_symlink_parents(root, &package.directory)?,
                "release.json",
            )?;
            if release["platforms"][&package.platform]
                != package_release["platforms"][&package.platform]
            {
                return Err(error(
                    "original package differs from the public preview binaries",
                ));
            }
        }
        let public =
            publication::verify_published(&config.publication, &preview, &assets, commands)?;
        if public["release"]["id"] != published["result"]["release"]["id"] {
            return Err(error("original public release was replaced"));
        }
        let (receipt_sha256, receipt_size) =
            file_identity(&original_publication.join("receipt.json"))?;
        Ok(Prepared {
            identity: preview.clone(),
            packages,
            selections,
            observation: json!({
                "schema":"chrono-release-promotion/v1","identity":preview,"run":run,"workflow":workflow_identity,
                "publication_artifact":publication_artifact,"package_artifacts":package_artifacts,
                "publication_receipt":{"path":"origin/publication/receipt.json","sha256":receipt_sha256,"size":receipt_size},
                "verified_publication":public,
            }),
        })
    }
}
