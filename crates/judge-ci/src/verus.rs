//! Candidate-bound coverage admission. This adapter is itself an outstanding
//! Verus migration obligation; the host cannot claim complete verification.
use super::{Policy, Snapshot, at, verus_policy};
use chrono_harness::{CheckResult, Request, Response, Status, facts::Reader, json};
use serde::Deserialize;
use serde_json::json as object;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Requirements {
    schema: String,
    tool: String,
    version: String,
    commit: String,
    external_interfaces: BTreeMap<String, String>,
}

pub(super) fn apply(
    req: &Request,
    reader: &Reader,
    policy: &Policy,
    snapshot: &Snapshot,
    response: &mut Response,
) -> Result<(), String> {
    let Some(path) = &policy.verus else {
        return Ok(());
    };
    let required_inputs: Vec<_> = policy.required_inputs.iter().map(String::as_str).collect();
    if !verus_policy::contains_text(&required_inputs, path) {
        return Err("E_VERUS_INPUT: requirements must be a registered required input".into());
    }
    let requirements: Requirements =
        serde_json::from_value(json(&at(reader, &req.host_root, &req.candidate, path)?)?)
            .map_err(|e| format!("E_VERUS_INPUT: {e}"))?;
    let tools: Vec<_> = policy.tools.keys().map(String::as_str).collect();
    let fields = verus_policy::RequirementFields {
        schema: &requirements.schema,
        tool: &requirements.tool,
        version: &requirements.version,
        commit: &requirements.commit,
        interfaces: requirements
            .external_interfaces
            .iter()
            .map(|(name, assumption)| (name.as_str(), assumption.as_str()))
            .collect(),
    };
    if !verus_policy::valid_requirement_fields(&fields, &tools) {
        return Err("E_VERUS_INPUT: invalid schema, tool or explicit external interfaces".into());
    }
    let mut ledger = verus_policy::Ledger::new();
    for execution in response.evidence["executed"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(operation) = execution["operation"].as_str() else {
            continue;
        };
        if snapshot
            .operations
            .get(operation)
            .is_none_or(|(_, tool, _)| tool != &requirements.tool)
        {
            continue;
        }
        let accepted = (|| -> Result<bool, String> {
            let original = chrono_harness::full::expand_process(&execution["receipt"]["process"])?;
            let bytes: Vec<u8> = serde_json::from_value(original["stdout_bytes"].clone())
                .map_err(|e| format!("E_VERUS_RESULT: original stdout bytes: {e}"))?;
            let proof = json(&bytes)?;
            let result = &proof["verification-results"];
            Ok(verus_policy::accept_compiler_result(
                &verus_policy::CompilerResult {
                    exit_code: original["exit_code"].as_i64(),
                    success: result["success"].as_bool(),
                    encountered_error: result["encountered-error"].as_bool(),
                    encountered_vir_error: result["encountered-vir-error"].as_bool(),
                    errors: result["errors"].as_u64(),
                    verified: result["verified"].as_u64(),
                    entire_crate: result["is-verifying-entire-crate"].as_bool(),
                    version: proof["verus"]["version"].as_str().map(str::as_bytes),
                    commit: proof["verus"]["commit"].as_str().map(str::as_bytes),
                },
                requirements.version.as_bytes(),
                requirements.commit.as_bytes(),
            ))
        })();
        ledger.record(
            format!("operation:{operation}:compiler-proof"),
            accepted.unwrap_or(false),
        );
    }
    // Roots are bound to exact registered invocations. Original execution and
    // receipt checking remain the projects owner's responsibility.
    let roots: BTreeMap<_, _> = snapshot
        .operations
        .iter()
        .filter_map(|(operation, (_, tool, argv))| {
            if tool != &requirements.tool
                || !verus_policy::canonical_verifier_arguments(argv)
                || chrono_harness::relative_path(&argv[10]).is_err()
            {
                return None;
            }
            let root = &argv[8];
            let planned = snapshot
                .plans
                .values()
                .any(|plan| plan.operations.contains(operation));
            planned.then_some((root.clone(), operation.clone()))
        })
        .collect();
    for (id, unit) in &snapshot.projects {
        ledger.record(
            format!("unit:{id}:language=verus"),
            unit["language"] == "verus",
        );
        ledger.record(
            format!("unit:{id}:registered-verifier"),
            unit["actions"].as_object().is_some_and(|actions| {
                actions.values().any(|action| {
                    action["operation"]
                        .as_str()
                        .is_some_and(|op| roots.values().any(|value| value == op))
                })
            }),
        );
    }
    let mut specifications = 0;
    for (path, file) in &snapshot.files {
        let verification = &file["verification"];
        let kind = verification["kind"].as_str();
        ledger.record(
            format!("file:{path}:declared-role"),
            matches!(kind, Some("program" | "specification" | "data")),
        );
        if kind == Some("specification") {
            specifications += 1;
        }
        if matches!(kind, Some("program" | "specification")) {
            ledger.record(
                format!("file:{path}:verification-roots"),
                verification["roots"].as_array().is_some_and(|declared| {
                    !declared.is_empty()
                        && declared.iter().all(|root| {
                            root.as_str().is_some_and(|root| {
                                roots.contains_key(root) && snapshot.files.contains_key(root)
                            })
                        })
                }),
            );
        }
    }
    ledger.record("specifications:nonempty".into(), specifications > 0);
    let coverage = ledger.finish();
    let complete = coverage.complete;
    response.evidence["verus"] = object!({
        "schema":"chrono-verus-coverage/v1",
        "complete":complete,
        "obligations":coverage.obligations,
        "missing":coverage.missing,
        "external_interfaces":requirements.external_interfaces,
        "assurance":"declaration and registered-route admission; compiler proofs and source correspondence remain required"
    });
    response.results.push(CheckResult {
        id: "ci.verus-coverage".into(),
        status: if complete {
            Status::Passed
        } else {
            Status::Failed
        },
        cause: if complete {
            "Verus declarations and canonical verification routes cover the inventory".into()
        } else {
            format!(
                "E_VERUS_COVERAGE: {} unmet obligations; first: {}",
                coverage.missing.len(),
                coverage.first_missing().unwrap_or("empty inventory")
            )
        },
        exit_code: None,
    });
    if !complete {
        response.status = Status::Failed;
    }
    Ok(())
}
