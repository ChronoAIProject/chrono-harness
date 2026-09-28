//! Explicit native-release workflow projection; commands remain host-owned.
use super::{action, output_preflight, scalar, shell, write_file};
use chrono_harness::{decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

pub const SCHEMA: &str = "chrono-github-release/v1";
const MARKER: &str = "# chrono-ci: owned github-release/v1\n";
const ADOPTED: &str = ".chrono-harness/ci/release.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub workflow_path: String,
    pub name: String,
    pub push_branches: Vec<String>,
    pub checkout_action: String,
    pub upload_artifact_action: String,
    pub jobs: Vec<Job>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub id: String,
    pub runs_on: String,
    pub timeout_minutes: u32,
    pub command: Vec<String>,
    pub artifact_name: String,
    pub artifact_directory: String,
}

fn literal(value: &str) -> Result<(), String> {
    if value.contains(['\0', '\r', '\n']) || value.contains("${{") {
        return Err("release settings must be literal single-line values".into());
    }
    Ok(())
}

pub(crate) fn validate(c: &Config) -> Result<(), String> {
    if c.schema != SCHEMA || c.name.is_empty() || c.jobs.is_empty() {
        return Err("invalid release workflow schema/name/jobs".into());
    }
    relative_path(&c.workflow_path)?;
    literal(&c.workflow_path)?;
    literal(&c.name)?;
    if !c.workflow_path.starts_with(".github/workflows/") || !c.workflow_path.ends_with(".yml") {
        return Err("release workflow must be .github/workflows/*.yml".into());
    }
    action(&c.checkout_action, "actions/checkout")?;
    action(&c.upload_artifact_action, "actions/upload-artifact")?;
    let mut branches = BTreeSet::new();
    for branch in &c.push_branches {
        literal(branch)?;
        if branch.is_empty() || !branches.insert(branch) {
            return Err("release push branches must be nonempty and unique".into());
        }
    }
    let mut jobs = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    for job in &c.jobs {
        if !job
            .id
            .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            || !job
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
            || !jobs.insert(&job.id)
            || job.runs_on.is_empty()
            || job.timeout_minutes == 0
            || job.command.first().is_none_or(String::is_empty)
        {
            return Err("invalid/duplicate release job identity or command".into());
        }
        for value in [&job.runs_on, &job.artifact_name, &job.artifact_directory]
            .into_iter()
            .chain(job.command.iter())
        {
            literal(value)?;
        }
        if job.artifact_name.is_empty()
            || job
                .artifact_name
                .contains(['"', ':', '<', '>', '|', '*', '?', '/', '\\'])
            || !artifacts.insert(&job.artifact_name)
        {
            return Err("invalid/duplicate release artifact name".into());
        }
        if !job.artifact_directory.ends_with('/')
            || job.artifact_directory.contains(['*', '?', '[', ']', '!'])
        {
            return Err("release artifact must be a literal directory ending /".into());
        }
        relative_path(job.artifact_directory.trim_end_matches('/'))?;
        if c.workflow_path.starts_with(&job.artifact_directory) {
            return Err("release artifact directory contains its workflow".into());
        }
    }
    Ok(())
}

pub fn render(c: &Config) -> Result<String, String> {
    validate(c)?;
    let mut output = format!("{MARKER}name: {}\non:\n", scalar(&c.name));
    if !c.push_branches.is_empty() {
        output.push_str(&format!(
            "  push:\n    branches: {}\n",
            serde_json::to_string(&c.push_branches).map_err(|e| e.to_string())?
        ));
    }
    output.push_str(
        "  workflow_dispatch:\n    inputs:\n      source:\n        description: Source revision to check out\n        required: true\n        type: string\npermissions:\n  contents: read\njobs:\n",
    );
    for job in &c.jobs {
        output.push_str(&format!(
            r#"  {id}:
    if: ${{{{ github.event.deleted != true }}}}
    runs-on: {runner}
    timeout-minutes: {timeout}
    steps:
      - name: Check out requested release source
        uses: {checkout}
        with:
          ref: ${{{{ inputs.source || github.sha }}}}
          fetch-depth: 0
      - name: Run registered release command
        shell: bash
        run: |
          {command}
      - name: Preserve original release artifacts
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: {artifact}
          path: {directory}
          if-no-files-found: error
"#,
            id = job.id,
            runner = scalar(&job.runs_on),
            timeout = job.timeout_minutes,
            checkout = c.checkout_action,
            command = job
                .command
                .iter()
                .map(|v| shell(v))
                .collect::<Vec<_>>()
                .join(" "),
            upload = c.upload_artifact_action,
            artifact = scalar(&job.artifact_name),
            directory = scalar(&job.artifact_directory),
        ));
    }
    Ok(output)
}

pub(crate) fn generate(root: &Path, c: &Config, verify: bool) -> Result<bool, String> {
    let output = render(c)?;
    let same = output_preflight(root, &c.workflow_path, &output, MARKER)?;
    if verify && !same {
        return Err(format!(
            "generated release workflow drift: {}",
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
        let c = decode(&fs::read(target).map_err(|e| e.to_string())?)?;
        validate(&c)?;
        c
    } else {
        incoming
    };
    let output = render(&c)?;
    let same = output_preflight(root, &c.workflow_path, &output, MARKER)?;
    if !existing {
        write_file(
            root,
            ADOPTED,
            &(serde_json::to_string_pretty(&c).map_err(|e| e.to_string())? + "\n").into_bytes(),
        )?;
    }
    if !same {
        write_file(root, &c.workflow_path, output.as_bytes())?;
    }
    Ok(!existing || !same)
}
