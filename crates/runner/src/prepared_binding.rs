//! Canonical consumers verify the original short acquisition observations.
use super::*;

/// Consumers compare acquisition identities with their actual fixed request and registry.
pub fn validate_binding(
    root: &Path,
    cfg: &Value,
    profile: &str,
    base: Option<&str>,
    candidate: &str,
    initial: bool,
    scope: &Option<units::Scope>,
    context: Option<&wire::Context>,
    binding: &Value,
) -> Result<(), String> {
    let c = declaration(cfg)?;
    let req: InputRequest = serde_json::from_value(binding["request"].clone())
        .map_err(|_| "missing short preparation request")?;
    let p: PreparedCheck = serde_json::from_value(binding["result"].clone())
        .map_err(|_| "missing short preparation result")?;
    req.validate()?;
    validate_result(&req, &p)?;
    match (&p.context, context) {
        (None, None) => {}
        (Some(prepared), Some(actual))
            if actual.sha256 == prepared.semantic_digest
                && fs::canonicalize(&actual.path).map_err(|e| e.to_string())?
                    == fs::canonicalize(no_symlink_parents(root, &prepared.path)?)
                        .map_err(|e| e.to_string())? => {}
        _ => return Err("prepared context differs from actual check context".into()),
    }
    if binding["schema"] != "chrono-prepared-check/v1"
        || req.host_root != root
        || c.profile != profile
        || p.profile != profile
        || p.base.as_deref() != base
        || p.candidate != candidate
        || p.initial != initial
        || &p.scope != scope
    {
        return Err("short preparation differs from actual check request".into());
    }
    let observed = binding["environment"]["source"].as_str().unwrap_or("local");
    let selector: Option<String> =
        serde_json::from_value(binding["environment"]["inherited"][SOURCE].clone())
            .map_err(|_| "check source selector disagreement")?;
    if observed != req.source
        || (selector.is_none() && req.source != "local")
        || selector
            .as_ref()
            .is_some_and(|h| h != &sha256(req.source.as_bytes()))
        || std::env::var(SOURCE).unwrap_or_else(|_| "local".into()) != req.source
    {
        return Err("check source selector disagreement".into());
    }
    let receipts = binding["receipts"]
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or("missing original producer receipts")?;
    let collecting = req.source == "local" && matches!(req.selection, Selection::Collect);
    if receipts.len() != if collecting { 2 } else { 1 } {
        return Err("short producer receipt count mismatch".into());
    }
    let mut first_request = req.clone();
    first_request.prepared = None;
    if receipts[0]["request"] != serde_json::to_value(&first_request).map_err(|e| e.to_string())? {
        return Err("short endpoint producer request mismatch".into());
    }
    if collecting
        && req
            .prepared
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?
            != Some(crate::json(
                &serde_json::from_value::<ProcessResult>(receipts[0]["process"].clone())
                    .map_err(|e| e.to_string())?
                    .stdout_bytes,
            )?)
    {
        return Err("short collection endpoint handoff mismatch".into());
    }
    let mut effective_env = BTreeMap::new();
    for key in cfg["environment"]["inherit"]
        .as_array()
        .ok_or("environment inherit missing")?
    {
        let key = key.as_str().ok_or("environment key")?;
        let observed = binding["environment"]["inherited"]
            .get(key)
            .ok_or("missing prepared environment observation")?;
        let value: Option<String> =
            serde_json::from_value(observed.clone()).map_err(|e| e.to_string())?;
        if let Some(v) = value {
            effective_env.insert(key.into(), v);
        }
    }
    for (key, value) in cfg["environment"]["values"]
        .as_object()
        .ok_or("environment values missing")?
    {
        effective_env.insert(
            key.clone(),
            sha256(value.as_str().ok_or("environment value")?.as_bytes()),
        );
    }
    if binding["environment"]["representation"] != "sha256"
        || binding["environment"]["effective"]
            != serde_json::to_value(&effective_env).map_err(|e| e.to_string())?
    {
        return Err("prepared effective environment mismatch".into());
    }
    for (i, r) in receipts.iter().enumerate() {
        let request: InputRequest =
            serde_json::from_value(r["request"].clone()).map_err(|e| e.to_string())?;
        request.validate()?;
        let process: ProcessResult =
            serde_json::from_value(r["process"].clone()).map_err(|e| e.to_string())?;
        let a: Action = serde_json::from_value(r["action"].clone()).map_err(|e| e.to_string())?;
        let expected = if req.source == "local" && i == 0 {
            &c.inputs.local
        } else {
            c.inputs.ci.as_ref().ok_or("missing CI binding")?
        };
        let result: PreparedCheck = decode(&process.stdout_bytes)?;
        validate_result(&request, &result)?;
        let tool: observation::Tool =
            serde_json::from_value(r["tool"].clone()).map_err(|e| e.to_string())?;
        let declared = cfg["tools"]
            .as_array()
            .ok_or("missing tools")?
            .iter()
            .find(|t| t["id"] == expected.tool)
            .ok_or("producer tool missing")?;
        if r["environment_representation"] != "sha256"
            || tool.version.environment != effective_env
            || tool.version.environment_digest != binding["environment"]["effective_digest"]
            || tool.program != declared["program"].as_str().ok_or("producer program")?
            || file_identity(&tool.path)?.0 != tool.sha256
            || tool.version.failure.is_some()
            || tool.version.exit_code != 0
            || declared["expected_version"]
                .as_str()
                .is_some_and(|v| tool.version.stdout.trim() != v)
        {
            return Err("prepared producer executable/version mismatch".into());
        }
        if &serde_json::to_value(&a).map_err(|e| e.to_string())?
            != &serde_json::to_value(expected).map_err(|e| e.to_string())?
            || process.failure.is_some()
            || process.exit_code != 0
            || process.stdin_sha256
                != sha256(&serde_json::to_vec(&request).map_err(|e| e.to_string())?)
            || process.stdout_sha256 != sha256(&process.stdout_bytes)
            || process.stderr_sha256 != sha256(&process.stderr_bytes)
            || process.environment_digest != binding["environment"]["effective_digest"]
            || process.argv.first().map(String::as_str) != process.executable.to_str()
            || process.stdout != String::from_utf8_lossy(&process.stdout_bytes)
            || process.stderr != String::from_utf8_lossy(&process.stderr_bytes)
            || process.cwd != root
            || process.argv.iter().skip(1).cloned().collect::<Vec<_>>() != a.argv
            || process.environment
                != serde_json::from_value::<BTreeMap<String, String>>(
                    binding["environment"]["effective"].clone(),
                )
                .map_err(|e| e.to_string())?
            || process.sha256 != r["tool"]["sha256"]
            || process.executable.to_str() != r["tool"]["path"].as_str()
        {
            return Err("short producer receipt mismatch".into());
        }
    }
    if receipts.last().unwrap()["request"] != binding["request"]
        || crate::json(
            &serde_json::from_value::<ProcessResult>(receipts.last().unwrap()["process"].clone())
                .map_err(|e| e.to_string())?
                .stdout_bytes,
        )? != binding["result"]
    {
        return Err("short final producer result mismatch".into());
    }
    Ok(())
}
