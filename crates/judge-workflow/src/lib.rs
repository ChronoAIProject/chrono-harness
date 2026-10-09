//! Workflow owns freshness, explicit evolution and integration evidence policy.
mod branch;
mod certificate;
mod evidence;
mod transition;
pub use branch::{branch, current_observation_age, observation_age};
use chrono_harness::{
    facts, json,
    wire::{self, Finding, Request, Response, Status},
};
use chrono_judge_projects::Results;
use chrono_judge_routes::Execution;
use serde_json::{Value, json as value};

pub fn judge(req: &Request) -> Response {
    let mut r = req.response(Status::Pass);
    let reader = facts::Reader::for_request(req);
    let result = reader
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|reader| evaluate(req, reader));
    if let Ok(reader) = &reader {
        reader.record(&mut r);
    }
    match result {
        Ok(v) => {
            r.outputs.insert("workflow".into(), v);
        }
        Err(message) => {
            let code = message
                .split(':')
                .next()
                .filter(|s| s.starts_with("E_"))
                .unwrap_or("E_WORKFLOW_INPUT")
                .to_string();
            r.status = if matches!(
                code.as_str(),
                "E_BRANCH_STALE"
                    | "E_RETIREMENT"
                    | "E_REQUIRED_TEST_REMOVED"
                    | "E_INTEGRATION_REQUIRED"
                    | "E_INTEGRATION_MISMATCH"
            ) {
                Status::Fail
            } else {
                Status::Error
            };
            r.findings.push(Finding {
                code,
                level: "error".into(),
                message,
                delta_refs: vec!["/request/context".into()],
                causes: vec![],
            });
        }
    }
    r
}
fn evaluate(req: &Request, reader: &facts::Reader) -> Result<Value, String> {
    req.validate()?;
    let (a, b, view) = chrono_judge_registration::views_with_reader(req, reader)?;
    let ctx = json(&std::fs::read(&req.context.path).map_err(|e| e.to_string())?)?;
    if wire::digest(&ctx)? != req.context.sha256
        || ctx["schema_version"] != 2
        || ctx["candidate"] != req.candidate.commit
        || ctx["base"] != req.base.commit
    {
        return Err("E_BRANCH_CONTEXT: workflow requires matching explicit v2 context".into());
    }
    let mut state = branch::with_reader(
        reader,
        &req.candidate.root,
        &req.candidate.commit,
        &ctx,
        b.workflow(),
    )?;
    let observation = &req.observations["preparation"]["result"]["evidence"]["current_observation"];
    let provider =
        req.observations["preparation"]["request"]["native_artifacts"]["config_path"].as_str();
    let requires_current = if let Some(path) = provider {
        let raw = std::fs::read(chrono_harness::no_symlink_parents(
            &req.candidate.root,
            path,
        )?)
        .map_err(|e| e.to_string())?;
        json(&raw)?.get("native_adoption").is_some()
    } else {
        false
    };
    if requires_current || !observation.is_null() {
        if observation["context_digest"] != req.context.sha256 {
            return Err("E_BRANCH_CONTEXT: current observation context binding differs".into());
        }
        let age = branch::current_observation_age(&ctx, b.workflow(), observation)?;
        state["age_seconds"] = value!(age.as_seconds_f64());
        state["current_observation"] = observation.clone();
    }
    if !matches!(ctx["run_kind"].as_str(), Some("integration" | "delivery"))
        || (ctx["run_kind"] == "integration" && state["kind"] != "integration")
    {
        return Err("E_BRANCH_CONTEXT: invalid explicit run kind".into());
    }
    let impact = evidence::output(req, "impact")?;
    if impact["schema"] != "chrono-filemap-impact/v2"
        || impact["delta"] != serde_json::to_value(&req.delta).unwrap()
        || impact["workflow"]["schema"] != "chrono-workflow-selection/v1"
    {
        return Err("E_WORKFLOW_INPUT: impact identity".into());
    }
    let required = impact["workflow"]["integration_required"]
        .as_bool()
        .ok_or("E_WORKFLOW_INPUT: integration selection")?;
    let plan: Execution = serde_json::from_value(evidence::output(req, "execution_plan")?.clone())
        .map_err(|e| e.to_string())?;
    let results: Results = serde_json::from_value(evidence::output(req, "tests")?.clone())
        .map_err(|e| e.to_string())?;
    let effective = chrono_judge_registration::inputs::validate(req, &a, &b)?;
    let binding = evidence::binding(req, &b, &plan, &results, impact, &effective)?;
    if let Some(chrono_harness::units::Scope::Unit { unit }) = &req.scope {
        let global_selected = req
            .prior_results
            .iter()
            .find_map(|result| result.outputs.get("global_selected"))
            .cloned()
            .unwrap_or_else(|| impact["tests"].clone());
        let selected: std::collections::BTreeSet<String> =
            serde_json::from_value(global_selected.clone())
                .map_err(|e| format!("E_WORKFLOW_INPUT: global selection: {e}"))?;
        let units: std::collections::BTreeMap<String, chrono_harness::units::Unit> =
            serde_json::from_value(req.observations["execution_units"]["units"].clone())
                .map_err(|e| format!("E_WORKFLOW_INPUT: execution units: {e}"))?;
        let required_units = chrono_harness::units::required(&units, &selected);
        return Ok(value!({
            "schema":"chrono-workflow-verdict/v1",
            "mode":"contribution",
            "acceptance":"unit-only; global completion requires collection",
            "unit":unit,
            "global_selected":global_selected,
            "selected":plan.selected,
            "required_units":required_units,
            "integration_required":impact["workflow"]["integration_required"],
            "integration":Value::Null,
            "completion":"pending collection",
            "binding":binding
        }));
    }
    let transitions = transition::evaluate(
        reader,
        req,
        &a,
        &b,
        impact,
        &results,
        &view,
        evidence::output(req, "costs")?,
    )?;
    certificate::validate_bind(&b)?;
    let (mode, integration) = if ctx["run_kind"] == "integration" {
        (
            "integration_run",
            certificate::produce(req, &b, &ctx, binding, &plan, &results, &effective)?,
        )
    } else if required {
        (
            "delivery",
            certificate::consume(
                reader,
                req,
                &b,
                &ctx,
                &binding,
                (!observation.is_null()).then_some(observation),
            )?,
        )
    } else {
        ("ordinary_delta", Value::Null)
    };
    Ok(
        value!({"schema":"chrono-workflow-verdict/v1","mode":mode,"branch":state,"integration_required":required,"integration":integration,"transitions":transitions,"input_completeness_proven":false}),
    )
}

// Explicit fixture source DELTA.
