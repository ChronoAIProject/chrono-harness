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

/// Acquire the early collection artifact through the existing registered transport.
/// A running collector is eligible once its unique seed is published; no business work is awaited.
pub(super) fn acquire_seed(
    root: &Path,
    path: &str,
    c: &units::Config,
    unit: &str,
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
    let endpoints = units::prepare_endpoints(root, path, c, Some(unit), event, payload, revision)?;
    let candidate = endpoints["candidate"].as_str().ok_or("seed candidate")?;
    if repository.split('/').count() != 2
        || repository
            .bytes()
            .any(|b| !b.is_ascii_alphanumeric() && !b"/-_.".contains(&b))
    {
        return Err("repository must be explicit owner/name".into());
    }
    let w = c.workflow(Some(unit))?;
    let directory = format!("{}seed/", w.artifact_directory);
    let mut transport = Transport::new(root, &c.gather)?;
    let result: Result<Value, String> = (|| {
        let deadline = Instant::now() + Duration::from_secs(c.gather.wait_seconds);
        let filename = c
            .collection
            .workflow_path
            .strip_prefix(".github/workflows/")
            .ok_or("collection workflow path")?;
        let run = loop {
            let pages=transport.api(&format!("repos/{repository}/actions/workflows/{}/runs?head_sha={candidate}&event={event}&per_page=100",encode(filename)),true)?;
            let mut runs = Vec::new();
            for page in pages.as_array().ok_or("paginated seed run pages")? {
                for run in page["workflow_runs"].as_array().ok_or("seed run list")? {
                    if run["head_sha"] == candidate
                        && run["event"] == event
                        && run["path"] == c.collection.workflow_path
                    {
                        runs.push(run.clone());
                    }
                }
            }
            if runs.len() > 1 {
                return Err("ambiguous shared seed runs".into());
            }
            if let Some(run) = runs.first() {
                let id = run["id"].as_u64().ok_or("seed run ID")?;
                let attempt = run["run_attempt"]
                    .as_u64()
                    .filter(|v| *v > 0)
                    .ok_or("seed run attempt")?;
                let name = format!("{}-{id}-{attempt}", a.seed_artifact);
                let pages = transport.api(
                    &format!("repos/{repository}/actions/runs/{id}/artifacts?per_page=100"),
                    true,
                )?;
                let mut matches = Vec::new();
                for page in pages.as_array().ok_or("seed artifact pages")? {
                    for artifact in page["artifacts"].as_array().ok_or("seed artifact list")? {
                        if artifact["name"] == name {
                            matches.push(artifact.clone());
                        }
                    }
                }
                if matches.len() > 1 {
                    return Err("ambiguous shared seed artifacts".into());
                }
                if let Some(artifact) = matches.first() {
                    if artifact["expired"] != false {
                        return Err("shared seed artifact expired".into());
                    }
                    break (run.clone(), name, artifact.clone());
                }
            }
            if Instant::now() >= deadline {
                return Err("shared seed acquisition wait expired".into());
            }
            std::thread::sleep(
                Duration::from_secs(c.gather.poll_seconds)
                    .min(deadline.saturating_duration_since(Instant::now())),
            );
        };
        let (run, name, artifact) = run;
        let id = run["id"].as_u64().ok_or("seed run ID")?;
        let attempt = run["run_attempt"].as_u64().ok_or("seed attempt")?;
        let absolute = no_symlink_parents(root, &directory)?;
        if absolute.exists() {
            return Err("seed download exists; retain original acquisition".into());
        }
        fs::create_dir_all(&absolute).map_err(|e| e.to_string())?;
        transport.run(&[
            "run".into(),
            "download".into(),
            id.to_string(),
            "--repo".into(),
            repository.into(),
            "--name".into(),
            name,
            "--dir".into(),
            absolute.to_str().ok_or("seed directory UTF8")?.into(),
        ])?;
        let read = |file: &str| {
            chrono_harness::units::read_bounded(
                root,
                &format!("{directory}{file}"),
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
        if fresh["run_attempt"] != attempt
            || fresh["head_sha"] != candidate
            || fresh["event"] != event
            || fresh["path"] != c.collection.workflow_path
        {
            return Err("shared seed run changed during acquisition".into());
        }
        write_file(
            root,
            &c.full_contexts
                .as_ref()
                .ok_or("full contexts missing")?
                .units[unit],
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
