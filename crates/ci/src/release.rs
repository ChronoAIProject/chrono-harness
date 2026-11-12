//! Explicit native-release workflow projection; commands remain host-owned.
use super::{action, output_preflight, scalar, shell, write_file};
use chrono_harness::{decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub const SCHEMA: &str = "chrono-github-release/v1";
pub const UNITS_SCHEMA: &str = "chrono-github-release/v2";
const MARKER: &str = "# chrono-ci: owned github-release/v1\n";
const ADOPTED: &str = ".chrono-harness/ci/release.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistent_cache: Option<super::cache::Config>,
    pub schema: String,
    pub workflow_path: String,
    pub name: String,
    pub push_branches: Vec<String>,
    pub checkout_action: String,
    pub upload_artifact_action: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_hidden_files: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_artifact_action: Option<String>,
    pub jobs: Vec<Job>,
}

/// 宿主选择成功落地 CI 与唯一发布作业；普通构建继续只读。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    pub workflow: String,
    pub branch: String,
    pub publisher: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub promotion_build_jobs: Vec<String>,
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
    if !crate::verified_literals::valid_literal(value.as_bytes()) {
        return Err("release settings must be literal single-line values".into());
    }
    Ok(())
}

pub(crate) fn validate(c: &Config) -> Result<(), String> {
    if let Some(delivery) = &c.delivery {
        if !super::verified_literals::delivery_mode(&c.schema, c.push_branches.len()) {
            return Err("delivery requires release v2 and a successful workflow trigger".into());
        }
        for value in [&delivery.workflow, &delivery.branch, &delivery.publisher] {
            literal(value)?;
            if !super::verified_literals::delivery_nonempty(value) {
                return Err("empty delivery workflow, branch or publisher".into());
            }
        }
        let ids = c.jobs.iter().map(|job| job.id.as_str()).collect::<Vec<_>>();
        let index = super::verified_literals::publisher_index(&ids, &delivery.publisher)
            .ok_or("unregistered release publisher")?;
        let publisher = &c.jobs[index];
        if !super::verified_literals::publisher_dependencies(
            publisher.needs.len(),
            publisher.always,
        ) {
            return Err("publisher requires successful declared dependencies".into());
        }
        let mut build_jobs = BTreeSet::new();
        for id in &delivery.promotion_build_jobs {
            if !ids.contains(&id.as_str()) || id == &delivery.publisher || !build_jobs.insert(id) {
                return Err(
                    "promotion needs unique registered build jobs, excluding the publisher".into(),
                );
            }
        }
    }
    if let Some(cache) = &c.persistent_cache {
        if c.schema != UNITS_SCHEMA {
            return Err("persistent release cache requires v2".into());
        }
        super::cache::validate(cache, &c.jobs.iter().map(|job| job.id.clone()).collect())?;
    }
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
    // Reuse only identical, explicitly registered downloads. Aliases retain one
    // execution per consumer job and do not add or remove dependency edges.
    let mut counts = BTreeMap::new();
    for download in c.jobs.iter().flat_map(|job| &job.downloads) {
        *counts
            .entry((download.job.as_str(), download.directory.as_str()))
            .or_insert(0) += 1;
    }
    let mut shared_downloads: BTreeMap<_, _> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .enumerate()
        .map(|(index, (key, _))| (key, (format!("chrono_release_download_{index}"), false)))
        .collect();
    let mut output = format!("{MARKER}name: {}\non:\n", scalar(&c.name));
    if let Some(delivery) = &c.delivery {
        output.push_str(&format!(
            "  workflow_run:\n    workflows: [{}]\n    branches: [{}]\n    types: [completed]\n",
            scalar(&delivery.workflow),
            scalar(&delivery.branch)
        ));
    }
    if !c.push_branches.is_empty() {
        output.push_str(&format!(
            "  push:\n    branches: {}\n",
            serde_json::to_string(&c.push_branches).map_err(|e| e.to_string())?
        ));
    }
    output.push_str(
        "  workflow_dispatch:\n    inputs:\n      source:\n        description: Source revision to check out\n        required: true\n        type: string\n",
    );
    if c.delivery.is_some() {
        output.push_str("      channel:\n        description: Build artifacts or publish a verified source\n        type: choice\n        default: build\n        options: [build, preview, stable]\n      verified_run:\n        description: Successful landing CI run ID\n        type: string\n      verified_attempt:\n        description: Original successful CI attempt\n        type: string\n      version:\n        description: Explicit stable version matching the source plan\n        type: string\n");
        if c.delivery
            .as_ref()
            .is_some_and(|d| !d.promotion_build_jobs.is_empty())
        {
            output.push_str("      preview_run:\n        description: Successful final preview release run ID required for stable\n        type: string\n      preview_attempt:\n        description: Original successful preview release attempt required for stable\n        type: string\n");
        }
    }
    output.push_str("permissions:\n  contents: read\n");
    output.push_str("jobs:\n");
    for job in &c.jobs {
        if c.schema == UNITS_SCHEMA {
            let mut body = String::new();
            render_unit(&mut body, c, job, &mut shared_downloads)?;
            output.push_str(&super::cache::project(
                c.persistent_cache.as_ref(),
                &job.id,
                body,
                "Run registered release command",
                "Run registered release command",
                None,
                None,
                Some("Preserve original release artifacts"),
            )?);
            if c.persistent_cache
                .as_ref()
                .is_some_and(|cache| cache.jobs.contains_key(&job.id))
            {
                output.push_str(&format!(
                    "      - name: Preserve failed cache diagnostics\n        if: ${{{{ failure() }}}}\n        uses: {}\n        with:\n          name: {}\n          path: .chrono-harness/state/\n          if-no-files-found: error\n{}",
                    c.upload_artifact_action,
                    scalar(&format!(
                        "diagnostics-{}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}",
                        job.artifact_name
                    )),
                    super::artifact_hidden_files(true)
                ));
            }
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
{hidden_files}"#,
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
            hidden_files = super::artifact_hidden_files(c.include_hidden_files),
            artifact = scalar(&job.artifact_name),
            directory = scalar(&job.artifact_directory),
        ));
    }
    Ok(output)
}

fn render_unit<'a>(
    output: &mut String,
    c: &Config,
    job: &'a Job,
    shared_downloads: &mut BTreeMap<(&'a str, &'a str), (String, bool)>,
) -> Result<(), String> {
    let mut condition = format!(
        "{}github.event.deleted != true",
        if job.always { "always() && " } else { "" }
    );
    if c.delivery.is_some() {
        condition.push_str(" && (github.event_name != 'workflow_run' || (github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.event == 'push'))");
    }
    if let Some(delivery) = &c.delivery {
        if !delivery.promotion_build_jobs.is_empty() {
            if job.id == delivery.publisher {
                // stable 的包来自显式原运行；其余渠道仍要求当前依赖成功。
                condition =
                    format!("always() && (inputs.channel == 'stable' || success()) && {condition}");
            } else if delivery.promotion_build_jobs.contains(&job.id) {
                condition.push_str(" && inputs.channel != 'stable'");
            }
        }
    }
    output.push_str(&format!("  {}:\n    if: ${{{{ {condition} }}}}\n", job.id));
    if super::verified_literals::delivery_publisher(
        c.delivery.as_ref().map(|d| d.publisher.as_str()),
        &job.id,
    ) {
        output.push_str("    permissions:\n      contents: write\n      actions: read\n");
    }
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
          ref: {source}
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
        checkout = c.checkout_action,
        source = super::verified_literals::delivery_source(c.delivery.is_some())
    ));
    for download in &job.downloads {
        let prefix =
            match shared_downloads.get_mut(&(download.job.as_str(), download.directory.as_str())) {
                Some((anchor, emitted)) if *emitted => {
                    output.push_str(&format!("      - *{anchor}\n"));
                    continue;
                }
                Some((anchor, emitted)) => {
                    *emitted = true;
                    format!("      - &{anchor}\n        name:")
                }
                None => "      - name:".to_string(),
            };
        output.push_str(&format!(
            r#"{prefix} Download selected original artifact from {producer}
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
"#,
        job = scalar(&job.id)
    ));
    if c.delivery.is_some() {
        output.push_str("          CHRONO_RELEASE_CHANNEL: ${{ github.event_name == 'workflow_run' && 'preview' || inputs.channel || 'build' }}\n          CHRONO_RELEASE_SOURCE: ${{ github.event.workflow_run.head_sha || inputs.source }}\n          CHRONO_RELEASE_CI_RUN_ID: ${{ github.event.workflow_run.id || inputs.verified_run }}\n          CHRONO_RELEASE_CI_ATTEMPT: ${{ github.event.workflow_run.run_attempt || inputs.verified_attempt }}\n          CHRONO_RELEASE_VERSION: ${{ inputs.version }}\n");
        if super::verified_literals::delivery_publisher(
            c.delivery.as_ref().map(|d| d.publisher.as_str()),
            &job.id,
        ) {
            if c.delivery
                .as_ref()
                .is_some_and(|d| !d.promotion_build_jobs.is_empty())
            {
                output.push_str("          CHRONO_RELEASE_PREVIEW_RUN: ${{ inputs.preview_run }}\n          CHRONO_RELEASE_PREVIEW_ATTEMPT: ${{ inputs.preview_attempt }}\n");
            }
            output.push_str("          GH_TOKEN: ${{ github.token }}\n");
        }
    }
    output.push_str("        run: |\n");
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
{hidden_files}"#,
        upload = c.upload_artifact_action,
        hidden_files = super::artifact_hidden_files(c.include_hidden_files),
        directory = scalar(&job.artifact_directory)
    ));
    Ok(())
}

pub(crate) fn generate(root: &Path, c: &Config, verify: bool) -> Result<bool, String> {
    super::cache::validate_registry(root, c.persistent_cache.as_ref())?;
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
    super::cache::validate_registry(root, c.persistent_cache.as_ref())?;
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
