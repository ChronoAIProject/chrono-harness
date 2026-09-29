//! GitHub transports fixed unit artifacts; the scoped judge decides their acceptance.
use super::*;
use chrono_harness::{CommandSpec, file_identity, run_process_observed, sha256};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

struct Transport<'a> {
    root: &'a Path,
    config: &'a units::Gather,
    environment: BTreeMap<String, String>,
    executable: String,
    digest: String,
    observations: Vec<Value>,
}

impl<'a> Transport<'a> {
    fn new(root: &'a Path, c: &'a units::Gather) -> Result<Self, String> {
        let mut environment = BTreeMap::new();
        for key in &c.inherit_environment {
            if let Ok(value) = std::env::var(key) {
                environment.insert(key.clone(), value);
            }
        }
        environment.extend(c.environment.clone());
        let exe = chrono_harness::resolve_program(
            root,
            &c.program,
            Some(environment.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        Ok(Self {
            root,
            config: c,
            environment,
            executable: exe.to_str().ok_or("non UTF-8 transport")?.into(),
            digest: file_identity(&exe)?.0,
            observations: vec![],
        })
    }

    fn run(&mut self, args: &[String]) -> Result<Vec<u8>, String> {
        let spec = CommandSpec {
            program: self.executable.clone(),
            args: args.to_vec(),
            env: self.environment.clone(),
            timeout_seconds: self.config.timeout_seconds,
            output_limit_bytes: self.config.output_limit_bytes,
        };
        let p = run_process_observed(self.root, &spec, &[], &self.digest)?;
        // Credentials are explicitly omitted, while the original nonsecret process bytes and environment digest remain.
        let env: BTreeMap<_, _> = p
            .environment
            .iter()
            .filter(|(k, _)| !self.config.credential_environment.contains(k))
            .collect();
        self.observations.push(json!({"argv":p.argv,"cwd":p.cwd,"executable":p.executable,"sha256":p.sha256,
            "environment":env,"omitted_credentials":self.config.credential_environment,"original_environment_digest":p.environment_digest,
            "stdin_sha256":p.stdin_sha256,"stdout_bytes":p.stdout_bytes,"stderr_bytes":p.stderr_bytes,
            "stdout_sha256":p.stdout_sha256,"stderr_sha256":p.stderr_sha256,"exit_code":p.exit_code,"failure":p.failure}));
        if p.failure.is_some() || p.exit_code != 0 {
            return Err(
                "GitHub transport failed; original bounded process evidence retained".into(),
            );
        }
        Ok(p.stdout_bytes)
    }

    fn api(&mut self, endpoint: &str, paginate: bool) -> Result<Value, String> {
        let mut args = vec!["api".into(), endpoint.into()];
        if paginate {
            args.extend(["--paginate".into(), "--slurp".into()]);
        }
        chrono_harness::json(&self.run(&args)?)
    }
}

fn inner(
    root: &Path,
    path: &str,
    c: &units::Config,
    repository: &str,
    transport: &mut Transport<'_>,
) -> Result<Value, String> {
    units::generate(root, path, c, true)?;
    let context: Value = decode(
        &fs::read(no_symlink_parents(root, &c.collection.context_path)?)
            .map_err(|e| e.to_string())?,
    )?;
    let candidate = context["candidate"]
        .as_str()
        .filter(|v| full_oid(v))
        .ok_or("fixed collection candidate missing")?;
    let event = context["event"]
        .as_str()
        .ok_or("collection event missing")?;
    if !matches!(event, "push" | "pull_request") {
        return Err("automatic gathering requires push/pull_request; manual unit checks use explicit collection manifests".into());
    }
    if context["scope"] != json!(c.scope(None))
        || context["provider_sha256"] != file_identity(&root.join(path))?.0
    {
        return Err(
            "collection context does not bind current provider and collection scope".into(),
        );
    }
    let profile = chrono_harness::load_config(&root.join(&c.collection.check_config))?;
    if context["check_config_sha256"] != file_identity(&root.join(&c.collection.check_config))?.0 {
        return Err("collection check profile changed".into());
    }
    let mut canonical = canonical_argv(
        &c.collection.runner,
        &c.collection.check_config,
        context["base"].as_str(),
        candidate,
        context["initial"] == true,
    );
    canonical.extend(c.scope(None).argv());
    if context["canonical_argv"] != json!(canonical) {
        return Err("collection command context mismatch".into());
    }
    let deadline = Instant::now() + Duration::from_secs(c.gather.wait_seconds);
    let mut pending: BTreeMap<_, _> = c.units.iter().collect();
    let mut completed = BTreeMap::new();
    while !pending.is_empty() {
        let mut ready = vec![];
        for (unit, workflow) in &pending {
            let filename = workflow
                .workflow_path
                .strip_prefix(".github/workflows/")
                .ok_or("workflow path")?;
            let endpoint = format!(
                "repos/{repository}/actions/workflows/{}/runs?head_sha={candidate}&event={event}&per_page=100",
                encode(filename)
            );
            let pages = transport.api(&endpoint, true)?;
            let mut matches = vec![];
            for page in pages.as_array().ok_or("paginated run pages missing")? {
                for run in page["workflow_runs"]
                    .as_array()
                    .ok_or("workflow run list missing")?
                {
                    if run["head_sha"] == candidate
                        && run["event"] == event
                        && run["path"] == workflow.workflow_path
                    {
                        matches.push(run.clone());
                    }
                }
            }
            if matches.len() > 1 {
                return Err(format!(
                    "ambiguous runs for unit {unit}; use an explicit local collection manifest"
                ));
            }
            if let Some(run) = matches.first() {
                if run["status"] == "completed" {
                    ready.push(((*unit).clone(), run.clone()));
                }
            }
        }
        for (unit, run) in ready {
            pending.remove(&unit);
            completed.insert(unit, run);
        }
        if pending.is_empty() {
            break;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "unit collection wait expired; unresolved units: {:?}",
                pending.keys()
            ));
        }
        std::thread::sleep(
            Duration::from_secs(c.gather.poll_seconds)
                .min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    let registered: BTreeMap<String, chrono_harness::units::Unit> =
        serde_json::from_value(profile.policy["units"].clone()).map_err(|e| e.to_string())?;
    let mut reports = vec![];
    let mut identities = vec![];
    for (unit, run) in completed {
        let id = run["id"].as_u64().ok_or("unit run ID missing")?;
        let attempt = run["run_attempt"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("unit run attempt missing")?;
        let w = &c.units[&unit];
        let destination = format!("{}{unit}/{id}/{attempt}", c.gather.download_directory);
        let absolute = no_symlink_parents(root, &destination)?;
        if absolute.exists() {
            return Err("unit download destination exists; retain original results and select a fresh collection output".into());
        }
        fs::create_dir_all(&absolute).map_err(|e| e.to_string())?;
        transport.run(&[
            "run".into(),
            "download".into(),
            id.to_string(),
            "--repo".into(),
            repository.into(),
            "--name".into(),
            format!("chrono-unit-{unit}-{id}-{attempt}"),
            "--dir".into(),
            absolute.to_str().ok_or("download path UTF-8")?.into(),
        ])?;
        let downloaded = |input: &str| -> Result<String, String> {
            let relative = input
                .strip_prefix(&w.artifact_directory)
                .ok_or("report/context outside uploaded unit artifacts")?;
            relative_path(relative)?;
            Ok(format!("{destination}/{relative}"))
        };
        let unit_context: Value = decode(
            &fs::read(no_symlink_parents(root, &downloaded(&w.context_path)?)?)
                .map_err(|e| e.to_string())?,
        )?;
        let scope = c.scope(Some(&unit));
        let mut expected = canonical_argv(
            &c.collection.runner,
            &c.collection.check_config,
            context["base"].as_str(),
            candidate,
            context["initial"] == true,
        );
        expected.extend(scope.argv());
        if [
            "event",
            "base",
            "candidate",
            "initial",
            "provider_sha256",
            "check_config_sha256",
        ]
        .iter()
        .any(|k| unit_context[*k] != context[*k])
            || unit_context["scope"] != json!(scope)
            || unit_context["canonical_argv"] != json!(expected)
        {
            return Err(format!(
                "unit {unit} context disagrees with fixed collection inputs"
            ));
        }
        let revision = unit_context["workflow_source_revision"]
            .as_str()
            .filter(|v| full_oid(v))
            .ok_or("workflow source revision missing")?;
        let source = transport.run(&[
            "api".into(),
            format!(
                "repos/{repository}/contents/{}?ref={revision}",
                w.workflow_path
            ),
            "--header".into(),
            "Accept: application/vnd.github.raw".into(),
        ])?;
        if source != fs::read(root.join(&w.workflow_path)).map_err(|e| e.to_string())? {
            return Err(format!("unit {unit} executed a different workflow source"));
        }
        let report_path = downloaded(&registered[&unit].report_path)?;
        reports.push(json!({"unit":unit,"path":report_path,"sha256":file_identity(&no_symlink_parents(root,&report_path)?)?.0,
            "runner_sha256":unit_context["executables"]["runner_sha256"],"judge_sha256":unit_context["executables"]["judge_sha256"]}));
        let fresh = transport.api(&format!("repos/{repository}/actions/runs/{id}"), false)?;
        if fresh["run_attempt"] != attempt
            || fresh["status"] != "completed"
            || fresh["head_sha"] != candidate
            || fresh["event"] != event
            || fresh["path"] != w.workflow_path
        {
            return Err("unit run changed while collecting original evidence".into());
        }
        identities.push(json!({"unit":unit,"run":id,"attempt":attempt,"conclusion":run["conclusion"],"workflow_source_revision":revision,"workflow_sha256":sha256(&source)}));
    }
    if identities.iter().any(|i| i["conclusion"] != "success") {
        return Err(format!(
            "native unit workflow failed; original downloaded reports retained: {}",
            json!(identities)
        ));
    }
    let manifest = json!({"schema":"chrono-ci-collection/v1","reports":reports});
    let manifest_path = no_symlink_parents(root, &c.gather.manifest_path)?;
    if manifest_path.exists() {
        return Err("collection manifest already exists; previous evidence retained".into());
    }
    write_file(
        root,
        &c.gather.manifest_path,
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    Ok(
        json!({"status":"gathered-unjudged","base":context["base"],"candidate":candidate,"units":identities,"manifest_path":c.gather.manifest_path,"manifest_sha256":file_identity(&manifest_path)?.0}),
    )
}

pub(super) fn gather(
    root: &Path,
    path: &str,
    c: &units::Config,
    repository: &str,
) -> Result<String, String> {
    units::validate(c)?;
    let parts: Vec<_> = repository.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err("repository must be explicit owner/name".into());
    }
    let target = no_symlink_parents(root, &c.gather.report_path)?;
    if target.exists() {
        return Err("gather report exists; original evidence retained".into());
    }
    let mut transport = Transport::new(root, &c.gather)?;
    let result = inner(root, path, c, repository, &mut transport);
    let report = json!({"schema":"chrono-ci-gather/v1","repository":repository,"result":result.as_ref().ok(),"error":result.as_ref().err(),"processes":transport.observations});
    write_file(
        root,
        &c.gather.report_path,
        &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )?;
    result?;
    Ok(serde_json::to_string(&report).map_err(|e| e.to_string())? + "\n")
}
