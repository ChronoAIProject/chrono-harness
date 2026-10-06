//! GitHub transports fixed unit artifacts; the scoped judge decides their acceptance.
use super::*;
use chrono_harness::{CommandSpec, ProcessResult, file_identity, run_process_observed_for, sha256};
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
    resources: Option<Value>,
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
            resources: c
                .resource_observation
                .as_ref()
                .map(|_| super::resources::unavailable("jobs-response-not-acquired")),
        })
    }

    fn run(&mut self, args: &[String]) -> Result<Vec<u8>, String> {
        let p = self.process(
            args,
            Duration::from_secs(self.config.timeout_seconds),
            self.config.output_limit_bytes,
        )?;
        if p.failure.is_some() || p.exit_code != 0 {
            return Err(
                "GitHub transport failed; original bounded process evidence retained".into(),
            );
        }
        Ok(p.stdout_bytes)
    }

    fn process(
        &mut self,
        args: &[String],
        remaining: Duration,
        output_limit: usize,
    ) -> Result<ProcessResult, String> {
        let spec = CommandSpec {
            program: self.executable.clone(),
            args: args.to_vec(),
            env: self.environment.clone(),
            timeout_seconds: self.config.timeout_seconds,
            output_limit_bytes: output_limit,
        };
        let p = run_process_observed_for(self.root, &spec, &[], &self.digest, remaining)?;
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
        Ok(p)
    }

    fn download(
        &mut self,
        run: u64,
        repository: &str,
        artifact: &str,
        directory: &str,
    ) -> Result<String, String> {
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(self.config.timeout_seconds))
            .ok_or("artifact download deadline overflow")?;
        let absolute = no_symlink_parents(self.root, directory)?;
        if absolute.exists() {
            return Err("artifact download destination exists; retain original acquisition".into());
        }
        fs::create_dir_all(&absolute).map_err(|e| e.to_string())?;
        let policy = self.config.download_retry.clone();
        let attempts = policy.as_ref().map_or(1, |p| p.max_attempts);
        let mut stdout_remaining = self.config.output_limit_bytes;
        let mut stderr_remaining = self.config.output_limit_bytes;
        for attempt in 1..=attempts {
            let destination = if policy.is_some() {
                let path = format!("{directory}/attempt-{attempt}");
                fs::create_dir(no_symlink_parents(self.root, &path)?).map_err(|e| e.to_string())?;
                path
            } else {
                directory.into()
            };
            let output_limit = stdout_remaining.min(stderr_remaining);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || output_limit == 0 {
                return Err(
                    "artifact download budget exhausted; original attempts retained".into(),
                );
            }
            let args = vec![
                "run".into(),
                "download".into(),
                run.to_string(),
                "--repo".into(),
                repository.into(),
                "--name".into(),
                artifact.into(),
                "--dir".into(),
                no_symlink_parents(self.root, &destination)?
                    .to_str()
                    .ok_or("download path UTF-8")?
                    .into(),
            ];
            let p = self.process(&args, remaining, output_limit)?;
            stdout_remaining = stdout_remaining.saturating_sub(p.stdout_bytes.len());
            stderr_remaining = stderr_remaining.saturating_sub(p.stderr_bytes.len());
            let success = p.failure.is_none() && p.exit_code == 0;
            let status = download_http_status(&p.stderr_bytes);
            let delay = policy
                .as_ref()
                .map(|p| Duration::from_secs(p.delay_seconds));
            let retry = !success
                && p.failure.is_none()
                && attempt < attempts
                && policy.as_ref().is_some_and(|policy| {
                    status.is_some_and(|s| policy.http_statuses.contains(&s))
                })
                && stdout_remaining > 0
                && stderr_remaining > 0
                && delay.is_some_and(|d| d < deadline.saturating_duration_since(Instant::now()));
            if policy.is_some() {
                self.observations
                    .last_mut()
                    .ok_or("download observation missing")?["download"] = json!({"artifact":artifact,"attempt":attempt,"max_attempts":attempts,
                        "directory":destination,"http_status":status,
                        "action":if success {"completed"} else if retry {"retry"} else {"failed"}});
            }
            if success {
                return Ok(destination);
            }
            if !retry {
                return Err(
                    "GitHub transport failed; original bounded process evidence retained".into(),
                );
            }
            std::thread::sleep(delay.ok_or("retry delay missing")?);
        }
        Err("artifact download attempts exhausted; original attempts retained".into())
    }

    fn api(&mut self, endpoint: &str, paginate: bool) -> Result<Value, String> {
        let mut args = vec!["api".into(), endpoint.into()];
        if paginate {
            args.extend(["--paginate".into(), "--slurp".into()]);
        }
        chrono_harness::json(&self.run(&args)?)
    }
}

/// The registered CLI protocol uses one unambiguous `HTTP nnn:` status in stderr.
fn download_http_status(stderr: &[u8]) -> Option<u16> {
    let mut status = None;
    for token in stderr
        .windows(9)
        .filter(|s| s.starts_with(b"HTTP ") && s[8] == b':')
    {
        let digits = &token[5..8];
        if !digits.iter().all(u8::is_ascii_digit) {
            continue;
        }
        let current = digits
            .iter()
            .fold(0_u16, |n, digit| n * 10 + u16::from(digit - b'0'));
        if status.is_some_and(|prior| prior != current) {
            return None;
        }
        status = Some(current);
    }
    status
}

fn parent_run(
    run: &Value,
    repository: &str,
    d: &super::gating::Detection,
    c: &units::Config,
    attempt: u64,
) -> Result<(), String> {
    if run["id"] != d.run_id
        || run["run_attempt"] != attempt
        || run["head_sha"] != d.candidate
        || run["event"] != d.event
        || run["path"] != c.collection.workflow_path
        || run["repository"]["full_name"] != repository
    {
        return Err("parent run/repository/event/source/attempt mismatch".into());
    }
    // The aggregate is currently running. Waiting for parent completion would wait on itself.
    Ok(())
}
fn validate_detector_jobs(
    jobs: &[&Value],
    d: &super::gating::Detection,
    parent_attempt: u64,
) -> Result<(), String> {
    let detector: Vec<_> = jobs
        .iter()
        .filter(|j| j["name"] == "Detect registered DELTA")
        .collect();
    let latest = detector
        .iter()
        .filter_map(|j| j["run_attempt"].as_u64())
        .max()
        .ok_or("detector API execution missing")?;
    let latest_jobs: Vec<_> = detector
        .iter()
        .filter(|j| j["run_attempt"] == latest)
        .collect();
    if latest_jobs.len() != 1
        || latest < d.run_attempt
        || latest > parent_attempt
        || latest_jobs[0]["run_id"] != d.run_id
        || latest_jobs[0]["head_sha"] != d.candidate
        || latest_jobs[0]["id"].as_u64().is_none_or(|id| id == 0)
        || latest_jobs[0]["status"] != "completed"
        || latest_jobs[0]["conclusion"] != "success"
    {
        return Err("latest detector execution did not uniquely succeed".into());
    }
    Ok(())
}
fn parent_units(
    root: &Path,
    c: &units::Config,
    repository: &str,
    context: &Value,
    transport: &mut Transport<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let d: super::gating::Detection = serde_json::from_value(context["detection"].clone())
        .map_err(|e| format!("fixed detection missing: {e}"))?;
    let needs: Value = decode(
        std::env::var(super::gating::NEEDS_ENV)
            .map_err(|_| "aggregate needs missing")?
            .as_bytes(),
    )?;
    super::gating::validate_statuses(c, &d, &needs)?;
    if d.repository != repository {
        return Err("detection repository mismatch".into());
    }
    let requirements =
        super::gating::requirements(root, c, d.base.clone(), d.candidate.clone(), d.initial)?;
    if requirements["required_units"] != json!(d.required_units)
        || chrono_harness::wire::digest(&requirements["global_selected"])? != d.selection_sha256
        || requirements["changed_paths"]
            .as_array()
            .ok_or("complete delta missing")?
            .len()
            != d.changed_paths_count
    {
        return Err("aggregate obligations disagree with fixed detection".into());
    }
    let native: super::gating::NativeRun = serde_json::from_value(context["native_run"].clone())
        .map_err(|e| format!("native parent binding missing: {e}"))?;
    if native != super::gating::native_run(None)?
        || native.run_id != d.run_id
        || native.repository != d.repository
        || native.run_attempt < d.run_attempt
    {
        return Err("aggregate parent run/attempt binding mismatch".into());
    }
    let run = transport.api(
        &format!("repos/{repository}/actions/runs/{}", d.run_id),
        false,
    )?;
    parent_run(&run, repository, &d, c, native.run_attempt)?;
    let pages = transport.api(
        &format!(
            "repos/{repository}/actions/runs/{}/jobs?filter=all&per_page=100",
            d.run_id
        ),
        true,
    )?;
    if let Some(config) = &c.gather.resource_observation {
        let mut observed =
            super::resources::observe(config, &pages, d.run_id, native.run_attempt, &d.candidate);
        let index = transport.observations.len() - 1;
        observed["source"] = json!({"process_index":index,
            "stdout_sha256":transport.observations[index]["stdout_sha256"]});
        transport.resources = Some(observed);
    }
    let jobs: Vec<_> = pages
        .as_array()
        .ok_or("job pages missing")?
        .iter()
        .map(|page| page["jobs"].as_array().ok_or("job list missing"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    validate_detector_jobs(&jobs, &d, native.run_attempt)?;
    let mut completed = BTreeMap::new();
    for (unit, workflow) in &c.units {
        // needs already binds every nonrequired skip. Only selected jobs need
        // execution metadata and artifacts; skipped jobs need no API row.
        if !d.required_units.contains(unit) {
            continue;
        }
        // Independent job retries keep successful prerequisites' original attempts.
        // Choose the last actual execution inside this exact parent run, never another run.
        let relevant: Vec<_> = jobs
            .iter()
            .filter(|job| job["name"] == workflow.name)
            .collect();
        if relevant.iter().any(|job| {
            job["run_id"] != d.run_id
                || job["head_sha"] != d.candidate
                || job["run_attempt"]
                    .as_u64()
                    .is_none_or(|a| a == 0 || a > native.run_attempt)
                || job["id"].as_u64().is_none_or(|id| id == 0)
        }) {
            return Err(format!("unit {unit} job run/attempt identity malformed"));
        }
        let latest_attempt = relevant
            .iter()
            .filter_map(|j| j["run_attempt"].as_u64())
            .max()
            .ok_or_else(|| format!("unit {unit} job missing in parent history"))?;
        let latest: Vec<_> = relevant
            .into_iter()
            .filter(|j| j["run_attempt"] == latest_attempt)
            .collect();
        if latest.len() != 1
            || latest[0]["status"] != "completed"
            || latest[0]["conclusion"] != "success"
        {
            return Err(format!(
                "unit {unit} latest native execution did not uniquely succeed"
            ));
        }
        // API attempts can describe carried jobs. needs outputs bind the actual
        // producer; API latest status remains a separate necessary predicate.
        let production: super::gating::Production = decode(
            needs[super::gating::job_id(unit)]["outputs"]["production"]
                .as_str()
                .ok_or("original unit production output missing")?
                .as_bytes(),
        )?;
        let p = &production.native_run;
        if production.schema != "chrono-ci-production/v1"
            || p.repository != repository
            || p.run_id != d.run_id
            || p.job != super::gating::job_id(unit)
            || p.run_attempt < d.run_attempt
            || p.run_attempt > latest_attempt
            || production.artifact != format!("chrono-unit-{unit}-{}-{}", p.run_id, p.run_attempt)
            || !chrono_harness::wire::is_digest(&production.context_sha256)
            || !chrono_harness::wire::is_digest(&production.report_sha256)
        {
            return Err(format!("unit {unit} original producer binding invalid"));
        }
        let mut binding = run.clone();
        binding["conclusion"] = latest[0]["conclusion"].clone();
        binding["unit_job_id"] = latest[0]["id"].clone();
        binding["run_attempt"] = json!(p.run_attempt);
        binding["latest_job_attempt"] = json!(latest_attempt);
        binding["production"] = json!(production);
        binding["parent_attempt"] = json!(native.run_attempt);
        completed.insert(unit.clone(), binding);
    }
    Ok(completed)
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
    let (_profile, registered) = units::profile(root, c)?;
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
    if c.collection.schema == "chrono-github-ci/v4" {
        canonical = vec![
            c.collection.runner.clone(),
            "check".into(),
            "--collect".into(),
        ];
    } else {
        canonical.extend(c.scope(None).argv());
    }
    if context["canonical_argv"] != json!(canonical) {
        return Err("collection command context mismatch".into());
    }
    let completed = if c.job_gating.is_some() {
        parent_units(root, c, repository, &context, transport)?
    } else {
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
        completed
    };
    let mut reports = vec![];
    let mut identities = vec![];
    let download_prefix = if c.collection.schema == "chrono-github-ci/v4" {
        let retained = chrono_harness::prepared::retain_directory(
            root,
            &c.gather.download_directory,
            "gather-",
        )?;
        format!(
            "{}/",
            retained
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("download path UTF-8")?
        )
    } else {
        c.gather.download_directory.clone()
    };
    for (unit, run) in completed {
        let id = run["id"].as_u64().ok_or("unit run ID missing")?;
        let attempt = run["run_attempt"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("unit run attempt missing")?;
        let w = &c.units[&unit];
        let destination = format!("{download_prefix}{unit}/{id}/{attempt}");
        let artifact = if c.job_gating.is_some() {
            run["production"]["artifact"]
                .as_str()
                .ok_or("original artifact name missing")?
                .to_owned()
        } else {
            format!("chrono-unit-{unit}-{id}-{attempt}")
        };
        let destination = transport.download(id, repository, &artifact, &destination)?;
        let downloaded = |input: &str| -> Result<String, String> {
            let relative = input
                .strip_prefix(&w.artifact_directory)
                .ok_or("report/context outside uploaded unit artifacts")?;
            relative_path(relative)?;
            Ok(format!("{destination}/{relative}"))
        };
        if c.job_gating.is_some() {
            if run["production"]["context_sha256"]
                != file_identity(&no_symlink_parents(root, &downloaded(&w.context_path)?)?)?.0
                || run["production"]["report_sha256"]
                    != file_identity(&no_symlink_parents(
                        root,
                        &downloaded(&registered[&unit].report_path)?,
                    )?)?
                    .0
            {
                return Err(format!(
                    "unit {unit} artifact differs from original production output"
                ));
            }
        }
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
        if c.collection.schema == "chrono-github-ci/v4" {
            expected = vec![
                c.collection.runner.clone(),
                "check".into(),
                "--unit".into(),
                unit.clone(),
            ];
        } else {
            expected.extend(scope.argv());
        }
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
        if c.job_gating.is_some()
            && (unit_context["detection"] != context["detection"]
                || unit_context["workflow_source_revision"] != context["workflow_source_revision"])
        {
            return Err(format!(
                "unit {unit} parent run/attempt/source detection mismatch"
            ));
        }
        if c.job_gating.is_some() {
            let native: super::gating::NativeRun =
                serde_json::from_value(unit_context["native_run"].clone())
                    .map_err(|e| format!("unit native run missing: {e}"))?;
            if native.repository != repository
                || native.run_id != id
                || native.run_attempt != attempt
                || native.job != super::gating::job_id(&unit)
            {
                return Err(format!("unit {unit} artifact run/attempt/job mismatch"));
            }
        }
        let workflow_path = if c.job_gating.is_some() {
            &c.collection.workflow_path
        } else {
            &w.workflow_path
        };
        let revision = unit_context["workflow_source_revision"]
            .as_str()
            .filter(|v| full_oid(v))
            .ok_or("workflow source revision missing")?;
        let source = transport.run(&[
            "api".into(),
            format!(
                "repos/{repository}/contents/{}?ref={revision}",
                workflow_path
            ),
            "--header".into(),
            "Accept: application/vnd.github.raw".into(),
        ])?;
        if source != fs::read(root.join(workflow_path)).map_err(|e| e.to_string())? {
            return Err(format!("unit {unit} executed a different workflow source"));
        }
        let report_path = downloaded(&registered[&unit].report_path)?;
        let mut input = json!({"unit":unit,"path":report_path,"sha256":file_identity(&no_symlink_parents(root,&report_path)?)?.0,
            "runner_sha256":unit_context["executables"]["runner_sha256"],"judge_sha256":unit_context["executables"]["judge_sha256"],
            "artifacts":if c.collection.schema=="chrono-github-ci/v4" {json!({"source_directory":w.artifact_directory,"directory":format!("{destination}/")})} else {Value::Null}});
        if c.schema == units::FULL_SCHEMA {
            if unit_context["full_context"]["semantic_digest"]
                != context["full_context"]["semantic_digest"]
                || unit_context["executables"]["judges"] != context["executables"]["judges"]
            {
                return Err("full unit context/seven-judge binding differs from collection".into());
            }
        }
        if input["artifacts"].is_null() {
            input.as_object_mut().unwrap().remove("artifacts");
        }
        reports.push(input);
        let fresh = transport.api(&format!("repos/{repository}/actions/runs/{id}"), false)?;
        if c.job_gating.is_some() {
            let d =
                serde_json::from_value(context["detection"].clone()).map_err(|e| e.to_string())?;
            parent_run(
                &fresh,
                repository,
                &d,
                c,
                run["parent_attempt"]
                    .as_u64()
                    .ok_or("parent attempt missing")?,
            )?;
        } else {
            if fresh["run_attempt"] != attempt
                || fresh["status"] != "completed"
                || fresh["head_sha"] != candidate
                || fresh["event"] != event
                || fresh["path"] != w.workflow_path
            {
                return Err("unit run changed while collecting original evidence".into());
            }
        }
        identities.push(json!({"unit":unit,"run":id,"attempt":attempt,"conclusion":run["conclusion"],"job_id":run["unit_job_id"],"workflow_source_revision":revision,"workflow_sha256":sha256(&source)}));
    }
    if identities.iter().any(|i| i["conclusion"] != "success") {
        return Err(format!(
            "native unit workflow failed; original downloaded reports retained: {}",
            json!(identities)
        ));
    }
    let manifest = json!({"schema":if c.schema==units::FULL_SCHEMA {"chrono-full-collection/v1"}else{"chrono-ci-collection/v1"},"reports":reports});
    let manifest_path = no_symlink_parents(root, &c.gather.manifest_path)?;
    if manifest_path.exists() && c.collection.schema != "chrono-github-ci/v4" {
        return Err("collection manifest already exists; previous evidence retained".into());
    }
    write_file(
        root,
        &c.gather.manifest_path,
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    let manifest_original = if c.collection.schema == "chrono-github-ci/v4" {
        json!(chrono_harness::prepared::retain_original(
            root,
            &format!("{}preparation/", c.collection.artifact_directory),
            "manifest",
            &fs::read(&manifest_path).map_err(|e| e.to_string())?
        )?)
    } else {
        Value::Null
    };
    Ok(
        json!({"status":"gathered-unjudged","base":context["base"],"candidate":candidate,"units":identities,"manifest_path":c.gather.manifest_path,"manifest_sha256":file_identity(&manifest_path)?.0,"manifest_original":manifest_original}),
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
    if target.exists() && c.collection.schema != "chrono-github-ci/v4" {
        return Err("gather report exists; original evidence retained".into());
    }
    let mut transport = Transport::new(root, &c.gather)?;
    let result = inner(root, path, c, repository, &mut transport);
    let mut report = json!({"schema":"chrono-ci-gather/v1","repository":repository,"result":result.as_ref().ok(),"error":result.as_ref().err(),"processes":transport.observations});
    if let Some(resources) = transport.resources {
        report["resources"] = resources;
        if let Some(summary) = c.gather.resource_observation.as_ref().and_then(|config| {
            super::resources::publish_summary(config, &report["resources"], &c.gather.report_path)
        }) {
            report["resources"]["summary"] = summary;
        }
    }
    if c.collection.schema == "chrono-github-ci/v4" {
        report["retained_report"] = json!(
            chrono_harness::prepared::retain_original(
                root,
                &format!("{}preparation/", c.collection.artifact_directory),
                "native-gather",
                &serde_json::to_vec(&report).map_err(|e| e.to_string())?
            )?
            .path
        );
    }
    write_file(
        root,
        &c.gather.report_path,
        &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )?;
    result?;
    Ok(serde_json::to_string(&report).map_err(|e| e.to_string())? + "\n")
}

/// Acquire the completed detector publication through the registered transport.
/// The dependency supplies its original producer attempt; there is no polling.
pub(super) fn acquire_seed(
    root: &Path,
    path: &str,
    c: &units::Config,
    unit: Option<&str>,
    repository: &str,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<String, String> {
    units::validate(c)?;
    let a = c
        .native_adoption
        .as_ref()
        .ok_or("native seed extension absent")?;
    let endpoints = units::prepare_endpoints(root, path, c, unit, event, payload, revision)?;
    let candidate = endpoints["candidate"].as_str().ok_or("seed candidate")?;
    if repository.split('/').count() != 2
        || repository
            .bytes()
            .any(|b| !b.is_ascii_alphanumeric() && !b"/-_.".contains(&b))
    {
        return Err("repository must be explicit owner/name".into());
    }
    let w = c.workflow(unit)?;
    let directory = format!("{}seed/", w.artifact_directory);
    let mut transport = Transport::new(root, &c.gather)?;
    let result: Result<Value, String> = (|| {
        let d: super::gating::Detection =
            serde_json::from_value(endpoints["detection"].clone()).map_err(|e| e.to_string())?;
        let id = d.run_id;
        let attempt = d.run_attempt;
        let parent_attempt = std::env::var("GITHUB_RUN_ATTEMPT")
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|_| "parent attempt")?;
        let run = transport.api(&format!("repos/{repository}/actions/runs/{id}"), false)?;
        parent_run(&run, repository, &d, c, parent_attempt)?;
        let job_pages = transport.api(
            &format!("repos/{repository}/actions/runs/{id}/jobs?filter=all&per_page=100"),
            true,
        )?;
        let jobs: Vec<_> = job_pages
            .as_array()
            .ok_or("detector job pages missing")?
            .iter()
            .map(|p| p["jobs"].as_array().ok_or("detector jobs missing"))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        validate_detector_jobs(&jobs, &d, parent_attempt)?;
        let name = format!("{}-{id}-{attempt}", a.seed_artifact);
        let pages = transport.api(
            &format!("repos/{repository}/actions/runs/{id}/artifacts?per_page=100"),
            true,
        )?;
        let artifacts: Vec<_> = pages
            .as_array()
            .ok_or("seed artifact pages")?
            .iter()
            .map(|p| p["artifacts"].as_array().ok_or("seed artifact list"))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .filter(|a| a["name"] == name)
            .collect();
        if artifacts.len() != 1 || artifacts[0]["expired"] != false {
            return Err("original detector seed missing/ambiguous/expired".into());
        }
        let artifact = artifacts[0].clone();
        let directory =
            transport.download(id, repository, &name, directory.trim_end_matches('/'))?;
        let read = |file: &str| {
            chrono_harness::units::read_bounded(
                root,
                &format!("{directory}/{file}"),
                64 * 1024 * 1024,
            )
        };
        let binding: Value = decode(&read("binding.json")?)?;
        let raw = read("context.json")?;
        let ctx = super::full::context_input(&raw)?;
        let lineage = read("lineage.json")?;
        let original_endpoints = read("endpoints.json")?;
        let original_payload = read("payload.json")?;
        let seed_endpoints: Value = decode(&original_endpoints)?;
        if binding["schema"] != "chrono-native-seed/v1"
            || binding["repository"] != repository
            || binding["event"] != event
            || binding["base"] != endpoints["base"]
            || binding["candidate"] != candidate
            || binding["workflow"] != c.collection.workflow_path
            || binding["workflow_revision"] != revision
            || binding["run"] != id
            || binding["attempt"] != attempt
            || binding["context_sha256"] != sha256(&raw)
            || binding["lineage_sha256"] != sha256(&lineage)
            || binding["lineage_sha256"] != a.lineage.sha256
            || binding["endpoints_sha256"] != sha256(&original_endpoints)
            || binding["payload_sha256"] != sha256(&original_payload)
            || binding["governance_sha256"] != sha256(&read("inputs.json")?)
            || ctx["base"] != endpoints["base"]
            || ctx["candidate"] != candidate
            || seed_endpoints["base"] != endpoints["base"]
            || seed_endpoints["candidate"] != candidate
            || seed_endpoints["workflow_source_revision"] != revision
            || seed_endpoints["lineage"] != endpoints["lineage"]
            || seed_endpoints["workflow_policy"] != endpoints["workflow_policy"]
        {
            return Err("shared seed event/repository/endpoints/workflow/run/attempt/digest binding mismatch".into());
        }
        for original in binding["originals"]
            .as_object()
            .ok_or("seed original publication closure missing")?
            .values()
        {
            let file = original["file"].as_str().ok_or("seed original file")?;
            let raw = read(file)?;
            if original["sha256"] != sha256(&raw) {
                return Err("seed original publication closure differs".into());
            }
        }
        let expected_role = if event == "pull_request" {
            a.pull_request_role.as_str()
        } else {
            let reference = payload["ref"].as_str().ok_or("seed event ref missing")?;
            let roles: Vec<_> = a
                .push_roles
                .iter()
                .filter(|(prefix, _)| reference.starts_with(prefix.as_str()))
                .map(|(_, role)| role.as_str())
                .collect();
            if roles.len() != 1 {
                return Err("seed event role missing/ambiguous".into());
            }
            roles[0]
        };
        let birth: Value = decode(&lineage)?;
        if ctx["run_kind"] != expected_role
            || a.integration_evidence
                .as_ref()
                .is_some_and(|digest| ctx["integration_evidence"] != *digest)
            || ctx["dev_tip"] != endpoints["base"]
            || ctx["operation"] != "validate.delta"
            || ctx["fork_point"] != birth["base"]
            || ctx["branch_ref"] != birth["branch_ref"]
            || ctx["branch_started_at"] != birth["branch_started_at"]
            || ctx["retained_inputs"] != a.retained_inputs
        {
            return Err("shared seed birth/input binding mismatch".into());
        }
        if a.integration_evidence_path.is_some() && !ctx["integration_evidence"].is_null() {
            let raw = read("integration.json")?;
            if ctx["integration_evidence"] != sha256(&raw)
                || binding["integration_sha256"] != ctx["integration_evidence"]
            {
                return Err("seed integration original digest differs".into());
            }
            write_file(
                root,
                a.integration_evidence_path
                    .as_ref()
                    .ok_or("integration path")?,
                &raw,
            )?;
        }
        let source = transport.run(&[
            "api".into(),
            format!(
                "repos/{repository}/contents/{}?ref={revision}",
                c.collection.workflow_path
            ),
            "--header".into(),
            "Accept: application/vnd.github.raw".into(),
        ])?;
        if source != fs::read(root.join(&c.collection.workflow_path)).map_err(|e| e.to_string())? {
            return Err("shared seed workflow source differs".into());
        }
        let fresh = transport.api(&format!("repos/{repository}/actions/runs/{id}"), false)?;
        parent_run(&fresh, repository, &d, c, parent_attempt)?;
        write_file(
            root,
            match unit {
                Some(unit) => {
                    &c.full_contexts
                        .as_ref()
                        .ok_or("full contexts missing")?
                        .units[unit]
                }
                None => {
                    &c.full_contexts
                        .as_ref()
                        .ok_or("full contexts missing")?
                        .collection
                }
            },
            &raw,
        )?;
        Ok(
            json!({"schema":"chrono-native-seed-acquisition/v1","binding":binding,"artifact":artifact,"workflow_sha256":sha256(&source),"endpoints":endpoints}),
        )
    })();
    let report = json!({"schema":"chrono-native-seed-transport/v1","repository":repository,"result":result.as_ref().ok(),"error":result.as_ref().err(),"processes":transport.observations});
    let original = chrono_harness::prepared::retain_original(
        root,
        &format!("{}preparation/", w.artifact_directory),
        "seed-transport",
        &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
    )?;
    result?;
    Ok(
        serde_json::to_string(&json!({"report":original,"result":report["result"]}))
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
