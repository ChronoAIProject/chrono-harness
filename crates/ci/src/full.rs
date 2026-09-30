//! Fixed full-governance context transport; the configured judges own admission.
use super::{action, full_oid, output_preflight, overlap, scalar, shell, write_file};
use chrono_harness::{
    canonical_argv, decode, facts::Reader, no_symlink_parents, relative_path, sha256,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};

pub const SCHEMA: &str = "chrono-github-full-ci/v1";
pub const SHORT_SCHEMA: &str = "chrono-github-full-ci/v2";
const MARKER: &str = "# chrono-ci: owned github-full-ci/v1\n";
const ADOPTED: &str = ".chrono-harness/ci/full.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub workflow_path: String,
    pub name: String,
    pub runs_on: String,
    pub checkout_action: String,
    pub upload_artifact_action: String,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub runner: String,
    pub generator: String,
    pub check_config: String,
    pub context_path: String,
    pub preparation_path: String,
    pub artifact_directory: String,
}

fn literal(value: &str) -> Result<(), String> {
    if value.contains(['\0', '\r', '\n']) || value.contains("${{") {
        return Err("full CI settings must be literal single-line values".into());
    }
    Ok(())
}

pub(crate) fn validate(c: &Config) -> Result<(), String> {
    if !matches!(c.schema.as_str(), SCHEMA | SHORT_SCHEMA)
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
        &c.generator,
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
    if !c.workflow_path.starts_with(".github/workflows/")
        || !c.workflow_path.ends_with(".yml")
        || !c.check_config.starts_with(".chrono-harness/")
        || c.check_config.starts_with(".chrono-harness/state/")
        || !c.artifact_directory.starts_with(".chrono-harness/state/")
        || !c.artifact_directory.ends_with('/')
        || c.artifact_directory.contains(['*', '?', '[', ']', '!'])
        || !c.context_path.starts_with(&c.artifact_directory)
        || !c.preparation_path.starts_with(&c.artifact_directory)
        || [&c.workflow_path, &c.runner, &c.generator, &c.check_config]
            .iter()
            .any(|p| p.starts_with(&c.artifact_directory))
    {
        return Err("invalid full CI policy/artifact ownership".into());
    }
    for name in ["prepare.stdout.json", "prepare.stderr"] {
        let output = format!("{}{name}", c.artifact_directory);
        if [&c.context_path, &c.preparation_path]
            .iter()
            .any(|p| overlap(p, &output))
        {
            return Err("full CI context/report overlaps preparation process output".into());
        }
    }
    relative_path(c.artifact_directory.trim_end_matches('/'))?;
    action(&c.checkout_action, "actions/checkout")?;
    action(&c.upload_artifact_action, "actions/upload-artifact")?;
    Ok(())
}

fn source_path(c: &Config, path: &str) -> Result<(), String> {
    relative_path(path)?;
    literal(path)?;
    if !path.starts_with(".chrono-harness/")
        || path.starts_with(".chrono-harness/state/")
        || [
            &c.workflow_path,
            &c.runner,
            &c.generator,
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

pub fn argv(c: &Config, base: &str, candidate: &str) -> Vec<String> {
    if c.schema == SHORT_SCHEMA {
        return vec![c.runner.clone(), "check".into()];
    }
    let mut args = canonical_argv(&c.runner, &c.check_config, Some(base), candidate, false);
    args.extend(["--context".into(), c.context_path.clone()]);
    args
}

pub fn render(c: &Config, config_path: &str) -> Result<String, String> {
    validate(c)?;
    source_path(c, config_path)?;
    let args = argv(c, "$CHRONO_BASE", "$CHRONO_CANDIDATE");
    let check = args
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i > 0 && matches!(args[i - 1].as_str(), "--base" | "--candidate") {
                format!("\"{v}\"")
            } else {
                shell(v)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let prepare = format!(
        "{} 'prepare' '--host-root' '.' '--config' {} '--event' \"$GITHUB_EVENT_NAME\" '--payload' \"$GITHUB_EVENT_PATH\" '--workflow-revision' \"$CHRONO_WORKFLOW_REVISION\" '--github-output' \"$GITHUB_OUTPUT\"",
        shell(&c.generator),
        shell(config_path)
    );
    let rendered = format!(
        r#"{MARKER}name: {name}
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
      - name: Preserve fixed full context
        id: inputs
        shell: bash
        env:
          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}
        run: |
          mkdir -p -- {artifact_dir}
          set -o noclobber
          {prepare} > {prepare_stdout} 2> {prepare_stderr}
      - name: Canonical full harness check
        shell: bash
        env:
          CHRONO_BASE: ${{{{ steps.inputs.outputs.base }}}}
          CHRONO_CANDIDATE: ${{{{ steps.inputs.outputs.candidate }}}}
        run: |
          {check}
      - name: Preserve original full check evidence
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: chrono-full-check-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}
          path: {artifacts}
          if-no-files-found: error
"#,
        name = scalar(&c.name),
        runner = scalar(&c.runs_on),
        timeout = c.timeout_minutes,
        checkout = c.checkout_action,
        upload = c.upload_artifact_action,
        bootstrap = c
            .bootstrap
            .iter()
            .map(|v| shell(v))
            .collect::<Vec<_>>()
            .join(" "),
        artifacts = scalar(&c.artifact_directory),
        artifact_dir = shell(&c.artifact_directory),
        prepare_stdout = shell(&format!("{}prepare.stdout.json", c.artifact_directory)),
        prepare_stderr = shell(&format!("{}prepare.stderr", c.artifact_directory))
    );
    if c.schema == SHORT_SCHEMA {
        let start = rendered
            .find("      - name: Preserve fixed full context")
            .ok_or("full prepare section")?;
        let end = rendered
            .find("      - name: Preserve original full check evidence")
            .ok_or("full artifact section")?;
        let check = format!(
            "      - name: Canonical full harness check\n        shell: bash\n        env:\n          CHRONO_CHECK_SOURCE: ci\n          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}\n        run: |\n          {} 'check'\n",
            shell(&c.runner)
        );
        Ok(format!(
            "{}{}{}",
            &rendered[..start],
            check,
            &rendered[end..]
        ))
    } else {
        Ok(rendered)
    }
}

pub(crate) fn generate(root: &Path, path: &str, c: &Config, verify: bool) -> Result<bool, String> {
    let output = render(c, path)?;
    let same = output_preflight(root, &c.workflow_path, &output, MARKER)?;
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
    let same = output_preflight(root, &c.workflow_path, &rendered, MARKER)?;
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

fn matching_output(root: &Path, path: &str, bytes: &[u8], replace: bool) -> Result<bool, String> {
    match fs::read(no_symlink_parents(root, path)?) {
        Ok(current) if current == bytes => Ok(true),
        Ok(_) if replace => Ok(false),
        Ok(_) => Err(format!("full CI output collision: {path}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
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
    let ctx: Value = decode(context)?;
    if ctx["schema_version"] != 2
        || !matches!(ctx["run_kind"].as_str(), Some("integration" | "delivery"))
    {
        return Err("full CI requires explicit context v2 and integration/delivery role".into());
    }
    let base = ctx["base"]
        .as_str()
        .filter(|s| full_oid(s))
        .ok_or("full context base OID required")?;
    let candidate = ctx["candidate"]
        .as_str()
        .filter(|s| full_oid(s))
        .ok_or("full context candidate OID required")?;
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
            "canonical_argv":argv(c,base,candidate),"governance":"not-evaluated","parity":"unestablished"});
        Ok(report)
    })();
    let mut report: Value = result.map_err(|message: String| {
        format!(
            "E_FULL_CI_GIT: {}",
            json!({"message":message,"git_facts":facts.observation()})
        )
    })?;
    report["git_facts"] = facts.observation();
    if fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())? != source {
        return Err("full CI source changed during preparation".into());
    }
    let report_bytes =
        (serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n").into_bytes();
    let replace = c.schema == SHORT_SCHEMA;
    let context_same = matching_output(root, &c.context_path, context, replace)?;
    let report_same = matching_output(root, &c.preparation_path, &report_bytes, replace)?;
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
