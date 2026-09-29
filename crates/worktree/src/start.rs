use crate::{Config, Start};
use chrono_harness::{
    CommandSpec, ProcessResult, facts, json, no_symlink_parents, observation, run_process_observed,
    sha256,
};
use chrono_judge_registration::Registrations;
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub(crate) struct Runner {
    pub(crate) config: Config,
    environment: BTreeMap<String, String>,
    tool: observation::Tool,
    processes: Vec<Value>,
}
impl Runner {
    pub(crate) fn command(&mut self, root: &Path, args: &[&str]) -> Result<ProcessResult, String> {
        self.input(root, args, &[])
    }
    pub(crate) fn input(
        &mut self,
        root: &Path,
        args: &[&str],
        input: &[u8],
    ) -> Result<ProcessResult, String> {
        self.observe(root, args, input, false)
    }
    fn observe(
        &mut self,
        root: &Path,
        args: &[&str],
        input: &[u8],
        literal_inventory: bool,
    ) -> Result<ProcessResult, String> {
        let mut environment = self.environment.clone();
        if literal_inventory {
            // Artifact ownership uses literal, case-sensitive registered prefixes.
            // Record these command-specific overrides in the actual process result.
            for key in [
                "GIT_LITERAL_PATHSPECS",
                "GIT_GLOB_PATHSPECS",
                "GIT_NOGLOB_PATHSPECS",
                "GIT_ICASE_PATHSPECS",
            ] {
                environment.insert(key.into(), "0".into());
            }
        }
        let mut argv = vec!["--no-replace-objects".into()];
        argv.extend(args.iter().map(|s| (*s).into()));
        let spec = CommandSpec {
            program: self
                .tool
                .path
                .to_str()
                .ok_or("Git path is not UTF-8")?
                .into(),
            args: argv.clone(),
            env: environment,
            timeout_seconds: self.config.timeout_seconds,
            output_limit_bytes: self.config.output_limit_bytes,
        };
        match run_process_observed(root, &spec, input, &self.tool.sha256) {
            Ok(process) => {
                self.processes
                    .push(value!({"root":root,"argv":argv,"process":process}));
                if let Some(error) = &process.failure {
                    return Err(error.clone());
                }
                Ok(process)
            }
            Err(error) => {
                self.processes
                    .push(value!({"root":root,"argv":argv,"process":null,"error":error}));
                Err(error)
            }
        }
    }
    pub(crate) fn git(&mut self, root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
        let process = self.command(root, args)?;
        Self::output(process, args)
    }
    fn literal_inventory(&mut self, root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
        let process = self.observe(root, args, &[], true)?;
        Self::output(process, args)
    }
    fn output(process: ProcessResult, args: &[&str]) -> Result<Vec<u8>, String> {
        if process.exit_code != 0 {
            return Err(format!(
                "Git {} exited {}: {}",
                args[0], process.exit_code, process.stderr
            ));
        }
        Ok(process.stdout_bytes)
    }
    pub(crate) fn text(&mut self, root: &Path, args: &[&str]) -> Result<String, String> {
        String::from_utf8(self.git(root, args)?).map_err(|_| "Git output is not UTF-8".into())
    }
    pub(crate) fn oid(&mut self, root: &Path, revision: &str) -> Result<String, String> {
        let oid = self
            .text(
                root,
                &["rev-parse", "--verify", "--end-of-options", revision],
            )?
            .trim()
            .to_string();
        facts::full_oid(&oid)?;
        Ok(oid)
    }
    pub(crate) fn blob(&mut self, root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
        self.git(root, &["show", &format!("{oid}:{path}")])
    }
    pub(crate) fn inventory(
        &mut self,
        root: &Path,
    ) -> Result<Vec<BTreeMap<String, String>>, String> {
        let text = self.text(root, &["worktree", "list", "--porcelain", "-z"])?;
        let mut rows = vec![];
        for record in text.split("\0\0").filter(|r| !r.is_empty()) {
            let mut fields = BTreeMap::new();
            for field in record.split('\0').filter(|s| !s.is_empty()) {
                let (key, v) = field.split_once(' ').unwrap_or((field, ""));
                if fields.insert(key.into(), v.into()).is_some() {
                    return Err("duplicate Git worktree inventory field".into());
                }
            }
            if !fields.contains_key("worktree") {
                return Err("missing Git worktree inventory path".into());
            }
            rows.push(fields);
        }
        Ok(rows)
    }
}
fn destination(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or("destination has no parent")?;
    let name = path.file_name().ok_or("destination has no name")?;
    let target = fs::canonicalize(parent)
        .map_err(|e| format!("destination parent: {e}"))?
        .join(name);
    match fs::symlink_metadata(&target) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(target),
        Ok(_) => Err("destination already exists; preserve it and choose a new path".into()),
        Err(e) => Err(e.to_string()),
    }
}
pub(crate) fn ensure_identity(
    r: &mut Runner,
    source: &Path,
    target: &Path,
    branch: &str,
    base: &str,
    token: &str,
) -> Result<(), String> {
    let inventory = r.inventory(source)?;
    let rows: Vec<_> = inventory
        .iter()
        .filter(|row| row.get("worktree").is_some_and(|s| Path::new(s) == target))
        .collect();
    if rows.len() != 1
        || rows[0].get("HEAD").map(String::as_str) != Some(base)
        || rows[0].get("branch") != Some(&format!("refs/heads/{branch}"))
        || rows[0].get("locked").map(String::as_str) != Some(token)
        || rows[0].contains_key("prunable")
    {
        return Err("created worktree identity or lock changed; preserve it for recovery".into());
    }
    if r.oid(target, "HEAD")? != base
        || r.text(target, &["symbolic-ref", "HEAD"])?.trim() != format!("refs/heads/{branch}")
        || fs::canonicalize(r.text(target, &["rev-parse", "--show-toplevel"])?.trim())
            .map_err(|e| e.to_string())?
            != target
    {
        return Err(
            "created checkout differs from requested clean snapshot; preserve it for recovery"
                .into(),
        );
    }
    Ok(())
}
pub(crate) fn paths(bytes: Vec<u8>) -> Result<BTreeSet<String>, String> {
    let text = String::from_utf8(bytes).map_err(|_| "Git path is not UTF-8")?;
    if !text.is_empty() && !text.ends_with('\0') {
        return Err("incomplete Git path inventory".into());
    }
    let mut paths = BTreeSet::new();
    for path in text.split('\0').filter(|p| !p.is_empty()) {
        chrono_harness::relative_path(path)?;
        if !paths.insert(path.to_string()) {
            return Err("duplicate Git path".into());
        }
    }
    Ok(paths)
}
pub(crate) fn cleanliness(r: &mut Runner, target: &Path, config: &Value) -> Result<(), String> {
    let head = r.oid(target, "HEAD")?;
    exact_checkout(r, target, &head)?;
    untracked(r, target, config)
}
pub(crate) fn exact_checkout(r: &mut Runner, target: &Path, tree: &str) -> Result<(), String> {
    let expected = facts::parse_tree(&r.git(target, &["ls-tree", "-rz", "--full-tree", tree])?)?;
    let index = r.git(target, &["ls-files", "--stage", "-z"])?;
    if !facts::checkout_changes(target, &expected, &index)?.is_empty() {
        return Err(
            "checkout differs from requested clean snapshot; preserve it for recovery".into(),
        );
    }
    Ok(())
}
pub(crate) fn untracked(r: &mut Runner, target: &Path, config: &Value) -> Result<(), String> {
    let exclusions: Vec<String> = config["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| format!(":(top,exclude,literal){}", a["path"].as_str().unwrap()))
        .collect();
    for ignored in [false, true] {
        let mut args = vec!["ls-files", "--others", "--exclude-standard", "-z"];
        if ignored {
            args.push("--ignored");
        }
        args.extend(["--", "."]);
        args.extend(exclusions.iter().map(String::as_str));
        let observed: Vec<_> = paths(r.literal_inventory(target, &args)?)?
            .into_iter()
            .collect();
        if !chrono_judge_registration::nonartifact_paths(config, &observed).is_empty() {
            return Err("checkout has unregistered files; preserve it for recovery".into());
        }
    }
    Ok(())
}
pub(crate) fn execute(
    r: &mut Runner,
    o: &Start,
    bytes: &[u8],
    token: &str,
    report: &mut Value,
    keep_locked: bool,
) -> Result<(), String> {
    let source = &o.root;
    if fs::canonicalize(r.text(source, &["rev-parse", "--show-toplevel"])?.trim())
        .map_err(|e| e.to_string())?
        != *source
    {
        return Err("host root must be the actual Git checkout root".into());
    }
    let source_head = r.oid(source, "HEAD")?;
    report["source_commit"] = value!(source_head);
    if r.blob(source, &source_head, &o.config_path)? != bytes {
        return Err("worktree configuration differs from source commit".into());
    }
    let source_config = json(&r.blob(source, &source_head, &r.config.host_config.clone())?)?;
    let source_workflow_path = source_config["registries"]["workflow"]
        .as_str()
        .ok_or("missing registered workflow path")?
        .to_string();
    chrono_harness::relative_path(&source_workflow_path)?;
    let source_workflow = json(&r.blob(source, &source_head, &source_workflow_path)?)?;
    let target_branch = source_workflow["target_branch"]
        .as_str()
        .ok_or("missing registered target branch")?
        .to_string();
    let target_ref = format!("refs/heads/{target_branch}");
    r.git(source, &["check-ref-format", &target_ref])?;
    let target = destination(&o.destination)?;
    for row in r.inventory(source)? {
        let existing = Path::new(row.get("worktree").unwrap());
        if target.starts_with(existing) {
            return Err("destination overlaps an existing worktree".into());
        }
    }
    let fetch_ref = format!("refs/chrono-harness/fetch/{token}");
    let remote = r.config.remote.clone();
    let spec = format!("{target_ref}:{fetch_ref}");
    report["fetch_ref"] = value!(fetch_ref);
    report["remote"] = value!(remote);
    report["target_ref"] = value!(target_ref);
    let source_config_path = r.config.host_config.clone();
    let (source_registrations, source_digest) =
        registrations(r, source, &source_head, &source_config_path)?;
    registered_policy(
        &source_registrations,
        &o.config_path,
        &r.config.report_directory,
    )?;
    report["source_registry_digest"] = value!(source_digest);
    crate::fetch_recovery::publish(source, report)?;
    r.git(
        source,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "--",
            &remote,
            &spec,
        ],
    )?;
    let base = r.oid(source, &fetch_ref)?;
    let tree = r.oid(source, &format!("{base}^{{tree}}"))?;
    if r.oid(source, &format!("{base}^{{commit}}"))? != base {
        return Err("fetched target is not a commit".into());
    }
    report["base"] = value!(base);
    report["base_tree"] = value!(tree);
    // The fetched object is fixed before removing this invocation's temporary ref.
    r.git(
        source,
        &["update-ref", "--no-deref", "-d", &fetch_ref, &base],
    )?;
    report["fetch_ref_removed"] = value!(true);
    if r.blob(source, &base, &o.config_path)? != bytes {
        return Err(
            "fetched target changed worktree configuration; adopt it before starting".into(),
        );
    }
    let config_path = r.config.host_config.clone();
    let (registrations, registry_digest) = registrations(r, source, &base, &config_path)?;
    let workflow = registrations.workflow();
    if workflow["target_branch"] != target_branch {
        return Err("fetched target changed target branch; adopt its configuration".into());
    }
    registered_policy(&registrations, &o.config_path, &r.config.report_directory)?;
    let prefix_key = format!("{}_prefix", o.kind);
    let prefix = workflow[&prefix_key]
        .as_str()
        .ok_or("missing registered branch prefix")?;
    let branch = format!("{prefix}{}", o.name);
    let branch_ref = format!("refs/heads/{branch}");
    r.git(source, &["check-ref-format", &branch_ref])?;
    let old = r.command(source, &["show-ref", "--verify", "--quiet", &branch_ref])?;
    match old.exit_code {
        1 => (),
        0 => return Err("branch already exists; preserve it and choose a new name".into()),
        n => return Err(format!("branch lookup exited {n}")),
    }
    if destination(&target)? != target {
        return Err("destination changed during preparation".into());
    }
    let started = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| e.to_string())?;
    report["branch_ref"] = value!(branch);
    report["destination"] = value!(target);
    report["branch_started_at"] = value!(started);
    report["registry_digest"] = value!(registry_digest);
    let path = target.to_str().ok_or("destination is not UTF-8")?;
    crate::recovery::publish(source, report)?;
    // No existing branch reset, no inherited tracking setup, no restore recipe.
    r.git(
        source,
        &[
            "worktree",
            "add",
            "--lock",
            "--reason",
            token,
            "--no-track",
            "-b",
            &branch,
            "--",
            path,
            &base,
        ],
    )?;
    ensure_identity(r, source, &target, &branch, &base, token)?;
    cleanliness(r, &target, registrations.config())?;
    report["context"] = value!({"base":base,"candidate":base,"dev_tip":base,"fork_point":base,"branch_ref":branch,"branch_started_at":started});
    if keep_locked {
        return Ok(());
    }
    unlock(r, source, path, &branch_ref, &base)
}
fn registered_policy(
    registrations: &Registrations,
    policy: &str,
    report_directory: &str,
) -> Result<(), String> {
    if !registrations.filemap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == policy)
    {
        return Err("worktree configuration is not explicitly registered in FILEMAP".into());
    }
    if !registrations.config()["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| {
            a["tracked"] == false
                && a["path"]
                    .as_str()
                    .is_some_and(|p| report_directory.starts_with(p))
        })
    {
        return Err("worktree report directory lacks a registered untracked artifact owner".into());
    }
    Ok(())
}
pub(crate) fn unlock(
    r: &mut Runner,
    source: &Path,
    path: &str,
    branch_ref: &str,
    base: &str,
) -> Result<(), String> {
    r.git(source, &["worktree", "unlock", path])?;
    let rows = r.inventory(source)?;
    if !rows.iter().any(|row| {
        row.get("worktree").is_some_and(|p| p == path)
            && row.get("HEAD").map(String::as_str) == Some(base)
            && row.get("branch").map(String::as_str) == Some(branch_ref)
            && !row.contains_key("locked")
    }) {
        return Err("worktree changed during final observation".into());
    }
    Ok(())
}
pub(super) fn create(
    o: Start,
    config: Config,
    bytes: Vec<u8>,
    plan: Option<crate::reconstruct::Input>,
) -> Result<Value, String> {
    let operation = if plan.is_some() {
        "reconstruct"
    } else {
        "start"
    };
    let success = if plan.is_some() {
        "reconstructed"
    } else {
        "created"
    };
    with_report(
        &o.root,
        &o.config_path,
        config,
        &bytes,
        operation,
        success,
        |runner, token, report| match plan {
            Some(plan) => crate::reconstruct::execute(runner, &o, &bytes, token, report, plan),
            None => execute(runner, &o, &bytes, token, report, false),
        },
    )
}
pub(crate) fn with_report(
    root: &Path,
    config_path: &str,
    config: Config,
    bytes: &[u8],
    operation: &str,
    success: &str,
    execute: impl FnOnce(&mut Runner, &str, &mut Value) -> Result<(), String>,
) -> Result<Value, String> {
    if fs::canonicalize(root).map_err(|e| e.to_string())? != root {
        return Err("host root must be canonical".into());
    }
    let dir = no_symlink_parents(&root, config.report_directory.trim_end_matches('/'))?;
    let mut inherited = serde_json::Map::new();
    let mut environment = BTreeMap::new();
    let mut names = BTreeSet::new();
    for key in &config.environment.inherit {
        if key.is_empty() || key.contains(['=', '\0']) || !names.insert(key) {
            return Err("invalid or duplicate inherited environment name".into());
        }
        match std::env::var(key) {
            Ok(v) => {
                inherited.insert(key.clone(), value!(v));
                environment.insert(key.clone(), v);
            }
            Err(std::env::VarError::NotPresent) => {
                inherited.insert(key.clone(), Value::Null);
            }
            Err(_) => return Err("non-UTF8 inherited environment value".into()),
        }
    }
    environment.extend(config.environment.values.clone());
    if environment
        .iter()
        .any(|(key, value)| key.is_empty() || key.contains(['=', '\0']) || value.contains('\0'))
    {
        return Err("invalid explicit environment entry".into());
    }
    if [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
    ]
    .iter()
    .any(|k| environment.contains_key(*k))
    {
        return Err("Git repository redirection environment is unsupported".into());
    }
    if let Some(expected) = &config.git.sha256 {
        if !config.git.program.contains('/') && !environment.contains_key("PATH") {
            return Err("Git resolution requires declared PATH".into());
        }
        let path = chrono_harness::resolve_program(
            &root,
            &config.git.program,
            Some(environment.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        if chrono_harness::file_identity(&path)?.0 != *expected {
            return Err("Git executable digest mismatch before version probe".into());
        }
    }
    // Tool observation retains a failed version process instead of converting it into a success.
    let tool = observation::tool(
        &root,
        &config.git.program,
        &["--version".into()],
        &environment,
        config.timeout_seconds,
        config.output_limit_bytes,
    )?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut output = tempfile::Builder::new()
        .prefix(&format!("{operation}-"))
        .suffix(".json")
        .tempfile_in(&dir)
        .map_err(|e| e.to_string())?;
    let name = output
        .path()
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let report_path = output
        .path()
        .strip_prefix(&root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("report path UTF-8")?
        .to_string();
    let mut report = value!({"schema":"chrono-worktree-report/v1","operation":operation,"status":"failed","governance":"not-evaluated","parity":"unestablished","source_root":root,"config_path":config_path,"config_sha256":sha256(&bytes),"report_path":report_path,"lock_reason":name,"environment":{"inherited":inherited,"effective":environment},"tool":tool,"fetch_ref_removed":false,"recovery":"Failed creation preserves worktrees and branches; inspect recorded identities before recovery."});
    let version_error = tool.version.failure.clone().or_else(|| {
        if tool.version.exit_code != 0 {
            Some(format!(
                "Git version probe exited {}",
                tool.version.exit_code
            ))
        } else if config
            .git
            .sha256
            .as_ref()
            .is_some_and(|s| s != &tool.sha256)
        {
            Some("Git executable digest mismatch".into())
        } else if config
            .git
            .expected_version
            .as_ref()
            .is_some_and(|v| tool.version.stdout.trim() != v)
        {
            Some("Git version mismatch".into())
        } else {
            None
        }
    });
    let mut runner = Runner {
        config,
        environment,
        tool,
        processes: vec![],
    };
    let result = match version_error {
        Some(e) => Err(e),
        None => execute(&mut runner, &name, &mut report),
    };
    match result {
        Ok(()) => report["status"] = value!(success),
        Err(e) => report["error"] = value!(e),
    }
    report["processes"] = value!(runner.processes);
    let mut data = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?;
    data.push(b'\n');
    output.write_all(&data).map_err(|e| e.to_string())?;
    output.as_file().sync_all().map_err(|e| e.to_string())?;
    output.keep().map_err(|e| e.to_string())?;
    Ok(report)
}

pub(crate) fn registrations(
    r: &mut Runner,
    root: &Path,
    head: &str,
    config_path: &str,
) -> Result<(Registrations, String), String> {
    let host = json(&r.blob(root, head, config_path)?)?;
    let mut values = BTreeMap::new();
    for path in facts::registry_paths(&host, config_path)? {
        values.insert(path.clone(), json(&r.blob(root, head, &path)?)?);
    }
    let registrations = Registrations::load(&values, config_path)?;
    Ok((
        registrations,
        chrono_harness::wire::digest(&value!(values))?,
    ))
}
