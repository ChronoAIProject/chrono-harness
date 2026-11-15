#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::{expect, policy};
#[cfg(not(verus_keep_ghost))]
use crate::verified_lifecycle::expect;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_lifecycle as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn age_preservation_is_skipped_only_without_disposal_and_with_retained_file() {
    expect(!policy::age_preservation_required(false, true));
    expect(policy::age_preservation_required(false, false));
    expect(policy::age_preservation_required(true, false));
    expect(policy::age_preservation_required(true, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_actual_unreleased_evidence_blocks_checkout_removal() {
    let needed = policy::retain_evidence(false, true);
    let absent = policy::retain_evidence(false, false);
    let released = policy::retain_evidence(true, true);
    let released_absent = policy::retain_evidence(true, false);
    expect(needed && !absent && !released && !released_absent);
}

pub open spec fn caller_observation_matches(expected: (String, String),
    result: (i32, String), observed: Result<(String, String, bool), String>,
    out: Result<(), String>) -> bool
{
    match observed {
        Err(error) => out == Err::<(), String>(error),
        Ok(fields) => out.is_ok() == (result.0 == 0 && result.1@ == expected.0@
            && fields.0@ == "unique caller source λ\n"@
            && fields.1@ == expected.1@ && !fields.2),
    }
}

// The same driver sequences the real fixture callbacks and the independent cases.
// Callback failures remain failures; only the observed values determine acceptance.
fn linked_caller_regression<C, G, B, R, O, P>(gate: G, build: B, run: R, observe: O, preserve: P)
    -> (out: Result<(), String>)
    where G: FnOnce() -> bool, B: FnOnce() -> (C, (String, String)),
        R: FnOnce(C) -> (C, (i32, String)),
        O: FnOnce(&C) -> Result<(String, String, bool), String>, P: FnOnce(C),
    requires gate.requires(()), build.requires(()),
        forall|state: C| run.requires((state,)),
        forall|state: &C| observe.requires((state,)),
        forall|state: C| preserve.requires((state,)),
    ensures (gate.ensures((), false) && out == Ok::<(), String>(()))
        || (gate.ensures((), true) && exists|prepared: C, expected: (String, String)|
            #[trigger] build.ensures((), (prepared, expected))
            && exists|executed: C, result: (i32, String)|
                #[trigger] run.ensures((prepared,), (executed, result))
                && exists|observed: Result<(String, String, bool), String>|
                    #[trigger] observe.ensures((&executed,), observed)
                    && caller_observation_matches(expected, result, observed, out)
                    && (out.is_err() ==> preserve.ensures((executed,), ()))),
{
    let active = gate();
    assert(gate.ensures((), active));
    if !active { return Ok(()); }
    let (state, expected) = build();
    let ghost prepared = state;
    let ghost original = expected;
    assert(build.ensures((), (prepared, original)));
    let (state, result) = run(state);
    let ghost executed = state;
    let ghost returned = result;
    assert(run.ensures((prepared,), (executed, returned)));
    let observed = observe(&state);
    let ghost observation = observed;
    assert(observe.ensures((&executed,), observation));
    let out = match observed {
        Err(error) => Err(error),
        Ok(fields) => {
            if result.0 == 0 && policy::equal_text(result.1.as_str(), expected.0.as_str())
                && policy::equal_text(fields.0.as_str(), "unique caller source λ\n")
                && policy::equal_text(fields.1.as_str(), expected.1.as_str()) && !fields.2 {
                Ok(())
            } else {
                Err("linked maintenance changed its caller or failed to retire its sibling".to_string())
            }
        },
    };
    assert(caller_observation_matches(original, returned, observation, out));
    if out.is_err() {
        preserve(state);
        assert(preserve.ensures((executed,), ()));
    }
    out
}

fn caller_case(code: i32, reported: &str, source: &str, head: &str,
    sibling: bool, failure: bool, expected: bool)
    requires expected == (!failure && code == 0 && reported@ == "/caller λ"@
        && source@ == "unique caller source λ\n"@ && head@ == "original head"@ && !sibling),
{
    let build = || -> (out: (u8, (String, String)))
        ensures out.0 == 1, out.1.0@ == "/caller λ"@, out.1.1@ == "original head"@,
    { (1, ("/caller λ".to_string(), "original head".to_string())) };
    let run = |stage: u8| -> (out: (u8, (i32, String)))
        ensures out.0 == (if stage == 1 { 2u8 } else { 0u8 }),
            out.1.0 == code, out.1.1@ == reported@,
    { (if stage == 1 { 2 } else { 0 }, (code, reported.to_string())) };
    let observe = |stage: &u8| -> (out: Result<(String, String, bool), String>)
        ensures match out {
            Err(error) => (failure || *stage != 2) && error@ == "observation λ\0\n"@,
            Ok(fields) => !failure && *stage == 2 && fields.0@ == source@
                && fields.1@ == head@ && fields.2 == sibling,
        },
    {
        if failure || *stage != 2 { Err("observation λ\0\n".to_string()) }
        else { Ok((source.to_string(), head.to_string(), sibling)) }
    };
    let gate = || -> (active: bool) ensures active { true };
    let preserve = |stage: u8| { let _ = stage; };
    let result = linked_caller_regression(gate, build, run, observe, preserve);
    expect(result.is_ok() == expected);
    if failure {
        match result {
            Err(error) => expect(policy::equal_text(error.as_str(), "observation λ\0\n")),
            Ok(()) => expect(false),
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn linked_caller_verdict_requires_each_original_observation() {
    proof {
        reveal_strlit("/caller λ"); reveal_strlit("/other");
        reveal_strlit("unique caller source λ\n"); reveal_strlit("changed");
        reveal_strlit("original head");
        assert("/caller λ"@.len() != "/other"@.len());
        assert("unique caller source λ\n"@.len() != "changed"@.len());
        assert("original head"@.len() != "changed"@.len());
    }
    caller_case(0, "/caller λ", "unique caller source λ\n", "original head", false, false, true);
    caller_case(1, "/caller λ", "unique caller source λ\n", "original head", false, false, false);
    caller_case(0, "/other", "unique caller source λ\n", "original head", false, false, false);
    caller_case(0, "/caller λ", "changed", "original head", false, false, false);
    caller_case(0, "/caller λ", "unique caller source λ\n", "changed", false, false, false);
    caller_case(0, "/caller λ", "unique caller source λ\n", "original head", true, false, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn linked_caller_verdict_preserves_observation_failures() {
    caller_case(0, "/caller λ", "unique caller source λ\n", "original head", false, true, false);
    caller_case(i32::MIN, "", "", "", true, true, false);
    caller_case(i32::MAX, "/caller λ\0", "unique caller source λ", "original head\n", true, true, false);
}
}

#[path = "../../worktree-tests/tests/support/fixture_coordinator.rs"]
#[cfg(not(verus_keep_ghost))]
mod chrono_git_boundary;
#[cfg(not(verus_keep_ghost))]
use super::automatic::{native_consumer, native_git};
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use std::os::unix::fs::MetadataExt;
#[cfg(not(verus_keep_ghost))]
use std::time::{Duration, SystemTime};

#[cfg(not(verus_keep_ghost))]
fn policy(h: &Host, reuse: u64, delete: u64, evidence: &str) {
    h.automatic(evidence);
    git(&h.root, &["config", "user.name", "Lifecycle fixture"]);
    git(
        &h.root,
        &["config", "user.email", "lifecycle@example.invalid"],
    );
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut p = json(&fs::read(&path).unwrap()).unwrap();
    p["schema"] = value!("chrono-worktree-automatic-cleanup/v2");
    p["age_policy"] = value!({"reuse_after_seconds":reuse,"delete_after_seconds":delete,
        "preserved_branch_prefix":"preserved/age/"});
    p["artifacts"][0]["disposition"] = value!("finish-dispose");
    fs::write(path, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}

// Wait once against the real filesystem birth time. Tests never change a ledger,
// a timestamp, the product clock or the machine clock to make an object eligible.
#[cfg(not(verus_keep_ghost))]
fn older_than(target: &Path, seconds: u64) {
    let created = fs::metadata(target).unwrap().created().unwrap();
    let boundary = created + Duration::from_secs(seconds) + Duration::from_millis(50);
    if let Ok(wait) = boundary.duration_since(SystemTime::now()) {
        std::thread::sleep(wait);
    }
}
#[cfg(not(verus_keep_ghost))]
fn entry(h: &Host, target: &Path) -> Value {
    json(&fs::read(h.ledger_path(target)).unwrap()).unwrap()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["status"] != "disposed")
        .unwrap()
        .clone()
}
#[cfg(not(verus_keep_ghost))]
fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/cache"), vec![11; 4096]).unwrap();
}

#[cfg(not(verus_keep_ghost))]
fn cache_identity(target: &Path) -> (u64, u64) {
    let cache = target.join("output λ/cache");
    assert_eq!(fs::read(&cache).unwrap(), vec![11; 4096]);
    let metadata = fs::metadata(cache).unwrap();
    (metadata.dev(), metadata.ino())
}

#[cfg(not(verus_keep_ghost))]
fn temporary_output(target: &Path) {
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/temporary"), b"disposable").unwrap();
}

#[cfg(not(verus_keep_ghost))]
fn start(h: &Host, name: &str, target: &Path) {
    let (code, report, error) = h.invoke("feature", name, target);
    assert_eq!(code, 0, "{report} {error}");
}

// These callbacks adapt existing fixtures and real process/filesystem interfaces.
// All sequencing, error propagation, and the verdict live in the Verus driver above.
#[test]
#[cfg(not(verus_keep_ghost))]
fn expired_linked_caller_preserves_its_source_and_head_while_retiring_another_checkout()
-> Result<(), String> {
    linked_caller_regression(
        chrono_git_boundary::isolated_git,
        || {
            let h = Host::new("payload");
            policy(&h, 1, 3, "dispose");
            let caller = h.parent.join("invoking-caller-λ");
            start(&h, "invoking-caller", &caller);
            let sibling = h.parent.join("independent-expired");
            start(&h, "independent-expired", &sibling);
            let expected = (
                caller.to_str().unwrap().to_owned(),
                git(&caller, &["rev-parse", "HEAD"]),
            );
            fs::write(caller.join("payload"), "unique caller source λ\n").unwrap();
            older_than(&sibling, 3);
            ((h, caller, sibling), expected)
        },
        |(h, caller, sibling)| {
            let mut command = h.auto_command_at(&caller, "maintain", &[]);
            let (code, report, _) = h.received(h._dir.capture_bound(&mut command));
            let result = (code, report["invoking_root"].as_str().unwrap().to_owned());
            ((h, caller, sibling), result)
        },
        |(_, caller, sibling): &(Host, PathBuf, PathBuf)| {
            let source =
                fs::read_to_string(caller.join("payload")).map_err(|error| error.to_string())?;
            let head = git(&caller, &["rev-parse", "HEAD"]);
            let sibling_exists = sibling.try_exists().map_err(|error| error.to_string())?;
            Ok((source, head, sibling_exists))
        },
        |(h, _, _)| {
            h.retain_fixture_diagnostics(
                &source().join(".chrono-harness/state/worktree-test-failures"),
            );
        },
    )
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn maintenance_selects_creation_age_from_each_targets_adopted_policy() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for (coordinator, target_policy, expired) in [
        (Some((1, 3)), Some((3600, 7200)), false),
        (Some((1, 3)), None, false),
        (Some((3600, 7200)), Some((1, 3)), true),
        (None, Some((1, 3)), true),
    ] {
        let h = Host::new("payload");
        let (reuse, delete) = coordinator.unwrap_or((3600, 7200));
        policy(&h, reuse, delete, "dispose");
        if coordinator.is_none() {
            let path = h.root.join(".chrono-harness/cleanup.json");
            let mut p = json(&fs::read(&path).unwrap()).unwrap();
            p.as_object_mut().unwrap().remove("age_policy");
            fs::write(path, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
            commit(&h.root);
            git(&h.root, &["push", "-q", "warehouse", "dev"]);
        }
        let coordinator_bytes = fs::read(h.root.join(".chrono-harness/cleanup.json")).unwrap();
        let target = h.parent.join("independent-age-policy");
        start(&h, "independent-age-policy", &target);
        let path = target.join(".chrono-harness/cleanup.json");
        let mut p = json(&fs::read(&path).unwrap()).unwrap();
        if let Some((reuse, delete)) = target_policy {
            p["age_policy"]["reuse_after_seconds"] = value!(reuse);
            p["age_policy"]["delete_after_seconds"] = value!(delete);
            p["age_policy"]["preserved_branch_prefix"] = value!("preserved/target-age/");
        } else {
            p.as_object_mut().unwrap().remove("age_policy");
        }
        fs::write(path, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
        let head = commit(&target);
        let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{report} {error}");
        output(&target);
        temporary_output(&target);
        older_than(&target, 3);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(target.exists(), !expired, "{report}");
        assert_eq!(
            fs::read(h.root.join(".chrono-harness/cleanup.json")).unwrap(),
            coordinator_bytes
        );
        if expired {
            let row = report["drain"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["path"] == value!(target))
                .unwrap();
            let original = drain_report(row);
            assert_eq!(
                original["source_preservation"]["original_head"],
                head.trim()
            );
            let reference = original["source_preservation"]["original_ref"]
                .as_str()
                .unwrap();
            assert!(reference.starts_with("refs/heads/preserved/target-age/"));
            assert_eq!(git(&h.root, &["rev-parse", reference]).trim(), head.trim());
        } else {
            assert!(target.join("output λ/cache").exists());
            assert!(!target.join("nested/cache").exists());
            assert_eq!(git(&target, &["rev-parse", "HEAD"]).trim(), head.trim());
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn real_creation_age_reuse_and_expiry_preserve_source_index_and_branches() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    // Reuse is one phase of the scenario. Keep expiry outside the registered
    // execution bound until its phase; native/mock Git work may take over 60s.
    // The expiry phase below still adopts and checks the original 60s age.
    policy(&h, 2, 3600, "dispose");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("real-aged-checkout");
    start(&h, "real-aged-checkout", &target);
    let created = fs::metadata(&target).unwrap().created().unwrap();
    let original = git(&target, &["rev-parse", "HEAD"]);
    fs::write(target.join("payload"), "reflog-only-source").unwrap();
    let reflog_only = commit(&target);
    git(&target, &["reset", "--hard", &original]);
    fs::write(target.join("payload"), "staged-original").unwrap();
    git(&target, &["add", "payload"]);
    fs::write(target.join("payload"), "current-source").unwrap();
    fs::write(
        target.join(".gitignore"),
        ".chrono-harness/state/\n.chrono-harness/bin/\noutput λ/\nnested/cache/\nignored-source\n",
    )
    .unwrap();
    fs::write(target.join("ignored-source"), "ignored but unique source").unwrap();
    fs::write(target.join("wildcard*[abc]$literal"), "literal source").unwrap();
    output(&target);
    let cache = cache_identity(&target);
    older_than(&target, 2);
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
    assert!(used["age_reuse"].is_null(), "{used}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), original);
    assert_eq!(git(&target, &["show", ":payload"]), "staged-original");
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "current-source"
    );
    assert!(target.join("output λ/cache").is_file());
    temporary_output(&target);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let result = maintained["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target) && !row["receipt"].is_null())
        .unwrap();
    let original_result = drain_report(result);
    let saved = &original_result["source_preservation"];
    assert_eq!(saved["original_head"], original);
    assert!(
        saved["reflog_refs"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v == &value!(reflog_only))
    );
    let show = |reference: &Value, path: &str| {
        git(
            &h.root,
            &["show", &format!("{}:{path}", reference.as_str().unwrap())],
        )
    };
    assert_eq!(show(&saved["index_ref"], "payload"), "staged-original");
    assert_eq!(show(&saved["retained_ref"], "payload"), "current-source");
    assert_eq!(
        show(&saved["retained_ref"], "ignored-source"),
        "ignored but unique source"
    );
    assert_eq!(
        show(&saved["retained_ref"], "wildcard*[abc]$literal"),
        "literal source"
    );
    assert!(target.is_dir());
    assert_eq!(cache_identity(&target), cache);
    assert!(!target.join("nested/cache").exists());
    assert_eq!(original_result["age_outcome"], "reusable-source-preserved");
    let reference = saved["retained_ref"].as_str().unwrap().to_owned();
    let head = saved["head"].as_str().unwrap().to_owned();
    temporary_output(&target);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let repeated = drain_report(
        maintained["drain"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == value!(target) && !row["receipt"].is_null())
            .unwrap(),
    );
    for reference in saved["reflog_refs"].as_object().unwrap().keys() {
        assert!(
            repeated["processes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| !p["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|arg| arg == reference)),
            "already retained reflog commit was queried separately: {reference}"
        );
        assert_eq!(
            git(&h.root, &["rev-parse", reference]),
            saved["reflog_refs"][reference].as_str().unwrap()
        );
    }
    assert_eq!(cache_identity(&target), cache);
    assert!(!target.join("nested/cache").exists());
    let (code, reused, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{reused} {error}");
    assert!(reused["age_reuse"].is_null(), "{reused}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert!(target.join("output λ/cache").is_file());
    assert_eq!(cache_identity(&target), cache);
    let path = target.join(".chrono-harness/cleanup.json");
    let mut adopted = json(&fs::read(&path).unwrap()).unwrap();
    adopted["age_policy"]["delete_after_seconds"] = value!(60);
    fs::write(path, serde_json::to_vec_pretty(&adopted).unwrap()).unwrap();
    commit(&target);
    let (code, migrated, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{migrated} {error}");
    assert_eq!(fs::metadata(&target).unwrap().created().unwrap(), created);
    // Modification now must not extend checkout creation age.
    fs::write(target.join("payload"), "source at deletion").unwrap();
    older_than(&target, 60);
    let (code, maintained, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{maintained} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (maintained_cleanup_code, maintained_cleanup, maintained_cleanup_error) =
        h.auto("maintain", &[]);
    assert_eq!(
        maintained_cleanup_code, 0,
        "{maintained_cleanup} {maintained_cleanup_error}"
    );
    assert!(!target.exists());
    assert_eq!(git(&h.root, &["rev-parse", &reference]), head);
    let disposed = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert_eq!(disposed["entries"][0]["status"], "disposed");
    let result = maintained_cleanup["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == value!(target))
        .unwrap();
    let original_result = drain_report(result);
    assert_eq!(
        original_result["age_outcome"],
        "expired-checkout-verified-absent-source-branches-retained"
    );
    assert_eq!(
        show(
            &original_result["source_preservation"]["retained_ref"],
            "payload"
        ),
        "source at deletion"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn reflog_custody_rejects_conflicting_direct_symbolic_and_dangling_refs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for conflict in ["different-oid", "symbolic", "dangling"] {
        let h = Host::new("payload");
        policy(&h, 1, 3600, "dispose");
        let target = h.parent.join("conflicting-reflog-custody");
        start(&h, "conflicting-reflog-custody", &target);
        let head = git(&target, &["rev-parse", "HEAD"]);
        fs::write(target.join("payload"), "reflog-only-source").unwrap();
        let unique = commit(&target);
        git(&target, &["reset", "--hard", &head]);
        fs::write(target.join("payload"), "source must survive failed custody").unwrap();
        let ledger = h.ledger();
        let entry = ledger["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == value!(target))
            .unwrap();
        let reference = format!(
            "refs/heads/preserved/age/{}/reflog-{unique}",
            sha256(entry["ownership"]["id"].as_str().unwrap().as_bytes())
        );
        match conflict {
            "different-oid" => {
                git(&h.root, &["update-ref", &reference, &head]);
            }
            "symbolic" => {
                git(&h.root, &["symbolic-ref", &reference, "refs/heads/dev"]);
            }
            "dangling" => {
                git(
                    &h.root,
                    &["symbolic-ref", &reference, "refs/heads/missing-source"],
                );
            }
            _ => unreachable!(),
        }
        output(&target);
        temporary_output(&target);
        older_than(&target, 1);
        let (code, maintained, error) = h.auto("maintain", &[]);
        assert_eq!(code, 2, "{conflict}: {maintained} {error}");
        assert!(
            !maintained["cleanup_failures"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
        assert_eq!(
            fs::read_to_string(target.join("payload")).unwrap(),
            "source must survive failed custody"
        );
        assert_eq!(
            fs::read(target.join("nested/cache/temporary")).unwrap(),
            b"disposable"
        );
        if conflict == "different-oid" {
            assert_eq!(git(&h.root, &["rev-parse", &reference]), head);
        } else {
            assert_eq!(
                git(&h.root, &["symbolic-ref", &reference]),
                if conflict == "symbolic" {
                    "refs/heads/dev"
                } else {
                    "refs/heads/missing-source"
                }
            );
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn aged_busy_consumer_defers_and_independent_expired_checkout_advances() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    policy(&h, 2, 5, "dispose");
    let busy = h.parent.join("aged-busy");
    let free = h.parent.join("aged-free");
    start(&h, "aged-busy", &busy);
    start(&h, "aged-free", &free);
    output(&busy);
    output(&free);
    fs::write(busy.join("payload"), "live-source").unwrap();
    let e = entry(&h, &busy);
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(e["ownership"]["path"].as_str().unwrap()),
        Some(e["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    older_than(&free, 5);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(busy.join("output λ/cache").exists());
    assert_eq!(
        fs::read_to_string(busy.join("payload")).unwrap(),
        "live-source"
    );
    assert!(!free.exists());
    assert!(report["drain"].as_array().unwrap().iter().any(|r| {
        r["path"] == value!(busy)
            && r["preserved_reason"]
                .as_str()
                .is_some_and(|s| s.contains("live registered"))
    }));
    drop(lease);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!busy.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn detached_age_reuse_preserves_source_and_index_without_moving_other_branches() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    // Keep expiry outside the reuse phase, as in the named checkout test.
    // Source preservation itself may take longer than the eventual 60s age.
    policy(&h, 1, 3600, "dispose");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("detached aged checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            target.to_str().unwrap(),
            "dev",
        ],
    );
    let created = fs::metadata(&target).unwrap().created().unwrap();
    let head = git(&target, &["rev-parse", "HEAD"]);
    let dev = git(&h.root, &["rev-parse", "dev"]);
    let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{imported} {error}");
    fs::write(target.join("payload"), "original staged source").unwrap();
    git(&target, &["add", "payload"]);
    fs::write(target.join("payload"), "current detached source").unwrap();
    output(&target);
    let cache = cache_identity(&target);
    older_than(&target, 1);
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert!(used["age_reuse"].is_null(), "{used}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(&target, &["show", ":payload"]),
        "original staged source"
    );
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "current detached source"
    );
    assert!(target.join("output λ/cache").is_file());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let result = maintained["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target) && !row["receipt"].is_null())
        .unwrap();
    let original_result = drain_report(result);
    let saved = &original_result["source_preservation"];
    assert_eq!(saved["original_head"], head);
    assert_ne!(saved["head"], head);
    let show = |reference: &Value| {
        git(
            &h.root,
            &["show", &format!("{}:payload", reference.as_str().unwrap())],
        )
    };
    assert_eq!(show(&saved["index_ref"]), "original staged source");
    assert_eq!(show(&saved["retained_ref"]), "current detached source");
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), dev);
    assert_eq!(git(&target, &["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");
    assert_eq!(
        git(&target, &["rev-parse", "HEAD"]),
        saved["head"].as_str().unwrap()
    );
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "current detached source"
    );
    assert_eq!(cache_identity(&target), cache);
    let path = target.join(".chrono-harness/cleanup.json");
    let mut adopted = json(&fs::read(&path).unwrap()).unwrap();
    adopted["age_policy"]["delete_after_seconds"] = value!(60);
    fs::write(path, serde_json::to_vec_pretty(&adopted).unwrap()).unwrap();
    commit(&target);
    let (code, migrated, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{migrated} {error}");
    assert_eq!(fs::metadata(&target).unwrap().created().unwrap(), created);
    assert_eq!(git(&target, &["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");
    older_than(&target, 60);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists(), "{finished}");
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), dev);
    assert_eq!(show(&saved["retained_ref"]), "current detached source");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn fresh_checkout_and_needed_evidence_are_preserved_by_ordinary_use() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    policy(&h, 30, 40, "evidence-retain");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("age-protected");
    start(&h, "age-protected", &target);
    fs::write(target.join("payload"), "fresh work").unwrap();
    let old = git(&target, &["rev-parse", "HEAD"]);
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert!(used["age_reuse"].is_null());
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), old);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/needed-original"),
        b"needed original\0\xff",
    )
    .unwrap();
    older_than(&target, 40);
    output(&target);
    let cache = cache_identity(&target);
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert!(used["age_reuse"].is_null(), "{used}");
    assert_eq!(cache_identity(&target), cache);
    assert!(target.exists());
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(target.exists());
    assert!(!target.join("output λ").exists(), "{maintained}");
    let current = entry(&h, &target);
    assert_eq!(current["status"], "active", "{current}");
    assert!(current["terminal"].is_null(), "{current}");
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "fresh work"
    );
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
    let observations = maintained["drain"].as_array().unwrap();
    assert!(
        observations
            .iter()
            .map(drain_report)
            .any(|r| !r["age_deletion_deferred"].is_null())
    );
    // A later operation can rebuild this cache without reopening or finishing work.
    output(&target);
    let rebuilt = cache_identity(&target);
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(cache_identity(&target), rebuilt);
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--artifacts-only"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(target.exists());
    assert!(!target.join("output λ").exists(), "{finished}");
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn empty_aged_cache_preserves_working_source_until_an_effect_is_needed() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    policy(&h, 1, 3, "evidence-retain");
    let target = h.parent.join("empty-aged-cache");
    start(&h, "empty-aged-cache", &target);
    let original_head = git(&target, &["rev-parse", "HEAD"]);
    fs::write(target.join("payload"), "staged source").unwrap();
    git(&target, &["add", "payload"]);
    let original_index = git(&target, &["write-tree"]);
    fs::write(target.join("payload"), "working source").unwrap();
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    let evidence = target.join(".chrono-harness/state/needed-original");
    fs::write(&evidence, b"required failure original\0\xff").unwrap();
    older_than(&target, 3);

    for _ in 0..2 {
        let (code, maintained, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{maintained} {error}");
        assert_eq!(git(&target, &["rev-parse", "HEAD"]), original_head);
        assert_eq!(git(&target, &["write-tree"]), original_index);
        assert_eq!(fs::read(target.join("payload")).unwrap(), b"working source");
        assert_eq!(
            fs::read(&evidence).unwrap(),
            b"required failure original\0\xff"
        );
        let current = entry(&h, &target);
        assert!(current["terminal"].is_null(), "{current}");
        assert!(target.exists());
    }

    // A new disposable output must still take the original preservation route.
    output(&target);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!target.join("output λ").exists());
    assert_eq!(fs::read(target.join("payload")).unwrap(), b"working source");
    assert_ne!(git(&target, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(
        fs::read(&evidence).unwrap(),
        b"required failure original\0\xff"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn finished_aged_checkout_records_retained_after_reusable_cache_cleanup() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    policy(&h, 1, 3600, "evidence-retain");
    let target = h.parent.join("finished-aged");
    start(&h, "finished-aged", &target);
    let head = git(&target, &["rev-parse", "HEAD"]);
    output(&target);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    let original = target.join(".chrono-harness/state/needed-original");
    fs::write(&original, b"needed original\0\xff").unwrap();
    older_than(&target, 1);
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--artifacts-only"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    let terminal = entry(&h, &target)["terminal"].clone();
    let receipt = h.root.join(terminal["receipt"]["path"].as_str().unwrap());
    let terminal_bytes = fs::read(&receipt).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let current = entry(&h, &target);
    assert_eq!(current["status"], "retained", "{current}");
    assert_eq!(current["terminal"], terminal);
    assert_eq!(fs::read(receipt).unwrap(), terminal_bytes);
    assert!(!target.join("output λ").exists(), "{maintained}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(original).unwrap(), b"needed original\0\xff");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn interrupted_age_disposal_reentry_preserves_original_unknown_result() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::os::unix::process::ExitStatusExt;
    let h = Host::new("payload");
    policy(&h, 1, 3, "dispose");
    native_git(&h, "age-interrupt", value!({}));
    let target = h.parent.join("interrupted-age");
    start(&h, "interrupted-age", &target);
    fs::write(target.join("payload"), "source survives interruption").unwrap();
    output(&target);
    older_than(&target, 3);
    fs::write(h.parent.join("interrupt-age"), "interrupt original removal").unwrap();
    let original = h._dir.capture_bound(&mut h.auto_command("maintain", &[]));
    assert_eq!(original.status.signal(), Some(9), "{original:?}");
    h.received(original);
    let pending = entry(&h, &target);
    assert_eq!(pending["status"], "terminal");
    assert!(pending["attempts"][0]["receipt"].is_null());
    let intent = pending["enrollment"]["age_retirement"].clone();
    let intent_bytes = fs::read(h.root.join(intent["path"].as_str().unwrap())).unwrap();
    fs::remove_file(h.parent.join("interrupt-age")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.exists());
    assert_eq!(
        fs::read(h.root.join(intent["path"].as_str().unwrap())).unwrap(),
        intent_bytes
    );
    let after = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(after["entries"][0]["attempts"][0]["receipt"].is_null());
    assert_eq!(after["entries"][0]["attempts"][0], pending["attempts"][0]);
    assert_eq!(after["entries"][0]["status"], "disposed");
}
