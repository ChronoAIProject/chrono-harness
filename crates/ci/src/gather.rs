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

struct AcquisitionBudget {
    deadline: Instant,
    stdout_remaining: usize,
    stderr_remaining: usize,
}

impl AcquisitionBudget {
    fn remaining(&self) -> Result<Duration, String> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("artifact acquisition cumulative deadline exhausted".into());
        }
        Ok(remaining)
    }
}

struct Transport<'a> {
    root: &'a Path,
    config: &'a units::Gather,
    environment: BTreeMap<String, String>,
    executable: String,
    digest: String,
    observations: Vec<Value>,
    acquisition_path: Option<String>,
    downloads: Option<chrono_harness::retained_artifacts::retention::Publication>,
    acquisition: Option<AcquisitionBudget>,
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
            acquisition_path: None,
            downloads: None,
            acquisition: None,
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
        let (remaining, output_limit) = if let Some(budget) = &self.acquisition {
            let limit = output_limit
                .min(budget.stdout_remaining)
                .min(budget.stderr_remaining);
            if limit == 0 {
                return Err("artifact acquisition cumulative stream limit exhausted".into());
            }
            (remaining.min(budget.remaining()?), limit)
        } else {
            (remaining, output_limit)
        };
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
        if let Some(budget) = &mut self.acquisition {
            budget.stdout_remaining = budget.stdout_remaining.saturating_sub(p.stdout_bytes.len());
            budget.stderr_remaining = budget.stderr_remaining.saturating_sub(p.stderr_bytes.len());
            budget.remaining()?;
        }
        Ok(p)
    }

    fn begin_artifact(&mut self) -> Result<(), String> {
        self.acquisition = Some(AcquisitionBudget {
            deadline: Instant::now()
                .checked_add(Duration::from_secs(self.config.timeout_seconds))
                .ok_or("artifact acquisition deadline overflow")?,
            stdout_remaining: self.config.output_limit_bytes,
            stderr_remaining: self.config.output_limit_bytes,
        });
        Ok(())
    }

    fn artifact_metadata(
        pages: &Value,
        run: u64,
        repository: &str,
        name: &str,
        candidate: &str,
    ) -> Result<(), String> {
        let mut found = Vec::new();
        for page in pages.as_array().ok_or("artifact metadata pages missing")? {
            for artifact in page["artifacts"]
                .as_array()
                .ok_or("artifact metadata list missing")?
            {
                if artifact["name"] == name {
                    found.push(artifact);
                }
            }
        }
        if found.len() != 1
            || found[0]["expired"] != false
            || found[0]["workflow_run"]["id"] != run
            || found[0]["workflow_run"]["head_sha"] != candidate
        {
            return Err("artifact missing/ambiguous/expired or run/source binding differs".into());
        }
        let id = found[0]["id"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("artifact ID missing")?;
        if found[0]["archive_download_url"]
            != format!("https://api.github.com/repos/{repository}/actions/artifacts/{id}/zip")
        {
            return Err("artifact archive endpoint differs from selected ID".into());
        }
        Ok(())
    }

    fn download(
        &mut self,
        run: u64,
        repository: &str,
        artifact: &str,
        directory: &str,
    ) -> Result<String, String> {
        let deadline = if let Some(budget) = &self.acquisition {
            budget.deadline
        } else {
            Instant::now()
                .checked_add(Duration::from_secs(self.config.timeout_seconds))
                .ok_or("artifact download deadline overflow")?
        };
        let absolute = no_symlink_parents(self.root, directory)?;
        if absolute.exists() {
            return Err("artifact download destination exists; retain original acquisition".into());
        }
        chrono_harness::retained_artifacts::retention::directory_adopted(
            self.downloads.as_ref(),
            &absolute,
        )?;
        let policy = self.config.download_retry.clone();
        let attempts = policy.as_ref().map_or(1, |p| p.max_attempts);
        let mut stdout_remaining = self.config.output_limit_bytes;
        let mut stderr_remaining = self.config.output_limit_bytes;
        for attempt in 1..=attempts {
            let destination = if policy.is_some() {
                let path = format!("{directory}/attempt-{attempt}");
                chrono_harness::retained_artifacts::retention::directory_adopted(
                    self.downloads.as_ref(),
                    &no_symlink_parents(self.root, &path)?,
                )?;
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
        || run["head_sha"] != d.event_head
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
        || latest_jobs[0]["head_sha"] != d.event_head
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
                || job["head_sha"] != d.event_head
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
    let event_head = context["event_head"]
        .as_str()
        .filter(|v| full_oid(v))
        .ok_or("fixed event head missing")?;
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
    let registered = units::profile(root, c)?.units;
    if context["check_config_sha256"] != file_identity(&root.join(&c.collection.check_config))?.0 {
        return Err("collection check profile changed".into());
    }
    let canonical = vec![
        c.collection.runner.clone(),
        "check".into(),
        "--collect".into(),
    ];
    if context["canonical_argv"] != json!(canonical) {
        return Err("collection command context mismatch".into());
    }
    let stream_profile = if c.schema == units::SCHEMA {
        let profile =
            chrono_harness::load_config(&no_symlink_parents(root, &c.collection.check_config)?)?;
        chrono_harness::units::stream_publication(&profile).then_some(profile)
    } else {
        None
    };
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
                    "repos/{repository}/actions/workflows/{}/runs?head_sha={event_head}&event={event}&per_page=100",
                    encode(filename)
                );
                let pages = transport.api(&endpoint, true)?;
                let mut matches = vec![];
                for page in pages.as_array().ok_or("paginated run pages missing")? {
                    for run in page["workflow_runs"]
                        .as_array()
                        .ok_or("workflow run list missing")?
                    {
                        if run["head_sha"] == event_head
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
    let download_prefix = {
        let parent = no_symlink_parents(root, &c.gather.download_directory)?;
        fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        let retained =
            chrono_harness::retained_artifacts::retention::unique_path(&parent, "gather-", "")?;
        transport.downloads =
            chrono_harness::retained_artifacts::retention::create_adopted_for_consumer(
                root,
                "check-original",
                &retained,
                true,
                json!({"kind":"native-acquisition","outcome":"not-established"}),
                None,
                "publication-recheck",
            )?;
        transport.acquisition_path = Some(
            retained
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("acquisition path UTF8")?
                .into(),
        );
        format!(
            "{}/",
            retained
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("download path UTF-8")?
        )
    };
    // Parent jobs share a completed artifact inventory and immutable workflow
    // revision. Reuse those observations only within this one acquisition.
    let mut parent_artifacts = None;
    let mut workflow_source = None;
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
        transport.begin_artifact()?;
        if c.job_gating.is_some() {
            if parent_artifacts.is_none() {
                parent_artifacts = Some(transport.api(
                    &format!("repos/{repository}/actions/runs/{id}/artifacts?per_page=100"),
                    true,
                )?);
            }
            Transport::artifact_metadata(
                parent_artifacts.as_ref().unwrap(),
                id,
                repository,
                &artifact,
                event_head,
            )?;
        }
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
        if let Some(profile) = &stream_profile {
            let limits: chrono_judge_ci::CollectionLimits = profile
                .policy
                .get("collection_limits")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            let bytes = chrono_harness::units::read_bounded(
                root,
                &downloaded(&registered[&unit].report_path)?,
                limits.report_bytes,
            )?;
            let mapping = chrono_harness::prepared::ArtifactTransport {
                source_directory: w.artifact_directory.clone(),
                directory: format!("{destination}/"),
            };
            chrono_judge_ci::unit_originals(
                root,
                profile,
                &unit,
                &bytes,
                Some(&mapping),
                |original, limit| {
                    let path = chrono_harness::prepared::original_path(original, Some(&mapping))?;
                    chrono_harness::units::read_bounded(root, &path, limit)
                },
            )?;
            transport
                .acquisition
                .as_ref()
                .ok_or("artifact budget missing")?
                .remaining()?;
        }
        let unit_context: Value = decode(
            &fs::read(no_symlink_parents(root, &downloaded(&w.context_path)?)?)
                .map_err(|e| e.to_string())?,
        )?;
        let scope = c.scope(Some(&unit));
        let expected = vec![
            c.collection.runner.clone(),
            "check".into(),
            "--unit".into(),
            unit.clone(),
        ];
        if [
            "event",
            "base",
            "candidate",
            "event_head",
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
        let source_args = [
            "api".into(),
            format!(
                "repos/{repository}/contents/{}?ref={revision}",
                workflow_path
            ),
            "--header".into(),
            "Accept: application/vnd.github.raw".into(),
        ];
        if c.job_gating.is_none() || workflow_source.is_none() {
            let source = transport.run(&source_args)?;
            if source != fs::read(root.join(workflow_path)).map_err(|e| e.to_string())? {
                return Err(format!("unit {unit} executed a different workflow source"));
            }
            workflow_source = Some(source);
        }
        let source = workflow_source.as_ref().unwrap();
        let report_path = downloaded(&registered[&unit].report_path)?;
        let mut input = json!({"unit":unit,"path":report_path,"sha256":file_identity(&no_symlink_parents(root,&report_path)?)?.0,
            "runner_sha256":unit_context["executables"]["runner_sha256"],"judge_sha256":unit_context["executables"]["judge_sha256"],
            "artifacts":{"source_directory":w.artifact_directory,"directory":format!("{destination}/")}});
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
        if c.job_gating.is_none() {
            let endpoint = format!("repos/{repository}/actions/runs/{id}");
            let fresh = transport.api(&endpoint, false)?;
            if fresh["run_attempt"] != attempt
                || fresh["status"] != "completed"
                || fresh["head_sha"] != event_head
                || fresh["event"] != event
                || fresh["path"] != w.workflow_path
            {
                return Err("unit run changed while collecting original evidence".into());
            }
        }
        transport
            .acquisition
            .as_ref()
            .ok_or("artifact budget missing")?
            .remaining()?;
        if c.job_gating.is_none() {
            transport.acquisition = None;
        }
        identities.push(json!({"unit":unit,"run":id,"attempt":attempt,"conclusion":run["conclusion"],"job_id":run["unit_job_id"],"workflow_source_revision":revision,"workflow_sha256":sha256(&source)}));
    }
    if c.job_gating.is_some() && !identities.is_empty() {
        let d: super::gating::Detection =
            serde_json::from_value(context["detection"].clone()).map_err(|e| e.to_string())?;
        let fresh = transport.api(
            &format!("repos/{repository}/actions/runs/{}", d.run_id),
            false,
        )?;
        parent_run(
            &fresh,
            repository,
            &d,
            c,
            context["native_run"]["run_attempt"]
                .as_u64()
                .ok_or("parent attempt missing")?,
        )?;
        transport
            .acquisition
            .as_ref()
            .ok_or("artifact budget missing")?
            .remaining()?;
        transport.acquisition = None;
    }
    if identities.iter().any(|i| i["conclusion"] != "success") {
        return Err(format!(
            "native unit workflow failed; original downloaded reports retained: {}",
            json!(identities)
        ));
    }
    let manifest = json!({"schema":if c.schema==units::FULL_SCHEMA {"chrono-full-collection/v1"}else{"chrono-ci-collection/v1"},"reports":reports});
    let manifest_path = no_symlink_parents(root, &c.gather.manifest_path)?;
    write_file(
        root,
        &c.gather.manifest_path,
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    let manifest_original = json!(chrono_harness::prepared::retain_original(
        root,
        &format!("{}preparation/", c.collection.artifact_directory),
        "manifest",
        &fs::read(&manifest_path).map_err(|e| e.to_string())?
    )?);
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
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let root = root.as_path();
    let _read_scope = chrono_harness::retained_artifacts::retention::read_scope(root)?;
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
    let mut transport = Transport::new(root, &c.gather)?;
    let result = inner(root, path, c, repository, &mut transport);
    let acquisition_path = transport.acquisition_path.take();
    let storage_result = transport.downloads.take().map(|publication| publication.complete(
        json!({"kind":"native-acquisition","result":result.as_ref().ok(),"error":result.as_ref().err(),"completion":"not-established"})
    )).transpose();
    let mut report = json!({"schema":"chrono-ci-gather/v1","repository":repository,"result":result.as_ref().ok(),"error":result.as_ref().err(),"storage_error":storage_result.as_ref().err(),"processes":transport.observations});
    let gather_original = chrono_harness::prepared::retain_original(
        root,
        &format!("{}preparation/", c.collection.artifact_directory),
        "native-gather",
        &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
    )?
    .path;
    report["retained_report"] = json!(gather_original);
    if result.is_err() || storage_result.is_err() {
        if let Some(inventory) =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?
        {
            inventory.protect_path_consumer(
                &format!("gather-recheck:{}", sha256(gather_original.as_bytes())),
                &gather_original,
            )?;
        }
    }
    let _slot = chrono_harness::retained_artifacts::retention::reference_report(
        root,
        &c.gather.report_path,
        &gather_original,
        &acquisition_path.into_iter().collect::<Vec<_>>(),
        None,
    )?;
    write_file(
        root,
        &c.gather.report_path,
        &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )?;
    chrono_harness::retained_artifacts::retention::settle_report(
        root,
        &c.gather.report_path,
        &gather_original,
    )?;
    result?;
    storage_result?;
    Ok(serde_json::to_string(&report).map_err(|e| e.to_string())? + "\n")
}
