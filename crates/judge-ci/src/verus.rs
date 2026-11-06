//! Candidate-bound coverage admission. This adapter is itself an outstanding
//! Verus migration obligation; the host cannot claim complete verification.
use super::{Policy, Snapshot, at, verus_policy};
use chrono_harness::{CheckResult, Request, Response, Status, facts::Reader, json};
use serde::{Deserialize, Serialize};
use serde_json::json as object;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompilerArtifact {
    path: String,
    pub original: chrono_harness::prepared::Original,
    pub byte_length: u64,
}

/// Typed acquisition only: proof/result/source acceptance remains separate.
pub(super) fn compiler_artifacts(
    evidence: &serde_json::Value,
) -> Result<Vec<CompilerArtifact>, String> {
    let Some(rows) = evidence
        .get("verus")
        .and_then(|value| value.get("compiler_evidence"))
    else {
        return Ok(Vec::new());
    };
    let rows = rows
        .as_array()
        .ok_or("E_VERUS_ARTIFACT: compiler evidence must be an array")?;
    let mut artifacts = Vec::new();
    for row in rows {
        let Some(sources) = row.get("sources").filter(|value| !value.is_null()) else {
            continue;
        };
        if sources["schema"] != "chrono-verus-source-observation/v2" {
            return Err("E_VERUS_ARTIFACT: unsupported compiler source observation".into());
        }
        for name in ["dependency", "library"] {
            artifacts.push(
                serde_json::from_value(sources[name].clone())
                    .map_err(|e| format!("E_VERUS_ARTIFACT: invalid {name} original: {e}"))?,
            );
        }
    }
    Ok(artifacts)
}

fn retain_compiler_artifact(
    req: &Request,
    path: &str,
    prefix: &str,
    bytes: &[u8],
) -> Result<CompilerArtifact, String> {
    let input = req.observations["preparation"].get("request");
    let directory = match input {
        Some(input) => {
            let input: chrono_harness::prepared::InputRequest =
                serde_json::from_value(input.clone()).map_err(|e| e.to_string())?;
            chrono_harness::prepared::retention_directory(&input)
        }
        None => ".chrono-harness/state/preparation/".into(),
    };
    let original =
        chrono_harness::prepared::retain_original(&req.host_root, &directory, prefix, bytes)?;
    let length = bytes.len() as u64;
    if !verus_policy::sources::admit_artifact(
        chrono_harness::sha256(bytes).as_bytes(),
        original.sha256.as_bytes(),
        length,
        length,
        length,
    ) {
        return Err("E_VERUS_ARTIFACT: retained compiler identity mismatch".into());
    }
    Ok(CompilerArtifact {
        path: path.into(),
        original,
        byte_length: length,
    })
}

fn candidate_sources(
    req: &Request,
    reader: &Reader,
    snapshot: &Snapshot,
    argv: &Vec<String>,
    bytes: &[u8],
) -> Result<serde_json::Value, String> {
    use verus_policy::sources::{self, SourceBinding};
    if !verus_policy::canonical_verifier_arguments(argv) {
        return Err("E_VERUS_SOURCES: verifier arguments differ from registered route".into());
    }
    let output = &argv[10];
    let paths = sources::compiler_dependency_paths(bytes, output.as_bytes())
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
        || !sources::matches_dependency_content(bytes, output.as_bytes(), &compiled)
    {
        return Err(
            "E_VERUS_SOURCES: candidate metadata, explicit root claims or compiler checksums differ"
                .into(),
        );
    }
    Ok(object!(identities))
}

fn compiler_sources(
    req: &Request,
    reader: &Reader,
    policy: &Policy,
    snapshot: &Snapshot,
    argv: &Vec<String>,
) -> Result<serde_json::Value, String> {
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
    let identities = candidate_sources(req, reader, snapshot, argv, &bytes)?;
    let library = super::collect::read(
        &req.host_root,
        output,
        policy,
        "report_bytes (remaining compiler artifact budget)",
        limit - bytes.len() as u64,
    )?;
    let dependency = retain_compiler_artifact(req, &dependency, "verus-dependency", &bytes)?;
    let library = retain_compiler_artifact(req, output, "verus-library", &library)?;
    Ok(
        object!({"schema":"chrono-verus-source-observation/v2","candidate":req.candidate,
        "root":argv[8],"sources":identities,
        "dependency":dependency,"library":library,
        "assurance":"local source comparison and addressed compiler originals"}),
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

fn requirements(
    req: &Request,
    reader: &Reader,
    policy: &Policy,
) -> Result<Option<Requirements>, String> {
    let Some(path) = &policy.verus else {
        return Ok(None);
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
    Ok(Some(requirements))
}

fn compiler_result(
    execution: &serde_json::Value,
    requirements: &Requirements,
) -> Result<bool, String> {
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
}

/// Constructed only after original receipt and candidate-source re-admission.
pub(super) struct CollectedAdmission {
    operation: String,
    sources: serde_json::Value,
}

pub(super) fn readmit_collected(
    req: &Request,
    reader: &Reader,
    snapshot: &Snapshot,
    evidence: &serde_json::Value,
    originals: &BTreeMap<(String, String), Vec<u8>>,
    transport: Option<&chrono_harness::prepared::ArtifactTransport>,
) -> Result<Vec<CollectedAdmission>, String> {
    let Some(requirements) = requirements(req, reader, &snapshot.policy)? else {
        return Ok(Vec::new());
    };
    if evidence["verus"]["schema"] != "chrono-verus-coverage/v1"
        || evidence["verus"]["complete"] != true
    {
        return Err("E_VERUS_COLLECTION: unit coverage observation was not admitted".into());
    }
    let rows = evidence["verus"]["compiler_evidence"]
        .as_array()
        .ok_or("E_VERUS_COLLECTION: compiler evidence missing")?;
    let mut supplied = BTreeMap::new();
    for row in rows {
        let operation = row["operation"]
            .as_str()
            .ok_or("E_VERUS_COLLECTION: compiler operation missing")?;
        if supplied.insert(operation, row).is_some() {
            return Err("E_VERUS_COLLECTION: duplicate compiler evidence".into());
        }
    }
    let mut admitted = Vec::new();
    for execution in evidence["executed"]
        .as_array()
        .ok_or("E_VERUS_COLLECTION: original executions missing")?
    {
        let operation = execution["operation"]
            .as_str()
            .ok_or("E_VERUS_COLLECTION: original operation missing")?;
        let Some((_, tool, argv)) = snapshot.operations.get(operation) else {
            return Err("E_VERUS_COLLECTION: unregistered original operation".into());
        };
        if tool != &requirements.tool {
            continue;
        }
        let row = supplied
            .remove(operation)
            .ok_or("E_VERUS_COLLECTION: compiler evidence does not cover original operations")?;
        if !compiler_result(execution, &requirements)?
            || row["compiler_accepted"] != true
            || !row["compiler_error"].is_null()
            || !row["source_error"].is_null()
            || !verus_policy::canonical_verifier_arguments(argv)
        {
            return Err(
                "E_VERUS_COLLECTION: original compiler result or route was not admitted".into(),
            );
        }
        let mut source = row["sources"].clone();
        if source["schema"] != "chrono-verus-source-observation/v2" {
            return Err("E_VERUS_COLLECTION: source observation schema mismatch".into());
        }
        let mut dependency: CompilerArtifact = serde_json::from_value(source["dependency"].clone())
            .map_err(|e| format!("E_VERUS_COLLECTION: dependency declaration: {e}"))?;
        let mut library: CompilerArtifact = serde_json::from_value(source["library"].clone())
            .map_err(|e| format!("E_VERUS_COLLECTION: library declaration: {e}"))?;
        let candidate = source["candidate"]
            .as_str()
            .ok_or("E_VERUS_COLLECTION: source candidate missing")?;
        let root = source["root"]
            .as_str()
            .ok_or("E_VERUS_COLLECTION: source root missing")?;
        if !verus_policy::sources::matches_observation_identity(
            &verus_policy::sources::ObservationIdentity {
                candidate: candidate.as_bytes(),
                root: root.as_bytes(),
                dependency: dependency.path.as_bytes(),
                library: library.path.as_bytes(),
            },
            req.candidate.as_bytes(),
            argv[8].as_bytes(),
            argv[10].as_bytes(),
        ) {
            return Err(
                "E_VERUS_COLLECTION: source candidate, root or artifact path mismatch".into(),
            );
        }
        let bytes = originals
            .get(&(
                dependency.original.path.clone(),
                dependency.original.sha256.clone(),
            ))
            .ok_or("E_VERUS_COLLECTION: acquired dependency original missing")?;
        if !originals.contains_key(&(
            library.original.path.clone(),
            library.original.sha256.clone(),
        )) {
            return Err("E_VERUS_COLLECTION: acquired library original missing".into());
        }
        let identities = candidate_sources(req, reader, snapshot, argv, bytes)?;
        if source["sources"] != identities {
            return Err(
                "E_VERUS_COLLECTION: source inventory differs from fixed candidate bytes".into(),
            );
        }
        dependency.original.path =
            chrono_harness::prepared::original_path(&dependency.original, transport)?;
        library.original.path =
            chrono_harness::prepared::original_path(&library.original, transport)?;
        source["dependency"] = object!(dependency);
        source["library"] = object!(library);
        source["assurance"] = object!(
            "collected original compiler result and artifacts re-admitted against fixed candidate sources"
        );
        admitted.push(CollectedAdmission {
            operation: operation.into(),
            sources: source,
        });
    }
    if !supplied.is_empty() {
        return Err(
            "E_VERUS_COLLECTION: extra compiler evidence without original operation".into(),
        );
    }
    Ok(admitted)
}

pub(super) fn apply(
    req: &Request,
    reader: &Reader,
    policy: &Policy,
    snapshot: &Snapshot,
    changed: &BTreeSet<String>,
    selected: &BTreeSet<String>,
    global_selected: &BTreeSet<String>,
    response: &mut Response,
    collected: Vec<CollectedAdmission>,
) -> Result<(), String> {
    let Some(requirements) = requirements(req, reader, policy)? else {
        return Ok(());
    };
    let mut ledger = verus_policy::Ledger::new();
    let mut compiler_evidence = Vec::new();
    let mut proofs = Vec::new();
    for admission in collected {
        let operation = admission.operation;
        proofs.push(verus_policy::RootProof {
            operation: operation.clone(),
            admitted: true,
        });
        ledger.record(format!("operation:{operation}:compiler-proof"), true);
        ledger.record(format!("operation:{operation}:compiler-sources"), true);
        compiler_evidence.push(object!({"operation":operation,"compiler_accepted":true,
            "compiler_error":null,"sources":admission.sources,"source_error":null,
            "origin":"collected original unit execution"}));
    }
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
        let accepted = compiler_result(execution, &requirements);
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
        proofs.push(verus_policy::RootProof {
            operation: operation.into(),
            admitted: compiler_accepted && sources.is_ok(),
        });
        compiler_evidence.push(object!({"operation":operation,"compiler_accepted":compiler_accepted,
            "compiler_error":accepted.err(),"sources":sources.as_ref().ok(),"source_error":sources.as_ref().err()}));
    }
    // Roots are bound to exact registered invocations. Original execution and
    // receipt checking remain the projects owner's responsibility.
    let plans = snapshot
        .plans
        .iter()
        .map(|(test, plan)| verus_policy::RootPlan {
            test,
            operations: plan.operations.iter().map(String::as_str).collect(),
        })
        .collect();
    let global = global_selected.iter().map(String::as_str).collect();
    let local = selected.iter().map(String::as_str).collect();
    let roots: Vec<_> = snapshot
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
            verus_policy::bind_root_route(root, operation, &plans, &global, &local)
        })
        .collect();
    for route in &roots {
        if route.locally_required {
            ledger.record(
                format!("root:{}:selected-proof", route.root),
                verus_policy::covers_root_delta(&roots, &proofs, route.root),
            );
        }
    }
    let registered_files = snapshot.files.keys().map(String::as_str).collect();
    for (id, unit) in &snapshot.projects {
        ledger.record(
            format!("unit:{id}:language=verus"),
            unit["language"] == "verus",
        );
        let operations = unit["actions"]
            .as_object()
            .into_iter()
            .flat_map(|actions| actions.values())
            .map(|action| action["operation"].as_str())
            .collect();
        ledger.record(
            format!("unit:{id}:registered-verifier"),
            verus_policy::admits_unit_verifier(&operations, &roots),
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
            let declared = verification["roots"]
                .as_array()
                .map(|roots| roots.iter().map(serde_json::Value::as_str).collect());
            ledger.record(
                format!("file:{path}:verification-roots"),
                verus_policy::admit_declared_roots(declared.as_ref(), &registered_files, &roots),
            );
            for root in verification["roots"].as_array().into_iter().flatten() {
                if let Some(root) = root.as_str() {
                    if changed.contains(path) || changed.contains(root) {
                        ledger.record(
                            format!("file:{path}:root:{root}:delta-proof"),
                            verus_policy::covers_root_delta(&roots, &proofs, root),
                        );
                    }
                }
            }
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
        "assurance":"declarations, unique canonical root routes and admitted original proofs for affected and selected roots; adapter proofs, full SPEC correspondence and complete repository migration remain required"
    });
    let artifacts = compiler_artifacts(&response.evidence)?;
    let retained = response
        .evidence
        .as_object_mut()
        .ok_or("E_VERUS_ARTIFACT: response evidence must be an object")?
        .entry("retained_originals")
        .or_insert_with(|| object!([]))
        .as_array_mut()
        .ok_or("E_VERUS_ARTIFACT: retained originals must be an array")?;
    retained.extend(
        artifacts
            .into_iter()
            .map(|artifact| object!(artifact.original.path)),
    );
    response.results.push(CheckResult {
        id: "ci.verus-coverage".into(),
        status: if complete {
            Status::Passed
        } else {
            Status::Failed
        },
        cause: if complete {
            "Verus declarations and original proof evidence cover the explicit DELTA roots".into()
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
