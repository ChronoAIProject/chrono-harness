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
    /// Omitted preserves all-cache saving; an empty list is restore-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_caches: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub save_after_bootstrap: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_directory: Option<String>,
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
    let mut saving_jobs = BTreeMap::new();
    for (job, consumer) in &c.jobs {
        let saves = consumer.save_caches.as_ref().unwrap_or(&consumer.caches);
        if let Some(directory) = &consumer.evidence_directory {
            relative_path(directory.trim_end_matches('/'))?;
            if ![".chrono-harness/state/", ".chrono-harness/cache/"]
                .iter()
                .any(|prefix| directory.starts_with(prefix))
                || !directory.ends_with('/')
                || directory.contains(['\0', '\r', '\n', '*', '?', '[', ']'])
                || directory.contains("${{")
            {
                return Err(format!("invalid cache evidence directory for {job}"));
            }
        }
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
            || saves.iter().collect::<BTreeSet<_>>().len() != saves.len()
            || saves.iter().any(|id| !consumer.caches.contains(id))
            || consumer
                .save_after_bootstrap
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != consumer.save_after_bootstrap.len()
            || consumer
                .save_after_bootstrap
                .iter()
                .any(|id| !saves.contains(id))
        {
            return Err(format!("invalid persistent cache consumer for job {job}"));
        }
        for cache in saves {
            if let Some(previous) = saving_jobs.insert(cache, job) {
                return Err(format!(
                    "E_CACHE_SAVE_OWNERSHIP: cache {cache} has multiple saving jobs: {previous}, {job}; select one writer with save_caches"
                ));
            }
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
    if !matches!(
        registry["schema"].as_str(),
        Some("chrono-cache/v1" | "chrono-cache/v2")
    ) || registry["require_primary_checkout"] != true
    {
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
    if before == work && !consumer.save_after_bootstrap.is_empty() {
        return Err("bootstrap cache saving requires a separate original bootstrap step".into());
    }
    let prepare = consumer
        .prepare
        .iter()
        .map(|s| shell(s))
        .collect::<Vec<_>>()
        .join(" ");
    let evidence_directory = consumer
        .evidence_directory
        .clone()
        .unwrap_or_else(|| format!(".chrono-harness/state/cache/{}/", consumer.consumer));
    // Release commands own creation of their output directory. Keep preparation
    // state separate until that command has finished, then publish for upload.
    let plan_path = format!(
        ".chrono-harness/state/cache/{}/plan.json",
        consumer.consumer
    );
    let mut prefix = format!(
        "      - name: Prepare registered caches\n        id: chrono_cache_plan\n        shell: bash\n        run: |\n          {prepare}\n          {} plan --host-root . --config {} --consumer {} --plan-output {} --github-output \"$GITHUB_OUTPUT\"\n",
        shell(&c.program),
        shell(&c.config),
        shell(&consumer.consumer),
        shell(&plan_path)
    );
    let mut suffix = String::new();
    // Compact references keep large native workflows within the provider limit.
    // Registry validation guarantees the same selected set as the planner.
    let selected: BTreeSet<_> = consumer.caches.iter().collect();
    let saves = consumer.save_caches.as_ref().unwrap_or(&consumer.caches);
    for (index, id) in selected.into_iter().enumerate() {
        let key = format!("cache_{index}");
        let step = &key;
        prefix.push_str(&format!("      - name: Restore registered cache {id}\n        id: {step}_restore\n        continue-on-error: true\n        uses: {}\n        with:\n          key: ${{{{ steps.chrono_cache_plan.outputs.{key}_key }}}}\n          path: ${{{{ steps.chrono_cache_plan.outputs.{key}_paths }}}}\n          restore-keys: ${{{{ steps.chrono_cache_plan.outputs.{key}_restore }}}}\n          fail-on-cache-miss: false\n", c.restore_action));
        if !saves.contains(id) {
            continue;
        }
        let producer_step = if consumer.save_after_bootstrap.contains(id) {
            "chrono_cache_bootstrap"
        } else {
            work_id
        };
        suffix.push_str(&format!("      - name: Save registered cache {id}\n        id: {step}_save\n        if: ${{{{ always() && !cancelled() && steps.chrono_cache_plan.outcome == 'success' && steps.{producer_step}.outcome == 'success' && steps.{step}_restore.outputs.cache-matched-key != steps.chrono_cache_plan.outputs.{key}_key }}}}\n        continue-on-error: true\n        uses: {}\n        with:\n          key: ${{{{ steps.chrono_cache_plan.outputs.{key}_key }}}}\n          path: ${{{{ steps.chrono_cache_plan.outputs.{key}_paths }}}}\n", c.save_action));
    }
    let bootstrap_arg = if consumer.save_after_bootstrap.is_empty() {
        ""
    } else {
        " --bootstrap chrono_cache_bootstrap"
    };
    let saves_arg = consumer
        .save_caches
        .as_ref()
        .map(|saves| {
            format!(
                " --save-caches {}",
                shell(&serde_json::to_string(saves).unwrap())
            )
        })
        .unwrap_or_default();
    suffix.push_str(&format!("      - name: Record original cache transport observations\n        if: ${{{{ always() && steps.chrono_cache_plan.outcome == 'success' }}}}\n        continue-on-error: true\n        shell: bash\n        env:\n          CHRONO_CACHE_STEPS: ${{{{ toJSON(steps) }}}}\n          GH_TOKEN: ${{{{ github.token }}}}\n        run: |\n          {} report --host-root . --plan {} --report-directory {} --steps-env CHRONO_CACHE_STEPS --work {}{bootstrap_arg}{saves_arg}\n", shell(&c.program), shell(&plan_path), shell(&evidence_directory), shell(work_id)));
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
    if !consumer.save_after_bootstrap.is_empty() {
        body = body.replacen(
            &marker,
            &format!("{marker}        id: chrono_cache_bootstrap\n"),
            1,
        );
    }
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
