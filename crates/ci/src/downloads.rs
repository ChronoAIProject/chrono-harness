//! 编译 planner 之前恢复下载资料；执行和传输仍由原生 cache action 负责。
use crate::{cache, scalar, verified_literals as policy};
use chrono_harness::{decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub namespace: String,
    pub environment: BTreeMap<String, String>,
    pub entries: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub owner: String,
    pub need: String,
    pub output_use: String,
    pub paths: Vec<String>,
    pub compatibility_files: Vec<String>,
    pub source_files: Vec<String>,
    pub consumers: Vec<String>,
    pub save_job: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_from: Option<String>,
}

fn path(value: &str) -> Result<(), String> {
    relative_path(value.trim_end_matches('/'))?;
    if !policy::valid_download_path(value.as_bytes()) {
        return Err(format!("invalid literal download cache path: {value}"));
    }
    Ok(())
}

fn unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

pub(crate) fn validate(c: &Config, jobs: &BTreeSet<String>) -> Result<(), String> {
    if !policy::valid_download_config(&c.schema, c.entries.len(), c.environment.len()) {
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
            unique(&entry.paths)
                && unique(&entry.compatibility_files)
                && unique(&entry.source_files)
                && unique(&entry.consumers),
            entry.consumers.iter().all(|id| jobs.contains(id))
                && entry.consumers.contains(&entry.save_job),
            entry.restore_from.is_none()
                || fallback.is_some_and(|other| {
                    entry.restore_from.as_ref() != Some(id)
                        && other.paths == entry.paths
                        && other.compatibility_files == entry.compatibility_files
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

pub(crate) fn validate_registry(
    root: &Path,
    c: &Config,
    registry: &Value,
    read: &mut impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let registry_path = registry["artifact_registry"]
        .as_str()
        .ok_or("download cache needs artifact registry")?;
    let artifacts: Value = decode(&read(registry_path)?)?;
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
            // hashFiles is a provider interface; every explicit input must be present.
            read(input).map_err(|error| format!("download cache {id} input {input}: {error}"))?;
        }
    }
    Ok(())
}

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

fn prefix(c: &Config, id: &str, entry: &Entry) -> String {
    format!(
        "chrono-download-v1-{}-{id}-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-{}-",
        c.namespace,
        hash(&entry.compatibility_files)
    )
}

pub(crate) fn project(parent: &cache::Config, job: &str, body: String) -> Result<String, String> {
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
    let checkout = "          persist-credentials: true\n";
    let steps = "    steps:\n";
    if !policy::single_download_binding(body.matches(checkout).count())
        || !policy::single_download_binding(body.matches(steps).count())
    {
        return Err("download cache projection requires one checkout and one steps section".into());
    }
    let mut environment = "    env:\n".to_owned();
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
        let stem = prefix(c, id, entry);
        let key = format!("{stem}{}", hash(&entry.source_files));
        let paths = entry
            .paths
            .iter()
            .map(|s| format!("            {s}\n"))
            .collect::<String>();
        let fallback = entry
            .restore_from
            .as_ref()
            .map(|id| format!("            {}\n", prefix(c, id, &c.entries[id])))
            .unwrap_or_default();
        restore.push_str(&format!("      - name: Restore download cache {id}\n        id: chrono_download_{index}\n        continue-on-error: true\n        uses: {}\n        with:\n          key: {}\n          path: |\n{paths}          restore-keys: |\n            {stem}\n{fallback}          fail-on-cache-miss: false\n", parent.restore_action, scalar(&key)));
        if &entry.save_job == job {
            save.push_str(&format!("      - name: Save download cache {id}\n        if: ${{{{ {event_guard}always() && !cancelled() && steps.chrono_download_{index}.outcome == 'success' && steps.chrono_download_{index}.outputs.cache-hit != 'true' }}}}\n        continue-on-error: true\n        uses: {}\n        with:\n          key: {}\n          path: |\n{paths}", parent.save_action, scalar(&key)));
        }
    }
    let mut body = body
        .replacen(steps, &format!("{environment}{steps}"), 1)
        .replacen(checkout, &format!("{checkout}{restore}"), 1);
    body.push_str(&save);
    Ok(body)
}
