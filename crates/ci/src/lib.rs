//! Owned GitHub Actions projection and event input preparation; no judgment or test discovery.
use chrono_harness::{canonical_argv, decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MARKER: &str = "# chrono-ci: owned github-actions/v1\n";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub workflow_path: String,
    pub name: String,
    pub runs_on: String,
    pub push_branches: Vec<String>,
    pub pull_request_branches: Vec<String>,
    pub branch_creation_base_ref: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub push_baselines: Vec<PushBaseline>,
    pub checkout_action: String,
    pub upload_artifact_action: String,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub runner: String,
    pub generator: String,
    pub check_config: String,
    pub context_path: String,
    pub artifact_directory: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushBaseline {
    pub ref_prefix: String,
    pub base_ref: String,
}
fn scalar(s: &str) -> String {
    serde_json::to_string(s).expect("string serialization")
}
fn shell(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}
fn full_oid(s: &str) -> bool {
    matches!(s.len(), 40 | 64)
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && !s.bytes().all(|b| b == b'0')
}
fn action(s: &str, repo: &str) -> Result<(), String> {
    let prefix = format!("{repo}@");
    if !s
        .strip_prefix(&prefix)
        .is_some_and(|v| v.len() == 40 && full_oid(v))
    {
        return Err(format!("expected pinned {repo}@full-commit"));
    }
    Ok(())
}
pub fn load(path: &Path) -> Result<Config, String> {
    let c: Config = decode(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)?;
    validate(&c)?;
    Ok(c)
}
fn validate(c: &Config) -> Result<(), String> {
    if c.schema != "chrono-github-ci/v1" {
        return Err("unsupported CI provider/schema".into());
    }
    for p in [
        &c.workflow_path,
        &c.runner,
        &c.generator,
        &c.check_config,
        &c.context_path,
    ] {
        relative_path(p)?;
    }
    if !c.workflow_path.starts_with(".github/workflows/") || !c.workflow_path.ends_with(".yml") {
        return Err("workflow must be .github/workflows/*.yml".into());
    }
    if !c.check_config.starts_with(".chrono-harness/ci/")
        || !c.context_path.starts_with(".chrono-harness/")
    {
        return Err("CI host configuration/context ownership invalid".into());
    }
    if !c.artifact_directory.ends_with('/') {
        return Err("artifact_directory must end /".into());
    }
    relative_path(c.artifact_directory.trim_end_matches('/'))?;
    if !c.context_path.starts_with(&c.artifact_directory) {
        return Err("context must reside in artifact directory".into());
    }
    if c.name.is_empty()
        || c.runs_on.is_empty()
        || c.push_branches.is_empty()
        || c.pull_request_branches.is_empty()
        || c.timeout_minutes == 0
        || c.bootstrap.is_empty()
        || c.bootstrap.iter().any(|v| v.contains(['\0', '\n', '\r']))
    {
        return Err("missing/invalid CI settings".into());
    }
    if !c.branch_creation_base_ref.starts_with("refs/heads/")
        || c.branch_creation_base_ref.contains([' ', '\n', '\0'])
    {
        return Err("branch creation requires explicit refs/heads/... baseline".into());
    }
    for (i, rule) in c.push_baselines.iter().enumerate() {
        if !rule.ref_prefix.starts_with("refs/heads/")
            || !rule.ref_prefix.ends_with('/')
            || rule.ref_prefix.len() <= "refs/heads/".len()
            || rule
                .ref_prefix
                .contains([' ', '\n', '\r', '\0', '*', '?', '[', '\\'])
            || !rule.base_ref.starts_with("refs/heads/")
            || rule.base_ref.contains([' ', '\n', '\r', '\0'])
            || rule.base_ref.starts_with(&rule.ref_prefix)
        {
            return Err(
                "invalid push baseline: literal branch prefix and separate baseline ref required"
                    .into(),
            );
        }
        if c.push_baselines[..i].iter().any(|other| {
            rule.ref_prefix.starts_with(&other.ref_prefix)
                || other.ref_prefix.starts_with(&rule.ref_prefix)
        }) {
            return Err("ambiguous overlapping push baseline prefixes".into());
        }
    }
    action(&c.checkout_action, "actions/checkout")?;
    action(&c.upload_artifact_action, "actions/upload-artifact")?;
    Ok(())
}
fn invoke(c: &Config, initial: bool) -> String {
    canonical_argv(
        &c.runner,
        &c.check_config,
        if initial { None } else { Some("$CHRONO_BASE") },
        "$CHRONO_CANDIDATE",
        initial,
    )
    .iter()
    .map(|v| {
        if v.starts_with("$CHRONO_") {
            format!("\"{v}\"")
        } else {
            shell(v)
        }
    })
    .collect::<Vec<_>>()
    .join(" ")
}
pub fn render(c: &Config, config_path: &str) -> Result<String, String> {
    validate(c)?;
    relative_path(config_path)?;
    let bootstrap = c
        .bootstrap
        .iter()
        .map(|v| shell(v))
        .collect::<Vec<_>>()
        .join(" ");
    let prepare = format!(
        "{} prepare --host-root . --config {} --event \"$GITHUB_EVENT_NAME\" --payload \"$GITHUB_EVENT_PATH\" --workflow-revision \"$CHRONO_WORKFLOW_REVISION\" --github-output \"$GITHUB_OUTPUT\"",
        shell(&c.generator),
        shell(config_path)
    );
    Ok(format!(
        r#"{MARKER}name: {name}
on:
  push:
    branches: {push}
  pull_request:
    branches: {pr}
  workflow_dispatch:
    inputs:
      base:
        description: Full base commit OID (omit only with initial)
        required: false
        type: string
      candidate:
        description: Full candidate commit OID
        required: true
        type: string
      initial:
        description: Explicit parentless initial inventory
        required: true
        default: false
        type: boolean
permissions:
  contents: read
jobs:
  check:
    if: ${{{{ github.event.deleted != true }}}}
    runs-on: {runs_on}
    timeout-minutes: {timeout}
    steps:
      - name: Check out exact candidate
        uses: {checkout}
        with:
          ref: ${{{{ github.event.pull_request.head.sha || inputs.candidate || github.event.after || github.sha }}}}
          fetch-depth: 0
          persist-credentials: true
      - name: Bootstrap registered tools
        shell: bash
        run: |
          {bootstrap}
      - name: Prepare fixed event inputs
        id: inputs
        shell: bash
        env:
          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}
        run: |
          {prepare}
      - name: Canonical harness check
        shell: bash
        env:
          CHRONO_BASE: ${{{{ steps.inputs.outputs.base }}}}
          CHRONO_CANDIDATE: ${{{{ steps.inputs.outputs.candidate }}}}
          CHRONO_INITIAL: ${{{{ steps.inputs.outputs.initial }}}}
        run: |
          if [ "$CHRONO_INITIAL" = true ]; then
            {initial}
          else
            {normal}
          fi
      - name: Preserve actual check evidence
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: chrono-check-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}
          path: {artifacts}
          if-no-files-found: error
"#,
        name = scalar(&c.name),
        push = serde_json::to_string(&c.push_branches).unwrap(),
        pr = serde_json::to_string(&c.pull_request_branches).unwrap(),
        runs_on = scalar(&c.runs_on),
        timeout = c.timeout_minutes,
        checkout = c.checkout_action,
        upload = c.upload_artifact_action,
        initial = invoke(c, true),
        normal = invoke(c, false),
        artifacts = scalar(&c.artifact_directory)
    ))
}
fn output_preflight(root: &Path, path: &str, expected: &str) -> Result<bool, String> {
    let target = no_symlink_parents(root, path)?;
    match fs::read(&target) {
        Ok(bytes) => {
            let text = std::str::from_utf8(&bytes).map_err(|_| "owned output is not UTF-8")?;
            if !text.starts_with(MARKER) {
                return Err(format!("unowned workflow collision: {path}"));
            }
            Ok(text == expected)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}
fn write_file(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let target = no_symlink_parents(root, path)?;
    let parent = target.parent().ok_or("no output parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".chrono-ci-{}.tmp", std::process::id()));
    let mut opts = fs::OpenOptions::new();
    use std::io::Write;
    let mut f = opts
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temp, &target).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn generate(root: &Path, config_path: &str, verify: bool) -> Result<bool, String> {
    relative_path(config_path)?;
    let c = load(&no_symlink_parents(root, config_path)?)?;
    let rendered = render(&c, config_path)?;
    let same = output_preflight(root, &c.workflow_path, &rendered)?;
    if verify && !same {
        return Err(format!("generated workflow drift: {}", c.workflow_path));
    }
    if !verify && !same {
        write_file(root, &c.workflow_path, rendered.as_bytes())?
    }
    Ok(!same)
}
pub fn init(root: &Path, input: &Path) -> Result<bool, String> {
    let incoming = load(input)?;
    let path = ".chrono-harness/ci/github.json";
    let dest = no_symlink_parents(root, path)?;
    let existing = dest.exists();
    let c = if existing { load(&dest)? } else { incoming };
    let rendered = render(&c, path)?;
    let same = output_preflight(root, &c.workflow_path, &rendered)?;
    if !existing {
        write_file(
            root,
            path,
            &(serde_json::to_string_pretty(&c).map_err(|e| e.to_string())? + "\n").into_bytes(),
        )?
    }
    if !same {
        write_file(root, &c.workflow_path, rendered.as_bytes())?
    }
    Ok(!existing || !same)
}
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    String::from_utf8(o.stdout).map_err(|_| "git output not UTF-8".into())
}
fn require_commit(root: &Path, oid: &str) -> Result<(), String> {
    if !full_oid(oid) {
        return Err(format!("invalid full OID: {oid}"));
    }
    if git(root, &["cat-file", "-e", &format!("{oid}^{{commit}}")]).is_err() {
        git(root, &["fetch", "--no-tags", "origin", oid])?;
    }
    if git(root, &["rev-parse", &format!("{oid}^{{commit}}")])?.trim() != oid {
        return Err("commit identity mismatch".into());
    }
    Ok(())
}
fn remote_baseline(root: &Path, reference: &str) -> Result<String, String> {
    git(root, &["check-ref-format", reference])?;
    let observed = git(root, &["ls-remote", "--refs", "origin", reference])?;
    let fields: Vec<_> = observed.split_whitespace().collect();
    if fields.len() != 2 || fields[1] != reference || !full_oid(fields[0]) {
        return Err(format!(
            "missing or ambiguous remote baseline ref: {reference}"
        ));
    }
    // Bind the observed ref once; later fetches never reread shared FETCH_HEAD.
    let base = fields[0].to_owned();
    require_commit(root, &base)?;
    Ok(base)
}
pub fn prepare(
    root: &Path,
    c: &Config,
    event: &str,
    payload: &Value,
    workflow_revision: &str,
) -> Result<Value, String> {
    validate(c)?;
    if !full_oid(workflow_revision) {
        return Err("workflow source revision must be full OID".into());
    }
    let string = |v: &Value| {
        v.as_str()
            .map(str::to_string)
            .ok_or_else(|| "event missing string input".to_string())
    };
    let (initial, base, candidate, source) = match event {
        "push" => {
            if payload["deleted"].as_bool() == Some(true) {
                return Err("branch deletion has no candidate".into());
            }
            let candidate = string(&payload["after"])?;
            let before = string(&payload["before"])?;
            if !full_oid(&candidate) {
                return Err(format!("invalid full OID: {candidate}"));
            }
            let creation = before.bytes().all(|b| b == b'0') && matches!(before.len(), 40 | 64);
            if creation && payload["created"].as_bool() != Some(true) {
                return Err("zero before without branch-creation event".into());
            }
            if !creation && !full_oid(&before) {
                return Err(format!("invalid full OID: {before}"));
            }
            let event_ref = if creation || !c.push_baselines.is_empty() {
                let reference = payload["ref"]
                    .as_str()
                    .filter(|r| r.starts_with("refs/heads/"))
                    .ok_or(if creation {
                        "branch-creation event requires refs/heads/... ref"
                    } else {
                        "configured push baseline requires refs/heads/... ref"
                    })?;
                git(root, &["check-ref-format", reference])?;
                Some(reference)
            } else {
                None
            };
            if let Some(rule) = c.push_baselines.iter().find(|rule| {
                event_ref.is_some_and(|reference| reference.starts_with(&rule.ref_prefix))
            }) {
                (
                    false,
                    Some(remote_baseline(root, &rule.base_ref)?),
                    candidate,
                    "push-configured-baseline-ref",
                )
            } else if creation {
                let event_ref = event_ref.unwrap();
                if event_ref == c.branch_creation_base_ref {
                    (true, None, candidate, "baseline-creation-initial-inventory")
                } else {
                    let base = remote_baseline(root, &c.branch_creation_base_ref)?;
                    (false, Some(base), candidate, "branch-creation-baseline-ref")
                }
            } else {
                (false, Some(before), candidate, "push-before-after")
            }
        }
        "pull_request" => (
            false,
            Some(string(&payload["pull_request"]["base"]["sha"])?),
            string(&payload["pull_request"]["head"]["sha"])?,
            "pr-base-head",
        ),
        "workflow_dispatch" => {
            let initial = match &payload["inputs"]["initial"] {
                Value::Bool(v) => *v,
                Value::String(v) if v == "true" => true,
                Value::String(v) if v == "false" => false,
                _ => return Err("manual initial must be explicit boolean".into()),
            };
            let base = payload["inputs"]["base"].as_str().unwrap_or("");
            if initial && !base.is_empty() {
                return Err("manual initial cannot include base".into());
            }
            (
                initial,
                if base.is_empty() {
                    None
                } else {
                    Some(base.to_string())
                },
                string(&payload["inputs"]["candidate"])?,
                "manual-explicit",
            )
        }
        _ => return Err(format!("unsupported event: {event}")),
    };
    if !initial && base.is_none() {
        return Err("base required".into());
    }
    require_commit(root, &candidate)?;
    if let Some(base) = &base {
        require_commit(root, base)?
    }
    if git(root, &["rev-parse", "HEAD"])?.trim() != candidate {
        return Err("checkout is not exact event candidate".into());
    }
    if initial && !chrono_harness::facts::parents(root, &candidate)?.is_empty() {
        return Err(if source == "baseline-creation-initial-inventory" {
            "baseline-creation candidate has parents and no prior baseline; supply an explicit range via workflow_dispatch or check --base/--candidate"
        } else {
            "initial candidate has parents"
        }.into());
    }
    Ok(
        json!({"schema":"chrono-ci-inputs/v1","event":event,"source":source,"mode":if initial {"initial-inventory"} else {"delta"},"workflow_source_revision":workflow_revision,"base":base,"candidate":candidate,"initial":initial,"canonical_argv":canonical_argv(&c.runner,&c.check_config,base.as_deref(),&candidate,initial)}),
    )
}
pub fn dispatch(args: &[String]) -> Result<String, String> {
    if args == ["--version"] {
        return Ok(format!("chrono-ci {}\n", env!("CARGO_PKG_VERSION")));
    }
    let command = args
        .first()
        .ok_or("use init/generate/verify/prepare --host-root H --config P")?;
    let mut opts = std::collections::BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let k = &args[i];
        let v = args.get(i + 1).ok_or("missing option value")?;
        if ![
            "--host-root",
            "--config",
            "--event",
            "--payload",
            "--workflow-revision",
            "--github-output",
        ]
        .contains(&k.as_str())
            || opts.insert(k.as_str(), v.as_str()).is_some()
        {
            return Err(format!("unknown/duplicate option {k}"));
        }
        i += 2
    }
    let root = PathBuf::from(*opts.get("--host-root").ok_or("--host-root required")?);
    let config = *opts.get("--config").ok_or("--config required")?;
    if command != "prepare" && opts.len() != 2 {
        return Err("extra arguments".into());
    }
    match command.as_str() {
        "init" => Ok(format!(
            "initialized; changed={}\n",
            init(&root, Path::new(config))?
        )),
        "generate" | "verify" => Ok(format!(
            "{}; changed={}\n",
            command,
            generate(&root, config, command == "verify")?
        )),
        "prepare" => {
            let c = load(&no_symlink_parents(&root, config)?)?;
            let payload = chrono_harness::json(
                &fs::read(opts.get("--payload").ok_or("--payload required")?)
                    .map_err(|e| e.to_string())?,
            )?;
            let context = prepare(
                &root,
                &c,
                opts.get("--event").ok_or("--event required")?,
                &payload,
                opts.get("--workflow-revision")
                    .ok_or("--workflow-revision required")?,
            )?;
            write_file(
                &root,
                &c.context_path,
                &(serde_json::to_string_pretty(&context).map_err(|e| e.to_string())? + "\n")
                    .into_bytes(),
            )?;
            if let Some(output) = opts.get("--github-output") {
                use std::io::Write;
                let mut f = fs::OpenOptions::new()
                    .append(true)
                    .open(output)
                    .map_err(|e| e.to_string())?;
                writeln!(
                    f,
                    "base={}\ncandidate={}\ninitial={}",
                    context["base"].as_str().unwrap_or(""),
                    context["candidate"].as_str().ok_or("candidate absent")?,
                    context["initial"]
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(serde_json::to_string(&context).map_err(|e| e.to_string())? + "\n")
        }
        _ => Err("unknown command".into()),
    }
}
