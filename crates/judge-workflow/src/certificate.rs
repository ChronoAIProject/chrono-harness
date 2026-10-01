use crate::evidence;
use chrono_harness::{
    json, no_symlink_parents, sha256,
    wire::{self, Request},
};
use chrono_judge_projects::Results;
use chrono_judge_registration::Registrations;
use chrono_judge_routes::Execution;
use serde_json::{Value, json as value};
use std::{collections::BTreeSet, fs, path::Path};
const FIELDS: [&str; 9] = [
    "base",
    "candidate_tree",
    "registry_digest",
    "executables",
    "tools",
    "environment",
    "effective_inputs",
    "required_tests",
    "results_digest",
];
fn location(req: &Request, r: &Registrations) -> Result<std::path::PathBuf, String> {
    let path = r.workflow()["integration"]["evidence"]
        .as_str()
        .ok_or("E_INTEGRATION_BINDING: evidence path")?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("E_INTEGRATION_BINDING: evidence must be in state".into());
    }
    no_symlink_parents(&req.candidate.root, path)
}
pub fn validate_bind(r: &Registrations) -> Result<(), String> {
    let keys = r.workflow()["integration"]["bind"].as_array().unwrap();
    let supplied: BTreeSet<_> = keys.iter().filter_map(Value::as_str).collect();
    if keys.len() != FIELDS.len() || supplied != FIELDS.into_iter().collect() {
        return Err(
            "E_INTEGRATION_BINDING: all specified evidence fields must be bound exactly once"
                .into(),
        );
    }
    Ok(())
}
pub fn produce(
    req: &Request,
    r: &Registrations,
    ctx: &Value,
    mut binding: Value,
    plan: &Execution,
    results: &Results,
    effective: &Value,
) -> Result<Value, String> {
    let report = req.observations["report_path"]
        .as_str()
        .filter(|p| p.starts_with(".chrono-harness/state/"))
        .ok_or("E_WORKFLOW_EVIDENCE: retained runner report path missing")?;
    binding["schema_version"] = 1.into();
    binding["status"] = "passed".into();
    binding["results_digest"] = wire::digest(results)?.into();
    for k in [
        "branch_ref",
        "fork_point",
        "branch_started_at",
        "observed_at",
    ] {
        binding[k] = ctx[k].clone();
    }
    binding["producer"] = value!({"judge":req.judge_id,"request_id":req.request_id,"candidate":req.candidate.commit,"root":req.candidate.root,"report_path":report,"context_digest":req.context.sha256});
    if let Some(directory) =
        req.observations["preparation"]["request"]["native_artifacts"]["directory"].as_str()
    {
        binding["producer"]["artifact_directory"] = value!(directory);
    }
    binding["proof"] = value!({"context":ctx,"plan":plan,"results":results,"effective_inputs":effective,"judges":req.observations["judges"]});
    let path = location(req, r)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = wire::canonical(&binding)?;
    fs::write(path, &bytes).map_err(|e| e.to_string())?;
    Ok(
        value!({"sha256":sha256(&bytes),"path":r.workflow()["integration"]["evidence"],"producer_report":report,"validation_scope":"declared inputs; complete runtime closure remains a separate obligation"}),
    )
}
fn report_output<'a>(report: &'a Value, key: &str) -> Result<&'a Value, String> {
    let values: Vec<_> = report["judges"]
        .as_array()
        .ok_or("missing judges")?
        .iter()
        .filter_map(|j| j["response"]["outputs"].get(key))
        .collect();
    if values.is_empty() || values.windows(2).any(|p| p[0] != p[1]) {
        return Err(format!("missing/conflicting report {key}"));
    }
    Ok(values[0])
}
pub fn consume(
    reader: &chrono_harness::facts::Reader,
    req: &Request,
    r: &Registrations,
    ctx: &Value,
    binding: &Value,
) -> Result<Value, String> {
    let digest = ctx["integration_evidence"]
        .as_str()
        .ok_or("E_INTEGRATION_REQUIRED: no evidence digest")?;
    let path = location(req, r)?;
    let bytes = fs::read(path).map_err(|e| format!("E_INTEGRATION_REQUIRED: {e}"))?;
    let check = || -> Result<Value, String> {
        if sha256(&bytes) != digest {
            return Err("certificate byte digest differs".into());
        }
        let c = json(&bytes)?;
        if c["schema_version"] != 1 || c["status"] != "passed" {
            return Err("certificate did not succeed".into());
        }
        for field in FIELDS.into_iter().filter(|k| *k != "results_digest") {
            if c[field] != binding[field] {
                return Err(format!("different binding {field}"));
            }
        }
        let proof = &c["proof"];
        let producer = &c["producer"];
        let candidate = producer["candidate"]
            .as_str()
            .ok_or("missing producer candidate")?;
        if reader.verify_oid(&req.candidate.root, candidate)? != req.candidate.tree {
            return Err("producer candidate tree differs".into());
        }
        let old_ctx = &proof["context"];
        if old_ctx["schema_version"] != 2
            || old_ctx["run_kind"] != "integration"
            || old_ctx["candidate"] != candidate
            || wire::digest(old_ctx)? != producer["context_digest"]
        {
            return Err("producer context differs".into());
        }
        for k in [
            "branch_ref",
            "fork_point",
            "branch_started_at",
            "observed_at",
        ] {
            if c[k] != old_ctx[k] {
                return Err(format!("producer {k} differs"));
            }
        }
        // Evidence must still be fresh at this observation, not only at its original run.
        let timestamp = |v: &Value| {
            time::OffsetDateTime::parse(
                v["observed_at"].as_str().unwrap_or(""),
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| e.to_string())
        };
        if timestamp(old_ctx)? > timestamp(ctx)? {
            return Err("producer observation is in the future".into());
        }
        let mut now = old_ctx.clone();
        now["observed_at"] = ctx["observed_at"].clone();
        if crate::branch::with_reader(reader, &req.candidate.root, candidate, &now, r.workflow())?["kind"]
            != "integration"
        {
            return Err("producer is not integration".into());
        }
        let plan: Execution =
            serde_json::from_value(proof["plan"].clone()).map_err(|e| e.to_string())?;
        let results: Results =
            serde_json::from_value(proof["results"].clone()).map_err(|e| e.to_string())?;
        evidence::execution(&plan, &results)?;
        if c["results_digest"] != wire::digest(&results)?
            || plan.root != Path::new(producer["root"].as_str().ok_or("producer root")?)
            || plan.binding["candidate"] != candidate
            || plan.binding["base"] != req.base.commit
            || plan.binding["registry"] != req.registries.digest
            || plan.binding["context"] != producer["context_digest"]
        {
            return Err("producer plan/results binding differs".into());
        }
        if evidence::inputs(
            &proof["effective_inputs"],
            &req.base.commit,
            candidate,
            &req.candidate.tree,
            &plan.root,
        )? != c["effective_inputs"]
        {
            return Err("retained effective inputs differ".into());
        }
        let report_path = producer["report_path"]
            .as_str()
            .filter(|p| p.starts_with(".chrono-harness/state/"))
            .ok_or("retained report path")?;
        // The producer transports its retained report together with the certificate.
        let transport = artifact_transport(req)?;
        if transport
            .as_ref()
            .is_some_and(|t| producer["artifact_directory"] != t.source_directory)
        {
            return Err("producer report transport root differs".into());
        }
        let physical = chrono_harness::prepared::original_path(
            &chrono_harness::prepared::Original {
                path: report_path.into(),
                sha256: String::new(),
            },
            transport.as_ref(),
        )?;
        let report_file = no_symlink_parents(&req.candidate.root, &physical)?;
        if results.completion.is_some()
            && fs::metadata(&report_file).map_err(|e| e.to_string())?.len()
                > r.config()["execution_units"]["collection_limits"]["report_bytes"]
                    .as_u64()
                    .ok_or("collected producer report bound")?
        {
            return Err("collected producer report exceeds registered bound".into());
        }
        let report_bytes = fs::read(&report_file)
            .map_err(|e| format!("producer report absent/not finalized: {e}"))?;
        let report = json(&report_bytes)?;
        if !matches!(report["status"].as_str(), Some("pass" | "warn"))
            || report["report_path"] != report_path
            || report["base"] != req.base.commit
            || report["candidate"] != candidate
            || report["candidate_tree"] != req.candidate.tree
            || report["registry_digest"] != req.registries.digest
            || report["context_digest"] != producer["context_digest"]
        {
            return Err("producer runner did not finalize a matching successful report".into());
        }
        if report_output(&report, "execution_plan")? != &proof["plan"]
            || report_output(&report, "tests")? != &proof["results"]
            || report_output(&report, "effective_inputs")? != &proof["effective_inputs"]
        {
            return Err("producer report outputs differ".into());
        }
        let expanded = report["judges"]
            .as_array()
            .ok_or("producer judges")?
            .iter()
            .map(chrono_harness::full::expand_record)
            .collect::<Result<Vec<_>, _>>()?;
        let records = &expanded;
        if results.completion.is_some() {
            let original: Request =
                serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
            original.validate()?;
            if original.candidate.root != plan.root
                || original.context.sha256 != producer["context_digest"]
            {
                return Err("collected producer request differs".into());
            }
            let bindings: Vec<wire::Binding> =
                serde_json::from_value(r.judges()["judges"].clone()).map_err(|e| e.to_string())?;
            let parsed = chrono_harness::full::retained_judges(&original, &bindings, records)?;
            let aggregate = parsed
                .values()
                .fold(wire::Status::Pass, |status, (_, _, response, _)| {
                    status.max(response.status.clone())
                });
            let findings: Vec<Value> = records
                .iter()
                .flat_map(|row| {
                    row["response"]["findings"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .cloned()
                })
                .collect();
            if report["status"] != serde_json::to_value(aggregate).map_err(|e| e.to_string())?
                || report["findings"] != value!(findings)
            {
                return Err("collected producer summary differs from actual judges".into());
            }
            let original = &parsed
                .get(producer["judge"].as_str().ok_or("producer judge")?)
                .ok_or("producer workflow input")?
                .0;
            if original.request_id != producer["request_id"]
                || original.impact != req.impact
                || plan.binding
                    != chrono_judge_routes::binding(original, &proof["effective_inputs"])
            {
                return Err("collected producer stdin/global obligation binding differs".into());
            }
            let current_plan: Execution =
                serde_json::from_value(evidence::output(req, "execution_plan")?.clone())
                    .map_err(|e| e.to_string())?;
            let expected = chrono_judge_routes::prepare_collection(
                &plan.root,
                plan.binding.clone(),
                &current_plan.selected.keys().cloned().collect(),
                &chrono_judge_registration::execution::plans(r.filemap())?,
                &chrono_judge_registration::execution::methods(r.projects())?,
                &chrono_judge_routes::execute_actions(r),
                current_plan.environment,
            )?;
            if serde_json::to_value(&expected).map_err(|e| e.to_string())? != proof["plan"] {
                return Err("collected global declaration plan differs".into());
            }
            let (old, _, _) = chrono_judge_registration::views_with_reader(req, reader)?;
            if chrono_judge_projects::validate_retained_request_at(
                original,
                &old,
                r,
                &report["artifacts"],
                &req.candidate.root,
                transport.as_ref(),
            )? != proof["effective_inputs"]
            {
                return Err("collected producer retained inputs differ".into());
            }
            chrono_judge_projects::verify_completion(
                original,
                r,
                &old,
                &proof["effective_inputs"],
                &plan,
                &results,
                Some(&chrono_harness::retained_artifacts::transport_map(
                    &report["artifacts"],
                    transport.as_ref(),
                )?),
                &req.candidate.root,
            )?;
        }
        let observations = proof["judges"].as_array().ok_or("producer observations")?;
        if records.len() != observations.len() + 1 {
            return Err("producer judge set differs".into());
        }
        for old in observations {
            let expanded = chrono_harness::full::expand_record(old)?;
            let old = &expanded;
            let matches: Vec<_> = records.iter().filter(|v| v["id"] == old["id"]).collect();
            if matches.len() != 1 {
                return Err("producer judge observation missing/duplicate".into());
            }
            for (k, v) in old.as_object().ok_or("producer observation")? {
                if matches[0][k] != *v {
                    return Err("producer observation changed".into());
                }
            }
        }
        let mut seen = BTreeSet::new();
        let mut executable_map = serde_json::Map::new();
        let runner = &report["executables"][0];
        let rp = runner["path"].as_str().ok_or("runner path")?;
        executable_map.insert(
            "runner".into(),
            value!({"path":evidence::relative(&plan.root,Path::new(rp)),"sha256":runner["sha256"]}),
        );
        for row in records {
            let (response, p) = evidence::record(row)?;
            if !seen.insert(response.judge_id.clone()) {
                return Err("duplicate observed judge".into());
            }
            executable_map.insert(
                format!("judge:{}", response.judge_id),
                value!({"path":evidence::relative(&plan.root,&p.executable),"sha256":p.sha256}),
            );
        }
        if Value::Object(executable_map) != c["executables"] {
            return Err("producer executable observations differ".into());
        }
        let own: Vec<_> = records
            .iter()
            .filter(|v| v["id"] == producer["judge"])
            .collect();
        if own.len() != 1
            || own[0]["request_id"] != producer["request_id"]
            || own[0]["response"]["outputs"]["workflow"]["mode"] != "integration_run"
            || own[0]["response"]["outputs"]["workflow"]["integration"]["sha256"] != digest
        {
            return Err("producer workflow result differs".into());
        }
        Ok(
            value!({"sha256":digest,"producer_report":report_path,"producer_report_sha256":sha256(&report_bytes),"producer_candidate":candidate,"consumer_candidate":req.candidate.commit,"candidate_tree":req.candidate.tree,"commit_metadata_mapped":candidate!=req.candidate.commit}),
        )
    };
    check().map_err(|e| format!("E_INTEGRATION_MISMATCH: {e}"))
}

/// Host selection comes from the bound native provider; no destination-origin inference.
fn artifact_transport(
    req: &Request,
) -> Result<Option<chrono_harness::prepared::ArtifactTransport>, String> {
    let native = &req.observations["preparation"]["request"]["native_artifacts"];
    let Some(path) = native["config_path"].as_str() else {
        return Ok(None);
    };
    let bytes =
        fs::read(no_symlink_parents(&req.candidate.root, path)?).map_err(|e| e.to_string())?;
    if native["config_sha256"] != sha256(&bytes) {
        return Err("certificate transport provider drift".into());
    }
    let provider = json(&bytes)?;
    if provider["native_adoption"].is_null() {
        return Ok(None);
    }
    if provider["native_adoption"]["schema"] != "chrono-native-adoption/v1" {
        return Err("certificate transport extension schema".into());
    }
    serde_json::from_value(provider["native_adoption"]["integration_transport"].clone())
        .map_err(|e| e.to_string())
}
