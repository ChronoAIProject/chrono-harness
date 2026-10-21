//! Producer-bound inventories for explicitly opted-in primary-checkout caches.
//! Inventories describe output bytes; they contain no cached execution verdicts.
use chrono_harness::{decode, file_identity, no_symlink_parents};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, time::Instant};

pub(crate) const MARKER: &str = ".chrono-cache-content-v1.json";
pub(crate) const METHOD: &str = "producer-manifest/v1";

fn inventory(root: &Path) -> Result<Value, String> {
    fn visit(
        root: &Path,
        relative: &Path,
        files: &mut BTreeMap<String, Value>,
    ) -> Result<(), String> {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        let key = relative
            .to_str()
            .ok_or("non-UTF8 cache content path")?
            .to_owned();
        let value = if metadata.is_file() {
            let (sha256, bytes) = file_identity(&path)?;
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111
            };
            #[cfg(not(unix))]
            let executable = 0;
            json!({"kind":"file","sha256":sha256,"bytes":bytes,"executable":executable})
        } else if metadata.is_symlink() {
            let target = fs::read_link(&path).map_err(|e| e.to_string())?;
            json!({"kind":"symlink","target":target.to_str().ok_or("non-UTF8 cache symlink")?})
        } else if metadata.is_dir() {
            json!({"kind":"directory"})
        } else {
            return Err(format!(
                "unsupported cache content type: {}",
                path.display()
            ));
        };
        files.insert(key, value);
        if metadata.is_dir() {
            for child in fs::read_dir(path).map_err(|e| e.to_string())? {
                let child = child.map_err(|e| e.to_string())?;
                if relative.as_os_str().is_empty() && child.file_name() == MARKER {
                    continue;
                }
                visit(root, &relative.join(child.file_name()), files)?;
            }
        }
        Ok(())
    }
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err("cache artifact must be a physical directory".into());
    }
    let mut files = BTreeMap::new();
    visit(root, Path::new(""), &mut files)?;
    Ok(json!(files))
}

pub(crate) fn revalidate(
    root: &Path,
    config_path: &str,
    plan: &Value,
) -> Result<super::Config, String> {
    if !config_path.starts_with(".chrono-harness/") {
        return Err("E_CACHE_CONTENT: expected host registration".into());
    }
    let config: super::Config =
        decode(&fs::read(no_symlink_parents(root, config_path)?).map_err(|e| e.to_string())?)?;
    let consumer = plan["consumer"]
        .as_str()
        .ok_or("E_CACHE_CONTENT: consumer missing")?;
    let mut expected = super::prepare_with_inputs(root, &config, consumer, Some(&plan["inputs"]))?;
    expected["planner"] = plan["planner"].clone();
    if expected != *plan {
        return Err("E_CACHE_CONTENT: plan differs from current registration or host".into());
    }
    Ok(config)
}

/// Keep the producer manifest and observed inventory before recovery removes outputs.
pub(crate) fn verify(
    root: &Path,
    directory: &str,
    id: &str,
    cache: &Value,
    matched: &str,
) -> Result<Value, String> {
    let start = Instant::now();
    let mut observations = BTreeMap::new();
    let mut valid = true;
    for (artifact_id, artifact) in cache["artifacts"]
        .as_object()
        .ok_or("E_CACHE_CONTENT: artifacts missing")?
    {
        let target = super::path(
            root,
            artifact["path"]
                .as_str()
                .ok_or("E_CACHE_CONTENT: artifact path")?,
        )?;
        let observed = inventory(&target);
        let marker = target.join(MARKER);
        // Refuse a linked marker; reading it must never follow an archive-supplied link.
        let raw = match fs::symlink_metadata(&marker) {
            Ok(metadata) if metadata.is_file() => fs::read(&marker).map_err(|e| e.to_string()),
            Ok(_) => Err("manifest is not a regular file".into()),
            Err(e) => Err(e.to_string()),
        };
        let mut row =
            json!({"observed":observed.as_ref().ok(),"observation_error":observed.as_ref().err()});
        let matches = match raw {
            Ok(raw) => {
                row["manifest_original"] = json!(super::report::retain_for_upload(
                    root,
                    directory,
                    "content-manifest",
                    &raw
                )?);
                match decode::<Value>(&raw) {
                    Ok(expected) => {
                        expected
                            == json!({"schema":METHOD,"cache":id,"producer":cache["producer"],"key":matched,
                            "artifact_id":artifact_id,"artifact":artifact,"files":observed.as_ref().ok()})
                            && observed.is_ok()
                    }
                    Err(error) => {
                        row["manifest_error"] = json!(error);
                        false
                    }
                }
            }
            Err(error) => {
                row["manifest_error"] = json!(error);
                false
            }
        };
        valid &= matches;
        row["status"] = json!(if matches { "matched" } else { "unusable" });
        observations.insert(artifact_id, row);
    }
    let raw = serde_json::to_vec(&json!({"schema":METHOD,"cache":id,"producer":cache["producer"],"matched_key":matched,"artifacts":observations})).map_err(|e| e.to_string())?;
    let original = super::report::retain_for_upload(root, directory, "content-verification", &raw)?;
    Ok(
        json!({"method":METHOD,"status":if valid {"matched"} else {"unusable"},"original":original,
        "elapsed_seconds":start.elapsed().as_secs_f64(),"measurement":"wall-clock inventory, manifest read and evidence publication"}),
    )
}

pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut options = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || ![
                "--host-root",
                "--config",
                "--plan",
                "--cache",
                "--steps-env",
                "--work",
                "--bootstrap",
                "--save-caches",
                "--save-after-bootstrap",
                "--github-output",
            ]
            .contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_CACHE_CONTENT: invalid or duplicate seal option".into());
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("E_CACHE_CONTENT: missing {key}"))
    };
    let root = fs::canonicalize(required("--host-root")?).map_err(|e| e.to_string())?;
    let plan_path = required("--plan")?;
    if !plan_path.starts_with(".chrono-harness/state/") {
        return Err("E_CACHE_CONTENT: expected retained state plan".into());
    }
    let plan_raw = fs::read(no_symlink_parents(&root, plan_path)?).map_err(|e| e.to_string())?;
    let plan: Value = decode(&plan_raw)?;
    revalidate(&root, required("--config")?, &plan)?;
    let current = super::current_executable_identity()?;
    if current["sha256"] != plan["planner"]["sha256"]
        || current["version"] != plan["planner"]["version"]
    {
        return Err("E_CACHE_CONTENT: seal executable differs from planner".into());
    }
    let steps_raw = std::env::var(required("--steps-env")?).map_err(|e| e.to_string())?;
    let steps: Value = decode(steps_raw.as_bytes())?;
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_CONTENT: plan directory")?
        .0
        .to_owned()
        + "/";
    let saves: Option<Vec<String>> = options
        .get("--save-caches")
        .map(|s| decode(s.as_bytes()))
        .transpose()?;
    let bootstrap_saves: Vec<String> = options
        .get("--save-after-bootstrap")
        .map(|s| decode(s.as_bytes()))
        .transpose()?
        .unwrap_or_default();
    let caches = plan["caches"]
        .as_object()
        .ok_or("E_CACHE_CONTENT: caches missing")?;
    for ids in [saves.as_deref().unwrap_or_default(), &bootstrap_saves] {
        let mut seen = std::collections::BTreeSet::new();
        if ids
            .iter()
            .any(|id| !caches.contains_key(id) || !seen.insert(id))
        {
            return Err("E_CACHE_CONTENT: invalid cache selection".into());
        }
    }
    if options
        .get("--cache")
        .is_some_and(|id| !caches.contains_key(*id))
    {
        return Err("E_CACHE_CONTENT: unknown cache".into());
    }
    let mut report = json!({"schema":"chrono-cache-seal/v1","plan":{"path":plan_path,"sha256":chrono_harness::sha256(&plan_raw)},
        "native_steps":super::report::retain_for_upload(&root,&directory,"producer-steps",steps_raw.as_bytes())?,"caches":{}});
    let mut outputs = String::new();
    for (index, (id, cache)) in caches.iter().enumerate() {
        if cache["content_verification"] != METHOD
            || options
                .get("--cache")
                .is_some_and(|selected| *selected != id)
            || saves.as_ref().is_some_and(|saves| !saves.contains(id))
        {
            continue;
        }
        let work = if bootstrap_saves.contains(id) {
            required("--bootstrap")?
        } else {
            required("--work")?
        };
        if steps[work]["outcome"] != "success" {
            return Err(format!(
                "E_CACHE_CONTENT: producer step {work} did not succeed for {id}"
            ));
        }
        let start = Instant::now();
        let mut manifests = BTreeMap::new();
        for (artifact_id, artifact) in cache["artifacts"]
            .as_object()
            .ok_or("E_CACHE_CONTENT: artifacts missing")?
        {
            let relative = artifact["path"]
                .as_str()
                .ok_or("E_CACHE_CONTENT: artifact path")?;
            let target = super::path(&root, relative)?;
            let files =
                inventory(&target).map_err(|e| format!("E_CACHE_CONTENT: {relative}: {e}"))?;
            let manifest = json!({"schema":METHOD,"cache":id,"producer":cache["producer"],"key":cache["key"],"artifact_id":artifact_id,"artifact":artifact,"files":files});
            let raw = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
            // Publication validates marker and parent links; a failed seal never authorizes save.
            let marker = chrono_harness::prepare_publication(
                &root,
                &format!("{}/{MARKER}", relative.trim_end_matches('/')),
            )?;
            fs::write(marker, &raw).map_err(|e| e.to_string())?;
            manifests.insert(
                artifact_id,
                super::report::retain_for_upload(&root, &directory, "producer-manifest", &raw)?,
            );
        }
        report["caches"][id] = json!({"status":"sealed","producer":cache["producer"],"manifests":manifests,"elapsed_seconds":start.elapsed().as_secs_f64(),"measurement":"wall-clock content hashing and manifest publication"});
        outputs.push_str(&format!("cache_{index}_sealed=true\n"));
    }
    let raw = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let original = super::report::retain_for_upload(&root, &directory, "producer-seal", &raw)?;
    let pointer = chrono_harness::prepare_publication(&root, &format!("{directory}seal.json"))?;
    fs::write(
        pointer,
        serde_json::to_vec(&original).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Some(output) = options.get("--github-output") {
        use std::io::Write;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(output)
            .and_then(|mut f| f.write_all(outputs.as_bytes()))
            .map_err(|e| e.to_string())?;
    }
    Ok(
        serde_json::to_string(&json!({"report":original,"observation":report}))
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
