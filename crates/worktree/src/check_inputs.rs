//! Automatic local endpoint/context production; start/reconstruct own branch birth.
use crate::{Config, configuration, start};
use chrono_harness::{
    decode, facts, file_identity, no_symlink_parents,
    prepared::{self, Context, InputRequest, PreparedCheck, Selection},
    sha256, units, wire,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::Path,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub origin_path: String,
    pub context_path: String,
    pub collection_manifest: String,
    pub roles: BTreeMap<String, String>,
}
pub(crate) fn validate(p: &Policy) -> Result<(), String> {
    for path in [&p.origin_path, &p.context_path, &p.collection_manifest] {
        units::artifact_path(path)?;
    }
    if p.origin_path == p.context_path
        || p.origin_path == p.collection_manifest
        || p.context_path == p.collection_manifest
        || p.roles.is_empty()
        || p.roles.iter().any(|(k, v)| {
            !matches!(k.as_str(), "feature" | "integration")
                || !matches!(v.as_str(), "integration" | "delivery")
        })
    {
        return Err("invalid local check origin/context/role binding".into());
    }
    Ok(())
}
/// Publish the final original producer report in the destination, then its exact association.
pub(crate) fn publish_origin(source: &Path, report: &Value, p: &Policy) -> Result<(), String> {
    validate(p)?;
    let root = Path::new(
        report["destination"]
            .as_str()
            .ok_or("origin destination missing")?,
    );
    let role = p
        .roles
        .get(
            report["creation_kind"]
                .as_str()
                .ok_or("origin creation kind missing")?,
        )
        .ok_or("creation kind lacks explicit run-role mapping")?;
    let original = report["report_path"]
        .as_str()
        .ok_or("origin original report missing")?;
    let bytes = fs::read(no_symlink_parents(source, original)?).map_err(|e| e.to_string())?;
    if decode::<Value>(&bytes)? != *report {
        return Err("origin report differs from actual finalized producer output".into());
    }
    let path = format!(".chrono-harness/state/origins/{}.json", sha256(&bytes));
    publish(root, &path, &bytes, false)?;
    let origin = json!({"schema":"chrono-worktree-origin/v1","destination":root,"birth_report":path,"birth_sha256":sha256(&bytes),"run_kind":role,"integration_evidence":null,"retained_inputs":null});
    publish(
        root,
        &p.origin_path,
        &serde_json::to_vec(&origin).map_err(|e| e.to_string())?,
        false,
    )
}
pub(crate) fn publish(root: &Path, path: &str, bytes: &[u8], replace: bool) -> Result<(), String> {
    let target = no_symlink_parents(root, path)?;
    if target.exists() && !replace {
        if fs::read(&target).map_err(|e| e.to_string())? == bytes {
            return Ok(());
        }
        return Err(format!("producer output collision: {path}"));
    }
    let parent = target.parent().ok_or("output parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut f = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    f.write_all(bytes).map_err(|e| e.to_string())?;
    if replace {
        f.persist(&target).map_err(|e| e.to_string())?;
    } else {
        f.persist_noclobber(&target).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn produce(
    req: &InputRequest,
    path: &str,
    config: Config,
    bytes: &[u8],
) -> Result<PreparedCheck, String> {
    req.validate()?;
    if req.source != "local" || req.prepared.is_some() || config.host_config != req.host_config {
        return Err("local input source/host binding mismatch".into());
    }
    let report = start::with_report(
        &req.host_root,
        path,
        config,
        bytes,
        "check-inputs",
        "prepared",
        |r, token, report| {
            let root = &req.host_root;
            let candidate = r.oid(root, "HEAD")?;
            let top = fs::canonicalize(r.text(root, &["rev-parse", "--show-toplevel"])?.trim())
                .map_err(|e| e.to_string())?;
            if top != *root {
                return Err("short check cwd must be the actual host root".into());
            }
            if r.blob(root, &candidate, path)? != bytes
                || sha256(&r.blob(root, &candidate, &req.profile)?) != req.profile_sha256
            {
                return Err("local input policy/profile differs from committed HEAD".into());
            }
            let (registrations, digest) =
                start::registrations(r, root, &candidate, &req.host_config)?;
            start::registered_policy(&registrations, path, &r.config.report_directory)?;
            start::cleanliness(r, root, registrations.config())?;
            let target_branch = registrations.workflow()["target_branch"]
                .as_str()
                .ok_or("local input target branch missing")?
                .to_string();
            let reference = format!("refs/heads/{target_branch}");
            r.git(root, &["check-ref-format", &reference])?;
            let remote = r.config.remote.clone();
            let fetched = format!("refs/chrono-harness/check/{token}");
            let spec = format!("{reference}:{fetched}");
            report["remote"] = json!(remote);
            report["target_ref"] = json!(reference);
            report["fetch_ref"] = json!(fetched);
            r.git(
                root,
                &[
                    "fetch",
                    "--no-tags",
                    "--no-write-fetch-head",
                    "--",
                    &remote,
                    &spec,
                ],
            )?;
            let base = r.oid(root, &fetched)?;
            r.oid(root, &format!("{base}^{{commit}}"))?;
            r.git(root, &["update-ref", "-d", &fetched, &base])?;
            report["fetch_ref_removed"] = json!(true);
            let profile: Value = decode(
                &fs::read(no_symlink_parents(root, &req.profile)?).map_err(|e| e.to_string())?,
            )?;
            let scoped = matches!(
                profile["schema"].as_str(),
                Some("chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3")
            );
            let scope = match &req.selection {
                Selection::All => None,
                Selection::Unit { unit } => Some(units::Scope::Unit { unit: unit.clone() }),
                Selection::Collect => Some(units::Scope::Collect {
                    manifest: r
                        .config
                        .check_inputs
                        .as_ref()
                        .ok_or("local collection manifest binding missing")?
                        .collection_manifest
                        .clone(),
                }),
            };
            let context = if scoped {
                None
            } else {
                let p = r
                    .config
                    .check_inputs
                    .as_ref()
                    .ok_or("full local check context/origin binding missing")?
                    .clone();
                let origin_bytes =
                    fs::read(no_symlink_parents(root, &p.origin_path)?).map_err(|e| {
                        format!("missing full check origin receipt {}: {e}", p.origin_path)
                    })?;
                let origin: Value = decode(&origin_bytes)?;
                if origin["schema"] != "chrono-worktree-origin/v1"
                    || origin["destination"] != json!(root)
                    || !matches!(
                        origin["run_kind"].as_str(),
                        Some("integration" | "delivery")
                    )
                {
                    return Err("invalid full check origin identity/role".into());
                }
                let birth_path = origin["birth_report"]
                    .as_str()
                    .ok_or("origin birth report reference missing")?;
                units::artifact_path(birth_path)?;
                let birth_bytes =
                    fs::read(no_symlink_parents(root, birth_path)?).map_err(|e| e.to_string())?;
                if origin["birth_sha256"] != sha256(&birth_bytes) {
                    return Err("full check origin birth report drift".into());
                }
                let birth: Value = decode(&birth_bytes)?;
                if !matches!(birth["status"].as_str(), Some("created" | "reconstructed"))
                    || birth["destination"] != json!(root)
                    || birth["context"]["fork_point"] != birth["base"]
                    || !birth["processes"].is_array()
                {
                    return Err("full check origin does not reference successful creation".into());
                }
                let branch = r
                    .text(root, &["symbolic-ref", "--short", "HEAD"])?
                    .trim()
                    .to_string();
                if birth["branch_ref"] != branch {
                    return Err("full check branch differs from original birth association".into());
                }
                let fork = birth["context"]["fork_point"]
                    .as_str()
                    .ok_or("origin fork point missing")?;
                facts::full_oid(fork)?;
                r.oid(root, &format!("{fork}^{{commit}}"))?;
                let mut ctx = json!({"schema_version":2,"base":base,"candidate":candidate,"dev_tip":base,"branch_ref":branch,"fork_point":fork,"branch_started_at":birth["branch_started_at"],"observed_at":OffsetDateTime::now_utc().format(&Rfc3339).map_err(|e|e.to_string())?,"operation":"validate.delta","run_kind":origin["run_kind"],"integration_evidence":origin["integration_evidence"]});
                if let Some(retained) = origin["retained_inputs"].as_str() {
                    units::artifact_path(retained)?;
                    file_identity(&no_symlink_parents(root, retained)?)?;
                    ctx["retained_inputs"] = json!(retained);
                }
                if origin["run_kind"] == "delivery" && origin["integration_evidence"].is_null() {
                    return Err("delivery origin requires explicit caller-produced integration evidence handoff".into());
                }
                // Independent contributions and collection bind the same exact context.
                // Reuse only a matching current projection; endpoints/origin still come from this producer.
                if scope.is_some() {
                    let current = no_symlink_parents(root, &p.context_path)?;
                    match fs::read(current) {
                        Ok(raw) => {
                            let previous: Value = decode(&raw)?;
                            if !previous.is_object() || previous["schema_version"] != 2 {
                                return Err(
                                    "E_LOCAL_CONTEXT: cached context must be a schema2 object"
                                        .into(),
                                );
                            }
                            match chrono_judge_workflow::observation_age(
                                &previous,
                                registrations.workflow(),
                            ) {
                                Ok(_) => {}
                                Err(e) if e.starts_with("E_BRANCH_STALE:") => {}
                                Err(e) => return Err(format!("E_LOCAL_CONTEXT: {e}")),
                            }
                            let mut comparison = previous.clone();
                            comparison["observed_at"] = ctx["observed_at"].clone();
                            if comparison == ctx {
                                match chrono_judge_workflow::observation_age(
                                    &ctx,
                                    registrations.workflow(),
                                ) {
                                    Ok(_) => ctx = previous,
                                    Err(e) if e.starts_with("E_BRANCH_STALE:") => {}
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                // Keep the exact producer context immutable; the configured path is its current projection.
                let retained = prepared::retain(root, "local-context", &ctx)?;
                let retained_raw =
                    fs::read(no_symlink_parents(root, &retained)?).map_err(|e| e.to_string())?;
                publish(root, &p.context_path, &retained_raw, true)?;
                report["origin"] = json!({"path":p.origin_path,"sha256":sha256(&origin_bytes),"birth_report":birth_path,"birth_sha256":sha256(&birth_bytes)});
                Some(Context {
                    path: retained,
                    sha256: sha256(&retained_raw),
                    semantic_digest: wire::digest(&ctx)?,
                    raw: retained_raw,
                })
            };
            if r.oid(root, "HEAD")? != candidate {
                return Err("HEAD advanced during local preparation".into());
            }
            start::cleanliness(r, root, registrations.config())?;
            req.validate()?;
            report["registry_digest"] = json!(digest);
            report["base"] = json!(base);
            report["candidate"] = json!(candidate);
            report["check_context"] = json!(context);
            report["scope"] = json!(scope);
            Ok(())
        },
    )?;
    if report["status"] != "prepared" {
        return Err(serde_json::to_string(&report).map_err(|e| e.to_string())?);
    }
    let mut originals = vec![prepared::original(
        &req.host_root,
        report["report_path"].as_str().ok_or("report path")?,
    )?];
    if let Some(path) = report["check_context"]["path"].as_str() {
        originals.push(prepared::original(&req.host_root, path)?);
    }
    if let Some(path) = report["origin"]["path"].as_str() {
        originals.push(prepared::original(&req.host_root, path)?);
    }
    if let Some(path) = report["origin"]["birth_report"].as_str() {
        originals.push(prepared::original(&req.host_root, path)?);
    }
    Ok(PreparedCheck {
        schema: prepared::RESPONSE.into(),
        request_sha256: sha256(&serde_json::to_vec(req).map_err(|e| e.to_string())?),
        source: req.source.clone(),
        profile: req.profile.clone(),
        base: Some(report["base"].as_str().ok_or("base missing")?.into()),
        candidate: report["candidate"]
            .as_str()
            .ok_or("candidate missing")?
            .into(),
        initial: false,
        context: serde_json::from_value(report["check_context"].clone())
            .map_err(|e| e.to_string())?,
        scope: serde_json::from_value(report["scope"].clone()).map_err(|e| e.to_string())?,
        originals,
        evidence: json!({"report_path":report["report_path"],"report_sha256":file_identity(&no_symlink_parents(&req.host_root,report["report_path"].as_str().ok_or("report path")?)?)?.0,"origin":report["origin"],"remote":report["remote"],"target_ref":report["target_ref"]}),
    })
}
pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    if args.len() != 3 || args[0] != "check-inputs" || args[1] != "--config" {
        return Err("check-inputs requires its registered worktree --config".into());
    }
    let mut raw = Vec::new();
    std::io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > 64 * 1024 * 1024 {
        return Err("check input request exceeds bound".into());
    }
    let req: InputRequest = decode(&raw)?;
    let (config, bytes) = configuration(&req.host_root, &args[2])?;
    let p = produce(&req, &args[2], config, &bytes)?;
    Ok(serde_json::to_string(&p).map_err(|e| e.to_string())? + "\n")
}
