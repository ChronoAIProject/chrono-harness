//! The existing CI producer owns event preparation and declared local collection.
use super::*;
use chrono_harness::{
    file_identity,
    prepared::{self, Context, InputRequest, PreparedCheck, Selection},
    sha256,
    units::{FullManifest, FullReportInput, Manifest, ReportInput, Scope},
    wire,
};
use std::{collections::BTreeMap, io::Read};

pub(crate) fn publish(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    super::write_file(root, path, bytes)
}

fn local_manifest(
    req: &InputRequest,
    c: &units::Config,
    p: &PreparedCheck,
) -> Result<Value, String> {
    let registered = units::profile(&req.host_root, c)?.units;
    let full = c.schema == units::FULL_SCHEMA;
    let pins = if full {
        super::full::executable_pins(&req.host_root, &req.profile, &c.collection.runner)?
    } else {
        let profile = chrono_harness::load_config(&req.host_root.join(&req.profile))?;
        let judge = chrono_harness::resolve_program(
            &req.host_root,
            &profile.judge.program,
            profile.judge.env.get("PATH").map(String::as_str),
        )?;
        json!({"runner_sha256":file_identity(&no_symlink_parents(&req.host_root,&c.collection.runner)?)?.0,"judge_sha256":file_identity(&judge)?.0})
    };
    let (requirements, report_limit) = if full {
        let cfg = chrono_harness::load_selected_config(&req.host_root, &req.profile)?;
        let limit = cfg["execution_units"]["collection_limits"]["report_bytes"]
            .as_u64()
            .ok_or("full report bound")?;
        let requirements = chrono_judge_ci::full_collection_requirements_with_inputs(
            &req.host_root,
            &req.profile,
            p,
            &|conversion_units| {
                let mut originals = FullManifest {
                    schema: "chrono-full-collection/v1".into(),
                    reports: vec![],
                };
                for unit in conversion_units {
                    let input = registered
                        .get(unit)
                        .ok_or("migration unit missing provider registration")?;
                    let raw = chrono_harness::units::read_bounded(
                        &req.host_root,
                        &input.report_path,
                        limit,
                    )?;
                    originals.reports.push(FullReportInput {
                        unit: unit.clone(),
                        path: input.report_path.clone(),
                        sha256: sha256(&raw),
                        runner_sha256: None,
                        judge_sha256: None,
                        artifacts: None,
                    });
                }
                Ok(originals)
            },
        )?;
        (requirements, limit)
    } else {
        (
            chrono_judge_ci::collection_requirements(
                &req.host_root,
                &req.profile,
                &c.gather.manifest_path,
                p.base.clone(),
                p.candidate.clone(),
                p.initial,
            )?,
            64 * 1024 * 1024,
        )
    };
    let required: Vec<String> = serde_json::from_value(requirements["required_units"].clone())
        .map_err(|e| e.to_string())?;
    let mut reports = vec![];
    for unit in required {
        let input = registered
            .get(&unit)
            .ok_or("required unit missing provider registration")?;
        let raw =
            chrono_harness::units::read_bounded(&req.host_root, &input.report_path, report_limit)
                .map_err(|e| {
                format!(
                    "missing/invalid registered unit report {unit} at {}: {e}",
                    input.report_path
                )
            })?;
        reports.push(FullReportInput {
            unit,
            path: input.report_path.clone(),
            sha256: sha256(&raw),
            runner_sha256: Some(pins["runner_sha256"].as_str().ok_or("runner pin")?.into()),
            judge_sha256: Some(pins["judge_sha256"].as_str().ok_or("judge pin")?.into()),
            artifacts: None,
        });
    }
    let manifest = if full {
        serde_json::to_value(FullManifest {
            schema: "chrono-full-collection/v1".into(),
            reports,
        })
        .map_err(|e| e.to_string())?
    } else {
        serde_json::to_value(Manifest {
            schema: "chrono-ci-collection/v1".into(),
            reports: reports
                .into_iter()
                .map(|r| ReportInput {
                    unit: r.unit,
                    path: r.path,
                    sha256: r.sha256,
                    runner_sha256: r.runner_sha256.unwrap(),
                    judge_sha256: r.judge_sha256.unwrap(),
                    artifacts: None,
                })
                .collect(),
        })
        .map_err(|e| e.to_string())?
    };
    let raw = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    if p.scope
        != Some(Scope::Collect {
            manifest: c.gather.manifest_path.clone(),
        })
    {
        return Err("local endpoint/CI collection manifest bindings disagree".into());
    }
    let unit_paths = registered
        .values()
        .map(|u| u.report_path.clone())
        .collect::<Vec<_>>();
    let profile = units::profile(&req.host_root, c)?;
    chrono_harness::units::validate_collection_paths(
        &c.gather.manifest_path,
        &profile.report_path,
        &unit_paths,
    )?;
    publish(&req.host_root, &c.gather.manifest_path, &raw)?;
    let mut evidence = json!({"schema":"chrono-local-collection-inputs/v1","manifest_path":c.gather.manifest_path,"manifest_sha256":sha256(&raw),"manifest":manifest,"expected_executables":pins,"endpoint_evidence":p.evidence,"requirements":requirements});
    // Collection must retain the endpoint producer's actual observation before
    // dispatch preserves it in the final response. Its context remains original.
    if let Some(observation) = p.evidence.get("current_observation") {
        evidence["current_observation"] = observation.clone();
    }
    Ok(evidence)
}
fn named_env(opts: &BTreeMap<&str, &str>, flag: &str) -> Result<String, String> {
    let key = opts
        .get(flag)
        .ok_or_else(|| format!("missing registered native input {flag}"))?;
    if key.is_empty() || key.contains(['=', '\0']) {
        return Err("invalid native input environment binding".into());
    }
    std::env::var(key).map_err(|_| format!("missing/non-UTF8 native input {key}"))
}
pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut opts = BTreeMap::new();
    if args.len() < 3 || args.len() % 2 != 1 {
        return Err("check-inputs requires declared paired arguments".into());
    }
    for pair in args[1..].chunks_exact(2) {
        if ![
            "--config",
            "--event-env",
            "--payload-env",
            "--revision-env",
            "--repository-env",
        ]
        .contains(&pair[0].as_str())
            || opts.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown/duplicate input producer argument".into());
        }
    }
    let path = *opts
        .get("--config")
        .ok_or("input producer --config missing")?;
    let mut raw = Vec::new();
    std::io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > 64 * 1024 * 1024 {
        return Err("check input request exceeds bound".into());
    }
    let req: InputRequest = decode(&raw)?;
    req.validate()?;
    let root = &req.host_root;
    let config_bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    let config: Value = decode(&config_bytes)?;
    let mut originals = vec![];
    let dir = prepared::retention_directory(&req);
    let p = if req.source == "local" {
        if req.selection != Selection::Collect {
            return Err("local CI producer only collects declared unit reports".into());
        }
        let c: units::Config = decode(&config_bytes)?;
        units::validate(&c)?;
        if c.collection.check_config != req.profile {
            return Err("collection profile binding mismatch".into());
        }
        let previous = req
            .prepared
            .as_ref()
            .ok_or("local collection endpoint preparation missing")?;
        let evidence = local_manifest(&req, &c, previous)?;
        PreparedCheck {
            schema: prepared::RESPONSE.into(),
            request_sha256: sha256(&serde_json::to_vec(&req).map_err(|e| e.to_string())?),
            source: req.source.clone(),
            profile: req.profile.clone(),
            base: previous.base.clone(),
            candidate: previous.candidate.clone(),
            initial: previous.initial,
            scope: previous.scope.clone(),
            context: previous.context.clone(),
            evidence,
            originals: previous.originals.clone(),
        }
    } else {
        if req.prepared.is_some() {
            return Err("native preparation cannot consume local prepared endpoints".into());
        }
        let event = named_env(&opts, "--event-env")?;
        let payload_path = named_env(&opts, "--payload-env")?;
        let payload_raw = fs::read(&payload_path)
            .map_err(|e| format!("native event payload {payload_path}: {e}"))?;
        let payload: Value = decode(&payload_raw)?;
        let revision = named_env(&opts, "--revision-env")?;
        let full_provider = config["schema"].as_str() == Some(full::SCHEMA);
        let (mut evidence, context, scope) = if full_provider {
            let c: full::Config = decode(&config_bytes)?;
            if c.check_config != req.profile || !req.selection.matches(&c.scope) {
                return Err("full native profile binding mismatch".into());
            }
            if event != "workflow_dispatch" {
                return Err("full native check requires exact context dispatch".into());
            }
            let bytes = payload["inputs"]["context"]
                .as_str()
                .ok_or("full native exact context missing")?
                .as_bytes();
            full::validate(&c)?;
            if req.native_artifacts.as_ref().is_none_or(|a| {
                a.config_path != path
                    || a.config_sha256 != sha256(&config_bytes)
                    || a.directory != c.artifact_directory
            }) {
                return Err("full producer/upload contract mismatch".into());
            }
            let mut evidence = full::prepare(root, path, &c, bytes, &revision)?;
            let report_raw = fs::read(no_symlink_parents(root, &c.preparation_path)?)
                .map_err(|e| e.to_string())?;
            let original = prepared::retain_original(root, &dir, "full-inputs", &report_raw)?;
            evidence["preparation_original"] = json!(original);
            originals.push(original);
            let ctx = Context {
                path: prepared::retain_original(root, &dir, "native-context", bytes)?.path,
                raw: bytes.to_vec(),
                sha256: sha256(bytes),
                semantic_digest: wire::digest(&decode::<Value>(bytes)?)?,
            };
            (evidence, Some(ctx), c.scope.clone())
        } else if matches!(
            config["schema"].as_str(),
            Some(units::SCHEMA | units::FULL_SCHEMA)
        ) {
            let c: units::Config = decode(&config_bytes)?;
            units::validate(&c)?;
            if c.collection.schema != "chrono-github-ci/v4"
                || c.collection.check_config != req.profile
                || c.collection.facts_config.as_deref() != Some(&req.host_config)
            {
                return Err("native short source/profile/facts binding mismatch".into());
            }
            let unit = match &req.selection {
                Selection::Unit { unit } => Some(unit.as_str()),
                _ => None,
            };
            units::profile(root, &c)?;
            let w = c.workflow(unit)?;
            if req.native_artifacts.as_ref().is_none_or(|a| {
                a.config_path != path
                    || a.config_sha256 != sha256(&config_bytes)
                    || a.directory != w.artifact_directory
            }) {
                return Err("unit producer/upload contract mismatch".into());
            }
            if req.selection == Selection::Collect && c.job_gating.is_none() {
                let policy: Value = decode(
                    &fs::read(no_symlink_parents(root, &req.effective_config)?)
                        .map_err(|e| e.to_string())?,
                )?;
                if policy["protocol"]["timeout_seconds"].as_u64().unwrap_or(0)
                    <= c.gather.wait_seconds
                {
                    return Err(
                        "native gathering wait must fit the registered acquisition timeout".into(),
                    );
                }
            }
            if c.job_gating.is_some() && req.selection == Selection::Collect {
                let needs: Value = decode(
                    std::env::var(super::gating::NEEDS_ENV)
                        .map_err(|_| "aggregate needs missing")?
                        .as_bytes(),
                )?;
                if needs["detect"]["result"] != "success" {
                    return Err("change detection did not succeed".into());
                }
            }
            let mut evidence = if req.selection == Selection::All {
                units::prepare_all(root, path, &c, &event, &payload, &revision)?
            } else {
                units::prepare(root, path, &c, unit, &event, &payload, &revision)?
            };
            let scope = if req.selection == Selection::All {
                None
            } else {
                Some(c.scope(unit))
            };
            evidence["canonical_argv"] = json!(
                std::iter::once(c.collection.runner.clone())
                    .chain(std::iter::once("check".into()))
                    .chain(req.selection.argv())
                    .collect::<Vec<_>>()
            );
            publish(
                root,
                &w.context_path,
                &serde_json::to_vec_pretty(&evidence).map_err(|e| e.to_string())?,
            )?;
            if matches!(req.selection, Selection::Collect) {
                let repository = named_env(&opts, "--repository-env")?;
                let gathered = super::gather::gather(root, path, &c, &repository)?;
                let gathered: Value = decode(gathered.as_bytes())?;
                let retained = gathered["retained_report"]
                    .as_str()
                    .ok_or("gather original report missing")?;
                let o = prepared::original(root, retained)?;
                evidence["gather"] = json!(o);
                originals.push(o);
                let o: prepared::Original =
                    serde_json::from_value(gathered["result"]["manifest_original"].clone())
                        .map_err(|e| e.to_string())?;
                prepared::read_original(root, &o, None)?;
                evidence["manifest"] = json!(o);
                originals.push(o);
            }
            let context_raw =
                fs::read(no_symlink_parents(root, &w.context_path)?).map_err(|e| e.to_string())?;
            let o = prepared::retain_original(root, &dir, "native-context", &context_raw)?;
            evidence["context_original"] = json!(o);
            originals.push(o);
            let context = if let Some(contexts) = &c.full_contexts {
                let path = match unit {
                    Some(id) => &contexts.units[id],
                    None => &contexts.collection,
                };
                let raw = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
                let ctx = full::context_input(&raw)?;
                let original = prepared::retain_original(root, &dir, "full-context", &raw)?;
                Some(Context {
                    path: original.path,
                    sha256: sha256(&raw),
                    semantic_digest: wire::digest(&ctx)?,
                    raw,
                })
            } else {
                None
            };
            (evidence, context, scope)
        } else {
            let c: Config = decode(&config_bytes)?;
            if req.selection != Selection::All
                || c.schema != "chrono-github-ci/v4"
                || c.check_config != req.profile
                || c.facts_config.as_deref() != Some(&req.host_config)
            {
                return Err("native short profile/facts/scope binding mismatch".into());
            }
            if req.native_artifacts.as_ref().is_none_or(|a| {
                a.config_path != path
                    || a.config_sha256 != sha256(&config_bytes)
                    || a.directory != c.artifact_directory
            }) {
                return Err("native producer/upload contract mismatch".into());
            }
            if c.initial_inventory.as_ref().map(|p| p.path.as_str())
                != req.initial_profile.as_ref().map(|p| p.path.as_str())
            {
                return Err("native initial profile binding mismatch".into());
            }
            if let (Some(inventory), Some(binding)) = (&c.initial_inventory, &req.initial_profile) {
                let raw = fs::read(no_symlink_parents(root, &binding.path)?)
                    .map_err(|e| e.to_string())?;
                if sha256(&raw) != binding.sha256
                    || decode::<Value>(&raw)?
                        != serde_json::to_value(&inventory.profile).map_err(|e| e.to_string())?
                {
                    return Err("native initial inventory differs from bound profile".into());
                }
            }
            generate(root, path, true)?;
            let evidence = super::prepare(root, &c, &event, &payload, &revision)?;
            publish(
                root,
                &c.context_path,
                &serde_json::to_vec_pretty(&evidence).map_err(|e| e.to_string())?,
            )?;
            let mut evidence = evidence;
            let context_raw =
                fs::read(no_symlink_parents(root, &c.context_path)?).map_err(|e| e.to_string())?;
            let o = prepared::retain_original(root, &dir, "native-context", &context_raw)?;
            evidence["context_original"] = json!(o);
            originals.push(o);
            (evidence, None, None)
        };
        let payload = prepared::retain_original(root, &dir, "native-payload", &payload_raw)?;
        evidence["payload"] = json!(payload);
        evidence["payload_input_path"] = json!(payload_path);
        originals.push(payload);
        if let Some(ctx) = &context {
            originals.push(prepared::original(root, &ctx.path)?);
        }
        evidence["originals"] = json!(originals);
        PreparedCheck {
            schema: prepared::RESPONSE.into(),
            request_sha256: sha256(&serde_json::to_vec(&req).map_err(|e| e.to_string())?),
            source: req.source.clone(),
            profile: req
                .profile_for(evidence["initial"].as_bool().unwrap_or(false))?
                .into(),
            base: evidence["base"].as_str().map(str::to_owned),
            candidate: evidence["candidate"]
                .as_str()
                .ok_or("native candidate missing")?
                .into(),
            initial: evidence["initial"].as_bool().unwrap_or(false),
            scope,
            context,
            evidence,
            originals,
        }
    };
    let facts = chrono_harness::facts::Reader::for_config(root, &req.host_config)?;
    facts.verify_config(root, &p.candidate)?;
    if facts.blob(root, &p.candidate, path)? != config_bytes {
        return Err("input producer configuration differs from candidate".into());
    }
    if let Some(binding) = &req.initial_profile {
        if sha256(&facts.blob(root, &p.candidate, &binding.path)?) != binding.sha256 {
            return Err("native initial profile differs from candidate".into());
        }
    }
    let mut p = p;
    let report = prepared::retain_original(
        root,
        &dir,
        "ci-inputs",
        &serde_json::to_vec(&p.evidence).map_err(|e| e.to_string())?,
    )?;
    let report_path = report.path.clone();
    p.originals.push(report);
    let current = p.evidence.get("current_observation").cloned();
    p.evidence = json!({"report_path":report_path,"report_sha256":file_identity(&no_symlink_parents(root,&report_path)?)?.0,"event":p.evidence["event"],"source":p.evidence["source"],"payload":{ "sha256":p.evidence["payload"]["sha256"],"path":p.evidence["payload"]["path"] }});
    if let Some(current) = current {
        p.evidence["current_observation"] = current;
    }
    req.validate()?;
    prepared::validate_result(&req, &p)?;
    Ok(serde_json::to_string(&p).map_err(|e| e.to_string())? + "\n")
}
