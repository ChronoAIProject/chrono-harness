//! Owned GitHub Actions projection and event input preparation; no judgment or test discovery.
use chrono_harness::{decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
pub mod cache;
mod check_inputs;
pub mod downloads;
mod event_git;
pub mod full;
mod gather;
pub mod gating;
pub mod generation;
pub mod inventory;
pub mod migrate;
pub mod release;
pub mod startup;
pub mod units;
pub mod verified_download;
pub mod verified_generation;
pub mod verified_inventory;
pub mod verified_literals;

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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_hidden_files: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub retention_delivery: bool,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub runner: String,
    pub generator: String,
    pub check_config: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "string_present"
    )]
    pub facts_config: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "inventory_present"
    )]
    pub initial_inventory: Option<InitialInventory>,
    pub context_path: String,
    pub artifact_directory: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "string_present"
    )]
    pub failure_artifact_directory: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialInventory {
    pub path: String,
    pub profile: chrono_harness::initial::Profile,
}
fn inventory_present<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<InitialInventory>, D::Error> {
    InitialInventory::deserialize(d).map(Some)
}
fn string_present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
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
fn artifact_hidden_files(enabled: bool) -> &'static str {
    if enabled {
        "          include-hidden-files: true\n"
    } else {
        ""
    }
}
fn shell(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}
fn full_oid(s: &str) -> bool {
    chrono_harness::verified_identifiers::valid_oid(s.as_bytes())
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
fn overlap(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
}
pub fn load(path: &Path) -> Result<Config, String> {
    let c: Config = decode(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)?;
    validate(&c)?;
    Ok(c)
}
enum Projection {
    Units(units::Config),
    Check(Config),
    Full(full::Config),
    Release(release::Config),
}
fn load_projection(path: &Path) -> Result<Projection, String> {
    decode_projection(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
}
fn decode_projection(bytes: &[u8]) -> Result<Projection, String> {
    let value: Value = decode(bytes)?;
    if matches!(
        value["schema"].as_str(),
        Some(units::SCHEMA | units::FULL_SCHEMA)
    ) {
        let c = decode(bytes)?;
        units::validate(&c)?;
        Ok(Projection::Units(c))
    } else if matches!(
        value["schema"].as_str(),
        Some(release::SCHEMA | release::UNITS_SCHEMA)
    ) {
        Ok(Projection::Release(release::decode_config(bytes)?))
    } else if matches!(value["schema"].as_str(), Some(full::SCHEMA)) {
        let c = decode(bytes)?;
        full::validate(&c)?;
        Ok(Projection::Full(c))
    } else {
        let c = decode(bytes)?;
        validate(&c)?;
        Ok(Projection::Check(c))
    }
}
fn validate(c: &Config) -> Result<(), String> {
    validate_policy(c, false)
}
fn validate_policy(c: &Config, full_policy: bool) -> Result<(), String> {
    if c.schema != "chrono-github-ci/v4" || c.facts_config.is_none() {
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
    let valid_policy = if full_policy {
        c.check_config.starts_with(".chrono-harness/")
            && !c.check_config.starts_with(".chrono-harness/state/")
    } else {
        c.check_config.starts_with(".chrono-harness/ci/")
    };
    if !valid_policy || !c.context_path.starts_with(".chrono-harness/") {
        return Err("CI host configuration/context ownership invalid".into());
    }
    for directory in std::iter::once(&c.artifact_directory).chain(&c.failure_artifact_directory) {
        if !directory.ends_with('/') {
            return Err("artifact directories must end /".into());
        }
        relative_path(directory.trim_end_matches('/'))?;
    }
    if !c.context_path.starts_with(&c.artifact_directory) {
        return Err("context must reside in artifact directory".into());
    }
    if let Some(path) = &c.facts_config {
        relative_path(path)?;
        if !path.starts_with(".chrono-harness/")
            || path.starts_with(&c.artifact_directory)
            || [&c.context_path, &c.runner, &c.generator]
                .iter()
                .any(|p| overlap(path, p))
            || c.initial_inventory
                .as_ref()
                .is_some_and(|p| overlap(path, &p.path))
        {
            return Err("facts_config must name a separate host policy input".into());
        }
    }
    if let Some(initial) = &c.initial_inventory {
        relative_path(&initial.path)?;
        initial.profile.validate()?;
        if !initial.path.starts_with(".chrono-harness/ci/")
            || initial.path.starts_with(&c.artifact_directory)
            || [
                &c.check_config,
                &c.context_path,
                &c.runner,
                &c.generator,
                &initial.profile.host_config,
            ]
            .iter()
            .any(|path| overlap(&initial.path, path))
            || initial
                .profile
                .judges
                .iter()
                .any(|b| overlap(&initial.path, &b.executable))
        {
            return Err("initial inventory output must have separate CI-owned path".into());
        }
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
pub fn render(c: &Config, config_path: &str) -> Result<String, String> {
    let job = render_job(c, config_path, None, "chrono-check", false)?;
    let header = render_header(c, None, false, MARKER);
    Ok(format!("{header}jobs:\n  check:\n{job}"))
}

fn render_header(
    c: &Config,
    scope: Option<&chrono_harness::units::Scope>,
    read_actions: bool,
    marker: &str,
) -> String {
    let manual_dispatch = if matches!(scope, Some(chrono_harness::units::Scope::Collect { .. })) {
        ""
    } else {
        r#"  workflow_dispatch:
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
"#
    };
    let permissions = if read_actions {
        "  actions: read\n"
    } else {
        ""
    };
    format!(
        r#"{marker}name: {name}
on:
  push:
    branches: {push}
  pull_request:
    branches: {pr}
{manual_dispatch}permissions:
  contents: read
{permissions}"#,
        name = scalar(&c.name),
        push = serde_json::to_string(&c.push_branches).unwrap(),
        pr = serde_json::to_string(&c.pull_request_branches).unwrap(),
    )
}

fn render_job(
    c: &Config,
    config_path: &str,
    scope: Option<&chrono_harness::units::Scope>,
    artifact_prefix: &str,
    full_policy: bool,
) -> Result<String, String> {
    validate_policy(c, full_policy)?;
    relative_path(config_path)?;
    if c.initial_inventory
        .as_ref()
        .is_some_and(|p| overlap(&p.path, config_path))
    {
        return Err("initial inventory output collides with CI source".into());
    }
    let bootstrap = c
        .bootstrap
        .iter()
        .map(|v| shell(v))
        .collect::<Vec<_>>()
        .join(" ");
    let check = {
        let suffix = match scope {
            None => String::new(),
            Some(chrono_harness::units::Scope::Unit { unit }) => {
                format!(" '--unit' {}", shell(unit))
            }
            Some(chrono_harness::units::Scope::Collect { .. }) => " '--collect'".into(),
        };
        let credentials = if matches!(scope, Some(chrono_harness::units::Scope::Collect { .. })) {
            "          GH_TOKEN: ${{ github.token }}\n"
        } else {
            ""
        };
        format!(
            "      - name: Canonical harness check\n        shell: bash\n        env:\n          CHRONO_CHECK_SOURCE: ci\n          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}\n{credentials}        run: |\n          {} 'check'{suffix}\n",
            shell(&c.runner)
        )
    };
    let job_name = if scope.is_some() {
        format!("    name: {}\n", scalar(&c.name))
    } else {
        String::new()
    };
    let rendered = format!(
        r#"{job_name}    if: ${{{{ github.event.deleted != true }}}}
    runs-on: {runs_on}
    timeout-minutes: {timeout}
    steps:
      - name: Check out exact candidate
        uses: {checkout}
        with:
          ref: ${{{{ inputs.candidate || github.sha }}}}
          fetch-depth: 0
          persist-credentials: true
      - name: Bootstrap registered tools
        shell: bash
        run: |
          {bootstrap}
{check}      - name: Preserve actual check evidence
        id: chrono_evidence_upload
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: {artifact_prefix}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}
          path: {artifacts}
          if-no-files-found: error
{hidden_files}"#,
        runs_on = scalar(&c.runs_on),
        timeout = c.timeout_minutes,
        checkout = c.checkout_action,
        upload = c.upload_artifact_action,
        hidden_files = artifact_hidden_files(c.include_hidden_files),
        artifacts = scalar(&c.artifact_directory)
    );
    let rendered = if let Some(directory) = c
        .failure_artifact_directory
        .as_ref()
        .filter(|d| **d != c.artifact_directory)
    {
        format!(
            "{rendered}      - name: Preserve failed job diagnostics\n        if: ${{{{ failure() }}}}\n        uses: {}\n        with:\n          name: diagnostics-{artifact_prefix}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}\n          path: {}\n          if-no-files-found: error\n{}",
            c.upload_artifact_action,
            scalar(directory),
            artifact_hidden_files(c.include_hidden_files)
        )
    } else {
        rendered
    };
    let rendered = if c.retention_delivery {
        let selected = match scope {
            Some(chrono_harness::units::Scope::Unit { unit }) => unit.as_str(),
            Some(chrono_harness::units::Scope::Collect { .. }) => "collect",
            None => "all",
        };
        format!(
            "{rendered}      - name: Acknowledge delivered original evidence\n        if: ${{{{ always() && steps.chrono_evidence_upload.outcome == 'success' }}}}\n        shell: bash\n        env:\n          CHRONO_DELIVERED_ARTIFACT: ${{{{ steps.chrono_evidence_upload.outputs.artifact-id }}}}\n          CHRONO_DELIVERED_DIGEST: ${{{{ steps.chrono_evidence_upload.outputs.artifact-digest }}}}\n        run: |\n          {} retention --host-root . --delivered-check {} --scope {} --artifact-id-env CHRONO_DELIVERED_ARTIFACT --artifact-digest-env CHRONO_DELIVERED_DIGEST\n",
            shell(&c.runner),
            shell(&c.check_config),
            shell(selected)
        )
    } else {
        rendered.replace("        id: chrono_evidence_upload\n", "")
    };
    Ok(rendered)
}
fn output_preflight(root: &Path, path: &str, expected: &str, marker: &str) -> Result<bool, String> {
    if path.starts_with(".github/workflows/") && expected.len() > 500_000 {
        return Err(format!(
            "GitHub workflow exceeds 500 KB: {path} ({} bytes); split the registered workflow",
            expected.len()
        ));
    }
    let target = no_symlink_parents(root, path)?;
    match fs::read(&target) {
        Ok(bytes) => {
            let text = std::str::from_utf8(&bytes).map_err(|_| "owned output is not UTF-8")?;
            if !text.starts_with(marker) {
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
fn inventory_output(c: &Config) -> Result<Option<(&str, Vec<u8>)>, String> {
    c.initial_inventory
        .as_ref()
        .map(|p| {
            let mut bytes = serde_json::to_vec_pretty(&p.profile).map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            Ok((p.path.as_str(), bytes))
        })
        .transpose()
}
fn projection_plan(
    root: &Path,
    path: &str,
    projection: &Projection,
    verify: bool,
    adopting: bool,
    inputs: &mut generation::Inputs,
) -> Result<generation::Plan, String> {
    match projection {
        Projection::Units(c) => units::projection_plan(root, path, c, verify, inputs),
        Projection::Full(c) => full::projection_plan(root, path, c, verify),
        Projection::Release(c) => {
            if !path.starts_with(".chrono-harness/") {
                return Err("release configuration must belong to .chrono-harness".into());
            }
            release::projection_plan(root, c, verify, inputs)
        }
        Projection::Check(c) => {
            let mut plan = generation::Plan::default();
            plan.workflow(
                root,
                &c.workflow_path,
                render(c, path)?,
                MARKER,
                verify,
                "generated workflow drift",
            )?;
            if let Some((path, bytes)) = inventory_output(c)? {
                plan.inventory(root, path, bytes, adopting, verify)?;
            }
            Ok(plan)
        }
    }
}

pub fn generate(root: &Path, config_path: &str, verify: bool) -> Result<bool, String> {
    relative_path(config_path)?;
    generation::run(
        root,
        if verify { "verify" } else { "generate" },
        config_path,
        |root, inputs| {
            let projection = decode_projection(&inputs.read(config_path)?)?;
            projection_plan(root, config_path, &projection, verify, false, inputs)
        },
    )
}
pub fn init(root: &Path, input: &Path) -> Result<bool, String> {
    let requested = input.to_str().ok_or("generation config path UTF8")?;
    generation::run(root, "init", requested, |root, inputs| {
        let incoming = decode_projection(&inputs.external(input)?)?;
        let path = match &incoming {
            Projection::Check(_) => ".chrono-harness/ci/github.json",
            Projection::Units(_) => ".chrono-harness/ci/units.json",
            Projection::Full(_) => ".chrono-harness/ci/full.json",
            Projection::Release(_) => ".chrono-harness/ci/release.json",
        };
        let adopted = inputs.optional(path)?;
        let projection = if let Some(bytes) = &adopted {
            let existing = decode_projection(bytes)?;
            if std::mem::discriminant(&existing) != std::mem::discriminant(&incoming) {
                return Err("adopted CI source has a different projection kind".into());
            }
            existing
        } else {
            incoming
        };
        let mut plan = projection_plan(root, path, &projection, false, adopted.is_none(), inputs)?;
        if adopted.is_none() {
            let mut bytes = match &projection {
                Projection::Check(c) => serde_json::to_vec_pretty(c),
                Projection::Units(c) => serde_json::to_vec_pretty(c),
                Projection::Full(c) => serde_json::to_vec_pretty(c),
                Projection::Release(c) => serde_json::to_vec_pretty(c),
            }
            .map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            plan.adopt(path, bytes)?;
        }
        Ok(plan)
    })
}
pub fn prepare(
    root: &Path,
    c: &Config,
    event: &str,
    payload: &Value,
    workflow_revision: &str,
) -> Result<Value, String> {
    prepare_policy(root, c, event, payload, workflow_revision, false)
}
fn prepare_policy(
    root: &Path,
    c: &Config,
    event: &str,
    payload: &Value,
    workflow_revision: &str,
    full_policy: bool,
) -> Result<Value, String> {
    prepare_policy_fixed(
        root,
        c,
        event,
        payload,
        workflow_revision,
        full_policy,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn prepare_policy_fixed(
    root: &Path,
    c: &Config,
    event: &str,
    payload: &Value,
    workflow_revision: &str,
    full_policy: bool,
    detection: Option<&Value>,
) -> Result<Value, String> {
    validate_policy(c, full_policy)?;
    if !full_oid(workflow_revision) {
        return Err("workflow source revision must be full OID".into());
    }
    let event_candidate = match event {
        "push" => payload["after"].as_str(),
        "pull_request" => Some(workflow_revision),
        "workflow_dispatch" => payload["inputs"]["candidate"].as_str(),
        _ => None,
    };
    let facts = event_git::EventGit::new(root, c, event_candidate.unwrap_or(""))?;
    let result = (|| {
        let string = |v: &Value| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| "event missing string input".to_string())
        };
        let (initial, base, candidate, source) = if let Some(d) = detection {
            (
                d["initial"].as_bool().ok_or("detector initial missing")?,
                d["base"].as_str().map(str::to_owned),
                string(&d["candidate"])?,
                d["source"].as_str().ok_or("detector source missing")?,
            )
        } else {
            match event {
                "push" => {
                    if payload["deleted"].as_bool() == Some(true) {
                        return Err("branch deletion has no candidate".into());
                    }
                    let candidate = string(&payload["after"])?;
                    let before = string(&payload["before"])?;
                    if !full_oid(&candidate) {
                        return Err(format!("invalid full OID: {candidate}"));
                    }
                    let creation =
                        before.bytes().all(|b| b == b'0') && matches!(before.len(), 40 | 64);
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
                        facts.git(&["check-ref-format", reference])?;
                        Some(reference)
                    } else {
                        None
                    };
                    if let Some(rule) = c.push_baselines.iter().find(|rule| {
                        event_ref.is_some_and(|reference| reference.starts_with(&rule.ref_prefix))
                    }) {
                        (
                            false,
                            Some(facts.remote_baseline(&rule.base_ref)?),
                            candidate,
                            "push-configured-baseline-ref",
                        )
                    } else if creation {
                        let event_ref = event_ref.unwrap();
                        if event_ref == c.branch_creation_base_ref {
                            (true, None, candidate, "baseline-creation-initial-inventory")
                        } else {
                            let base = facts.remote_baseline(&c.branch_creation_base_ref)?;
                            (false, Some(base), candidate, "branch-creation-baseline-ref")
                        }
                    } else {
                        (false, Some(before), candidate, "push-before-after")
                    }
                }
                "pull_request" => {
                    let head = string(&payload["pull_request"]["head"]["sha"])?;
                    let base = facts.pull_request_baseline(workflow_revision, &head)?;
                    (
                        false,
                        Some(base),
                        workflow_revision.to_owned(),
                        "pr-workflow-merge-tree",
                    )
                }
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
            }
        };
        if !initial && base.is_none() {
            return Err("base required".into());
        }
        facts.require_commit(&candidate)?;
        if let Some(base) = &base {
            facts.require_commit(base)?
        }
        if facts.git(&["rev-parse", "HEAD"])?.trim() != candidate {
            return Err("checkout is not exact event candidate".into());
        }
        if initial && !facts.parents(&candidate)?.is_empty() {
            return Err(if source == "baseline-creation-initial-inventory" {
            "baseline-creation candidate has parents and no prior baseline; supply an explicit range via workflow_dispatch or check --base/--candidate"
        } else {
            "initial candidate has parents"
        }.into());
        }
        Ok(
            json!({"schema":"chrono-ci-inputs/v1","event":event,"source":source,"mode":if initial {"initial-inventory"} else {"delta"},"workflow_source_revision":workflow_revision,"base":base,"candidate":candidate,"event_head":if event == "pull_request" {string(&payload["pull_request"]["head"]["sha"])?} else {candidate.clone()},"initial":initial,"canonical_argv":vec![c.runner.clone(),"check".into()]}),
        )
    })();
    match result {
        Ok(mut context) => {
            facts.record(&mut context);
            Ok(context)
        }
        Err(error) => Err(facts.error(error)),
    }
}
pub fn dispatch(args: &[String]) -> Result<String, String> {
    if args.first().map(String::as_str) == Some("production") {
        return gating::production(args);
    }
    if args.first().map(String::as_str) == Some("detect") {
        return gating::dispatch(args);
    }
    if args.first().map(String::as_str) == Some("check-inputs") {
        return check_inputs::dispatch(args);
    }
    if args.first().is_some_and(|command| {
        matches!(
            command.as_str(),
            "generation-status" | "generation-resume" | "generation-maintain"
        )
    }) {
        return generation::dispatch(args);
    }
    if args == ["--version"] {
        return Ok(format!("chrono-ci {}\n", env!("CARGO_PKG_VERSION")));
    }
    let command = args
        .first()
        .ok_or("use init/generate/verify/migrate --host-root H --config P")?;
    let mut opts = std::collections::BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let k = &args[i];
        let v = args.get(i + 1).ok_or("missing option value")?;
        if !["--host-root", "--config", "--from", "--previous-config"].contains(&k.as_str())
            || opts.insert(k.as_str(), v.as_str()).is_some()
        {
            return Err(format!("unknown/duplicate option {k}"));
        }
        i += 2
    }
    let root = PathBuf::from(*opts.get("--host-root").ok_or("--host-root required")?);
    let config = *opts.get("--config").ok_or("--config required")?;
    if command != "migrate" && opts.len() != 2 {
        return Err("extra arguments".into());
    }
    match command.as_str() {
        "test-inventory" => inventory::execute(&root, config),
        "migrate" => {
            let previous = *opts.get("--from").ok_or("--from required")?;
            migrate::migrate(
                &root,
                previous,
                opts.get("--previous-config").copied().unwrap_or(previous),
                config,
            )
        }
        "init" => Ok(format!(
            "initialized; changed={}\n",
            init(&root, Path::new(config))?
        )),
        "generate" | "verify" => Ok(format!(
            "{}; changed={}\n",
            command,
            generate(&root, config, command == "verify")?
        )),
        _ => Err("unknown command".into()),
    }
}
