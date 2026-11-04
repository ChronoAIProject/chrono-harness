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
    if !policy.required_inputs.contains(path) {
        return Err("E_VERUS_INPUT: requirements must be a registered required input".into());
    }
    let requirements: Requirements =
        serde_json::from_value(json(&at(reader, &req.host_root, &req.candidate, path)?)?)
            .map_err(|e| format!("E_VERUS_INPUT: {e}"))?;
    if requirements.schema != "chrono-verus/v1"
        || !policy.tools.contains_key(&requirements.tool)
        || requirements.version.is_empty()
        || requirements.commit.is_empty()
        || requirements.external_interfaces.is_empty()
        || requirements
            .external_interfaces
            .iter()
            .any(|(name, assumption)| name.is_empty() || assumption.is_empty())
    {
        return Err("E_VERUS_INPUT: invalid schema, tool or explicit external interfaces".into());
    }
    let mut identities = Vec::new();
    let mut obligations = Vec::new();
    let mut add = |identity: String, accepted: bool| {
        identities.push(identity);
        obligations.push(accepted);
    };
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
            Ok(original["exit_code"] == 0
                && result["success"] == true
                && result["encountered-error"] == false
                && result["encountered-vir-error"] == false
                && result["errors"] == 0
                && result["verified"].as_u64().is_some_and(|count| count > 0)
                && result["is-verifying-entire-crate"] == true
                && proof["verus"]["version"] == requirements.version
                && proof["verus"]["commit"] == requirements.commit)
        })();
        add(
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
                || argv.len() != 9
                || argv[7] != "-o"
                || chrono_harness::relative_path(&argv[8]).is_err()
                || argv[..6]
                    != [
                        "--crate-type=lib",
                        "--no-cheating",
                        "--no-external-by-default",
                        "--num-threads",
                        "2",
                        "--output-json",
                    ]
            {
                return None;
            }
            let root = &argv[6];
            let planned = snapshot
                .plans
                .values()
                .any(|plan| plan.operations.contains(operation));
            planned.then_some((root.clone(), operation.clone()))
        })
        .collect();
    for (id, unit) in &snapshot.projects {
        add(
            format!("unit:{id}:language=verus"),
            unit["language"] == "verus",
        );
        add(
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
        add(
            format!("file:{path}:declared-role"),
            matches!(kind, Some("program" | "specification" | "data")),
        );
        if kind == Some("specification") {
            specifications += 1;
        }
        if matches!(kind, Some("program" | "specification")) {
            add(
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
    add("specifications:nonempty".into(), specifications > 0);
    let complete = verus_policy::complete(&obligations);
    let missing: Vec<_> = identities
        .iter()
        .zip(&obligations)
        .filter_map(|(id, accepted)| (!accepted).then_some(id))
        .collect();
    response.evidence["verus"] = object!({
        "schema":"chrono-verus-coverage/v1",
        "complete":complete,
        "obligations":identities.len(),
        "missing":missing,
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
                missing.len(),
                missing[0]
            )
        },
        exit_code: None,
    });
    if !complete {
        response.status = Status::Failed;
    }
    Ok(())
}
