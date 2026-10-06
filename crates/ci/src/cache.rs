//! Native cache transport projection. Cache identities remain the planner's output.
use crate::{action, shell};
use chrono_harness::{decode, no_symlink_parents, relative_path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub program: String,
    pub config: String,
    pub restore_action: String,
    pub save_action: String,
    pub jobs: BTreeMap<String, Consumer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub need: String,
    pub output_use: String,
    pub prepare: Vec<String>,
    pub consumer: String,
    pub caches: Vec<String>,
}

fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}

pub(crate) fn validate(c: &Config, jobs: &BTreeSet<String>) -> Result<(), String> {
    if c.schema != "chrono-github-cache/v1" || c.jobs.is_empty() {
        return Err("persistent cache requires nonempty chrono-github-cache/v1".into());
    }
    for path in [&c.program, &c.config] {
        relative_path(path)?;
        if !path.starts_with(".chrono-harness/")
            || path.contains(['\0', '\r', '\n'])
            || path.contains("${{")
        {
            return Err("persistent cache paths must belong to host .chrono-harness".into());
        }
    }
    action(&c.restore_action, "actions/cache/restore")?;
    action(&c.save_action, "actions/cache/save")?;
    for (job, consumer) in &c.jobs {
        if !jobs.contains(job)
            || !name(&consumer.consumer)
            || consumer.need.trim().is_empty()
            || consumer.output_use.trim().is_empty()
            || consumer.prepare.is_empty()
            || consumer
                .prepare
                .iter()
                .any(|s| s.contains(['\0', '\r', '\n']) || s.contains("${{"))
            || consumer.prepare[0].is_empty()
            || consumer.caches.is_empty()
            || consumer.caches.iter().any(|s| !name(s))
            || consumer.caches.iter().collect::<BTreeSet<_>>().len() != consumer.caches.len()
        {
            return Err(format!("invalid persistent cache consumer for job {job}"));
        }
    }
    Ok(())
}

/// Resolve references against the host's single cache registry before projecting.
pub(crate) fn validate_registry(root: &Path, c: Option<&Config>) -> Result<(), String> {
    let Some(c) = c else {
        return Ok(());
    };
    let registry: Value =
        decode(&fs::read(no_symlink_parents(root, &c.config)?).map_err(|e| e.to_string())?)?;
    if registry["schema"] != "chrono-cache/v1" || registry["require_primary_checkout"] != true {
        return Err(
            "native persistent cache needs the planner schema and explicit primary-checkout guard"
                .into(),
        );
    }
    let caches = registry["caches"]
        .as_object()
        .ok_or("persistent cache registry needs caches")?;
    for consumer in c.jobs.values() {
        let selected = caches
            .iter()
            .filter(|(_, c)| {
                c["consumers"]
                    .as_array()
                    .is_some_and(|ids| ids.iter().any(|id| id == &consumer.consumer))
            })
            .map(|(id, _)| id.as_str())
            .collect::<BTreeSet<_>>();
        if selected
            != consumer
                .caches
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        {
            return Err(format!(
                "persistent cache references differ from consumer {}",
                consumer.consumer
            ));
        }
    }
    Ok(())
}

pub(crate) fn project(
    c: Option<&Config>,
    job: &str,
    body: String,
    before: &str,
    work: &str,
    existing_work_id: Option<&str>,
    after: Option<&str>,
) -> Result<String, String> {
    let Some(c) = c else {
        return Ok(body);
    };
    let Some(consumer) = c.jobs.get(job) else {
        return Ok(body);
    };
    let work_id = existing_work_id.unwrap_or("chrono_cache_work");
    let prepare = consumer
        .prepare
        .iter()
        .map(|s| shell(s))
        .collect::<Vec<_>>()
        .join(" ");
    let mut prefix = format!(
        "      - name: Prepare registered caches\n        id: chrono_cache_plan\n        shell: bash\n        run: |\n          {prepare}\n          {} plan --host-root . --config {} --consumer {} --github-output \"$GITHUB_OUTPUT\"\n",
        shell(&c.program),
        shell(&c.config),
        shell(&consumer.consumer)
    );
    let mut suffix = String::new();
    for id in &consumer.caches {
        let key = format!(
            "cache_{}",
            id.bytes().map(|b| format!("{b:02x}")).collect::<String>()
        );
        // Native reference names are shorter than 100 characters. Output names
        // keep the planner's encoding; step references bind the complete cache ID.
        let step = format!("chrono_cache_{}", chrono_harness::sha256(id.as_bytes()));
        prefix.push_str(&format!("      - name: Restore registered cache {id}\n        id: {step}_restore\n        continue-on-error: true\n        uses: {}\n        with:\n          key: ${{{{ steps.chrono_cache_plan.outputs.{key}_key }}}}\n          path: ${{{{ steps.chrono_cache_plan.outputs.{key}_paths }}}}\n          restore-keys: ${{{{ steps.chrono_cache_plan.outputs.{key}_restore }}}}\n          fail-on-cache-miss: false\n", c.restore_action));
        suffix.push_str(&format!("      - name: Save registered cache {id}\n        id: {step}_save\n        if: ${{{{ always() && steps.chrono_cache_plan.outcome == 'success' && steps.{work_id}.outcome == 'success' && steps.{step}_restore.outputs.cache-matched-key != steps.chrono_cache_plan.outputs.{key}_key }}}}\n        continue-on-error: true\n        uses: {}\n        with:\n          key: ${{{{ steps.chrono_cache_plan.outputs.{key}_key }}}}\n          path: ${{{{ steps.chrono_cache_plan.outputs.{key}_paths }}}}\n", c.save_action));
    }
    let marker = format!("      - name: {before}\n");
    let work_marker = format!("      - name: {work}\n");
    if body.matches(&marker).count() != 1 || body.matches(&work_marker).count() != 1 {
        return Err("persistent cache requires one original bootstrap/work step".into());
    }
    let mut body = if existing_work_id.is_none() {
        body.replacen(
            &work_marker,
            &format!("{work_marker}        id: {work_id}\n"),
            1,
        )
    } else {
        body
    };
    body = body.replacen(&marker, &format!("{prefix}{marker}"), 1);
    if let Some(after) = after {
        let marker = format!("      - name: {after}\n");
        if body.matches(&marker).count() != 1 {
            return Err("persistent cache requires one original evidence step".into());
        }
        body = body.replacen(&marker, &format!("{suffix}{marker}"), 1);
    } else {
        body.push_str(&suffix);
    }
    Ok(body)
}
