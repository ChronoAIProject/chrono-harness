//! Discard registered host outputs after unusable native restores, before producers run.
use chrono_harness::{decode, no_symlink_parents, prepared, sha256};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

fn publish_once(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let destination = chrono_harness::prepare_publication(root, path)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| format!("E_CACHE_RECOVERY: {path}: {e}"))
}

pub(crate) fn retained(
    root: &Path,
    plan_path: &str,
) -> Result<(prepared::Original, Value), String> {
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_RECOVERY: plan directory")?
        .0;
    let pointer = fs::read(no_symlink_parents(
        root,
        &format!("{directory}/recovery.json"),
    )?)
    .map_err(|e| format!("E_CACHE_RECOVERY: missing recovery result: {e}"))?;
    let original: prepared::Original = decode(&pointer)?;
    let raw = prepared::read_original(root, &original, None)?;
    Ok((original, decode(&raw)?))
}

pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut options = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || !["--host-root", "--config", "--plan", "--steps-env"].contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option".into());
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("E_CACHE_RECOVERY: missing {key}"))
    };
    let root = fs::canonicalize(required("--host-root")?).map_err(|e| e.to_string())?;
    let config_path = required("--config")?;
    let plan_path = required("--plan")?;
    if !config_path.starts_with(".chrono-harness/")
        || !plan_path.starts_with(".chrono-harness/state/")
    {
        return Err("E_CACHE_RECOVERY: expected host config and retained state plan".into());
    }
    let plan_raw = fs::read(no_symlink_parents(&root, plan_path)?).map_err(|e| e.to_string())?;
    let plan: Value = decode(&plan_raw)?;
    let steps_raw = std::env::var(required("--steps-env")?).map_err(|e| e.to_string())?;
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_RECOVERY: plan directory")?
        .0
        .to_owned()
        + "/";
    let output = format!("{directory}recovery.json");
    let intent_path = format!("{directory}recovery-intent.json");
    let binding = json!({"path":plan_path,"sha256":sha256(&plan_raw)});
    let steps_hash = sha256(steps_raw.as_bytes());
    let output_file = no_symlink_parents(&root, &output)?;
    match fs::symlink_metadata(&output_file) {
        Ok(_) => {
            let (original, report) = retained(&root, plan_path)?;
            if report["schema"] != "chrono-cache-recovery/v1"
                || report["plan"] != binding
                || report["native_steps"]["sha256"] != steps_hash
            {
                return Err(
                    "E_CACHE_RECOVERY: previous recovery belongs to different inputs".into(),
                );
            }
            if !matches!(
                report["status"].as_str(),
                Some("recovered" | "not-required")
            ) {
                return Err(format!(
                    "E_CACHE_RECOVERY: previous attempt failed; original {}",
                    original.path
                ));
            }
            // This is the previous operation's result, never a second deletion of
            // outputs that a later producer may already have rebuilt.
            return Ok(serde_json::to_string(
                &json!({"report":original,"observation":report,"replayed":true}),
            )
            .map_err(|e| e.to_string())?
                + "\n");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("E_CACHE_RECOVERY: result metadata: {e}")),
    }
    match fs::symlink_metadata(no_symlink_parents(&root, &intent_path)?) {
        Ok(_) => {
            return Err(format!(
                "E_CACHE_RECOVERY: unfinished original intent {intent_path}; no automatic repeat"
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("E_CACHE_RECOVERY: intent metadata: {e}")),
    }
    let steps_original =
        super::report::retain_for_upload(&root, &directory, "restore-steps", steps_raw.as_bytes())?;
    let mut report = json!({"schema":"chrono-cache-recovery/v1","plan":binding,"native_steps":steps_original,
        "status":"not-required","caches":{},"error":null,
        "limits":["isolated provider restore phase, before producers", "registered native observations only; no archive integrity or build verdict", "missing matched keys do not distinguish miss from backend or extraction failure", "linked worktree and external artifact recovery unsupported"]});
    let result = (|| -> Result<(), String> {
        let config: super::Config =
            decode(&fs::read(no_symlink_parents(&root, config_path)?).map_err(|e| e.to_string())?)?;
        if !config.recover_failed_restores {
            return Err("E_CACHE_RECOVERY: recovery is not registered".into());
        }
        let consumer = plan["consumer"]
            .as_str()
            .ok_or("E_CACHE_RECOVERY: consumer missing")?;
        let expected = super::prepare_with_inputs(&root, &config, consumer, Some(&plan["inputs"]))?;
        // Recovery may run from the installed cache binary while the plan was
        // prepared by the same release's bootstrap copy.  The report command
        // performs the strict planner/current executable check; recovery binds
        // the registration, host and selected observations independently.
        let mut expected_binding = expected;
        expected_binding["planner"] = plan["planner"].clone();
        if expected_binding != plan {
            return Err(
                "E_CACHE_RECOVERY: original plan differs from current registration or host".into(),
            );
        }
        let steps: Value = decode(steps_raw.as_bytes())?;
        if !steps.is_object() {
            return Err("E_CACHE_RECOVERY: native step observations missing".into());
        }
        let caches: BTreeMap<_, _> = plan["caches"]
            .as_object()
            .ok_or("E_CACHE_RECOVERY: caches missing")?
            .iter()
            .collect();
        let mut paths = vec![];
        for (index, (id, cache)) in caches.into_iter().enumerate() {
            let restore = &steps[format!("cache_{index}_restore")];
            let reason = super::report::restore_status(cache, restore);
            let discard = match restore["outcome"].as_str() {
                Some("failure") => true,
                Some("success") if config.recover_unconfirmed_restores => {
                    let outputs = &restore["outputs"];
                    if (!outputs.is_null() && !outputs.is_object())
                        || outputs
                            .get("cache-matched-key")
                            .is_some_and(|key| !key.is_null() && !key.is_string())
                    {
                        return Err(format!(
                            "E_CACHE_RECOVERY: malformed restore outputs for {id}"
                        ));
                    }
                    matches!(reason, "miss-or-unavailable" | "incompatible")
                }
                Some("success" | "skipped") => false,
                _ => {
                    return Err(format!(
                        "E_CACHE_RECOVERY: missing, cancelled or unknown restore outcome for {id}"
                    ));
                }
            };
            report["caches"][id] = json!({"restore":restore,"reason":reason,"status":if discard {"pending"} else {"not-required"},"artifacts":[]});
            if !discard {
                continue;
            }
            let artifacts: BTreeMap<_, _> = cache["artifacts"]
                .as_object()
                .ok_or("E_CACHE_RECOVERY: artifacts missing")?
                .iter()
                .collect();
            for (_, artifact) in artifacts {
                let path = artifact["path"]
                    .as_str()
                    .ok_or("E_CACHE_RECOVERY: artifact path")?;
                let target = super::path(&root, path)?;
                if !target.starts_with(&root) || artifact["external"] != false {
                    return Err(
                        "E_CACHE_RECOVERY: only registered host artifacts may be discarded".into(),
                    );
                }
                let position = report["caches"][id]["artifacts"].as_array().unwrap().len();
                report["caches"][id]["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"path":path,"status":"pending"}));
                paths.push((id.clone(), position, path.to_owned()));
            }
        }
        // Validate the complete selected set before the first removal, and
        // publish the original plan/restore binding before touching any output.
        publish_once(
            &root,
            &intent_path,
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )?;
        for (id, position, path) in paths {
            let removed = (|| -> Result<&str, String> {
                let target = super::path(&root, &path)?;
                match fs::symlink_metadata(&target) {
                    Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(target),
                    Ok(metadata) if metadata.is_file() => fs::remove_file(target),
                    Ok(_) => {
                        return Err(
                            "cache artifact is neither a regular file nor a directory".into()
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok("absent"),
                    Err(e) => return Err(e.to_string()),
                }
                .map_err(|e| e.to_string())?;
                Ok("removed")
            })();
            let row = &mut report["caches"][&id]["artifacts"][position];
            match removed {
                Ok(status) => row["status"] = json!(status),
                Err(error) => {
                    row["status"] = json!("failed");
                    row["error"] = json!(error);
                    return Err(format!("E_CACHE_RECOVERY: {path}: {error}"));
                }
            }
        }
        for cache in report["caches"].as_object_mut().unwrap().values_mut() {
            if cache["status"] == "pending" {
                cache["status"] = json!("discarded");
            }
        }
        if report["caches"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v["status"] == "discarded")
        {
            report["status"] = json!("recovered");
        }
        Ok(())
    })();
    if let Err(error) = &result {
        report["status"] = json!("failed");
        report["error"] = json!(error);
    }
    let raw = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let publication = (|| {
        let original = super::report::retain_for_upload(&root, &directory, "cache-recovery", &raw)?;
        publish_once(
            &root,
            &output,
            &serde_json::to_vec(&original).map_err(|e| e.to_string())?,
        )?;
        Ok::<_, String>(original)
    })();
    let original = publication.map_err(|publication| {
        format!(
            "E_CACHE_RECOVERY: recovery {:?}; publication {publication}",
            result.as_ref().err()
        )
    })?;
    if let Err(error) = result {
        return Err(format!("{error}; original {}", original.path));
    }
    let changed_or_failed = report["caches"].as_object().unwrap().values().any(|cache| {
        cache["restore"]["outcome"] == "failure"
            || cache["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["status"] == "removed")
    });
    if report["status"] == "recovered" && changed_or_failed {
        eprintln!(
            "W_CACHE_RESTORE_RECOVERED: unusable restore outputs cleared before registered build; original {}",
            original.path
        );
    }
    Ok(
        serde_json::to_string(&json!({"report":original,"observation":report,"replayed":false}))
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
