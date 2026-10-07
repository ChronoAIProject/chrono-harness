//! Observe native action outputs without promoting action success to cache success.
use chrono_harness::{decode, no_symlink_parents, prepared, sha256};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
};

pub(crate) fn retain_for_upload(
    root: &Path,
    directory: &str,
    stem: &str,
    raw: &[u8],
) -> Result<prepared::Original, String> {
    let original =
        prepared::retain_original(root, ".chrono-harness/state/cache-reports/", stem, raw)?;
    let name = Path::new(&original.path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("E_CACHE_REPORT: original filename")?;
    let path = format!("{directory}{name}");
    let target = chrono_harness::prepare_publication(root, &path)?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
    {
        Ok(mut file) => file.write_all(raw).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(target).map_err(|e| e.to_string())? != raw {
                return Err("E_CACHE_REPORT: upload original differs".into());
            }
        }
        Err(e) => return Err(format!("E_CACHE_REPORT: {e}")),
    }
    Ok(prepared::Original { path, ..original })
}

pub fn transport_report(
    plan: &Value,
    steps: &Value,
    work: &str,
    bootstrap: Option<&str>,
) -> Result<Value, String> {
    transport_report_with_saves(plan, steps, work, bootstrap, None)
}

fn transport_report_with_saves(
    plan: &Value,
    steps: &Value,
    work: &str,
    bootstrap: Option<&str>,
    saves: Option<&[String]>,
) -> Result<Value, String> {
    if !matches!(
        plan["schema"].as_str(),
        Some("chrono-cache-plan/v1" | "chrono-cache-plan/v2")
    ) || !steps.is_object()
    {
        return Err("E_CACHE_REPORT: expected cache plan and native step observations".into());
    }
    let selected: BTreeMap<_, _> = plan["caches"]
        .as_object()
        .ok_or("E_CACHE_REPORT: caches missing")?
        .iter()
        .collect();
    if let Some(saves) = saves {
        if saves.iter().collect::<BTreeSet<_>>().len() != saves.len()
            || saves.iter().any(|id| !selected.contains_key(id))
        {
            return Err("E_CACHE_REPORT: save selection must contain unique planned caches".into());
        }
    }
    let mut caches = BTreeMap::new();
    let mut warnings = Vec::new();
    for (index, (id, cache)) in selected.into_iter().enumerate() {
        let requested = cache["key"].as_str().ok_or("E_CACHE_REPORT: key missing")?;
        let restore = &steps[format!("cache_{index}_restore")];
        let save = &steps[format!("cache_{index}_save")];
        let matched = restore["outputs"]["cache-matched-key"]
            .as_str()
            .filter(|s| !s.is_empty());
        let restored = match restore["outcome"].as_str() {
            Some("success") => match matched {
                Some(key) if key == requested => "exact",
                Some(key)
                    if cache["restore_keys"].as_array().is_some_and(|prefixes| {
                        prefixes.iter().any(|prefix| {
                            prefix
                                .as_str()
                                .is_some_and(|p| !p.is_empty() && key.starts_with(p))
                        })
                    }) =>
                {
                    "compatible"
                }
                Some(_) => "incompatible",
                None => "miss-or-unavailable",
            },
            Some("failure" | "cancelled") => "error",
            Some("skipped") => "not-attempted",
            _ => "unavailable",
        };
        let save_requested = saves.is_none_or(|saves| saves.contains(id));
        let save_observed = matches!(
            save["outcome"].as_str(),
            Some("success" | "failure" | "cancelled")
        );
        let saved = match save["outcome"].as_str() {
            None | Some("skipped") if !save_requested => "not-requested",
            Some("success") => "unconfirmed",
            Some("failure" | "cancelled") => "error",
            Some("skipped") => "not-attempted",
            _ => "unavailable",
        };
        for (condition, code, message) in [
            (
                !save_requested && save_observed,
                "W_CACHE_UNREGISTERED_SAVE",
                "native save was observed for a cache outside this job's registered save selection",
            ),
            (
                restored == "incompatible",
                "W_CACHE_RESTORE_INCOMPATIBLE",
                "reported restored key is outside the registered compatibility domain",
            ),
            (
                saved == "unconfirmed",
                "W_CACHE_SAVE_UNCONFIRMED",
                "native save action completed; its outcome does not confirm that an archive was saved",
            ),
        ] {
            if condition {
                warnings.push(json!({"code":code,"cache":id,"message":message}));
            }
        }
        caches.insert(
            id,
            json!({"owner":cache["owner"],"producer":cache["producer"],
            "requested_key":requested,
            "restore":{"status":restored,"matched_key":matched,"observation":restore},
            "save":{"status":saved,"requested":save_requested,"confirmed":false,"observation":save},
            "current_executable_verification":"not-observed-by-cache-report"}),
        );
    }
    Ok(
        json!({"schema":"chrono-cache-transport/v1","consumer":plan["consumer"],
        "verdict":"not-a-judgment","caches":caches,"work":steps[work],
        "bootstrap":bootstrap.map(|id| &steps[id]),"warnings":warnings,
        "limits":["successful native save actions may contain backend warnings", "empty restore outputs do not distinguish cache miss from unavailable backend", "step outcomes do not prove compiler reuse or current executable identity"]}),
    )
}

pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut options = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || ![
                "--host-root",
                "--plan",
                "--steps-env",
                "--work",
                "--bootstrap",
                "--report-directory",
                "--save-caches",
            ]
            .contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_CACHE_REPORT: invalid or duplicate report option".into());
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("E_CACHE_REPORT: missing {key}"))
    };
    let root = Path::new(required("--host-root")?);
    let plan_path = required("--plan")?;
    if ![".chrono-harness/state/", ".chrono-harness/cache/"]
        .iter()
        .any(|p| plan_path.starts_with(p))
    {
        return Err("E_CACHE_REPORT: plan must be retained host state".into());
    }
    let directory = required("--report-directory")?;
    if !directory.ends_with('/')
        || ![".chrono-harness/state/", ".chrono-harness/cache/"]
            .iter()
            .any(|p| directory.starts_with(p))
    {
        return Err("E_CACHE_REPORT: expected host evidence directory".into());
    }
    let plan_raw = fs::read(no_symlink_parents(root, plan_path)?).map_err(|e| e.to_string())?;
    let steps_raw =
        std::env::var(required("--steps-env")?).map_err(|e| format!("E_CACHE_REPORT: {e}"))?;
    let plan = decode(&plan_raw)?;
    let steps = decode(steps_raw.as_bytes())?;
    let saves: Option<Vec<String>> = options
        .get("--save-caches")
        .map(|raw| {
            serde_json::from_str(raw).map_err(|e| format!("E_CACHE_REPORT: save selection: {e}"))
        })
        .transpose()?;
    let mut report = transport_report_with_saves(
        &plan,
        &steps,
        required("--work")?,
        options.get("--bootstrap").copied(),
        saves.as_deref(),
    )?;
    let executable = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|e| format!("E_CACHE_REPORT: producer executable: {e}"))?;
    let producer_bytes = fs::read(&executable)
        .map_err(|e| format!("E_CACHE_REPORT: producer executable bytes: {e}"))?;
    report["producer"] = json!({
        "executable": executable,
        "sha256": sha256(&producer_bytes),
        "version": env!("CARGO_PKG_VERSION"),
    });
    report["plan"] = json!({"path":plan_path,"sha256":sha256(&plan_raw)});
    report["plan_upload"] = json!(retain_for_upload(root, directory, "plan", &plan_raw)?);
    if plan.get("recovery").is_some() {
        let (original, recovery) = super::recovery::retained(root, plan_path)?;
        if recovery["schema"] != "chrono-cache-recovery/v1" || recovery["plan"] != report["plan"] {
            return Err("E_CACHE_REPORT: recovery belongs to another plan".into());
        }
        let raw = prepared::read_original(root, &original, None)?;
        let steps: prepared::Original = serde_json::from_value(recovery["native_steps"].clone())
            .map_err(|e| format!("E_CACHE_REPORT: recovery steps: {e}"))?;
        let steps_raw = prepared::read_original(root, &steps, None)?;
        report["recovery"] = json!({"original":retain_for_upload(root, directory, "cache-recovery", &raw)?,
            "native_steps":retain_for_upload(root, directory, "restore-steps", &steps_raw)?,
            "status":recovery["status"],"error":recovery["error"]});
    }
    let mut probes = Vec::new();
    for input in plan["inputs"]
        .as_object()
        .ok_or("E_CACHE_REPORT: plan inputs missing")?
        .values()
    {
        if let Some(source) = input["original"].as_str() {
            let raw = fs::read(no_symlink_parents(root, source)?).map_err(|e| e.to_string())?;
            probes.push(json!({"source":source,"upload":retain_for_upload(root, directory, "probe", &raw)?}));
        }
    }
    report["probe_uploads"] = json!(probes);
    report["native_steps"] = json!(retain_for_upload(
        root,
        directory,
        "native-steps",
        steps_raw.as_bytes()
    )?);
    if let Some(backend) = plan.get("backend") {
        let config: super::Backend =
            serde_json::from_value(backend.clone()).map_err(|e| format!("E_CACHE_BACKEND: {e}"))?;
        super::backend::apply(root, directory, &config, &mut report)?;
    }
    let raw = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let original = retain_for_upload(root, directory, "cache-transport", &raw)?;
    for warning in report["warnings"].as_array().into_iter().flatten() {
        eprintln!(
            "{}: {}: {}; original {}",
            warning["code"].as_str().unwrap(),
            warning["cache"].as_str().unwrap(),
            warning["message"].as_str().unwrap(),
            original.path
        );
    }
    Ok(
        serde_json::to_string(&json!({"report":original,"observation":report}))
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
