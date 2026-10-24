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
    /// A named, explicitly registered pre-save native acceptance fixture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_fixture: Option<String>,
    /// Omitted retains the original per-run/attempt Variables transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_source: Option<AcceptanceSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "method", deny_unknown_fields)]
pub enum AcceptanceSource {
    #[serde(rename = "pull-request-body/v1")]
    PullRequestBody { program: String, reader: String },
}

/// Select transport data only. The existing cache fixture remains the authority
/// for the full activation schema, plan keys, producer seal and all mutations.
pub fn pull_request_activation(
    response: &Value,
    native: &BTreeMap<String, String>,
    fixture: &str,
    consumer: &str,
) -> Result<Option<String>, String> {
    const MARKER: &str = "<!-- chrono-cache-acceptance/v1";
    let get = |name: &str| -> Result<&str, String> {
        native
            .get(name)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("E_CACHE_ACCEPTANCE: missing {name}"))
    };
    if !response.as_object().is_some_and(|v| v.contains_key("body"))
        || get("GITHUB_EVENT_NAME")? != "pull_request"
        || get("CHRONO_CACHE_PR_NUMBER")?
            .parse::<u64>()
            .ok()
            .is_none_or(|n| n == 0 || response["number"] != n)
        || response["base"]["repo"]["full_name"] != get("GITHUB_REPOSITORY")?
        || response["head"]["sha"]
            .as_str()
            .is_none_or(|s| s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("E_CACHE_ACCEPTANCE: PR response differs from requested identity".into());
    }
    let body = match &response["body"] {
        Value::Null => "",
        Value::String(body) => body,
        _ => return Err("E_CACHE_ACCEPTANCE: PR body is not text".into()),
    };
    let Some((_, rest)) = body.split_once(MARKER) else {
        return Ok(None);
    };
    if body.matches(MARKER).count() != 1 {
        return Err("E_CACHE_ACCEPTANCE: duplicate PR activation block".into());
    }
    let (raw, _) = rest
        .split_once("-->")
        .ok_or("E_CACHE_ACCEPTANCE: unterminated PR activation block")?;
    let activation: Value = decode(raw.trim().as_bytes())?;
    for name in ["GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"] {
        let actual = get(name)?;
        if !actual.bytes().all(|b| b.is_ascii_digit()) || actual.bytes().all(|b| b == b'0') {
            return Err(format!("E_CACHE_ACCEPTANCE: invalid {name}"));
        }
    }
    let run = activation["run_id"]
        .as_str()
        .ok_or("E_CACHE_ACCEPTANCE: run_id missing")?;
    let attempt = activation["run_attempt"]
        .as_str()
        .ok_or("E_CACHE_ACCEPTANCE: run_attempt missing")?;
    if run != get("GITHUB_RUN_ID")? || attempt != get("GITHUB_RUN_ATTEMPT")? {
        return Ok(None);
    }
    let source = get("CHRONO_CACHE_SOURCE_COMMIT")?;
    if source.len() != 40
        || !source.bytes().all(|b| b.is_ascii_hexdigit())
        || response["head"]["sha"] != source
        || activation["schema"] != "chrono-cache-acceptance/v1"
        || activation["fixture"] != fixture
        || activation["consumer"] != consumer
        || activation["source_commit"] != source
        || activation["git_ref"] != get("GITHUB_REF")?
    {
        return Err(
            "E_CACHE_ACCEPTANCE: selected PR activation differs from native source/ref/scope"
                .into(),
        );
    }
    // Forward the original payload; the shared decoder already rejected duplicate
    // fields, and the existing fixture still owns the full activation contract.
    Ok(Some(raw.trim().to_owned()))
}

pub(crate) fn activation_dispatch(args: &[String]) -> Result<String, String> {
    use std::io::Write;
    let mut options = BTreeMap::new();
    for pair in args[1..].chunks(2) {
        if pair.len() != 2
            || ![
                "--host-root",
                "--response",
                "--fixture",
                "--consumer",
                "--github-output",
            ]
            .contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_CACHE_ACCEPTANCE: unknown/duplicate activation reader option".into());
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("E_CACHE_ACCEPTANCE: missing {key}"))
    };
    let root = Path::new(required("--host-root")?);
    let response_path = required("--response")?;
    if !response_path.starts_with(".chrono-harness/state/cache/") {
        return Err("E_CACHE_ACCEPTANCE: response must be retained with cache evidence".into());
    }
    let raw = chrono_harness::units::read_bounded(root, response_path, 1024 * 1024)?;
    let native = [
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_REF",
        "GITHUB_EVENT_NAME",
        "GITHUB_REPOSITORY",
        "CHRONO_CACHE_PR_NUMBER",
        "CHRONO_CACHE_SOURCE_COMMIT",
    ]
    .into_iter()
    .filter_map(|name| std::env::var(name).ok().map(|value| (name.into(), value)))
    .collect();
    let activation = pull_request_activation(
        &decode(&raw)?,
        &native,
        required("--fixture")?,
        required("--consumer")?,
    )?;
    let output = required("--github-output")?;
    let record = if let Some(value) = &activation {
        let delimiter = format!("chrono-cache-{}", chrono_harness::sha256(value.as_bytes()));
        if value.lines().any(|line| line == delimiter) {
            return Err("E_CACHE_ACCEPTANCE: activation output delimiter collision".into());
        }
        format!("activation<<{delimiter}\n{value}\n{delimiter}\n")
    } else {
        "activation=\n".into()
    };
    fs::OpenOptions::new()
        .append(true)
        .open(output)
        .and_then(|mut file| file.write_all(record.as_bytes()))
        .map_err(|e| format!("E_CACHE_ACCEPTANCE: activation output: {e}"))?;
    Ok(serde_json::json!({"response":response_path,"sha256":chrono_harness::sha256(&raw),"selection":if activation.is_some() {"selected"} else {"inactive"}}).to_string() + "\n")
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
        if consumer.acceptance_fixture.as_ref().is_some_and(|id| {
            !name(id) || saves.is_empty() || !consumer.save_after_bootstrap.is_empty()
        }) {
            return Err(format!(
                "E_CACHE_ACCEPTANCE: invalid fixture consumer {job}"
            ));
        }
        if let Some(AcceptanceSource::PullRequestBody { program, reader }) =
            &consumer.acceptance_source
        {
            if consumer.acceptance_fixture.is_none()
                || [program, reader]
                    .iter()
                    .any(|s| s.is_empty() || s.contains(['\0', '\r', '\n']) || s.contains("${{"))
            {
                return Err(format!(
                    "E_CACHE_ACCEPTANCE: invalid activation source for {job}"
                ));
            }
        }
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
        if let Some(id) = &consumer.acceptance_fixture {
            let fixture = &registry["acceptance_fixtures"][id];
            let ids = fixture["caches"]
                .as_array()
                .ok_or_else(|| format!("E_CACHE_ACCEPTANCE: undeclared fixture {id}"))?;
            let saves = consumer.save_caches.as_ref().unwrap_or(&consumer.caches);
            if fixture["method"] != "remove-producer-manifest/v1"
                || fixture["consumer"] != consumer.consumer
                || ids.is_empty()
                || registry["recover_failed_restores"] != true
                || registry["schema"] != "chrono-cache/v2"
                || ids.iter().map(Value::as_str).collect::<BTreeSet<_>>().len() != ids.len()
                || ids.iter().any(|id| {
                    id.as_str().is_none_or(|id| {
                        !saves.iter().any(|s| s == id)
                            || caches.get(id).is_none_or(|c| {
                                c["verify_contents"] != true
                                    || c["artifacts"].as_array().is_none_or(|artifacts| {
                                        artifacts.iter().any(|a| {
                                            a.as_str().is_none_or(|a| {
                                                registry["artifacts"][a]["external"] != false
                                            })
                                        })
                                    })
                            })
                    })
                })
            {
                return Err(format!(
                    "E_CACHE_ACCEPTANCE: fixture {id} differs from saving consumer"
                ));
            }
        }
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn project(
    c: Option<&Config>,
    job: &str,
    body: String,
    before: &str,
    work: &str,
    existing_work_id: Option<&str>,
    existing_bootstrap_id: Option<&str>,
    after: Option<&str>,
) -> Result<String, String> {
    let Some(c) = c else {
        return Ok(body);
    };
    let Some(consumer) = c.jobs.get(job) else {
        return Ok(body);
    };
    let work_id = existing_work_id.unwrap_or("chrono_cache_work");
    let bootstrap_id = existing_bootstrap_id.unwrap_or("chrono_cache_bootstrap");
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
    let acceptance_variable = if consumer.acceptance_source.is_some() {
        "steps.chrono_cache_activation.outputs.activation"
    } else {
        "vars[format('CHRONO_CACHE_ACCEPTANCE_{0}_{1}', github.run_id, github.run_attempt)]"
    };
    for (index, id) in selected.into_iter().enumerate() {
        let key = format!("cache_{index}");
        let step = &key;
        prefix.push_str(&format!("      - name: Restore registered cache {id}\n        id: {step}_restore\n        continue-on-error: true\n        uses: {}\n        with:\n          key: ${{{{ steps.chrono_cache_plan.outputs.{key}_key }}}}\n          path: ${{{{ steps.chrono_cache_plan.outputs.{key}_paths }}}}\n          restore-keys: ${{{{ steps.chrono_cache_plan.outputs.{key}_restore }}}}\n          fail-on-cache-miss: false\n", c.restore_action));
        if !saves.contains(id) {
            continue;
        }
        let producer_step = if consumer.save_after_bootstrap.contains(id) {
            bootstrap_id
        } else {
            work_id
        };
        let fixture_guard = if consumer.acceptance_fixture.is_some() {
            let guard = format!(
                " && ({acceptance_variable} == '' || steps.chrono_cache_acceptance.outcome == 'success')"
            );
            if consumer.acceptance_source.is_some() {
                format!(
                    " && (github.event_name != 'pull_request' || !contains(github.event.pull_request.body, '<!-- chrono-cache-acceptance-source/v1 -->') || steps.chrono_cache_activation.outcome == 'success'){guard}"
                )
            } else {
                guard
            }
        } else {
            String::new()
        };
        suffix.push_str(&format!("      - name: Save registered cache {id}\n        id: {step}_save\n        if: ${{{{ always() && !cancelled() && steps.chrono_cache_plan.outcome == 'success' && steps.{producer_step}.outcome == 'success' && steps.{step}_restore.outputs.cache-matched-key != steps.chrono_cache_plan.outputs.{key}_key && (steps.chrono_cache_plan.outputs.{key}_verification != 'producer-manifest/v1' || steps.chrono_cache_seal.outputs.{key}_sealed == 'true'){fixture_guard} }}}}\n        continue-on-error: true\n        uses: {}\n        with:\n          key: ${{{{ steps.chrono_cache_plan.outputs.{key}_key }}}}\n          path: ${{{{ steps.chrono_cache_plan.outputs.{key}_paths }}}}\n", c.save_action));
    }
    prefix.push_str(&format!("      - name: Recover failed cache restores\n        if: ${{{{ steps.chrono_cache_plan.outputs.cache_recovery == 'discard-failed-host-restores' }}}}\n        shell: bash\n        env:\n          CHRONO_CACHE_RESTORE_STEPS: ${{{{ toJSON(steps) }}}}\n        run: |\n          {} recover --host-root . --config {} --plan {} --steps-env CHRONO_CACHE_RESTORE_STEPS\n", shell(&c.program), shell(&c.config), shell(&plan_path)));
    let bootstrap_arg = if consumer.save_after_bootstrap.is_empty() {
        String::new()
    } else {
        format!(" --bootstrap {bootstrap_id}")
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
    let bootstrap_seal_arg = if consumer.save_after_bootstrap.is_empty() {
        String::new()
    } else {
        format!(
            " --bootstrap {bootstrap_id} --save-after-bootstrap {}",
            shell(&serde_json::to_string(&consumer.save_after_bootstrap).unwrap())
        )
    };
    let mut seal = format!(
        "      - name: Seal producer cache contents\n        id: chrono_cache_seal\n        if: ${{{{ always() && !cancelled() && steps.chrono_cache_plan.outputs.cache_content_verification == 'producer-manifest/v1' }}}}\n        shell: bash\n        env:\n          CHRONO_CACHE_PRODUCER_STEPS: ${{{{ toJSON(steps) }}}}\n        run: |\n          {} seal --host-root . --config {} --plan {} --steps-env CHRONO_CACHE_PRODUCER_STEPS --work {}{bootstrap_seal_arg}{saves_arg} --github-output \"$GITHUB_OUTPUT\"\n",
        shell(&c.program),
        shell(&c.config),
        shell(&plan_path),
        shell(work_id)
    );
    if let Some(fixture) = &consumer.acceptance_fixture {
        if let Some(AcceptanceSource::PullRequestBody { program, reader }) =
            &consumer.acceptance_source
        {
            let response = format!(
                ".chrono-harness/state/cache/{}/activation-pr.json",
                consumer.consumer
            );
            seal.push_str(&format!("      - name: Read selected native cache activation\n        id: chrono_cache_activation\n        if: ${{{{ success() && github.event_name == 'pull_request' && contains(github.event.pull_request.body, '<!-- chrono-cache-acceptance-source/v1 -->') }}}}\n        timeout-minutes: 1\n        shell: bash\n        env:\n          GH_TOKEN: ${{{{ github.token }}}}\n          CHRONO_CACHE_PR_NUMBER: ${{{{ github.event.pull_request.number }}}}\n          CHRONO_CACHE_SOURCE_COMMIT: ${{{{ github.event.pull_request.head.sha || github.event.after || github.sha }}}}\n        run: |\n          {} api \"repos/$GITHUB_REPOSITORY/pulls/$CHRONO_CACHE_PR_NUMBER\" > {}\n          {} cache-activation --host-root . --response {} --fixture {} --consumer {} --github-output \"$GITHUB_OUTPUT\"\n", shell(program), shell(&response), shell(reader), shell(&response), shell(fixture), shell(&consumer.consumer)));
        }
        seal.push_str(&format!("      - name: Apply selected native cache acceptance fixture\n        id: chrono_cache_acceptance\n        if: ${{{{ success() && {acceptance_variable} != '' }}}}\n        shell: bash\n        env:\n          CHRONO_CACHE_ACCEPTANCE: ${{{{ {acceptance_variable} }}}}\n          CHRONO_CACHE_SOURCE_COMMIT: ${{{{ github.event.pull_request.head.sha || github.event.after || github.sha }}}}\n          CHRONO_CACHE_ACCEPTANCE_STEPS: ${{{{ toJSON(steps) }}}}\n        run: |\n          {} acceptance-fixture --host-root . --config {} --plan {} --fixture {} --steps-env CHRONO_CACHE_ACCEPTANCE_STEPS --work {}\n", shell(&c.program), shell(&c.config), shell(&plan_path), shell(fixture), shell(work_id)));
    }
    suffix.insert_str(0, &seal);
    suffix.push_str(&format!("      - name: Record original cache transport observations\n        if: ${{{{ always() && steps.chrono_cache_plan.outcome == 'success' }}}}\n        shell: bash\n        env:\n          CHRONO_CACHE_STEPS: ${{{{ toJSON(steps) }}}}\n          GH_TOKEN: ${{{{ github.token }}}}\n        run: |\n          {} report --host-root . --plan {} --report-directory {} --steps-env CHRONO_CACHE_STEPS --work {}{bootstrap_arg}{saves_arg}\n", shell(&c.program), shell(&plan_path), shell(&evidence_directory), shell(work_id)));
    let marker = format!("      - name: {before}\n");
    let work_marker = format!("      - name: {work}\n");
    if body.matches(&marker).count() != 1 || body.matches(&work_marker).count() != 1 {
        return Err("persistent cache requires one original bootstrap/work step".into());
    }
    let body = if consumer.acceptance_source.is_some() {
        body.replacen("    steps:\n", "    permissions:\n      contents: read\n      actions: read\n      pull-requests: read\n    steps:\n", 1)
    } else {
        body
    };
    let mut body = if existing_work_id.is_none() {
        body.replacen(
            &work_marker,
            &format!("{work_marker}        id: {work_id}\n"),
            1,
        )
    } else {
        body
    };
    if !consumer.save_after_bootstrap.is_empty() && existing_bootstrap_id.is_none() {
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
