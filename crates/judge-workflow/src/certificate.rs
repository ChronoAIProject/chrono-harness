use crate::{branch, evidence};
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
        if chrono_harness::facts::verify_oid(&req.candidate.root, candidate)? != req.candidate.tree
        {
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
        if branch(&req.candidate.root, candidate, &now, r.workflow())?["kind"] != "integration" {
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
        let report_bytes = fs::read(no_symlink_parents(&req.candidate.root, report_path)?)
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
        let records = report["judges"].as_array().ok_or("producer judges")?;
        let observations = proof["judges"].as_array().ok_or("producer observations")?;
        if records.len() != observations.len() + 1 {
            return Err("producer judge set differs".into());
        }
        for old in observations {
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
