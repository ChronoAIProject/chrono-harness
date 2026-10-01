//! Finite candidate-owned decoders for explicitly named historical profiles.
use crate::{Registrations, execution};
use chrono_harness::{
    CommandSpec, facts, json,
    observation::{self, Tool},
    run_process_observed, sha256,
    wire::{self, Request},
};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
type Values = BTreeMap<String, Value>;
fn config_binding(base: &Values, candidate: &Values, entry: &str) -> Result<Value, String> {
    Ok(value!({"base":facts::registry_identity(base, entry)?,
        "candidate":facts::registry_identity(candidate, entry)?}))
}
fn load_view(view: &Value, config: &str) -> Result<(Registrations, Registrations), String> {
    let a: Values = serde_json::from_value(view["base"].clone()).map_err(|e| e.to_string())?;
    let b: Values = serde_json::from_value(view["candidate"].clone()).map_err(|e| e.to_string())?;
    let identities = config_binding(&a, &b, config)?;
    if identities["base"]["selection"].is_object()
        || identities["candidate"]["selection"].is_object()
    {
        if view["schema"] != "chrono-registration-view/v2" || view["config_bindings"] != identities
        {
            return Err("registration view selection binding mismatch".into());
        }
        if !view["conversion"].is_null() {
            let input = &view["conversion"]["input"];
            if input["schema"] != "chrono-historical-decode/v3"
                || input["config_path"] != config
                || input["config_bindings"] != identities
            {
                return Err("historical decoder selection binding mismatch".into());
            }
        }
    }
    let mut old = Registrations::load(&a, config)?;
    old.historical =
        serde_json::from_value(view["historical"].clone()).map_err(|e| e.to_string())?;
    if let Some(raw) = view["conversion"]["input"].get("original") {
        let raw: Values = serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
        old.original_schemas = Registrations::schemas(&raw, config)?;
        // Supported config versions already have readers. Historical snapshots
        // bind their original config, never a decoder's replacement semantics.
        let original = facts::registry_identity(&raw, config)?;
        if facts::registry_identity(&a, config)? != original
            || a.get(config) != raw.get(config)
            || a.get(&original.effective_path) != raw.get(&original.effective_path)
        {
            return Err(
                "E_MIGRATION_INPUT_SEMANTICS: historical config must remain original".into(),
            );
        }
    }
    Ok((old, Registrations::load(&b, config)?))
}
pub fn views(req: &Request) -> Result<(Registrations, Registrations, Value), String> {
    let reader = facts::Reader::for_request(req)?;
    views_with_reader(req, &reader)
}
fn view_binding(
    req: &Request,
    base: &facts::RegistrySnapshot,
    candidate: &facts::RegistrySnapshot,
) -> Value {
    let mut binding = value!({"base":req.base.commit,"candidate":req.candidate.commit,"registry":req.registries.digest,"context":req.context.sha256});
    if base.selection.is_some() || candidate.selection.is_some() {
        binding["entry_path"] = value!(req.config_path);
        binding["base_config"] = value!({"entry_path":base.entry_path,"effective_path":base.effective_path,"selection":base.selection});
        binding["candidate_config"] = value!({"entry_path":candidate.entry_path,"effective_path":candidate.effective_path,"selection":candidate.selection});
    }
    binding
}
/// Rebuild the original registration view from current immutable endpoint
/// data and retained conversion observations. This never invokes the decoder.
pub fn validate_retained_view(
    req: &Request,
    reader: &facts::Reader,
    declaration_root: &Path,
    supplied: &Value,
) -> Result<(), String> {
    let base = reader.registry_snapshot(declaration_root, &req.base.commit, &req.config_path)?;
    let candidate =
        reader.registry_snapshot(declaration_root, &req.candidate.commit, &req.config_path)?;
    if facts::registry_digest(&base.values, &candidate.values)? != req.registries.digest {
        return Err("E_COLLECTION_INPUT: original registration endpoint digest".into());
    }
    let binding = view_binding(req, &base, &candidate);
    let conversion = &supplied["conversion"];
    if !conversion.is_null() && conversion["process"]["cwd"] != value!(req.candidate.root) {
        return Err("E_COLLECTION_INPUT: original conversion coordinates differ".into());
    }
    let env = serde_json::from_value(req.observations["environment"]["effective"].clone())
        .map_err(|e| e.to_string())?;
    let (_, _, mut expected) = interpret_mode(
        reader,
        declaration_root,
        &req.base.commit,
        &req.candidate.commit,
        &req.config_path,
        base.values,
        candidate.values,
        &env,
        Some("registration"),
        Some(conversion),
    )?;
    expected["binding"] = binding;
    if *supplied != expected {
        return Err("E_COLLECTION_INPUT: original registration view differs".into());
    }
    Ok(())
}
pub fn views_with_reader(
    req: &Request,
    reader: &facts::Reader,
) -> Result<(Registrations, Registrations, Value), String> {
    views_using_reports(req, reader, None)
}
/// The local collection producer supplies original report inputs before publishing
/// its required-unit manifest. Registration remains the conversion owner.
pub fn views_for_collection_inputs(
    req: &Request,
    reader: &facts::Reader,
    reports: &chrono_harness::units::FullManifest,
) -> Result<(Registrations, Registrations, Value), String> {
    if !matches!(
        req.scope,
        Some(chrono_harness::units::Scope::Collect { .. })
    ) {
        return Err("collection report inputs require collection scope".into());
    }
    views_using_reports(req, reader, Some(reports))
}
fn views_using_reports(
    req: &Request,
    reader: &facts::Reader,
    reports: Option<&chrono_harness::units::FullManifest>,
) -> Result<(Registrations, Registrations, Value), String> {
    let base_snapshot =
        reader.registry_snapshot(&req.candidate.root, &req.base.commit, &req.config_path)?;
    let candidate_snapshot =
        reader.registry_snapshot(&req.candidate.root, &req.candidate.commit, &req.config_path)?;
    let selector_required =
        base_snapshot.selection.is_some() || candidate_snapshot.selection.is_some();
    let binding = view_binding(req, &base_snapshot, &candidate_snapshot);
    let prior: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("registration_view"))
        .collect();
    if let Some(v) = prior.first() {
        if prior.iter().any(|p| *p != *v) || v["binding"] != binding {
            return Err("registration view binding mismatch".into());
        }
        if selector_required {
            let expected = config_binding(
                &base_snapshot.values,
                &candidate_snapshot.values,
                &req.config_path,
            )?;
            if v["schema"] != "chrono-registration-view/v2"
                || v["config_bindings"] != expected
                || v["candidate"] != value!(candidate_snapshot.values)
            {
                return Err("registration view selection binding mismatch".into());
            }
            let original = if v["conversion"].is_null() {
                &v["base"]
            } else {
                &v["conversion"]["input"]["original"]
            };
            if *original != value!(base_snapshot.values)
                || (!v["conversion"].is_null()
                    && v["conversion"]["input"]["original_bytes"] != value!(base_snapshot.bytes))
            {
                return Err("registration view original endpoint mismatch".into());
            }
        }
        let (a, b) = load_view(v, &req.config_path)?;
        return Ok((a, b, (*v).clone()));
    }
    let a = base_snapshot.values;
    let b = candidate_snapshot.values;
    if facts::registry_digest(&a, &b)? != req.registries.digest {
        return Err("registry digest mismatch".into());
    }
    let env = serde_json::from_value(req.observations["environment"]["effective"].clone())
        .unwrap_or_default();
    let retained = retained_conversion(req, &b, reports)?;
    let (a, b, mut view) = interpret_mode(
        reader,
        &req.candidate.root,
        &req.base.commit,
        &req.candidate.commit,
        &req.config_path,
        a,
        b,
        &env,
        Some(&req.judge_id),
        retained.as_ref(),
    )?;
    view["binding"] = binding;
    Ok((a, b, view))
}
pub fn interpret(
    root: &Path,
    base: &str,
    candidate_oid: &str,
    config: &str,
    raw: Values,
    candidate: Values,
    env: &BTreeMap<String, String>,
) -> Result<(Registrations, Registrations, Value), String> {
    let reader = facts::Reader::for_config(root, config)?;
    interpret_with_reader(
        &reader,
        root,
        base,
        candidate_oid,
        config,
        raw,
        candidate,
        env,
    )
}
/// Reuse the caller's selected facts reader and retain its complete acquisition record.
pub fn interpret_with_reader(
    reader: &facts::Reader,
    root: &Path,
    base: &str,
    candidate_oid: &str,
    config: &str,
    raw: Values,
    candidate: Values,
    env: &BTreeMap<String, String>,
) -> Result<(Registrations, Registrations, Value), String> {
    reader.verify_config(root, candidate_oid)?;
    interpret_mode(
        reader,
        root,
        base,
        candidate_oid,
        config,
        raw,
        candidate,
        env,
        None,
        None,
    )
}
fn retained_conversion(
    req: &Request,
    candidate: &Values,
    reports: Option<&chrono_harness::units::FullManifest>,
) -> Result<Option<Value>, String> {
    let Some(chrono_harness::units::Scope::Collect { manifest }) = &req.scope else {
        return Ok(None);
    };
    let r = Registrations::load(candidate, &req.config_path)?;
    let limits = &r.config()["execution_units"]["collection_limits"];
    let manifest: chrono_harness::units::FullManifest = match reports {
        Some(reports) => reports.clone(),
        None => {
            let bytes = chrono_harness::units::read_bounded(
                &req.candidate.root,
                manifest,
                limits["manifest_bytes"].as_u64().ok_or("manifest bound")?,
            )?;
            chrono_harness::decode(&bytes)?
        }
    };
    if !matches!(
        manifest.schema.as_str(),
        "chrono-full-collection/v1" | "chrono-ci-collection/v1"
    ) {
        return Err("E_COLLECTION_INPUT: manifest schema".into());
    }
    let mut seen = BTreeSet::new();
    let mut retained = None;
    for input in manifest.reports {
        let source = r.config()["execution_units"]["units"][&input.unit]["report_path"]
            .as_str()
            .ok_or("E_COLLECTION_INPUT: unregistered original unit")?;
        let transported = chrono_harness::prepared::original_path(
            &chrono_harness::prepared::Original {
                path: source.into(),
                sha256: input.sha256.clone(),
            },
            input.artifacts.as_ref(),
        )?;
        if !seen.insert(input.unit.clone()) || transported != input.path {
            return Err("E_COLLECTION_INPUT: original unit registration".into());
        }
        let bytes = chrono_harness::units::read_bounded(
            &req.candidate.root,
            &input.path,
            limits["report_bytes"].as_u64().ok_or("report bound")?,
        )?;
        if sha256(&bytes) != input.sha256 {
            return Err("E_COLLECTION_INPUT: original report digest".into());
        }
        let report = json(&bytes)?;
        let record = report["judges"]
            .as_array()
            .ok_or("original judges")?
            .iter()
            .find(|j| j["id"] == "registration")
            .ok_or("original registration")?;
        let c = &record["response"]["outputs"]["registration_view"]["conversion"];
        if !c.is_null() {
            let original: Request =
                serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
            original.validate()?;
            let expanded = chrono_harness::full::expand_record(record)?;
            let process: chrono_harness::ProcessResult =
                serde_json::from_value(expanded["process"].clone()).map_err(|e| e.to_string())?;
            observation::process_success(&process)?;
            if original.base.commit != req.base.commit
                || original.base.tree != req.base.tree
                || original.candidate.commit != req.candidate.commit
                || original.candidate.tree != req.candidate.tree
                || original.registries.digest != req.registries.digest
                || original.context.sha256 != req.context.sha256
                || original.runner.sha256 != req.runner.sha256
                || original.scope
                    != Some(chrono_harness::units::Scope::Unit {
                        unit: input.unit.clone(),
                    })
                || original.observations["registry_bindings"]
                    != req.observations["registry_bindings"]
                || process.stdin_sha256 != wire::digest(&original)?
                || process.sha256
                    != r.judges()["judges"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|j| j["id"] == "registration")
                        .ok_or("registration binding")?["sha256"]
                        .as_str()
                        .ok_or("registration pin")?
                || json(&process.stdout_bytes)? != record["response"]
                || c["process"]["cwd"] != serde_json::to_value(&original.candidate.root).unwrap()
            {
                return Err("E_COLLECTION_INPUT: original conversion bootstrap binding".into());
            }
            // Different roots carry different process coordinates. The conversion's declared input/output must agree.
            if let Some(previous) = &retained {
                let previous: &Value = previous;
                if previous["input"] != c["input"]
                    || previous["output"] != c["output"]
                    || previous["script_sha256"] != c["script_sha256"]
                {
                    return Err("E_COLLECTION_INPUT: conflicting original conversion".into());
                }
            }
            if retained.is_none() {
                retained = Some(c.clone());
            }
        }
    }
    Ok(Some(retained.unwrap_or(Value::Null)))
}
fn interpret_mode(
    reader: &facts::Reader,
    root: &Path,
    base: &str,
    candidate_oid: &str,
    config: &str,
    raw: Values,
    candidate: Values,
    env: &BTreeMap<String, String>,
    consumer: Option<&str>,
    retained: Option<&Value>,
) -> Result<(Registrations, Registrations, Value), String> {
    let new = Registrations::load(&candidate, config).map_err(|e| format!("candidate: {e}"))?;
    let deferred = consumer.is_some_and(|id| downstream_validator(&new, id));
    let raw_config = facts::registry_identity(&raw, config)?.effective_path;
    let filemap = raw[&raw_config]["registries"]["filemap"]
        .as_str()
        .ok_or("historical filemap path")?;
    let mut view = value!({"base":raw,"candidate":candidate,"historical":{},"conversion":null});
    let bindings = config_binding(&raw, &candidate, config)?;
    let selected =
        bindings["base"]["selection"].is_object() || bindings["candidate"]["selection"].is_object();
    if selected {
        view["schema"] = value!("chrono-registration-view/v2");
        view["config_bindings"] = bindings.clone();
    }
    let versions = |values: &Values| -> Result<Value, String> {
        Ok(value!(
            Registrations::schemas(values, config)?
                .into_iter()
                .map(|(key, (_, version))| (key, version))
                .collect::<BTreeMap<_, _>>()
        ))
    };
    let from = versions(&raw)?;
    let to = versions(&candidate)?;
    let mut matching = vec![];
    let mut legacy_required = false;
    for profile in new.workflow()["historical_profiles"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if profile.get("from_versions").is_some() {
            if profile["from_versions"] == from && profile["to_versions"] == to {
                matching.push((profile, Value::Null, Value::Null, Value::Null));
            }
        } else if profile["filemap_version"] == raw[filemap]["schema_version"] {
            legacy_required = true;
            let path = profile["profile_path"].as_str().unwrap();
            let original = reader.blob(root, base, path);
            if let Err(e) = &original {
                if e.starts_with("E_GIT_FACTS:") {
                    return Err(e.clone());
                }
            }
            if let Ok(bytes) = original {
                let value = json(&bytes)?;
                if value["schema"] == profile["id"] {
                    matching.push((profile, value!(path), value!(bytes), value));
                }
            }
        }
    }
    if matching.is_empty() && !legacy_required {
        let old = Registrations::load(&raw, config).map_err(|e| format!("base: {e}"))?;
        return Ok((old, new, view));
    }
    if matching.len() != 1 {
        return Err("E_MIGRATION_PROFILE: missing/ambiguous historical profile".into());
    }
    let (profile, path, profile_bytes, profile_value) = matching.remove(0);
    if retained.is_some_and(Value::is_null) {
        return Err("E_MIGRATION: collection lacks original conversion evidence".into());
    }
    let scripts = new.projects()["scripts"].as_array().unwrap();
    let script = scripts
        .iter()
        .find(|s| s["id"] == profile["script"])
        .ok_or("missing registered migration script")?;
    let test = scripts
        .iter()
        .find(|s| s["id"] == profile["test"])
        .ok_or("missing dedicated migration test")?;
    if script["test_script"] != test["id"] || test["tests_for"] != script["id"] {
        return Err("E_TEST_PAIR: migration script".into());
    }
    let op = script["actions"]["execute"]["operation"]
        .as_str()
        .ok_or("migration execute action")?;
    let all = execution::methods(new.projects())?;
    let methods = all.get(op).ok_or("migration method missing")?;
    if methods.len() != 1 {
        return Err("E_ROUTE_AMBIGUOUS: migration".into());
    }
    let method = &methods[0];
    let t = new.config()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == method.tool)
        .ok_or("migration tool missing")?;
    let argv: Vec<String> =
        serde_json::from_value(t["version_argv"].clone()).map_err(|e| e.to_string())?;
    let raw_bytes: BTreeMap<_, _> = raw
        .keys()
        .map(|p| Ok((p.clone(), reader.blob(root, base, p)?)))
        .collect::<Result<_, String>>()?;
    let mut input = if profile.get("from_versions").is_some() {
        value!({"schema":"chrono-historical-decode/v2","config_path":config,"original":raw,"original_bytes":raw_bytes,"candidate":candidate,"profile":profile})
    } else {
        value!({"schema":"chrono-historical-decode/v1","config_path":config,"original":raw,"original_bytes":raw_bytes,"profile":profile,"profile_bytes":profile_bytes,"profile_value":profile_value})
    };
    if selected {
        input["schema"] = value!("chrono-historical-decode/v3");
        input["config_bindings"] = bindings;
        input["candidate"] = value!(candidate);
    }
    let script_path = script["path"].as_str().unwrap();
    let script_bytes = reader.blob(root, candidate_oid, script_path)?;
    let (tool, receipt) = if let Some(c) = retained {
        let tool: Tool = serde_json::from_value(c["tool"].clone()).map_err(|e| e.to_string())?;
        let receipt: chrono_harness::ProcessResult =
            serde_json::from_value(c["process"].clone()).map_err(|e| e.to_string())?;
        observation::process_success(&tool.version)?;
        observation::process_success(&receipt)?;
        let mut version_argv = vec![tool.path.to_str().ok_or("migration tool path")?.to_owned()];
        version_argv.extend(argv.clone());
        let mut invocation = vec![tool.path.to_str().ok_or("migration tool path")?.to_owned()];
        invocation.extend(method.argv.clone());
        let declared_input = new.config()["environment"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| {
                Path::new(i["location"].as_str().unwrap()).is_absolute()
                    && i["location"].as_str() == tool.path.to_str()
                    && i["sha256"] == tool.sha256
            });
        let candidate_file = tool
            .path
            .strip_prefix(&receipt.cwd)
            .ok()
            .and_then(|p| p.to_str())
            .is_some_and(|p| {
                new.filemap()["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["path"] == p)
                    && reader
                        .blob(root, candidate_oid, p)
                        .is_ok_and(|b| sha256(&b) == tool.sha256)
            });
        if c["input"] != input
            || c["input_digest"] != wire::digest(&input)?
            || c["output_digest"] != receipt.stdout_sha256
            || c["script"] != script_path
            || c["script_sha256"] != sha256(&script_bytes)
            || c["tool_id"] != method.tool
            || tool.program != t["program"].as_str().ok_or("migration program")?
            || tool.path.file_name().and_then(|s| s.to_str()) != Some(&tool.basename)
            || tool.version.executable != tool.path
            || tool.version.sha256 != tool.sha256
            || tool.version.argv != version_argv
            || tool.version.cwd != receipt.cwd
            || tool.version.environment != *env
            || tool.version.stdin_sha256 != sha256(&[])
            || t["expected_version"].as_str() != Some(tool.version.stdout.trim_end())
            || receipt.executable != tool.path
            || receipt.sha256 != tool.sha256
            || receipt.argv != invocation
            || receipt.environment != *env
            || receipt.stdin_sha256 != wire::digest(&input)?
            || c["output"] != json(&receipt.stdout_bytes)?
            || (!declared_input && !candidate_file)
        {
            return Err(
                "E_MIGRATION: retained conversion differs from declared original contract".into(),
            );
        }
        (tool, receipt)
    } else {
        let artifacts = facts::artifact_directories(new.config())?;
        let before = reader.checkout_excluding(root, candidate_oid, &artifacts)?;
        let tool = observation::tool(
            root,
            t["program"].as_str().unwrap(),
            &argv,
            env,
            30,
            1048576,
        )?;
        if tool.version.exit_code != 0
            || tool.version.failure.is_some()
            || t["expected_version"].as_str() != Some(tool.version.stdout.trim_end())
        {
            return Err(format!(
                "E_TOOL_BINDING: migration version mismatch; observation={}",
                value!({"tool_id":method.tool,"expected_version":t["expected_version"],"tool":tool})
            ));
        }
        if fs::read(root.join(script_path)).map_err(|e| e.to_string())? != script_bytes {
            return Err("E_MIGRATION: decoder differs from fixed candidate".into());
        }
        let spec = CommandSpec {
            program: tool.path.to_str().unwrap().into(),
            args: method.argv.clone(),
            env: env.clone(),
            timeout_seconds: 30,
            output_limit_bytes: 16 * 1024 * 1024,
        };
        let receipt = run_process_observed(root, &spec, &wire::canonical(&input)?, &tool.sha256)?;
        if receipt.exit_code != 0 || receipt.failure.is_some() {
            return Err(format!(
                "E_MIGRATION: exit {}: {}",
                receipt.exit_code, receipt.stderr
            ));
        }
        if before != reader.checkout_excluding(root, candidate_oid, &artifacts)? {
            return Err("E_SNAPSHOT_DIRTY: migration changed candidate inputs".into());
        }
        (tool, receipt)
    };
    let output = json(&receipt.stdout_bytes)?;
    if output["mappings"] != profile["mappings"] {
        return Err("E_MIGRATION: mapping mismatch".into());
    }
    view["base"] = output["values"].clone();
    view["historical"] = output["historical"].clone();
    view["conversion"] = if let Some(c) = retained {
        c.clone()
    } else {
        value!({"input":input,"input_digest":receipt.stdin_sha256,"output":output,"output_digest":receipt.stdout_sha256,"profile_path":path,"script":script_path,"script_sha256":sha256(&script_bytes),"tool_id":method.tool,"tool":tool,"process":receipt,"certification":"unresolved: workflow producer required"})
    };
    let (old, _) = load_view(&view, config)?;
    // Every explicitly removed malformed definition must survive as its original bytes/value.
    for record in profile["legacy_records"].as_array().unwrap() {
        let raw_config = facts::registry_identity(&raw, config)?.effective_path;
        let original = raw[raw[&raw_config]["registries"]["projects"].as_str().unwrap()]
            [record["collection"].as_str().unwrap()]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == record["id"]);
        if original != Some(&record["definition"]) {
            return Err("E_MIGRATION: historical definition mismatch".into());
        }
        for node in record["nodes"].as_array().unwrap() {
            if !old
                .historical
                .get(node.as_str().unwrap())
                .is_some_and(|defs| defs.iter().any(|(_, v)| Some(v) == original))
            {
                return Err("E_MIGRATION: lost historical definition".into());
            }
        }
    }
    let nodes = new.node_data();
    let mappings = profile["mappings"].as_array().unwrap();
    // A replacement is an exact, same-operation declaration, not an inference
    // from a renamed path or a waiver of the original execution obligation.
    let mut replacements = BTreeSet::new();
    for replacement in profile["method_replacements"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let from: execution::Method = serde_json::from_value(replacement["from"].clone())
            .map_err(|e| format!("E_MIGRATION: {e}"))?;
        let to: execution::Method = serde_json::from_value(replacement["to"].clone())
            .map_err(|e| format!("E_MIGRATION: {e}"))?;
        let originals: Vec<_> = profile["legacy_records"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|record| {
                record["definition"]["actions"]
                    .as_object()
                    .into_iter()
                    .flat_map(|actions| actions.values())
                    .map(move |action| (record, action))
            })
            .filter(|(_, action)| action["operation"] == from.operation)
            .collect();
        let original_matches = originals.len() == 1 && {
            let (record, action) = originals[0];
            from.owner == format!("script:{}", record["id"].as_str().unwrap())
                && action["tool"] == from.tool
                && action["argv"] == value!(from.argv)
        };
        let target = all.get(&to.operation);
        let owner_mapped = from.owner == to.owner
            || mappings
                .iter()
                .any(|m| m["from"] == from.owner && m["to"] == to.owner);
        if !original_matches || !owner_mapped || target != Some(&vec![to]) {
            return Err("E_MIGRATION: method replacement must bind original and unique candidate methods with an explicit owner mapping".into());
        }
        replacements.insert(from.operation);
    }
    for record in profile["legacy_records"].as_array().unwrap() {
        for node in record["nodes"].as_array().unwrap() {
            let matches: Vec<_> = mappings.iter().filter(|m| m["from"] == *node).collect();
            if matches.len() != 1
                || !nodes
                    .get(matches[0]["to"].as_str().unwrap())
                    .is_some_and(|n| n.unique().is_some())
            {
                return Err(
                    "E_MIGRATION: missing/ambiguous mapping to unique candidate identity".into(),
                );
            }
        }
        let owner = format!("owner:{}", record["id"].as_str().unwrap());
        if !new.projects()["owners"]
            .as_array()
            .unwrap()
            .contains(&record["id"])
        {
            let matches: Vec<_> = mappings.iter().filter(|m| m["from"] == owner).collect();
            if matches.len() != 1
                || !matches[0]["to"]
                    .as_str()
                    .and_then(|s| s.strip_prefix("owner:"))
                    .is_some_and(|id| {
                        new.projects()["owners"]
                            .as_array()
                            .unwrap()
                            .contains(&value!(id))
                    })
            {
                return Err("E_MIGRATION: missing retired owner mapping".into());
            }
        }
        for action in record["definition"]["actions"]
            .as_object()
            .unwrap()
            .values()
        {
            if deferred || replacements.contains(action["operation"].as_str().unwrap()) {
                continue;
            }
            let current = all
                .get(action["operation"].as_str().unwrap())
                .ok_or("E_MIGRATION: historical method lost")?;
            if current.len() != 1
                || current[0].tool != action["tool"]
                || value!(current[0].argv) != action["argv"]
            {
                return Err(
                    "E_MIGRATION: historical method changed without supported mapping".into(),
                );
            }
        }
    }
    Ok((old, new, view))
}
pub fn reused_tools(view: &Value) -> Result<BTreeMap<String, Tool>, String> {
    let mut out = BTreeMap::new();
    let c = &view["conversion"];
    if let Some(id) = c["tool_id"].as_str() {
        out.insert(
            id.into(),
            serde_json::from_value(c["tool"].clone()).map_err(|e| e.to_string())?,
        );
    }
    Ok(out)
}
pub fn replacements(r: &Registrations) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for row in r.workflow()["retirements"].as_array().unwrap() {
        if row["kind"] != "test" {
            continue;
        }
        if let Some(to) = row["replacement"].as_str() {
            let from = format!("test:{}", row["id"].as_str().unwrap());
            let to = if to.starts_with("test:") {
                to.to_string()
            } else {
                format!("test:{to}")
            };
            if out.insert(from, to).is_some() {
                return Err("E_REQUIRED_TEST_REMOVED: ambiguous replacement".into());
            }
        }
    }
    Ok(out)
}
pub fn ambiguity_repaired(r: &Registrations, node: &str, definitions: &[String]) -> bool {
    let repairs: Vec<_> = r.workflow()["historical_profiles"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|p| p["ambiguities"].as_array().into_iter().flatten())
        .filter(|p| p["node"] == node)
        .collect();
    if repairs.len() != 1 {
        return false;
    }
    let repair = repairs[0];
    let declared: BTreeSet<_> = repair["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let actual: BTreeSet<_> = definitions.iter().map(String::as_str).collect();
    let Some(to) = repair["replacement"].as_str() else {
        return false;
    };
    declared == actual
        && r.node_data().get(to).is_some_and(|n| n.unique().is_some())
        && (to == node
            || replacements(r)
                .ok()
                .is_some_and(|m| m.get(node).is_some_and(|v| v == to)))
}

/// Explicit requests only: null is a requested retirement, never its approval.
pub fn retirement_requests(r: &Registrations) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut out = BTreeMap::new();
    for row in r.workflow()["retirements"].as_array().unwrap() {
        if row["kind"] != "test" {
            continue;
        }
        let from = format!("test:{}", row["id"].as_str().unwrap());
        let to = row["replacement"].as_str().map(|s| {
            if s.starts_with("test:") {
                s.into()
            } else {
                format!("test:{s}")
            }
        });
        if out.insert(from, to).is_some() {
            return Err("E_REQUIRED_TEST_REMOVED: ambiguous retirement request".into());
        }
    }
    Ok(out)
}
/// A declared later validator must consume this judge before a pending obligation
/// can leave planning/execution. The validator, not this graph query, approves it.
pub fn downstream_validator(r: &Registrations, id: &str) -> bool {
    let judges = r.judges()["judges"].as_array().unwrap();
    let Some(validator) = r.judges()["migration_validator"].as_str() else {
        return false;
    };
    if validator == id {
        return false;
    }
    let mut pending = vec![validator];
    let mut seen = BTreeSet::new();
    while let Some(n) = pending.pop() {
        if !seen.insert(n) {
            continue;
        }
        let Some(j) = judges.iter().find(|j| j["id"] == n) else {
            return false;
        };
        for p in j["after"].as_array().unwrap() {
            let Some(p) = p.as_str() else { return false };
            if p == id {
                return true;
            }
            pending.push(p)
        }
    }
    false
}
