//! One bounded native acceptance operation, after seal and before the native save.
use chrono_harness::{decode, no_symlink_parents, prepared, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

pub const METHOD: &str = "remove-producer-manifest/v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub method: String,
    pub consumer: String,
    pub caches: Vec<String>,
}

pub(crate) fn validate(config: &super::Config) -> Result<(), String> {
    for (id, fixture) in &config.acceptance_fixtures {
        super::name(id)?;
        super::unique(&fixture.caches)?;
        if fixture.method != METHOD
            || fixture.caches.is_empty()
            || !config.require_primary_checkout
            || !config.recover_failed_restores
            || config.schema != super::CONSUMER_SCHEMA
            || fixture.caches.iter().any(|id| {
                config.caches.get(id).is_none_or(|cache| {
                    !cache.verify_contents
                        || !cache.consumers.contains(&fixture.consumer)
                        || cache
                            .artifacts
                            .iter()
                            .any(|id| config.artifacts.get(id).is_none_or(|a| a.external))
                })
            })
        {
            return Err(format!("E_CACHE_ACCEPTANCE: invalid fixture {id}"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
    schema: String,
    fixture: String,
    consumer: String,
    keys: BTreeMap<String, String>,
    git_ref: String,
    run_id: String,
    run_attempt: String,
    source_commit: String,
}

fn publish_once(root: &Path, path: &str, value: &Value) -> Result<(), String> {
    let target = chrono_harness::prepare_publication(root, path)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .and_then(|mut file| file.write_all(&serde_json::to_vec(value)?))
        .map_err(|e| format!("E_CACHE_ACCEPTANCE: {path}: {e}"))
}

pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut options = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || ![
                "--host-root",
                "--config",
                "--plan",
                "--fixture",
                "--steps-env",
                "--work",
            ]
            .contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_CACHE_ACCEPTANCE: invalid or duplicate option".into());
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("E_CACHE_ACCEPTANCE: missing {key}"))
    };
    let root = fs::canonicalize(required("--host-root")?).map_err(|e| e.to_string())?;
    let plan_path = required("--plan")?;
    if !plan_path.starts_with(".chrono-harness/state/") {
        return Err("E_CACHE_ACCEPTANCE: expected retained plan".into());
    }
    let raw_plan = fs::read(no_symlink_parents(&root, plan_path)?).map_err(|e| e.to_string())?;
    let plan: Value = decode(&raw_plan)?;
    let config = super::content::revalidate(&root, required("--config")?, &plan)?;
    let fixture_id = required("--fixture")?;
    let fixture = config
        .acceptance_fixtures
        .get(fixture_id)
        .ok_or("E_CACHE_ACCEPTANCE: undeclared fixture")?;
    let current = super::current_executable_identity()?;
    if current != plan["planner"] {
        return Err("E_CACHE_ACCEPTANCE: executable differs from planner".into());
    }
    let activation_raw = std::env::var("CHRONO_CACHE_ACCEPTANCE")
        .map_err(|e| format!("E_CACHE_ACCEPTANCE: activation missing: {e}"))?;
    let activation: Activation = decode(activation_raw.as_bytes())?;
    let native = [
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_REF",
        "GITHUB_SHA",
    ]
    .into_iter()
    .map(|name| {
        let value =
            std::env::var(name).map_err(|_| format!("E_CACHE_ACCEPTANCE: missing {name}"))?;
        if value.is_empty() {
            return Err(format!("E_CACHE_ACCEPTANCE: empty {name}"));
        }
        Ok((name, value))
    })
    .collect::<Result<BTreeMap<_, _>, String>>()?;
    if activation.schema != "chrono-cache-acceptance/v1"
        || activation.fixture != fixture_id
        || activation.consumer != fixture.consumer
        || plan["consumer"] != fixture.consumer
        || activation.keys.is_empty()
        || activation.git_ref != native["GITHUB_REF"]
        || activation.run_id != native["GITHUB_RUN_ID"]
        || activation.run_attempt != native["GITHUB_RUN_ATTEMPT"]
        || activation.source_commit != native["GITHUB_SHA"]
        || !native["GITHUB_RUN_ID"].bytes().all(|b| b.is_ascii_digit())
        || !native["GITHUB_RUN_ATTEMPT"]
            .bytes()
            .all(|b| b.is_ascii_digit())
        || native["GITHUB_RUN_ATTEMPT"] == "0"
        || native["GITHUB_SHA"].len() != 40
        || !native["GITHUB_SHA"].bytes().all(|b| b.is_ascii_hexdigit())
        || activation
            .keys
            .iter()
            .any(|(id, key)| !fixture.caches.contains(id) || plan["caches"][id]["key"] != *key)
    {
        return Err("E_CACHE_ACCEPTANCE: activation differs from registered scope, native identity or plan keys".into());
    }
    let steps_raw = std::env::var(required("--steps-env")?).map_err(|e| e.to_string())?;
    let steps: Value = decode(steps_raw.as_bytes())?;
    if steps[required("--work")?]["outcome"] != "success"
        || steps["chrono_cache_seal"]["outcome"] != "success"
    {
        return Err("E_CACHE_ACCEPTANCE: original work and seal must succeed".into());
    }
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_ACCEPTANCE: plan directory")?
        .0;
    let seal_pointer: prepared::Original = decode(
        &fs::read(no_symlink_parents(
            &root,
            &format!("{directory}/seal.json"),
        )?)
        .map_err(|e| e.to_string())?,
    )?;
    let raw_seal = prepared::read_original(&root, &seal_pointer, None)?;
    let seal: Value = decode(&raw_seal)?;
    let binding = json!({"path":plan_path,"sha256":sha256(&raw_plan)});
    if seal["schema"] != "chrono-cache-seal/v1" || seal["plan"] != binding {
        return Err("E_CACHE_ACCEPTANCE: seal belongs to another plan".into());
    }
    let intent_path = format!("{directory}/acceptance-intent.json");
    match fs::symlink_metadata(no_symlink_parents(&root, &intent_path)?) {
        Ok(_) => return Err("E_CACHE_ACCEPTANCE: prior intent exists; no repeat".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    // Preflight ALL selected scopes, preserving the exact manifest originals supplied by seal.
    let mut removals = Vec::new();
    for (id, key) in &activation.keys {
        let index = plan["caches"]
            .as_object()
            .unwrap()
            .keys()
            .position(|k| k == id)
            .unwrap();
        if steps[format!("cache_{index}_restore")]["outputs"]["cache-matched-key"] == *key {
            return Err(
                "E_CACHE_ACCEPTANCE: exact archive already exists; native save would skip".into(),
            );
        }
        let cache = &plan["caches"][id];
        if seal["caches"][id]["status"] != "sealed"
            || seal["caches"][id]["producer"] != cache["producer"]
        {
            return Err("E_CACHE_ACCEPTANCE: selected producer was not sealed".into());
        }
        for (artifact_id, artifact) in cache["artifacts"].as_object().unwrap() {
            let path = format!(
                "{}/{}",
                artifact["path"].as_str().unwrap().trim_end_matches('/'),
                super::content::MARKER
            );
            let target = no_symlink_parents(&root, &path)?;
            if !fs::symlink_metadata(&target)
                .map_err(|e| e.to_string())?
                .is_file()
            {
                return Err("E_CACHE_ACCEPTANCE: manifest is not a physical file".into());
            }
            let original: prepared::Original = decode(
                &serde_json::to_vec(&seal["caches"][id]["manifests"][artifact_id])
                    .map_err(|e| e.to_string())?,
            )?;
            let preimage = prepared::read_original(&root, &original, None)?;
            if fs::read(&target).map_err(|e| e.to_string())? != preimage {
                return Err("E_CACHE_ACCEPTANCE: manifest differs from seal preimage".into());
            }
            let manifest: Value = decode(&preimage)?;
            if manifest["schema"] != super::content::METHOD
                || manifest["key"] != *key
                || manifest["cache"] != *id
                || manifest["artifact_id"] != *artifact_id
                || manifest["artifact"] != *artifact
                || manifest["producer"] != cache["producer"]
            {
                return Err("E_CACHE_ACCEPTANCE: preimage binding differs".into());
            }
            removals.push(json!({"cache":id,"key":key,"artifact":artifact_id,"path":path,"preimage":original,"status":"pending"}));
        }
    }
    let directory = format!("{directory}/");
    let mut report = json!({"schema":"chrono-cache-acceptance-result/v1","fixture":fixture_id,"method":METHOD,
        "plan":binding,"seal":seal_pointer,"native":native,"executable":current,
        "activation":super::report::retain_for_upload(&root,&directory,"acceptance-activation",activation_raw.as_bytes())?,
        "native_steps":super::report::retain_for_upload(&root,&directory,"acceptance-steps",steps_raw.as_bytes())?,
        "status":"prepared","removals":removals,"error":null,
        "limits":["deliberately unusable payload; native save and later native restore remain separately unverified","missing manifest tests content recovery, not archive extraction failure or backend outage"]});
    publish_once(&root, &intent_path, &report)?;
    let mut failure = None;
    for removal in report["removals"].as_array_mut().unwrap() {
        let path = removal["path"].as_str().unwrap();
        let result = (|| -> Result<(), String> {
            let target = no_symlink_parents(&root, path)?;
            let original: prepared::Original =
                decode(&serde_json::to_vec(&removal["preimage"]).map_err(|e| e.to_string())?)?;
            if fs::read(&target).map_err(|e| e.to_string())?
                != prepared::read_original(&root, &original, None)?
            {
                return Err("manifest changed after preflight".into());
            }
            fs::remove_file(&target).map_err(|e| e.to_string())?;
            match fs::symlink_metadata(&target) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
                Ok(_) => return Err("manifest still present".into()),
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                removal["status"] = json!("removed");
                removal["postimage"] = json!("absent");
            }
            Err(error) => {
                removal["status"] = json!("failed");
                removal["error"] = json!(error);
                failure = Some(error);
                break;
            }
        }
    }
    report["status"] = json!(if failure.is_some() {
        "failed"
    } else {
        "applied"
    });
    report["error"] = json!(failure);
    let original = super::report::retain_for_upload(
        &root,
        &directory,
        "acceptance-result",
        &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
    )?;
    publish_once(
        &root,
        &format!("{directory}acceptance.json"),
        &json!(original),
    )?;
    if let Some(error) = failure {
        return Err(format!(
            "E_CACHE_ACCEPTANCE: {error}; original {}",
            original.path
        ));
    }
    Ok(
        serde_json::to_string(&json!({"report":original,"observation":report}))
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
