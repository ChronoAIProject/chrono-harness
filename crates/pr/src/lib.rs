mod snapshot;
mod state;

use chrono_harness::{
    CliOutput, CommandSpec, ProcessEvidence, file_identity, resolve_program,
    run_process_observed_retained_for,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const HELP: &str = "chrono-pr pr-open --head BRANCH --head-sha SHA --title TITLE --body-file FILE [--draft]\nchrono-pr pr-status --pr NUMBER --head-sha SHA [--continue-operation PATH]\nchrono-pr pr-watch --pr NUMBER --head-sha SHA\nAll commands: [--host-root ROOT] [--config .chrono-harness/pr.json]\nResults: JSON; exits 0 complete, 1 failed/closed, 2 unknown/error, 3 pending, 4 stale.\n";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    repository: String,
    base: String,
    gh: String,
    required_checks: Vec<String>,
    api_timeout_seconds: u64,
    watch_timeout_seconds: u64,
    watch_interval_seconds: u64,
    output_limit_bytes: usize,
    state_directory: String,
    state_producer: String,
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        let parts = self.repository.split('/').collect::<Vec<_>>();
        if self.schema != "chrono-pr-config/v2"
            || parts.len() != 2
            || parts.iter().any(|s| {
                s.is_empty()
                    || !s.as_bytes()[0].is_ascii_alphanumeric()
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            })
        {
            return Err("invalid config schema or owner/repository".into());
        }
        chrono_harness::relative_path(&self.state_directory)?;
        if !self.state_directory.starts_with(".chrono-harness/state/")
            || self.state_directory.ends_with('/')
            || self.state_producer.is_empty()
            || self.state_producer.len() > 128
            || self.state_producer.chars().any(char::is_control)
        {
            return Err("invalid PR state directory or producer".into());
        }
        if !branch(&self.base)
            || self.gh.is_empty()
            || self.gh.contains('\0')
            || self.required_checks.is_empty()
            || self
                .required_checks
                .iter()
                .any(|s| s.trim().is_empty() || s.chars().any(char::is_control))
            || self
                .required_checks
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.required_checks.len()
            || !(1..=3600).contains(&self.api_timeout_seconds)
            || !(1..=86400).contains(&self.watch_timeout_seconds)
            || !(60..=3600).contains(&self.watch_interval_seconds)
            || !(4096..=16777216).contains(&self.output_limit_bytes)
        {
            return Err("invalid PR config: branch, tool, checks or resource bounds (watch interval >=60 seconds)".into());
        }
        Ok(())
    }
}

struct Args {
    command: String,
    values: BTreeMap<String, String>,
    draft: bool,
}
impl Args {
    fn parse(raw: &[OsString]) -> Result<Self, String> {
        let raw = raw
            .iter()
            .map(|s| s.to_str().ok_or("arguments must be UTF-8"))
            .collect::<Result<Vec<_>, _>>()?;
        let command = *raw.first().ok_or("missing command; use --help")?;
        if !["pr-open", "pr-status", "pr-watch"].contains(&command) {
            return Err("unknown command; use --help".into());
        }
        let mut result = Self {
            command: command.into(),
            values: BTreeMap::new(),
            draft: false,
        };
        let mut index = 1;
        while index < raw.len() {
            let flag = raw[index];
            if flag == "--draft" && command == "pr-open" {
                if result.draft {
                    return Err("duplicate --draft".into());
                }
                result.draft = true;
                index += 1;
                continue;
            }
            let common = ["--host-root", "--config", "--head-sha"];
            let specific = if command == "pr-open" {
                &["--head", "--title", "--body-file"][..]
            } else if command == "pr-status" {
                &["--pr", "--continue-operation"][..]
            } else {
                &["--pr"][..]
            };
            if !common.contains(&flag) && !specific.contains(&flag) {
                return Err(format!("unknown option {flag}"));
            }
            let value = *raw
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            if value.is_empty()
                || value.contains('\0')
                || value.starts_with("--")
                || result.values.insert(flag.into(), value.into()).is_some()
            {
                return Err(format!("invalid or repeated option {flag}"));
            }
            index += 2;
        }
        if !snapshot::sha(result.get("--head-sha")?) {
            return Err("--head-sha must be 40 lowercase hex digits".into());
        }
        if command == "pr-open" {
            if !branch(result.get("--head")?)
                || result.get("--title")?.trim().is_empty()
                || result.get("--title")?.chars().any(char::is_control)
            {
                return Err("invalid head branch or title".into());
            }
            result.get("--body-file")?;
        } else {
            result.number()?;
        }
        Ok(result)
    }
    fn get(&self, key: &str) -> Result<&str, String> {
        self.values
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("missing {key}"))
    }
    fn number(&self) -> Result<u64, String> {
        let value = self.get("--pr")?;
        if !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err("invalid PR number".into());
        }
        value
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0 && *n <= i32::MAX as u64)
            .ok_or("invalid PR number".into())
    }
}

fn branch(value: &str) -> bool {
    !value.is_empty()
        && value != "@"
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && !value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c))
        && value
            .split('/')
            .all(|p| !p.starts_with('.') && !p.ends_with('.') && !p.ends_with(".lock"))
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > limit {
        return Err(format!("{} exceeds byte limit", path.display()));
    }
    Ok(data)
}

struct Client {
    root: PathBuf,
    config: Config,
    executable: String,
    digest: String,
    env: BTreeMap<String, String>,
    started: Instant,
    budget: Duration,
    pending_deadline: bool,
    result: Value,
    state: state::State,
}

impl Client {
    fn new(args: &Args, result: Value) -> Result<Self, String> {
        let root = fs::canonicalize(
            args.values
                .get("--host-root")
                .map(String::as_str)
                .unwrap_or("."),
        )
        .map_err(|e| e.to_string())?;
        let config_path = args
            .values
            .get("--config")
            .map(String::as_str)
            .unwrap_or(".chrono-harness/pr.json");
        let path = chrono_harness::no_symlink_parents(&root, config_path)?;
        let config_bytes = read_bounded(&path, 1048576)?;
        let config: Config = chrono_harness::decode(&config_bytes)?;
        config.validate()?;
        let mut env = BTreeMap::new();
        for key in [
            "PATH",
            "HOME",
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GH_CONFIG_DIR",
            "SSH_AUTH_SOCK",
            "SSL_CERT_FILE",
            "SSL_CERT_DIR",
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "NO_PROXY",
            "TMPDIR",
        ] {
            if let Ok(value) = std::env::var(key) {
                env.insert(key.into(), value);
            }
        }
        env.extend([
            ("GH_PROMPT_DISABLED".into(), "1".into()),
            ("GH_HOST".into(), "github.com".into()),
            ("GH_PAGER".into(), "cat".into()),
            ("NO_COLOR".into(), "1".into()),
        ]);
        let executable = resolve_program(
            &root,
            &config.gh,
            Some(env.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        let (digest, _) = file_identity(&executable)?;
        let budget = Duration::from_secs(if args.command == "pr-watch" {
            config.watch_timeout_seconds
        } else {
            config.api_timeout_seconds.saturating_mul(4)
        });
        let state = state::State::begin(&root, &config, &config_bytes, args)?;
        let mut client = Self {
            root,
            config,
            executable: executable.to_str().ok_or("gh path must be UTF-8")?.into(),
            digest,
            env,
            started: Instant::now(),
            budget,
            pending_deadline: false,
            result,
            state,
        };
        client.result["continued_operation"] = json!(client.state.continued_path());
        client.result["operation"] = json!(client.state.path);
        client.result["repository"] = json!(client.config.repository);
        client.result["base"] = json!(client.config.base);
        Ok(client)
    }

    fn call(
        &mut self,
        argv: Vec<String>,
        input: &[u8],
        wait: bool,
    ) -> Result<(i32, Value), String> {
        let remaining = self.budget.saturating_sub(self.started.elapsed());
        if remaining.is_zero() {
            self.pending_deadline = self.result["outcome"] == "pending";
            return Err("operation deadline reached; remote result remains unverified".into());
        }
        let timeout = if wait {
            remaining
        } else {
            remaining.min(Duration::from_secs(self.config.api_timeout_seconds))
        };
        let spec = CommandSpec {
            program: self.executable.clone(),
            args: argv,
            env: self.env.clone(),
            timeout_seconds: timeout.as_secs().max(1),
            output_limit_bytes: self.config.output_limit_bytes,
        };
        let attempt = self
            .state
            .call_intent(&spec, input, &self.digest, timeout)?;
        let paths = self.state.process_paths(attempt);
        let evidence = ProcessEvidence {
            stdout: &paths[0],
            stderr: &paths[1],
            launch: &paths[2],
        };
        let process = run_process_observed_retained_for(
            &self.root,
            &spec,
            input,
            &self.digest,
            timeout,
            &evidence,
        )?;
        let code = process.exit_code;
        let receipt = match self.state.receipt(attempt, &process) {
            Ok(receipt) => receipt,
            Err(error) => {
                let mut original = state::State::process_fields(&process);
                original["stdout_bytes"] = json!(process.stdout_bytes);
                original["stderr_bytes"] = json!(process.stderr_bytes);
                original["storage_error"] = json!(error);
                self.result["receipts"]
                    .as_array_mut()
                    .unwrap()
                    .push(original);
                return Err(format!("PR original publication failed: {error}"));
            }
        };
        self.result["receipts"]
            .as_array_mut()
            .unwrap()
            .push(receipt);
        if let Some(failure) = process.failure {
            // Deadline classification is based on the local clock and the original
            // producer failure, never on an assumed remote CI terminal state.
            self.pending_deadline = wait && failure == "process timed out";
            return Err(failure);
        }
        if wait {
            return Ok((code, Value::Null));
        }
        if code != 0 {
            return Err(format!(
                "GitHub command exited {code}; see original receipt"
            ));
        }
        let value = if spec.args.get(1).map(String::as_str) == Some("create") {
            json!(String::from_utf8(process.stdout_bytes).map_err(|_| "non-UTF-8 creation output")?)
        } else {
            chrono_harness::json(&process.stdout_bytes)?
        };
        Ok((code, value))
    }

    fn graphql(&mut self, query: &str, variables: Vec<(&str, String)>) -> Result<Value, String> {
        let (owner, name) = self.config.repository.split_once('/').unwrap();
        let mut argv = vec![
            "api".into(),
            "graphql".into(),
            "-f".into(),
            format!("query={query}"),
            "-f".into(),
            format!("owner={owner}"),
            "-f".into(),
            format!("name={name}"),
        ];
        for (key, value) in variables {
            argv.push(if key == "number" {
                "-F".into()
            } else {
                "-f".into()
            });
            argv.push(format!("{key}={value}"));
        }
        self.call(argv, &[], false).map(|(_, v)| v)
    }

    fn snapshot(&mut self, number: u64, head: &str) -> Result<snapshot::Snapshot, String> {
        self.state.reserve_snapshot(number)?;
        let raw = self.graphql(
            snapshot::QUERY,
            vec![("number", number.to_string()), ("head", head.into())],
        )?;
        let parsed = snapshot::parse(
            &raw,
            &self.config.repository,
            &self.config.base,
            number,
            head,
            &self.config.required_checks,
        )?;
        self.state
            .observed_snapshot(chrono_harness::verified_release::open_pr(&parsed.state))?;
        self.result["pr"] = json!(parsed.number);
        self.result["url"] = json!(parsed.url);
        self.result["observed_head"] = json!(parsed.head);
        self.result["pr_state"] = json!(parsed.state);
        self.result["merge_commit"] = json!(parsed.merge_commit);
        self.result["draft"] = json!(parsed.draft);
        self.result["checks"] = json!(parsed.checks);
        self.result["outcome"] = json!(parsed.outcome);
        Ok(parsed)
    }

    fn open(&mut self, args: &Args) -> Result<(), String> {
        let branch = args.get("--head")?;
        let head = args.get("--head-sha")?;
        let body = read_bounded(
            &self.root.join(args.get("--body-file")?),
            self.config.output_limit_bytes,
        )?;
        std::str::from_utf8(&body).map_err(|_| "body must be UTF-8")?;
        let raw=self.graphql("query($owner:String!,$name:String!,$ref:String!){repository(owner:$owner,name:$name){nameWithOwner ref(qualifiedName:$ref){target{... on Commit{oid}}}}}",vec![("ref",format!("refs/heads/{branch}"))])?;
        snapshot::repository(&raw, &self.config.repository)?;
        let observed = snapshot::field(&raw, "/data/repository/ref/target/oid")?;
        if !snapshot::sha(observed) {
            return Err("invalid remote branch SHA".into());
        }
        self.result["observed_head"] = json!(observed);
        if observed != head {
            self.result["outcome"] = json!("stale");
            return Ok(());
        }
        let (_, list) = self.call(
            vec![
                "pr".into(),
                "list".into(),
                "--repo".into(),
                self.config.repository.clone(),
                "--head".into(),
                branch.into(),
                "--base".into(),
                self.config.base.clone(),
                "--state".into(),
                "open".into(),
                "--json".into(),
                "number".into(),
            ],
            &[],
            false,
        )?;
        let items = list.as_array().ok_or("invalid PR list")?;
        let (number, outcome) = match items.as_slice() {
            [] => {
                let mut argv = vec![
                    "pr".into(),
                    "create".into(),
                    "--repo".into(),
                    self.config.repository.clone(),
                    "--base".into(),
                    self.config.base.clone(),
                    "--head".into(),
                    branch.into(),
                    "--title".into(),
                    args.get("--title")?.into(),
                    "--body-file".into(),
                    "-".into(),
                ];
                if args.draft {
                    argv.push("--draft".into());
                }
                let (_, created) = self.call(argv, &body, false)?;
                let url = created.as_str().ok_or("invalid create response")?.trim();
                let prefix = format!("https://github.com/{}/pull/", self.config.repository);
                let number = url
                    .strip_prefix(&prefix)
                    .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|n| *n > 0 && *n <= i32::MAX as u64)
                    .ok_or(
                        "creation returned an invalid PR URL; query existing PR before retrying",
                    )?;
                (number, "opened")
            }
            [item] => (
                item["number"]
                    .as_u64()
                    .filter(|n| *n > 0 && *n <= i32::MAX as u64)
                    .ok_or("invalid existing PR number")?,
                "existing",
            ),
            _ => return Err("ambiguous existing PRs; no creation attempted".into()),
        };
        self.result["pr"] = json!(number);
        let parsed = self.snapshot(number, head)?;
        if parsed.branch != branch {
            return Err("PR head branch mismatch".into());
        }
        self.state.confirm_create();
        if parsed.outcome == "stale" || parsed.outcome == "closed" {
            return Ok(());
        }
        if parsed.state != "OPEN" {
            return Err("PR merged during open; creation result is no longer open".into());
        }
        if args.draft && !parsed.draft {
            return Err("requested draft state not present; existing PR was preserved".into());
        }
        self.result["outcome"] = json!(outcome);
        Ok(())
    }

    fn execute(&mut self, args: &Args) -> Result<(), String> {
        if args.command == "pr-open" {
            return self.open(args);
        }
        let number = args.number()?;
        let head = args.get("--head-sha")?;
        let snapshot = self.snapshot(number, head)?;
        if args.command == "pr-status" || snapshot.outcome != "pending" {
            return Ok(());
        }
        // One native blocking channel owns all remote waiting. No harness polling.
        let wait = self.call(
            vec![
                "pr".into(),
                "checks".into(),
                number.to_string(),
                "--repo".into(),
                self.config.repository.clone(),
                "--watch".into(),
                "--interval".into(),
                self.config.watch_interval_seconds.to_string(),
            ],
            &[],
            true,
        );
        let (code, _) = match wait {
            Ok(result) => result,
            Err(error) => {
                if self.result["outcome"] != "pending" {
                    self.result["outcome"] = json!("unknown");
                }
                return Err(error);
            }
        };
        // Transport/auth failures are unknown; ordinary check failure (1) and
        // still pending (8) still permit one factual readback.
        if ![0, 1, 8].contains(&code) {
            self.result["outcome"] = json!("unknown");
            return Err(format!("native wait exited {code}; see original receipt"));
        }
        self.snapshot(number, head)?;
        Ok(())
    }
}

pub fn run(raw: &[OsString]) -> CliOutput {
    if raw.len() == 1 && (raw[0] == "--help" || raw[0] == "-h") {
        return CliOutput {
            exit_code: 0,
            stdout: HELP.into(),
            stderr: String::new(),
        };
    }
    let mut result = json!({"schema":"chrono-pr-result/v2","continued_operation":null,"operation":null,"operation_state":"unknown","pr_state":null,"merge_commit":null,"command":raw.first().and_then(|s|s.to_str()),"repository":null,"base":null,"pr":null,"expected_head":null,"observed_head":null,"outcome":"unknown","checks":[],"error":null,"receipts":[]});
    match Args::parse(raw) {
        Err(error) => result["error"] = json!(error),
        Ok(args) => {
            result["expected_head"] = json!(args.get("--head-sha").unwrap());
            if let Ok(number) = args.number() {
                result["pr"] = json!(number);
            }
            match Client::new(&args, result.clone()) {
                Err(error) => result["error"] = json!(error),
                Ok(mut client) => {
                    if let Err(error) = client.execute(&args) {
                        // Only an actual watch deadline retains pending. All
                        // other errors invalidate an earlier snapshot outcome.
                        if !client.pending_deadline {
                            client.result["outcome"] = json!("unknown");
                        }
                        client.result["error"] = json!(error);
                    }
                    client.result["operation_state"] = json!(client.state.operation_state());
                    client.result["command_exit_code"] = json!(exit_code(&client.result));
                    match state::now() {
                        Ok(now) => client.result["updated_at_unix_millis"] = json!(now),
                        Err(error) => {
                            client.result["outcome"] = json!("unknown");
                            client.result["error"] = json!(error);
                        }
                    }
                    if let Err(error) = client.state.finish(&client.result) {
                        client.result["operation_state"] = json!("unknown");
                        client.result["outcome"] = json!("unknown");
                        client.result["storage_error"] = json!(error);
                    }
                    result = client.result;
                }
            }
        }
    }
    let exit_code = exit_code(&result);
    result["command_exit_code"] = json!(exit_code);
    CliOutput {
        exit_code,
        stdout: format!("{}\n", serde_json::to_string(&result).unwrap()),
        stderr: String::new(),
    }
}

fn exit_code(result: &Value) -> u8 {
    match result["outcome"].as_str().unwrap_or("unknown") {
        "opened" | "existing" | "passed" => 0,
        "failed" | "closed" => 1,
        "pending" => 3,
        "stale" => 4,
        _ => 2,
    }
}
