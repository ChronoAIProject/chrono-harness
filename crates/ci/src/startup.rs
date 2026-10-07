//! Explicit startup transfer; hosts own production, identity and installation.
use crate::{action, gating, overlap, scalar, shell, units};
use chrono_harness::relative_path;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub directory: String,
    pub artifact: String,
    pub download_action: String,
    pub consumers: BTreeMap<String, Consumer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub need: String,
    pub output_use: String,
    pub install: Vec<String>,
}

pub(crate) fn validate(c: &units::Config, s: &Config) -> Result<(), String> {
    let directory = s.directory.trim_end_matches('/');
    relative_path(directory)?;
    if s.schema != "chrono-shared-startup/v1"
        || !s.directory.starts_with(".chrono-harness/")
        || !s.directory.ends_with('/')
        || s.directory.ends_with("//")
        || s.directory
            .contains(['\0', '\r', '\n', '*', '?', '[', ']', '!'])
        || s.directory.contains("${{")
        || s.artifact.is_empty()
        || s.artifact.len() > 128
        || !s
            .artifact
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || s.consumers.is_empty()
    {
        return Err(
            "startup requires a literal host transfer directory, artifact and consumers".into(),
        );
    }
    action(&s.download_action, "actions/download-artifact")?;
    if matches!(
        s.artifact.as_str(),
        "chrono-collection" | "chrono-detector-evidence"
    ) || c
        .units
        .keys()
        .any(|unit| s.artifact == format!("chrono-unit-{unit}"))
        || c.native_adoption
            .as_ref()
            .is_some_and(|a| s.artifact == a.seed_artifact)
    {
        return Err("startup artifact collides with an evidence artifact".into());
    }
    let g = c.job_gating.as_ref().ok_or("startup requires job gating")?;
    let mut evidence = vec![
        ".chrono-harness/state/",
        c.collection.artifact_directory.as_str(),
        c.gather.download_directory.as_str(),
    ];
    evidence.extend(c.units.values().map(|u| u.artifact_directory.as_str()));
    evidence.extend(g.detector.evidence_directory.as_deref());
    if let Some(a) = &c.native_adoption {
        evidence.push(&a.seed_directory);
    }
    if let Some(cache) = &c.persistent_cache {
        evidence.extend(
            cache
                .jobs
                .values()
                .filter_map(|job| job.evidence_directory.as_deref()),
        );
    }
    if evidence
        .into_iter()
        .any(|path| overlap(directory, path.trim_end_matches('/')))
    {
        return Err("startup transfer must not overlap evidence directories".into());
    }
    let jobs: BTreeSet<_> = c
        .units
        .keys()
        .map(|id| gating::job_id(id))
        .chain(["aggregate".into()])
        .collect();
    for (job, consumer) in &s.consumers {
        if !jobs.contains(job)
            || consumer.need.trim().is_empty()
            || consumer.output_use.trim().is_empty()
            || consumer.install.is_empty()
            || consumer.install[0].trim().is_empty()
            || consumer
                .install
                .iter()
                .any(|s| s.contains(['\0', '\r', '\n']) || s.contains("${{"))
        {
            return Err(format!("invalid startup consumer for job {job}"));
        }
    }
    Ok(())
}

pub(crate) fn producer(c: &units::Config, s: &Config, body: String) -> Result<String, String> {
    let marker = "      - name: Bootstrap registered detector tools\n";
    let detection = "      - name: Detect required units from fixed Git endpoints\n";
    if body.matches(marker).count() != 1 || body.matches(detection).count() != 1 {
        return Err("startup requires one detector bootstrap and detection step".into());
    }
    let upload = format!(
        "      - name: Publish registered startup artifact\n        id: chrono_startup_upload\n        uses: {}\n        with:\n          name: {}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}\n          path: {}\n          if-no-files-found: error\n          include-hidden-files: true\n",
        c.collection.upload_artifact_action,
        s.artifact,
        scalar(&s.directory)
    );
    Ok(body
        .replacen(marker, &format!("{marker}        id: chrono_startup\n"), 1)
        .replacen(detection, &format!("{upload}{detection}"), 1))
}

pub(crate) fn consumer(
    s: &Config,
    job: &str,
    body: String,
    before: &str,
) -> Result<String, String> {
    let Some(consumer) = s.consumers.get(job) else {
        return Ok(body);
    };
    let marker = format!("      - name: {before}\n");
    if body.matches(&marker).count() != 1 {
        return Err("startup requires one original consumer step".into());
    }
    let command = consumer
        .install
        .iter()
        .map(|s| shell(s))
        .collect::<Vec<_>>()
        .join(" ");
    let prefix = format!(
        "      - name: Download registered startup artifact\n        if: ${{{{ needs.detect.result == 'success' && needs.detect.outputs.startup_artifact != '' }}}}\n        uses: {}\n        with:\n          artifact-ids: ${{{{ needs.detect.outputs.startup_artifact }}}}\n          path: {}\n          merge-multiple: true\n      - name: Install registered startup tools\n        shell: bash\n        env:\n          CHRONO_STARTUP_BINDING: ${{{{ needs.detect.outputs.startup_binding }}}}\n        run: |\n          {command}\n",
        s.download_action,
        scalar(&s.directory)
    );
    Ok(body.replacen(&marker, &format!("{prefix}{marker}"), 1))
}
