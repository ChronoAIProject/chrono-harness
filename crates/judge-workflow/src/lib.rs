//! Workflow owns freshness, explicit evolution and integration evidence policy.
mod branch;
mod certificate;
mod evidence;
mod transition;
pub use branch::branch;
use chrono_harness::{
    json,
    wire::{self, Finding, Request, Response, Status},
};
use chrono_judge_projects::Results;
use chrono_judge_routes::Execution;
use serde_json::{Value, json as value};

pub fn judge(req: &Request) -> Response {
    let mut r = req.response(Status::Pass);
    match evaluate(req) {
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
fn evaluate(req: &Request) -> Result<Value, String> {
    req.validate()?;
    let (a, b, view) = chrono_judge_registration::views(req)?;
    let ctx = json(&std::fs::read(&req.context.path).map_err(|e| e.to_string())?)?;
    if wire::digest(&ctx)? != req.context.sha256
        || ctx["schema_version"] != 2
        || ctx["candidate"] != req.candidate.commit
        || ctx["base"] != req.base.commit
    {
        return Err("E_BRANCH_CONTEXT: workflow requires matching explicit v2 context".into());
    }
    let state = branch(
        &req.candidate.root,
        &req.candidate.commit,
        &ctx,
        b.workflow(),
    )?;
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
    let transitions = transition::evaluate(
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
        ("delivery", certificate::consume(req, &b, &ctx, &binding)?)
    } else {
        ("ordinary_delta", Value::Null)
    };
    Ok(
        value!({"schema":"chrono-workflow-verdict/v1","mode":mode,"branch":state,"integration_required":required,"integration":integration,"transitions":transitions,"input_completeness_proven":false}),
    )
}
