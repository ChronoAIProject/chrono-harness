//! Candidate-bound coverage admission. This adapter is itself an outstanding
//! Verus migration obligation; the host cannot claim complete verification.
use super::{Policy, Snapshot, at, verus_policy};
use chrono_harness::{CheckResult, Request, Response, Status, facts::Reader, json};
use serde::Deserialize;
use serde_json::json as object;
use std::collections::BTreeMap;

fn compiler_sources(
    req: &Request,
    reader: &Reader,
    policy: &Policy,
    snapshot: &Snapshot,
    argv: &Vec<String>,
) -> Result<serde_json::Value, String> {
    use verus_policy::sources::{self, SourceBinding};
    if !verus_policy::canonical_verifier_arguments(argv) {
        return Err("E_VERUS_SOURCES: verifier arguments differ from registered route".into());
    }
    let output = &argv[10];
    let dependency = format!("{output}.d");
    let limit = policy
        .collection_limits
        .clone()
        .unwrap_or_default()
        .report_bytes;
    let bytes = super::collect::read(
        &req.host_root,
        &dependency,
        policy,
        "report_bytes (compiler artifacts)",
        limit,
    )?;
    let paths = sources::compiler_dependency_paths(&bytes, output.as_bytes())
        .ok_or("E_VERUS_SOURCES: missing or invalid compiler dependency target")?;
    let mut compiled = Vec::new();
    let mut fixed = Vec::new();
    let mut identities = Vec::new();
    for path in paths {
        let normalized = sources::normalize_relative_path(&path)
            .ok_or("E_VERUS_SOURCES: compiler source escapes the relative path domain")?;
        let name = std::str::from_utf8(&normalized)
            .map_err(|e| format!("E_VERUS_SOURCES: source path is not UTF-8: {e}"))?;
        if !snapshot.files.contains_key(name)
            || snapshot
                .tree
                .get(name)
                .is_none_or(|(mode, _)| !matches!(mode.as_str(), "100644" | "100755"))
        {
            return Err(format!(
                "E_VERUS_SOURCES: source is not an explicit regular candidate file: {name}"
            ));
        }
        let source = at(reader, &req.host_root, &req.candidate, name)?;
        let digest = chrono_harness::sha256(&source);
        identities.push(object!({"path":name,"compiler_path_bytes":path,
            "sha256":digest,"byte_length":source.len()}));
        compiled.push(SourceBinding {
            path,
            sha256: digest.as_bytes().to_vec(),
            length: source.len() as u64,
        });
        fixed.push(SourceBinding {
            path: normalized,
            sha256: digest.into_bytes(),
            length: source.len() as u64,
        });
    }
    let root = &argv[8];
    let claims = snapshot
        .files
        .iter()
        .filter(|(_, file)| {
            file["verification"]["roots"]
                .as_array()
                .is_some_and(|roots| roots.iter().any(|value| value.as_str() == Some(root)))
        })
        .map(|(path, _)| path.as_bytes().to_vec())
        .collect();
    if !sources::admit_source_metadata(&compiled, &fixed, root.as_bytes(), &claims)
        || !sources::matches_dependency_content(&bytes, output.as_bytes(), &compiled)
    {
        return Err(
            "E_VERUS_SOURCES: candidate metadata, explicit root claims or compiler checksums differ"
                .into(),
        );
    }
    let library = super::collect::read(
        &req.host_root,
        output,
        policy,
        "report_bytes (remaining compiler artifact budget)",
        limit - bytes.len() as u64,
    )?;
    if library.is_empty() {
        return Err("E_VERUS_SOURCES: compiled library is empty".into());
    }
    Ok(
        object!({"schema":"chrono-verus-source-observation/v1","candidate":req.candidate,
        "root":root,"sources":identities,
        "dependency":{"path":dependency,"sha256":chrono_harness::sha256(&bytes),"bytes":bytes},
        "library":{"path":output,"sha256":chrono_harness::sha256(&library),"byte_length":library.len()},
        "assurance":"local post-process source comparison; library bytes and cross-unit transport remain unretained"}),
    )
}

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
    let mut compiler_evidence = Vec::new();
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
        let compiler_accepted = accepted.as_ref().is_ok_and(|value| *value);
        ledger.record(
            format!("operation:{operation}:compiler-proof"),
            compiler_accepted,
        );
        let sources = if compiler_accepted {
            compiler_sources(
                req,
                reader,
                policy,
                snapshot,
                &snapshot.operations[operation].2,
            )
        } else {
            Err("E_VERUS_SOURCES: original compiler result was not admitted".into())
        };
        ledger.record(
            format!("operation:{operation}:compiler-sources"),
            sources.is_ok(),
        );
        compiler_evidence.push(object!({"operation":operation,"compiler_accepted":compiler_accepted,
            "compiler_error":accepted.err(),"sources":sources.as_ref().ok(),"source_error":sources.as_ref().err()}));
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
        "compiler_evidence":compiler_evidence,
        "external_interfaces":requirements.external_interfaces,
        "assurance":"declarations, compiler results and local source comparison; adapter proofs, retained libraries and cross-unit transport remain required"
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
