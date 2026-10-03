//! Offline verification of explicit unit reports. Business operations are never executed here.
use super::*;
use chrono_harness::{
    ProcessResult,
    units::{Manifest, Scope},
};
use chrono_judge_registration::execution::Method;
use chrono_judge_routes::{Execution, Receipt};
use std::io::Read;

fn read(
    root: &Path,
    path: &str,
    policy: &Policy,
    limit_name: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    chrono_harness::units::artifact_path(path)?;
    if !policy.artifacts.iter().any(|a| path.starts_with(a)) {
        return Err("collection input is not a declared artifact".into());
    }
    let resolved = chrono_harness::no_symlink_parents(root, path)?;
    let meta = fs::symlink_metadata(&resolved).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(format!("collection input must be a regular file: {path}"));
    }
    let overflow = |observed| {
        format!("collection input exceeds {limit_name}: {path} ({observed} > {limit} bytes)")
    };
    if meta.len() > limit {
        return Err(overflow(meta.len()));
    }
    let mut bytes = Vec::new();
    fs::File::open(resolved)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(overflow(bytes.len() as u64));
    }
    Ok(bytes)
}

fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn process(p: &ProcessResult) -> Result<(), String> {
    if !digest(&p.sha256)
        || p.stdout_sha256 != sha256(&p.stdout_bytes)
        || p.stderr_sha256 != sha256(&p.stderr_bytes)
        || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
        || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
        || p.environment_digest != chrono_harness::wire::digest(&p.environment)?
    {
        return Err("unit original process bytes or identity mismatch".into());
    }
    Ok(())
}

pub(super) fn collect(
    req: &Request,
    manifest: &str,
    config: &CheckConfig,
    snapshot: &Snapshot,
    selected: &BTreeSet<String>,
    methods: &BTreeMap<String, Vec<Method>>,
    declarations: &Value,
    environment_policy: &Value,
) -> Result<Response, String> {
    let p = &snapshot.policy;
    let units = p
        .units
        .as_ref()
        .ok_or("collection requires registered units")?;
    let required = units::required(units, selected);
    let limits = p.collection_limits.clone().unwrap_or_default();
    let manifest_bytes = read(
        &req.host_root,
        manifest,
        p,
        "manifest_bytes",
        limits.manifest_bytes,
    )?;
    let inputs: Manifest = decode(&manifest_bytes)?;
    if inputs.schema != "chrono-ci-collection/v1" {
        return Err("unsupported collection manifest".into());
    }
    let mut supplied = BTreeSet::new();
    let mut paths = BTreeSet::from([manifest.to_owned(), config.report_path.clone()]);
    for input in &inputs.reports {
        if !units.contains_key(&input.unit)
            || !supplied.insert(input.unit.clone())
            || !paths.insert(input.path.clone())
            || ![&input.sha256, &input.runner_sha256, &input.judge_sha256]
                .iter()
                .all(|s| digest(s))
        {
            return Err("unknown/duplicate unit report, colliding path or invalid identity".into());
        }
    }
    if !required.iter().all(|u| supplied.contains(u)) {
        return Err("missing required CI unit report".into());
    }
    // Nonselected units may report successful inventory too; unknown or duplicate reports never pass.
    let not_required: BTreeSet<_> = snapshot
        .plans
        .keys()
        .filter(|t| !selected.contains(*t))
        .cloned()
        .collect();
    let mut results = vec![CheckResult {
        id: "ci.inventory".into(),
        status: Status::Passed,
        cause: "fixed endpoints, complete global selection and explicit unit assignments validated"
            .into(),
        exit_code: None,
    }];
    let mut verified = vec![];
    for input in &inputs.reports {
        let bytes = read(
            &req.host_root,
            &input.path,
            p,
            "report_bytes",
            limits.report_bytes,
        )?;
        if sha256(&bytes) != input.sha256 {
            return Err(format!("unit report digest mismatch: {}", input.unit));
        }
        let report = json(&bytes)?;
        if report["schema"] != "chrono-check-report/v1" || report.get("transport_failure").is_some()
        {
            return Err(format!("unit transport failed: {}", input.unit));
        }
        let original: Request =
            serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
        let response: Response =
            serde_json::from_value(report["response"].clone()).map_err(|e| e.to_string())?;
        if input
            .artifacts
            .as_ref()
            .is_some_and(|t| !p.artifacts.iter().any(|a| t.directory.starts_with(a)))
        {
            return Err("original evidence transport is not a declared artifact".into());
        }
        if let Some(binding) = original
            .observations
            .get("preparation")
            .filter(|b| b.is_object())
        {
            chrono_harness::prepared::validate_portable_binding(
                &req.host_root,
                binding,
                input.artifacts.as_ref(),
            )?;
            let prepared: chrono_harness::prepared::PreparedCheck =
                serde_json::from_value(binding["result"].clone()).map_err(|e| e.to_string())?;
            if prepared.base != req.base
                || prepared.candidate != req.candidate
                || prepared.profile != req.config_path
                || prepared.scope != original.scope
            {
                return Err("original preparation differs from collected unit".into());
            }
        } else if input.artifacts.is_some() {
            return Err("native short report missing original preparation".into());
        }
        let producer: ProcessResult =
            serde_json::from_value(report["judge"].clone()).map_err(|e| e.to_string())?;
        process(&producer)?;
        chrono_harness::validate_response_protocol(
            &response,
            &original.request_id,
            producer.exit_code,
            chrono_harness::units::PROTOCOL,
        )?;
        if original.protocol != chrono_harness::units::PROTOCOL
            || original.scope
                != Some(Scope::Unit {
                    unit: input.unit.clone(),
                })
            || original.base != req.base
            || original.candidate != req.candidate
            || original.initial != req.initial
            || original.config_path != req.config_path
            || original.config_sha256 != req.config_sha256
            || report["runner"]["sha256"] != input.runner_sha256
            || producer.sha256 != input.judge_sha256
            || producer.stdin_sha256
                != sha256(&serde_json::to_vec(&original).map_err(|e| e.to_string())?)
            || json(&producer.stdout_bytes)? != report["response"]
            || producer.cwd != original.host_root
            || producer.failure.is_some()
        {
            return Err(format!(
                "unit endpoints/config/scope/executable/original response mismatch: {}",
                input.unit
            ));
        }
        let candidates = chrono_harness::program_candidates(
            &original.host_root,
            &config.judge.program,
            Some(
                producer
                    .environment
                    .get("PATH")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
        )?;
        let expected_argv: Vec<_> =
            std::iter::once(producer.executable.to_string_lossy().into_owned())
                .chain(config.judge.args.clone())
                .collect();
        if !candidates.contains(&producer.executable) || producer.argv != expected_argv {
            return Err("unit judge command differs from profile".into());
        }
        if response.status != Status::Passed {
            results.push(CheckResult {
                id: format!("ci.unit.{}", input.unit),
                status: Status::Failed,
                cause: format!(
                    "unit failed; original report {} ({})",
                    input.path, input.sha256
                ),
                exit_code: Some(producer.exit_code),
            });
            verified.push(object!({"unit":input.unit,"path":input.path,"sha256":input.sha256,"status":"failed-original-retained"}));
            continue;
        }
        let owned = units::selection(units, selected, &input.unit)?;
        let e = &response.evidence;
        if e["scope"] != config.schema
            || e["base"] != object!(req.base)
            || e["candidate"] != req.candidate
            || e["execution_scope"] != object!(original.scope)
            || e["selected"] != object!(owned)
            || e["global_selected"] != object!(selected)
            || e["required_units"] != object!(required)
            || e["assigned_elsewhere"] != object!(selected.difference(&owned).collect::<Vec<_>>())
            || e["not_required"] != object!(not_required)
            || e["blocked"] != object!([])
        {
            return Err("unit obligation partition differs from current global plan".into());
        }
        let plan: Execution =
            serde_json::from_value(e["plan"].clone()).map_err(|e| e.to_string())?;
        plan.validate()?;
        if plan.root != original.host_root
            || plan.binding
                != object!({"base":req.base,"candidate":req.candidate,"config":req.config_sha256,"run":original.request_id,"entry":original.observations["entry"]})
        {
            return Err("unit plan binding mismatch".into());
        }
        let literals = environment_policy["values"]
            .as_object()
            .ok_or("environment literals missing")?;
        if literals.iter().any(|(key, value)| {
            plan.environment.get(key).map(|s| object!(s)) != Some(value.clone())
        }) {
            return Err("unit environment differs from registered literal values".into());
        }
        if let Some(inherit) = environment_policy["inherit"].as_array() {
            let allowed: BTreeSet<_> = inherit
                .iter()
                .filter_map(Value::as_str)
                .chain(literals.keys().map(String::as_str))
                .collect();
            if plan
                .environment
                .keys()
                .any(|key| !allowed.contains(key.as_str()))
            {
                return Err("unit environment contains unregistered inherited input".into());
            }
        }
        let (chosen, mut operations) =
            chrono_judge_routes::order(&owned, &snapshot.plans, methods, &snapshot.execute)?;
        for op in &mut operations {
            op.method.argv = chrono_judge_routes::expand(&op.method.argv, &plan.binding);
        }
        if plan.scheduling != snapshot.scheduling
            || object!(chosen) != object!(plan.selected)
            || object!(operations) != object!(plan.operations)
        {
            return Err(
                "unit execution plan differs from registered operations/order/bounds".into(),
            );
        }
        let tool_ids: BTreeSet<_> = operations.iter().map(|o| o.method.tool.clone()).collect();
        if plan.tools.keys().cloned().collect::<BTreeSet<_>>() != tool_ids {
            return Err("unit tool set differs from plan".into());
        }
        for (id, tool) in &plan.tools {
            let definitions: Vec<_> = declarations
                .as_array()
                .ok_or("tool declarations missing")?
                .iter()
                .filter(|v| v["id"] == *id)
                .collect();
            if definitions.len() != 1 {
                return Err("unit tool declaration missing/ambiguous".into());
            }
            let declaration = definitions[0];
            process(&tool.version)?;
            let mut argv = vec![tool.path.to_string_lossy().into_owned()];
            argv.extend(strings(&declaration["version_argv"])?);
            if tool.program != declaration["program"].as_str().ok_or("tool program")?
                || tool.version.sha256 != tool.sha256
                || tool.version.executable != tool.path
                || !tool.path.is_absolute()
                || tool.version.argv != argv
                || tool.version.cwd != plan.root
                || tool.version.environment != plan.environment
                || tool.version.stdin_sha256 != sha256(&[])
                || declaration["expected_version"]
                    .as_str()
                    .is_some_and(|expected| {
                        tool.version.exit_code != 0
                            || tool.version.failure.is_some()
                            || tool.version.stdout.trim_end() != expected
                    })
            {
                return Err("unit tool version/binding mismatch".into());
            }
        }
        let expected_ops: Vec<_> = operations
            .iter()
            .map(|o| o.method.operation.clone())
            .collect();
        let executed = e["executed"].as_array().ok_or("unit executions missing")?;
        if e["operations"] != object!(expected_ops) || executed.len() != operations.len() {
            return Err("unit execution coverage missing or extra".into());
        }
        for (operation, row) in operations.iter().zip(executed) {
            let receipt: Receipt =
                serde_json::from_value(row["receipt"].clone()).map_err(|e| e.to_string())?;
            chrono_judge_routes::compare(&plan, operation, Some(&receipt))?;
            if row["operation"] != operation.method.operation
                || row["process"] != object!(receipt.process)
                || receipt.process.exit_code != 0
                || receipt.process.failure.is_some()
                || !response.results.iter().any(|r| {
                    r.id == operation.method.operation
                        && r.status == Status::Passed
                        && r.exit_code == Some(0)
                })
            {
                return Err("unit operation original evidence failed or disagrees".into());
            }
        }
        let expected_results: BTreeSet<_> = std::iter::once("ci.inventory".to_owned())
            .chain(expected_ops)
            .chain(not_required.iter().cloned())
            .collect();
        // Successful replacement markers are not emitted by the scoped producer.
        if !response
            .results
            .iter()
            .any(|r| r.id == "ci.inventory" && r.status == Status::Passed && r.exit_code.is_none())
            || response
                .results
                .iter()
                .map(|r| r.id.clone())
                .collect::<BTreeSet<_>>()
                != expected_results
            || response
                .results
                .iter()
                .any(|r| not_required.contains(&r.id) && r.status != Status::NotRequired)
        {
            return Err("unit result coverage differs from plan".into());
        }
        verified.push(object!({"unit":input.unit,"path":input.path,"sha256":input.sha256,"runner_sha256":input.runner_sha256,"judge_sha256":input.judge_sha256,"plan":plan.identity,"status":"verified"}));
        results.push(CheckResult {
            id: format!("ci.unit.{}", input.unit),
            status: Status::Passed,
            cause: "original unit report verified against fixed endpoints and candidate plan"
                .into(),
            exit_code: None,
        });
    }
    let failed = results.iter().any(|r| r.status == Status::Failed);
    Ok(Response {
        protocol: units::scope_protocol(req).into(),
        request_id: req.request_id.clone(),
        status: if failed {
            Status::Failed
        } else {
            Status::Passed
        },
        results,
        evidence: object!({"scope":config.schema,"execution_scope":req.scope,"base":req.base,"candidate":req.candidate,
            "global_selected":selected,"required_units":required,"manifest_sha256":sha256(&manifest_bytes),"reports":verified,
            "collection_limits":limits,
            "executed":[],"acceptance":"global collection; no business operations executed",
            "input_closure":"incomplete: report identity pins are caller supplied; build provenance and external input closure are not certified",
            "parity":"unestablished"}),
    })
}
