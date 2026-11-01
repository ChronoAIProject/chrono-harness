//! Bounded byte acquisition. Semantic owners request each original explicitly.
use super::*;
use chrono_harness::prepared::{ArtifactTransport, Original};

const MEMBER_SOURCE: &[u8] = include_bytes!("../../../../.chrono-harness/ci/artifact_members.py");

pub(super) struct Artifact {
    deadline: Instant,
    started: Instant,
    stdout_remaining: usize,
    stderr_remaining: usize,
    retries_used: u32,
    processes: usize,
    pub(super) directory: String,
    staging: String,
    archive: String,
    length: u64,
    executable: String,
    executable_digest: String,
    script: String,
    inputs: BTreeMap<String, String>,
}

impl Artifact {
    pub(super) fn new(
        transport: &mut Transport<'_>,
        run: u64,
        repository: &str,
        name: &str,
        destination: &str,
        candidate: &str,
    ) -> Result<Self, String> {
        // This end precedes metadata lookup, runtime qualification, all member
        // processes, semantic decoding and retry delays. It is never reset.
        let started = Instant::now();
        let deadline = started
            .checked_add(Duration::from_secs(transport.config.timeout_seconds))
            .ok_or("artifact acquisition deadline overflow")?;
        let config = transport
            .config
            .typed_acquisition
            .as_ref()
            .ok_or("typed acquisition missing")?;
        let script_path = no_symlink_parents(transport.root, &config.script)?;
        if fs::read(&script_path).map_err(|e| e.to_string())? != MEMBER_SOURCE {
            return Err("artifact member runtime differs from candidate executable input".into());
        }
        let executable = chrono_harness::resolve_program(
            transport.root,
            &config.program,
            Some(
                transport
                    .environment
                    .get("PATH")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
        )?;
        let executable_digest = file_identity(&executable)?.0;
        let absolute = no_symlink_parents(transport.root, destination)?;
        if absolute.exists() {
            return Err("artifact acquisition destination exists; preserve original input".into());
        }
        chrono_harness::retained_artifacts::retention::directory_adopted(
            transport.downloads.as_ref(),
            &absolute,
        )?;
        let mut artifact = Self {
            deadline,
            started,
            stdout_remaining: transport.config.output_limit_bytes,
            stderr_remaining: transport.config.output_limit_bytes,
            retries_used: 0,
            processes: 0,
            directory: format!("{destination}/inputs"),
            staging: format!("{destination}/attempts"),
            archive: String::new(),
            length: 0,
            executable: executable.to_str().ok_or("member runtime UTF8")?.into(),
            executable_digest,
            script: script_path.to_str().ok_or("member script UTF8")?.into(),
            inputs: BTreeMap::new(),
        };
        let args = vec![
            "api".into(),
            format!("repos/{repository}/actions/runs/{run}/artifacts?per_page=100"),
            "--paginate".into(),
            "--slurp".into(),
        ];
        let pages: Value = decode(&artifact.run(transport, &args)?)?;
        let mut found = Vec::new();
        for page in pages.as_array().ok_or("artifact metadata pages missing")? {
            for value in page["artifacts"]
                .as_array()
                .ok_or("artifact metadata list missing")?
            {
                if value["name"] == name {
                    found.push(value);
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
        artifact.archive =
            format!("https://api.github.com/repos/{repository}/actions/artifacts/{id}/zip");
        if found[0]["archive_download_url"] != artifact.archive {
            return Err("artifact archive endpoint differs from selected ID".into());
        }
        artifact.length = found[0]["size_in_bytes"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("artifact ZIP size missing")?;
        artifact.remaining()?;
        Ok(artifact)
    }

    fn remaining(&self) -> Result<Duration, String> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("artifact acquisition cumulative deadline exhausted".into());
        }
        Ok(remaining)
    }

    fn process(
        &mut self,
        transport: &mut Transport<'_>,
        args: &[String],
        stdin: &[u8],
        member: bool,
    ) -> Result<Vec<u8>, String> {
        loop {
            let remaining = self.remaining()?;
            let limit = self.stdout_remaining.min(self.stderr_remaining);
            if limit == 0 {
                return Err("artifact acquisition cumulative stream limit exhausted".into());
            }
            let (executable, digest) = if member {
                (self.executable.clone(), self.executable_digest.clone())
            } else {
                (transport.executable.clone(), transport.digest.clone())
            };
            self.processes += 1;
            let p =
                transport.process_program(&executable, &digest, args, stdin, remaining, limit)?;
            self.stdout_remaining = self.stdout_remaining.saturating_sub(p.stdout_bytes.len());
            self.stderr_remaining = self.stderr_remaining.saturating_sub(p.stderr_bytes.len());
            transport
                .observations
                .last_mut()
                .ok_or("acquisition observation missing")?["acquisition"] = json!({"kind":"typed-canonical-inputs","directory":self.directory,"process":self.processes,
                    "runtime_source_sha256":sha256(MEMBER_SOURCE),"elapsed_seconds":self.started.elapsed().as_secs_f64(),
                    "status":download_http_status(&p.stderr_bytes),"process_exit":p.exit_code,
                    "process_failure":p.failure,"full_archive_verified":false});
            self.remaining()?;
            if p.failure.is_none() && p.exit_code == 0 {
                transport
                    .observations
                    .last_mut()
                    .ok_or("acquisition observation missing")?["acquisition"]["action"] =
                    json!("completed");
                return Ok(p.stdout_bytes);
            }
            let status = download_http_status(&p.stderr_bytes);
            let policy = transport.config.download_retry.as_ref();
            let retry = p.failure.is_none()
                && policy.is_some_and(|policy| {
                    self.retries_used + 1 < policy.max_attempts
                        && status.is_some_and(|s| policy.http_statuses.contains(&s))
                });
            transport
                .observations
                .last_mut()
                .ok_or("acquisition observation missing")?["acquisition"]["action"] =
                json!(if retry { "retry" } else { "failed" });
            if !retry {
                return Err(
                    "typed artifact transport failed; original bounded process evidence retained"
                        .into(),
                );
            }
            let delay = Duration::from_secs(policy.ok_or("retry policy missing")?.delay_seconds);
            if delay >= self.remaining()? {
                return Err("artifact acquisition retry exceeds cumulative deadline".into());
            }
            self.retries_used += 1;
            std::thread::sleep(delay);
        }
    }

    pub(super) fn run(
        &mut self,
        transport: &mut Transport<'_>,
        args: &[String],
    ) -> Result<Vec<u8>, String> {
        self.process(transport, args, &[], false)
    }

    pub(super) fn member(
        &mut self,
        transport: &mut Transport<'_>,
        member: &str,
        expected: Option<&str>,
        limit: u64,
    ) -> Result<Vec<u8>, String> {
        relative_path(member)?;
        self.remaining()?;
        let logical = format!("{}/{member}", self.directory);
        let path = no_symlink_parents(transport.root, &logical)?;
        if !self.inputs.contains_key(member) {
            if path.exists() {
                return Err("typed acquisition input collision".into());
            }
            let staged = format!("{}/member-{}", self.staging, self.processes);
            let staged_path = no_symlink_parents(transport.root, &staged)?;
            let args = vec!["-B".into(), self.script.clone()];
            let request = serde_json::to_vec(&json!({"schema":"chrono-artifact-member-request/v1",
                "archive_url":self.archive,"archive_length":self.length,"member":member,
                "destination":staged_path.to_str().ok_or("staged input UTF8")?,"limit":limit,
                "remaining_seconds":self.remaining()?.as_secs_f64()}))
            .map_err(|e| e.to_string())?;
            // HTTP failures precede output publication; retries share the
            // artifact's outer deadline, stream allowance and retry count.
            let output = self.process(transport, &args, &request, true)?;
            let receipt: Value = decode(&output)?;
            let raw = chrono_harness::units::read_bounded(transport.root, &staged, limit)?;
            let digest = sha256(&raw);
            if receipt["schema"] != "chrono-artifact-member/v1"
                || receipt["member"] != member
                || receipt["sha256"] != digest
                || receipt["bytes"] != raw.len()
                || receipt["full_archive_verified"] != false
                || expected.is_some_and(|v| v != digest)
            {
                return Err("typed artifact member receipt/original identity mismatch".into());
            }
            fs::create_dir_all(path.parent().ok_or("typed input parent missing")?)
                .map_err(|e| e.to_string())?;
            fs::rename(staged_path, &path).map_err(|e| e.to_string())?;
            self.inputs.insert(member.into(), digest);
        }
        let raw = chrono_harness::units::read_bounded(transport.root, &logical, limit)?;
        let digest = sha256(&raw);
        if self.inputs.get(member) != Some(&digest) || expected.is_some_and(|v| v != digest) {
            return Err("typed acquired original changed or digest disagrees".into());
        }
        self.remaining()?;
        Ok(raw)
    }

    pub(super) fn original(
        &mut self,
        transport: &mut Transport<'_>,
        original: &Original,
        mapping: &ArtifactTransport,
        limit: u64,
    ) -> Result<Vec<u8>, String> {
        let path = chrono_harness::prepared::original_path(original, Some(mapping))?;
        let member = path
            .strip_prefix(&mapping.directory)
            .ok_or("typed original mapping differs")?;
        self.member(transport, member, Some(&original.sha256), limit)
    }

    pub(super) fn finish(&self) -> Result<(), String> {
        self.remaining().map(|_| ())
    }
}
