//! Initial inventory checks share reference policy without inventing prior history.
use crate::{Registrations, Result, issue};
use chrono_harness::{
    facts,
    initial::{Profile, Request},
    no_symlink_parents, sha256,
    wire::{self, Response, Status},
};
use serde_json::json;
use std::fs;

pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    if let Err(e) = evaluate(req, &mut response) {
        issue(&mut response, "E_INITIAL_INPUT", e, "/request", true);
    }
    response
}

fn evaluate(req: &Request, response: &mut Response) -> Result<()> {
    req.validate()?;
    let root = &req.candidate.root;
    let actual =
        fs::canonicalize(facts::utf8(facts::git(root, &["rev-parse", "--show-toplevel"])?)?.trim())
            .map_err(|e| e.to_string())?;
    if actual != *root
        || facts::verify_oid(root, &req.candidate.commit)? != req.candidate.tree
        || !facts::parents(root, &req.candidate.commit)?.is_empty()
    {
        return Err("initial requires the actual checkout root and a parentless candidate".into());
    }
    if !req.profile_path.starts_with(".chrono-harness/") || req.profile_path == req.config_path {
        return Err("initial profile must name a separate host config".into());
    }
    let bytes =
        fs::read(no_symlink_parents(root, &req.profile_path)?).map_err(|e| e.to_string())?;
    if sha256(&bytes) != req.profile_sha256
        || bytes != facts::blob(root, &req.candidate.commit, &req.profile_path)?
    {
        return Err("initial profile differs from fixed candidate".into());
    }
    let profile: Profile = chrono_harness::decode(&bytes)?;
    profile.validate()?;
    if profile.host_config != req.config_path {
        return Err("initial host config mismatch".into());
    }
    let binding = profile
        .judges
        .iter()
        .find(|b| b.id == req.judge_id)
        .ok_or("initial judge unregistered")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if fs::canonicalize(no_symlink_parents(root, &binding.executable)?)
        .map_err(|e| e.to_string())?
        != fs::canonicalize(&exe).map_err(|e| e.to_string())?
        || binding.sha256.as_deref()
            != Some(sha256(&fs::read(&exe).map_err(|e| e.to_string())?).as_str())
        || binding.version != env!("CARGO_PKG_VERSION")
    {
        return Err("initial binding differs from actual registration executable".into());
    }
    let values = facts::registry_values(root, &req.candidate.commit, &req.config_path)?;
    if wire::digest(&values)? != req.registry_digest {
        return Err("initial registry digest mismatch".into());
    }
    for path in values.keys() {
        if fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?
            != facts::blob(root, &req.candidate.commit, path)?
        {
            return Err(format!("checkout registry differs from candidate: {path}"));
        }
    }
    let r = Registrations::load(&values, &req.config_path)?;
    if values.values().any(|v| v["status"] != "proposed")
        || r.config()["enforcement"] != "not-implemented"
    {
        return Err(
            "initial inventory retains proposed registries; activation requires a real DELTA"
                .into(),
        );
    }
    let observed = facts::checkout(root, &req.candidate.commit)?;
    let nonartifact = |paths: &[String]| -> Vec<String> {
        paths
            .iter()
            .filter(|p| {
                !r.config()["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|a| p.starts_with(a["path"].as_str().unwrap()))
            })
            .cloned()
            .collect()
    };
    if observed.head != req.candidate.commit
        || observed.head != req.checkout.head
        || !observed.tracked.is_empty()
        || !req.checkout.tracked.is_empty()
        || !nonartifact(&observed.untracked).is_empty()
        || !nonartifact(&req.checkout.untracked).is_empty()
        || observed.index_flags != req.checkout.index_flags
    {
        return Err("initial candidate checkout is dirty or changed".into());
    }
    if observed
        .index_flags
        .iter()
        .any(|f| f.tag.as_bytes()[0].is_ascii_lowercase() || f.tag.eq_ignore_ascii_case("S"))
    {
        return Err("initial inventory rejects unsupported index flags".into());
    }
    let tree = facts::tree(root, &req.candidate.commit)?;
    if tree
        .values()
        .any(|e| e.kind != "blob" || !matches!(e.mode.as_str(), "100644" | "100755" | "120000"))
    {
        return Err("initial inventory rejects unsupported tree entries".into());
    }
    crate::references(root, &req.candidate.commit, None, &r, &[], &tree, response)?;
    if response.status == Status::Pass {
        response.outputs.insert("inventory".into(), json!({"scope":"schema-references-checkout",
            "candidate":req.candidate.commit,"tree":req.candidate.tree,"files":tree.len(),
            "registry_digest":req.registry_digest,"profile_sha256":req.profile_sha256,
            "governance":"not-evaluated","input_closure":r.config()["input_closure"],
            "remaining":["activation","judge execution for a real DELTA","complete input declarations and observations"]}));
    }
    Ok(())
}
