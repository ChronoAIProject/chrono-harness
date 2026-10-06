//! Explicit native-release workflow projection; commands remain host-owned.
use super::{action, output_preflight, scalar, shell, write_file};
use chrono_harness::{decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

pub const SCHEMA: &str = "chrono-github-release/v1";
pub const UNITS_SCHEMA: &str = "chrono-github-release/v2";
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_artifact_action: Option<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downloads: Vec<Download>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub always: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_metadata: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Download {
    pub job: String,
    pub directory: String,
}

pub(crate) fn decode_config(bytes: &[u8]) -> Result<Config, String> {
    let value: Value = decode(bytes)?;
    if value["schema"] == SCHEMA
        && (value.get("download_artifact_action").is_some()
            || value["jobs"].as_array().is_some_and(|jobs| {
                jobs.iter().any(|job| {
                    ["needs", "downloads", "always", "dependency_metadata"]
                        .iter()
                        .any(|key| job.get(key).is_some())
                })
            }))
    {
        return Err("release dependency fields require v2".into());
    }
    let c = decode(bytes)?;
    validate(&c)?;
    Ok(c)
}

fn literal(value: &str) -> Result<(), String> {
    if value.contains(['\0', '\r', '\n']) || value.contains("${{") {
        return Err("release settings must be literal single-line values".into());
    }
    Ok(())
}

pub(crate) fn validate(c: &Config) -> Result<(), String> {
    if !matches!(c.schema.as_str(), SCHEMA | UNITS_SCHEMA) || c.name.is_empty() || c.jobs.is_empty()
    {
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
    if c.schema == UNITS_SCHEMA {
        action(
            c.download_artifact_action
                .as_deref()
                .ok_or("release v2 needs pinned download action")?,
            "actions/download-artifact",
        )?;
    } else if c.download_artifact_action.is_some()
        || c.jobs.iter().any(|j| {
            !j.needs.is_empty()
                || !j.downloads.is_empty()
                || j.always
                || j.dependency_metadata.is_some()
        })
    {
        return Err("release dependency fields require v2".into());
    }
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
    for job in &c.jobs {
        let mut needs = BTreeSet::new();
        for need in &job.needs {
            if need == &job.id || !jobs.contains(need) || !needs.insert(need) {
                return Err("invalid/duplicate release dependency".into());
            }
        }
        let mut downloads = BTreeSet::new();
        let mut directories: Vec<&str> = Vec::new();
        for download in &job.downloads {
            literal(&download.directory)?;
            if !needs.contains(&download.job)
                || !downloads.insert(&download.job)
                || !download.directory.ends_with('/')
                || download.directory.contains(['*', '?', '[', ']', '!'])
            {
                return Err(
                    "release downloads need unique declared dependencies and literal directories"
                        .into(),
                );
            }
            let dir = download.directory.trim_end_matches('/');
            relative_path(dir)?;
            if super::overlap(dir, job.artifact_directory.trim_end_matches('/'))
                || directories.iter().any(|old| super::overlap(dir, old))
                || c.workflow_path.starts_with(&download.directory)
            {
                return Err("overlapping release download directory".into());
            }
            directories.push(dir);
        }
        if let Some(metadata) = &job.dependency_metadata {
            literal(metadata)?;
            relative_path(metadata)?;
            if directories.iter().any(|dir| super::overlap(metadata, dir))
                || super::overlap(metadata, job.artifact_directory.trim_end_matches('/'))
                || metadata == &c.workflow_path
            {
                return Err("overlapping release dependency metadata".into());
            }
        }
    }
    // Validate the explicit DAG without deriving any edges from commands or paths.
    let mut done = BTreeSet::new();
    loop {
        let previous = done.len();
        for job in &c.jobs {
            if job.needs.iter().all(|n| done.contains(n)) {
                done.insert(job.id.clone());
            }
        }
        if done.len() == c.jobs.len() {
            break;
        }
        if previous == done.len() {
            return Err("release dependency cycle".into());
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
        if c.schema == UNITS_SCHEMA {
            render_unit(&mut output, c, job)?;
            continue;
        }
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
          include-hidden-files: true
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

fn render_unit(output: &mut String, c: &Config, job: &Job) -> Result<(), String> {
    output.push_str(&format!(
        "  {}:\n    if: ${{{{ {}github.event.deleted != true }}}}\n",
        job.id,
        if job.always { "always() && " } else { "" }
    ));
    if !job.needs.is_empty() {
        output.push_str(&format!(
            "    needs: {}\n",
            serde_json::to_string(&job.needs).map_err(|e| e.to_string())?
        ));
    }
    output.push_str(&format!(
        r#"    runs-on: {runner}
    timeout-minutes: {timeout}
    outputs:
      artifact_id: ${{{{ steps.release_upload.outputs.artifact-id }}}}
      run_id: ${{{{ steps.release_identity.outputs.run_id }}}}
      attempt: ${{{{ steps.release_identity.outputs.attempt }}}}
    steps:
      - name: Check out requested release source
        uses: {checkout}
        with:
          ref: ${{{{ inputs.source || github.sha }}}}
          fetch-depth: 0
      - name: Bind original producing attempt
        id: release_identity
        if: ${{{{ always() }}}}
        shell: bash
        env:
          RELEASE_RUN: ${{{{ github.run_id }}}}
          RELEASE_ATTEMPT: ${{{{ github.run_attempt }}}}
        run: |
          printf 'run_id=%s\nattempt=%s\n' "$RELEASE_RUN" "$RELEASE_ATTEMPT" >> "$GITHUB_OUTPUT"
"#,
        runner = scalar(&job.runs_on),
        timeout = job.timeout_minutes,
        checkout = c.checkout_action
    ));
    for download in &job.downloads {
        output.push_str(&format!(
            r#"      - name: Download selected original artifact from {producer}
        if: ${{{{ always() && needs.{producer}.outputs.artifact_id != '' }}}}
        uses: {action}
        with:
          artifact-ids: ${{{{ needs.{producer}.outputs.artifact_id }}}}
          merge-multiple: true
          path: {directory}
"#,
            producer = download.job,
            action = c.download_artifact_action.as_deref().unwrap(),
            directory = scalar(&download.directory)
        ));
    }
    output.push_str(&format!(
        r#"      - name: Run registered release command
        if: ${{{{ always() }}}}
        shell: bash
        env:
          CHRONO_RELEASE_DEPENDENCIES: ${{{{ toJson(needs) }}}}
          CHRONO_RELEASE_RUN: ${{{{ github.run_id }}}}
          CHRONO_RELEASE_ATTEMPT: ${{{{ github.run_attempt }}}}
          CHRONO_RELEASE_JOB: {job}
        run: |
"#,
        job = scalar(&job.id)
    ));
    if let Some(metadata) = &job.dependency_metadata {
        let parent = Path::new(metadata)
            .parent()
            .unwrap()
            .to_str()
            .ok_or("metadata path encoding")?;
        output.push_str(&format!("          mkdir -p {}\n          printf '%s\\n' \"$CHRONO_RELEASE_DEPENDENCIES\" > {}\n",shell(if parent.is_empty(){"."}else{parent}),shell(metadata)));
    }
    let command = job
        .command
        .iter()
        .map(|v| shell(v))
        .collect::<Vec<_>>()
        .join(" ");
    let artifact = scalar(&format!(
        "{}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}",
        job.artifact_name
    ));
    output.push_str(&format!(
        r#"          {command}
      - name: Preserve original release artifacts
        id: release_upload
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: {artifact}
          path: {directory}
          if-no-files-found: error
          include-hidden-files: true
"#,
        upload = c.upload_artifact_action,
        directory = scalar(&job.artifact_directory)
    ));
    Ok(())
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
        decode_config(&fs::read(target).map_err(|e| e.to_string())?)?
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
