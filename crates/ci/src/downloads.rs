//! 编译 planner 之前恢复下载资料；执行和传输仍由原生 cache action 负责。
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub enum DownloadDomain {
    Files,
    InstallLayout,
    Invalid,
}

pub fn download_domain(use_layout: bool, files: usize) -> (domain: DownloadDomain)
    ensures
        (domain is Files) == !use_layout,
        (domain is InstallLayout) == (use_layout && files == 1),
        (domain is Invalid) == (use_layout && files != 1),
{
    if !use_layout { DownloadDomain::Files }
    else if files == 1 { DownloadDomain::InstallLayout }
    else { DownloadDomain::Invalid }
}

pub fn saves_download(producer: &str, job: &str) -> (save: bool)
    ensures save == (producer@ == job@),
{
    identifiers::equal_text(producer, job)
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{cache, scalar, shell, verified_literals as policy};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{decode, no_symlink_parents, relative_path};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Config {
    pub schema: String,
    pub namespace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_registry: Option<String>,
    pub environment: BTreeMap<String, String>,
    pub entries: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Entry {
    pub owner: String,
    pub need: String,
    pub output_use: String,
    pub paths: Vec<String>,
    pub compatibility_files: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub release_install_layout: bool,
    pub source_files: Vec<String>,
    pub consumers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_job: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_from: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
fn path(value: &str) -> Result<(), String> {
    relative_path(value.trim_end_matches('/'))?;
    if !policy::valid_download_path(value.as_bytes()) {
        return Err(format!("invalid literal download cache path: {value}"));
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn validate(c: &Config, jobs: &BTreeSet<String>) -> Result<(), String> {
    if !policy::valid_download_config(&c.schema, c.entries.len()) {
        return Err("download caches require nonempty chrono-github-downloads/v1".into());
    }
    path(&c.namespace)?;
    for (key, value) in &c.environment {
        if !policy::valid_download_environment(key) {
            return Err(format!("unsupported download environment: {key}"));
        }
        path(value)?;
    }
    for (id, entry) in &c.entries {
        path(id)?;
        let fallback = entry.restore_from.as_ref().and_then(|id| c.entries.get(id));
        let valid = policy::valid_download_entry(
            entry.paths.len(),
            entry.compatibility_files.len(),
            entry.source_files.len(),
            entry.consumers.len(),
            !matches!(
                download_domain(
                    entry.release_install_layout,
                    entry.compatibility_files.len()
                ),
                DownloadDomain::Invalid
            ) && unique(&entry.paths)
                && unique(&entry.compatibility_files)
                && unique(&entry.source_files)
                && unique(&entry.consumers),
            entry.consumers.iter().all(|id| jobs.contains(id))
                && entry
                    .save_job
                    .as_ref()
                    .is_none_or(|writer| entry.consumers.contains(writer)),
            entry.restore_from.is_none()
                || fallback.is_some_and(|other| {
                    entry.restore_from.as_ref() != Some(id)
                        && other.paths == entry.paths
                        && other.compatibility_files == entry.compatibility_files
                        && other.release_install_layout == entry.release_install_layout
                }),
            !entry.owner.trim().is_empty()
                && !entry.need.trim().is_empty()
                && !entry.output_use.trim().is_empty(),
        );
        if !valid {
            return Err(format!("invalid download cache contract: {id}"));
        }
        for value in entry
            .paths
            .iter()
            .chain(&entry.compatibility_files)
            .chain(&entry.source_files)
        {
            path(value)?;
        }
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn validate_registry(
    root: &Path,
    c: &Config,
    registry_path: &str,
) -> Result<(), String> {
    let artifacts: Value =
        decode(&fs::read(no_symlink_parents(root, registry_path)?).map_err(|e| e.to_string())?)?;
    let artifacts = artifacts["artifacts"]
        .as_array()
        .ok_or("download cache artifact registry needs artifacts")?;
    for (id, entry) in &c.entries {
        for path in &entry.paths {
            let matches = artifacts
                .iter()
                .filter(|a| {
                    policy::valid_download_artifact(
                        a["path"].as_str(),
                        path,
                        a["owner"].as_str(),
                        &entry.owner,
                        a["tracked"].as_bool(),
                        a["kind"].as_str(),
                    )
                })
                .count();
            if !policy::single_download_binding(matches) {
                return Err(format!(
                    "download cache {id} needs one registered non-evidence artifact: {path}"
                ));
            }
            no_symlink_parents(root, path.trim_end_matches('/'))?;
        }
        for input in entry.compatibility_files.iter().chain(&entry.source_files) {
            let path = no_symlink_parents(root, input)?;
            // hashFiles is a provider interface; every explicit input must be present.
            fs::read(path)
                .map_err(|error| format!("download cache {id} input {input}: {error}"))?;
        }
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn hash(files: &[String]) -> String {
    format!(
        "${{{{ hashFiles({}) }}}}",
        files
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(not(verus_keep_ghost))]
fn prefix(c: &Config, id: &str, domain: &str) -> String {
    format!(
        "chrono-download-v1-{}-{id}-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-{}-",
        c.namespace, domain
    )
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn project(
    parent: &cache::Config,
    job: &str,
    body: String,
    checkout_end: &str,
) -> Result<String, String> {
    let Some(c) = &parent.downloads else {
        return Ok(body);
    };
    let selected: Vec<_> = c
        .entries
        .iter()
        .filter(|(_, e)| e.consumers.iter().any(|s| s == job))
        .collect();
    if selected.is_empty() {
        return Ok(body);
    }
    let steps = "    steps:\n";
    if !policy::single_download_binding(body.matches(checkout_end).count())
        || !policy::single_download_binding(body.matches(steps).count())
    {
        return Err("download cache projection requires one checkout and one steps section".into());
    }
    let mut environment = String::new();
    if !c.environment.is_empty() {
        environment.push_str("    env:\n");
    }
    for (key, path) in &c.environment {
        environment.push_str(&format!(
            "      {key}: {}\n",
            scalar(&format!("${{{{ github.workspace }}}}/{path}"))
        ));
    }
    let mut restore = String::new();
    let mut save = String::new();
    let event_guard = policy::cache_save_event_guard(parent.save_on_push_only);
    for (index, (id, entry)) in selected.iter().enumerate() {
        let compatibility = hash(&entry.compatibility_files);
        let (domain, restore_guard, pin, save_status) = match download_domain(
            entry.release_install_layout,
            entry.compatibility_files.len(),
        ) {
            DownloadDomain::InstallLayout => {
                let lock = shell(&entry.compatibility_files[0]);
                let step = format!("chrono_layout_{index}");
                restore.push_str(&format!(
                    r#"      - name: Read release install layout {id}
        id: {step}
        continue-on-error: true
        shell: bash
        run: |
          set -eo pipefail
          digest=$(jq -sceS '
            select(length == 1) | .[0]
            | select(.schema == "chrono-install/v1")
            | select(.install | type == "object" and length > 0 and all(.[]; type == "string" and length > 0))
            | [.schema, .install]
          ' < {lock} | shasum -a 256)
          printf 'digest=%s\n' "${{digest%% *}}" >> "$GITHUB_OUTPUT"
"#
                ));
                (
                    format!("layout-v1-${{{{ steps.{step}.outputs.digest }}}}"),
                    format!("        if: ${{{{ steps.{step}.outcome == 'success' }}}}\n"),
                    format!("{compatibility}-"),
                    "success()",
                )
            }
            DownloadDomain::Files => (compatibility, String::new(), String::new(), "always()"),
            DownloadDomain::Invalid => {
                return Err(format!(
                    "release install layout requires one lock file: {id}"
                ));
            }
        };
        let stem = prefix(c, id, &domain);
        let key = format!("{stem}{pin}{}", hash(&entry.source_files));
        let paths = entry
            .paths
            .iter()
            .map(|s| format!("            {s}\n"))
            .collect::<String>();
        let fallback = entry
            .restore_from
            .as_ref()
            .map(|id| format!("            {}\n", prefix(c, id, &domain)))
            .unwrap_or_default();
        restore.push_str(&format!("      - name: Restore download cache {id}\n        id: chrono_download_{index}\n{restore_guard}        continue-on-error: true\n        uses: {}\n        with:\n          key: {}\n          path: |\n{paths}          restore-keys: |\n            {stem}\n{fallback}          fail-on-cache-miss: false\n", parent.restore_action, scalar(&key)));
        if entry
            .save_job
            .as_deref()
            .is_some_and(|writer| saves_download(writer, job))
        {
            save.push_str(&format!("      - name: Save download cache {id}\n        if: ${{{{ {event_guard}{save_status} && !cancelled() && steps.chrono_download_{index}.outcome == 'success' && steps.chrono_download_{index}.outputs.cache-hit != 'true' }}}}\n        continue-on-error: true\n        uses: {}\n        with:\n          key: {}\n          path: |\n{paths}", parent.save_action, scalar(&key)));
        }
    }
    let mut body = body
        .replacen(steps, &format!("{environment}{steps}"), 1)
        .replacen(checkout_end, &format!("{checkout_end}{restore}"), 1);
    body.push_str(&save);
    Ok(body)
}
