//! Fixed full-governance context transport; the configured judges own admission.
use super::{action, full_oid, output_preflight, scalar, shell, write_file};
use chrono_harness::{
    decode,
    facts::Reader,
    file_identity, no_symlink_parents, relative_path, sha256,
    units::{self, Scope},
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};

pub const SCHEMA: &str = "chrono-github-full-ci/v2";
const MARKER: &str = "# chrono-ci: owned github-full-ci/v1\n";
const ADOPTED: &str = ".chrono-harness/ci/full.json";

fn marker(path: &str) -> String {
    format!("{MARKER}# chrono-ci: full source {}\n", scalar(path))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub workflow_path: String,
    pub name: String,
    pub runs_on: String,
    pub checkout_action: String,
    pub upload_artifact_action: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_hidden_files: bool,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub runner: String,
    pub check_config: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "scope_present"
    )]
    pub scope: Option<Scope>,
    pub context_path: String,
    pub preparation_path: String,
    pub artifact_directory: String,
}

fn scope_present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Scope>, D::Error> {
    Scope::deserialize(d).map(Some)
}

fn literal(value: &str) -> Result<(), String> {
    if value.contains(['\0', '\r', '\n']) || value.contains("${{") {
        return Err("full CI settings must be literal single-line values".into());
    }
    Ok(())
}

fn overlap(a: &str, b: &str) -> bool {
    // Match the filesystem components consumed by no_symlink_parents/write_file
    // without changing any declared path bytes used in the source or argv.
    let (a, b) = (Path::new(a), Path::new(b));
    a.starts_with(b) || b.starts_with(a)
}

pub(crate) fn validate(c: &Config) -> Result<(), String> {
    if c.schema != SCHEMA
        || c.name.is_empty()
        || c.runs_on.is_empty()
        || c.timeout_minutes == 0
        || c.bootstrap.first().is_none_or(String::is_empty)
    {
        return Err("invalid full CI schema/name/runner/bootstrap/bounds".into());
    }
    for value in [&c.name, &c.runs_on, &c.artifact_directory]
        .into_iter()
        .chain(c.bootstrap.iter())
    {
        literal(value)?;
    }
    let files = [
        &c.workflow_path,
        &c.runner,
        &c.check_config,
        &c.context_path,
        &c.preparation_path,
    ];
    for (i, path) in files.iter().enumerate() {
        relative_path(path)?;
        literal(path)?;
        if files[..i].iter().any(|other| overlap(path, other)) {
            return Err("full CI paths must have distinct ownership".into());
        }
    }
    relative_path(c.artifact_directory.trim_end_matches('/'))?;
    let policy = Path::new(".chrono-harness");
    let state = policy.join("state");
    let artifact = Path::new(&c.artifact_directory);
    let check = Path::new(&c.check_config);
    if !Path::new(&c.workflow_path).starts_with(".github/workflows")
        || !c.workflow_path.ends_with(".yml")
        || !check.starts_with(policy)
        || check == policy
        || check.starts_with(&state)
        || !artifact.starts_with(&state)
        || !c.artifact_directory.ends_with('/')
        || c.artifact_directory.contains(['*', '?', '[', ']', '!'])
        || [&c.context_path, &c.preparation_path]
            .iter()
            .any(|p| !Path::new(p).starts_with(artifact) || Path::new(p) == artifact)
        || [&c.workflow_path, &c.runner, &c.check_config]
            .iter()
            .any(|p| Path::new(p).starts_with(artifact))
    {
        return Err("invalid full CI policy/artifact ownership".into());
    }
    match &c.scope {
        Some(Scope::Unit { unit }) => units::id(unit)?,
        Some(Scope::Collect { manifest }) => {
            units::artifact_path(manifest)?;
            literal(manifest)?;
            if artifact.starts_with(manifest) || files.iter().any(|p| overlap(manifest, p)) {
                return Err("full CI collection manifest overlaps owned input/output paths".into());
            }
        }
        None => {}
    }
    action(&c.checkout_action, "actions/checkout")?;
    action(&c.upload_artifact_action, "actions/upload-artifact")?;
    Ok(())
}

fn source_path(c: &Config, path: &str) -> Result<(), String> {
    relative_path(path)?;
    literal(path)?;
    let source = Path::new(path);
    let policy = Path::new(".chrono-harness");
    if !source.starts_with(policy)
        || source == policy
        || source.starts_with(policy.join("state"))
        || [
            &c.workflow_path,
            &c.runner,
            &c.check_config,
            &c.context_path,
            &c.preparation_path,
        ]
        .iter()
        .any(|p| overlap(path, p))
    {
        return Err("full CI source must be a separate host policy input".into());
    }
    Ok(())
}

pub fn argv(c: &Config) -> Vec<String> {
    let mut args = vec![c.runner.clone(), "check".into()];
    if let Some(scope) = &c.scope {
        args.extend(scope.short_argv());
    }
    args
}

pub fn render(c: &Config, config_path: &str) -> Result<String, String> {
    validate(c)?;
    source_path(c, config_path)?;
    let check = argv(c)
        .iter()
        .map(|arg| shell(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let marker = marker(config_path);
    Ok(format!(
        r#"{marker}name: {name}
on:
  workflow_dispatch:
    inputs:
      context:
        description: Exact full context JSON; provision its referenced evidence through the registered bootstrap
        required: true
        type: string
permissions:
  contents: read
jobs:
  check:
    runs-on: {runner}
    timeout-minutes: {timeout}
    steps:
      - name: Check out supplied candidate
        uses: {checkout}
        with:
          ref: ${{{{ fromJSON(inputs.context).candidate }}}}
          fetch-depth: 0
          persist-credentials: true
      - name: Bootstrap registered tools and evidence
        shell: bash
        run: |
          {bootstrap}
      - name: Canonical full harness check
        shell: bash
        env:
          CHRONO_CHECK_SOURCE: ci
          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}
        run: |
          {check}
      - name: Preserve original full check evidence
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: chrono-full-check-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}
          path: {artifacts}
          if-no-files-found: error
{hidden_files}"#,
        name = scalar(&c.name),
        runner = scalar(&c.runs_on),
        timeout = c.timeout_minutes,
        checkout = c.checkout_action,
        upload = c.upload_artifact_action,
        hidden_files = super::artifact_hidden_files(c.include_hidden_files),
        bootstrap = c
            .bootstrap
            .iter()
            .map(|v| shell(v))
            .collect::<Vec<_>>()
            .join(" "),
        artifacts = scalar(&c.artifact_directory),
    ))
}

fn scope_preflight(root: &Path, c: &Config) -> Result<(), String> {
    if let Some(Scope::Collect { manifest }) = &c.scope {
        // Evidence provisioning and admission remain caller/bootstrap and full-core work.
        no_symlink_parents(root, manifest)?;
    }
    Ok(())
}

pub(crate) fn generate(root: &Path, path: &str, c: &Config, verify: bool) -> Result<bool, String> {
    let output = render(c, path)?;
    scope_preflight(root, c)?;
    let same = output_preflight(root, &c.workflow_path, &output, &marker(path))?;
    if verify && !same {
        return Err(format!(
            "generated full CI workflow drift: {}",
            c.workflow_path
        ));
    }
    if !verify && !same {
        write_file(root, &c.workflow_path, output.as_bytes())?;
    }
    Ok(!same)
}

pub(crate) fn init(root: &Path, incoming: Config) -> Result<bool, String> {
    let target = no_symlink_parents(root, ADOPTED)?;
    let existing = target.exists();
    let c = if existing {
        decode(&fs::read(target).map_err(|e| e.to_string())?)?
    } else {
        incoming
    };
    let rendered = render(&c, ADOPTED)?;
    scope_preflight(root, &c)?;
    let same = output_preflight(root, &c.workflow_path, &rendered, &marker(ADOPTED))?;
    if !existing {
        write_file(
            root,
            ADOPTED,
            &(serde_json::to_string_pretty(&c).map_err(|e| e.to_string())? + "\n").into_bytes(),
        )?;
    }
    if !same {
        write_file(root, &c.workflow_path, rendered.as_bytes())?;
    }
    Ok(!existing || !same)
}

fn matching_output(root: &Path, path: &str, bytes: &[u8]) -> Result<bool, String> {
    match fs::read(no_symlink_parents(root, path)?) {
        Ok(current) if current == bytes => Ok(true),
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// The dispatch and automatic units consumers share exact full context parsing.
pub(crate) fn context_input(bytes: &[u8]) -> Result<Value, String> {
    let ctx: Value = decode(bytes)?;
    if ctx["schema_version"] != 2
        || !matches!(ctx["run_kind"].as_str(), Some("integration" | "delivery"))
    {
        return Err("full CI requires explicit context v2 and integration/delivery role".into());
    }
    for key in ["base", "candidate"] {
        if !ctx[key].as_str().is_some_and(full_oid) {
            return Err(format!("full context {key} OID required"));
        }
    }
    Ok(ctx)
}
pub(crate) fn executable_pins(root: &Path, config: &str, runner: &str) -> Result<Value, String> {
    let cfg = chrono_harness::load_selected_config(root, config)?;
    let path = cfg["registries"]["judges"]
        .as_str()
        .ok_or("full judges registry")?;
    let bindings: Vec<chrono_harness::wire::Binding> = decode::<Value>(
        &fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?,
    )?["judges"]
        .clone()
        .as_array()
        .ok_or("full judges")?
        .iter()
        .map(|b| serde_json::from_value(b.clone()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    chrono_harness::full::schedule(&bindings)?;
    let ids: std::collections::BTreeSet<_> = bindings.iter().map(|b| b.id.as_str()).collect();
    if ids
        != [
            "registration",
            "filemap",
            "routes",
            "projects",
            "cost",
            "mixed",
            "workflow",
        ]
        .into_iter()
        .collect()
    {
        return Err("full provider requires all seven judge bindings".into());
    }
    for binding in &bindings {
        if binding.sha256.as_deref()
            != Some(
                file_identity(&no_symlink_parents(root, &binding.executable)?)?
                    .0
                    .as_str(),
            )
        {
            return Err("full judge executable binding mismatch".into());
        }
    }
    let pins: Vec<_> = bindings
        .iter()
        .map(|b| json!({"id":b.id,"sha256":b.sha256}))
        .collect();
    Ok(
        json!({"runner_sha256":file_identity(&no_symlink_parents(root,runner)?)?.0,"judge_sha256":chrono_harness::wire::digest(&pins)?,"judges":pins}),
    )
}
/// Preserve caller context bytes separately from transport observations. Does not judge its policy.
pub fn prepare(
    root: &Path,
    path: &str,
    c: &Config,
    context: &[u8],
    workflow_revision: &str,
) -> Result<Value, String> {
    validate(c)?;
    source_path(c, path)?;
    scope_preflight(root, c)?;
    let ctx = context_input(context)?;
    let base = ctx["base"].as_str().unwrap();
    let candidate = ctx["candidate"].as_str().unwrap();
    if !full_oid(workflow_revision) {
        return Err("workflow source revision must be full OID".into());
    }
    let source = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    if serde_json::to_value(c).map_err(|e| e.to_string())? != decode::<Value>(&source)? {
        return Err("full CI source differs from selected configuration".into());
    }
    let facts = Reader::for_config_observed(root, &c.check_config).map_err(|e| {
        format!(
            "E_FULL_CI_GIT: {}",
            json!({"message":e.message,"git_facts":e.observation})
        )
    })?;
    if !facts.is_bound() {
        return Err("full CI requires candidate full config v3 Git binding".into());
    }
    let result = (|| {
        facts.verify_config(root, candidate)?;
        if facts.git(root, &["rev-parse", "HEAD"])? != format!("{candidate}\n").as_bytes() {
            return Err("full CI checkout is not supplied candidate".into());
        }
        if facts.blob(root, candidate, path)? != source {
            return Err("full CI source differs from fixed candidate".into());
        }
        // Acquisition never replaces or derives caller context fields.
        for oid in [base, workflow_revision] {
            if !facts.commit_available(root, oid)? {
                facts.git(root, &["fetch", "--no-tags", "origin", oid])?;
            }
            if !facts.commit_available(root, oid)? {
                return Err("full CI fixed commit remains missing".into());
            }
        }
        if facts.blob(root, workflow_revision, &c.workflow_path)? != render(c, path)?.as_bytes() {
            return Err("full CI workflow revision differs from declared projection".into());
        }
        let report = json!({"schema":"chrono-full-ci-inputs/v1","status":"prepared",
            "workflow_source_revision":workflow_revision,"config_path":path,"config_sha256":sha256(&source),
            "base":base,"candidate":candidate,"context":{"path":c.context_path,"sha256":sha256(context),"input_bytes":context},
            "canonical_argv":argv(c),"governance":"not-evaluated","parity":"unestablished"});
        Ok(report)
    })();
    let mut report: Value = result.map_err(|message: String| {
        format!(
            "E_FULL_CI_GIT: {}",
            json!({"message":message,"git_facts":facts.observation()})
        )
    })?;
    let original = chrono_harness::prepared::retain_original(
        root,
        &format!("{}preparation/", c.artifact_directory),
        "native-context",
        context,
    )?;
    report["context"]["current_path"] = json!(c.context_path);
    report["context"]["path"] = json!(original.path);
    report["originals"] = json!([original]);
    report["git_facts"] = facts.observation();
    if fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())? != source {
        return Err("full CI source changed during preparation".into());
    }
    let report_bytes =
        (serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n").into_bytes();
    let context_same = matching_output(root, &c.context_path, context)?;
    let report_same = matching_output(root, &c.preparation_path, &report_bytes)?;
    if !context_same {
        write_file(root, &c.context_path, context)?;
    }
    if !report_same {
        write_file(root, &c.preparation_path, &report_bytes)?;
    }
    Ok(report)
}

pub(crate) fn prepare_dispatch(
    root: &Path,
    path: &str,
    c: &Config,
    event: &str,
    payload: &Value,
    workflow_revision: &str,
    github_output: Option<&str>,
) -> Result<String, String> {
    if event != "workflow_dispatch" {
        return Err("full CI requires an explicit context dispatch".into());
    }
    let context = payload["inputs"]["context"]
        .as_str()
        .ok_or("dispatch requires context JSON string")?;
    let report = prepare(root, path, c, context.as_bytes(), workflow_revision)?;
    if let Some(output) = github_output {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(output)
            .map_err(|e| e.to_string())?;
        writeln!(
            file,
            "base={}\ncandidate={}",
            report["base"].as_str().unwrap(),
            report["candidate"].as_str().unwrap()
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(serde_json::to_string(&report).map_err(|e| e.to_string())? + "\n")
}
