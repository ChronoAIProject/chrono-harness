use chrono_harness::retained_artifacts::{
    self,
    retention::{Inventory, POLICY_PATH},
};
use serde_json::{Value, json};
use std::{fs, path::Path};
fn host(nodes: usize, summaries: usize) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join(".chrono-harness/state/fixtures")).unwrap();
    fs::write(d.path().join(POLICY_PATH),serde_json::to_vec(&json!({"schema":"chrono-output-retention/v1","producers":{"fixture":[".chrono-harness/state/fixtures/"]},"keep_seconds":0,"max_entries":32,"max_summaries":summaries,"max_nodes_per_round":nodes,"max_bytes_per_round":1048576,"max_millis_per_round":1000})).unwrap()).unwrap();
    d
}
fn inventory(root: &Path) -> Inventory {
    Inventory::adopted(root).unwrap().unwrap()
}
fn tree(root: &Path, name: &str) -> String {
    let path = format!(".chrono-harness/state/fixtures/{name}");
    fs::create_dir(root.join(&path)).unwrap();
    path
}
fn ledger(root: &Path) -> Value {
    serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/state/output-retention-v1/inventory.json")).unwrap(),
    )
    .unwrap()
}
#[test]
fn creation_intent_survives_no_finish_and_admission_creates_nothing() {
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/not-created";
    let publication = inventory(h.path())
        .plan(
            "fixture",
            path,
            true,
            json!({"exit":null,"operation":"create"}),
        )
        .unwrap();
    assert!(!h.path().join(path).exists());
    assert_eq!(ledger(h.path())["outputs"][0]["phase"], "creating");
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    drop(publication);
    let round = retained_artifacts::retention::maintain_adopted(h.path())
        .unwrap()
        .unwrap();
    assert_eq!(round["remaining_outputs"], 0);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["original"]["operation"],
        "create"
    );
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_entries"] = json!(1);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    inventory(h.path()).migrate_policy(&serde_json::to_vec(&json!({"schema":"chrono-output-retention/v1","producers":{"fixture":[".chrono-harness/state/fixtures/"]},"keep_seconds":0,"max_entries":32,"max_summaries":16,"max_nodes_per_round":256,"max_bytes_per_round":1048576,"max_millis_per_round":1000})).unwrap()).unwrap();
    let live = inventory(h.path())
        .plan("fixture", path, true, json!(null))
        .unwrap();
    let refused = h.path().join(".chrono-harness/state/fixtures/refused");
    assert!(
        retained_artifacts::retention::create_adopted(
            h.path(),
            "fixture",
            &refused,
            true,
            json!(null)
        )
        .is_err()
    );
    assert!(!refused.exists());
    drop(live);
}

#[test]
fn controlled_fixture_writes_refuse_before_copy_and_keep_original_outcomes() {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);
    policy["max_total_bytes"] = json!(256);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = h.path().join(".chrono-harness/state/fixtures/copied");
    let publication = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &path,
        true,
        json!({"original_exit":101}),
    )
    .unwrap()
    .unwrap();
    let input = h.path().join("input");
    fs::write(&input, [3; 129]).unwrap();
    assert!(
        publication
            .copy_file(&input, &path.join("too-large"))
            .unwrap_err()
            .contains("before writing")
    );
    assert!(!path.join("too-large").exists());
    fs::write(&input, [3; 64]).unwrap();
    publication.copy_file(&input, &path.join("first")).unwrap();
    publication
        .write_file(&path.join("second"), &[4; 64])
        .unwrap();
    assert!(publication.write_file(&path.join("third"), &[5]).is_err());
    assert!(!path.join("third").exists());
    publication
        .complete(json!({"exit":101,"error":"original failure"}))
        .unwrap();
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["removed_logical_bytes"], 128);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["exit"],
        101
    );
}

#[test]
fn released_byte_overflow_is_accounted_and_reclaimed_under_unchanged_policy() {
    let h = host(8, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);
    policy["max_total_bytes"] = json!(256);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = tree(h.path(), "external-build");
    let p = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    inventory(h.path())
        .reference("current", &[p.id().into()])
        .unwrap();
    fs::write(h.path().join(&path).join("build.bin"), [8; 2048]).unwrap();
    assert!(p.complete(json!({"exit":7})).is_err());
    let accounted = inventory(h.path()).maintain().unwrap();
    assert_eq!(accounted["reserved_logical_bytes"], 2048);
    assert_eq!(accounted["overflow_outputs"], 1);
    assert!(
        accounted["known_retained_allocated_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(h.path().join(&path).exists());
    inventory(h.path()).reference("current", &[]).unwrap();
    for _ in 0..8 {
        let round = retained_artifacts::retention::maintain_adopted(h.path())
            .unwrap()
            .unwrap();
        assert!(round["nodes"].as_u64().unwrap() <= 8);
        if round["remaining_outputs"] == 0 {
            break;
        }
    }
    assert!(!h.path().join(path).exists());
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["exit"],
        7
    );
}
#[test]
fn released_original_larger_than_verification_round_is_not_permanently_blocked() {
    let h = host(8, 4);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_bytes_per_round"] = json!(64);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = ".chrono-harness/state/fixtures/large-original";
    fs::write(h.path().join(path), [9; 4096]).unwrap();
    let p = inventory(h.path())
        .begin("fixture", path, json!(null))
        .unwrap();
    let id = p.id().to_string();
    p.complete(json!({"exit":101})).unwrap();
    let reader = inventory(h.path()).consume(&id).unwrap();
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    drop(reader);
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["remaining_outputs"], 0);
    assert_eq!(round["removed_logical_bytes"], 4096);
    assert_eq!(round["verified_bytes"], 0);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["exit"],
        101
    );
    assert_eq!(
        ledger(h.path())["summaries"][0]["large_original_recheck"],
        "sealed-identity-and-content-stamp; digest-not-reread"
    );
}

#[test]
fn killed_creation_intent_recovers_on_ordinary_entry_without_a_callback() {
    use std::io::BufRead;
    let h = host(8, 4);
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
        .args([h.path().to_str().unwrap(), "creation-intent"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut id = String::new();
    reader.read_line(&mut id).unwrap();
    assert!(id.starts_with("output-"));
    let live = inventory(h.path()).maintain().unwrap();
    assert_eq!(live["remaining_outputs"], 1);
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    for _ in 0..4 {
        if retained_artifacts::retention::maintain_adopted(h.path())
            .unwrap()
            .unwrap()["remaining_outputs"]
            == 0
        {
            break;
        }
    }
    assert_eq!(ledger(h.path())["outputs"].as_array().unwrap().len(), 0);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["publication"],
        "interrupted-creation"
    );
}

#[test]
fn released_manifest_overflow_makes_bounded_progress_without_policy_growth() {
    let h = host(4, 4);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_manifest_nodes"] = json!(4);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = tree(h.path(), "many-build-files");
    let p = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    inventory(h.path())
        .reference("build-reader", &[p.id().into()])
        .unwrap();
    for n in 0..20 {
        fs::write(h.path().join(&path).join(format!("{n}.bin")), [7; 32]).unwrap();
    }
    assert!(
        p.complete(json!({"exit":101,"error":"build original"}))
            .is_err()
    );
    inventory(h.path()).maintain().unwrap();
    assert_eq!(fs::read_dir(h.path().join(&path)).unwrap().count(), 20);
    inventory(h.path()).reference("build-reader", &[]).unwrap();
    let mut removed = 0;
    for _ in 0..64 {
        let round = retained_artifacts::retention::maintain_adopted(h.path())
            .unwrap()
            .unwrap();
        assert!(round["nodes"].as_u64().unwrap() <= 4);
        assert!(
            round["metadata_bytes"].as_u64().unwrap()
                <= policy["max_metadata_bytes_per_round"]
                    .as_u64()
                    .unwrap_or(64 * 1024 * 1024)
        );
        removed += round["removed_logical_bytes"].as_u64().unwrap();
        if round["remaining_outputs"] == 0 {
            break;
        }
    }
    assert!(!h.path().join(path).exists());
    assert_eq!(removed, 640);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["exit"],
        101
    );
    assert_eq!(
        fs::read_dir(h.path().join(".chrono-harness/state/output-retention-v1"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("output-"))
            .count(),
        0
    );
}

#[test]
fn creation_destination_replacement_is_preserved_and_controlled_nodes_are_refused() {
    let h = host(8, 4);
    let path = ".chrono-harness/state/fixtures/planned";
    let p = inventory(h.path())
        .plan("fixture", path, false, json!({"exit":null}))
        .unwrap();
    fs::write(h.path().join(path), b"unrelated replacement").unwrap();
    assert!(p.materialize().is_err());
    drop(p);
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["remaining_outputs"], 1);
    assert_eq!(
        fs::read(h.path().join(path)).unwrap(),
        b"unrelated replacement"
    );
    assert!(
        round["accounting_errors"]
            .to_string()
            .contains("identity changed")
    );
    let h = host(8, 4);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_manifest_nodes"] = json!(2);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = h.path().join(".chrono-harness/state/fixtures/two-nodes");
    let p = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &path,
        true,
        json!(null),
    )
    .unwrap()
    .unwrap();
    p.write_file(&path.join("first"), b"first").unwrap();
    assert!(
        p.create_directory(&path.join("excess"))
            .unwrap_err()
            .contains("before writing")
    );
    assert!(!path.join("excess").exists());
    p.complete(json!({"exit":0})).unwrap();
}

#[test]
fn preparation_write_refusal_leaves_owner_intent_and_ordinary_recovery() {
    let h = report_host();
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);
    policy["max_total_bytes"] = json!(256);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    assert!(
        chrono_harness::prepared::retain_bytes(h.path(), "refused-", &[7; 129])
            .unwrap_err()
            .contains("before writing")
    );
    let before = ledger(h.path());
    assert_eq!(before["outputs"].as_array().unwrap().len(), 1);
    assert_eq!(before["outputs"][0]["phase"], "publishing");
    let path = before["outputs"][0]["path"].as_str().unwrap();
    assert_eq!(fs::metadata(h.path().join(path)).unwrap().len(), 0);
    retained_artifacts::retention::maintain_adopted(h.path()).unwrap();
    assert!(!h.path().join(path).exists());
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["completion"],
        "not-established"
    );
}

#[test]
fn ordinary_entry_reclaims_completed_nested_outputs_with_real_savings() {
    let h = host(256, 16);
    let p = tree(h.path(), "sdk");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!({"status":"running"}))
        .unwrap();
    fs::create_dir_all(h.path().join(&p).join("target/incremental")).unwrap();
    fs::write(
        h.path().join(&p).join("target/incremental/data"),
        vec![3; 2 * 1024 * 1024],
    )
    .unwrap();
    publication
        .complete(json!({"status":"failed","exit":7,"error":"original failure"}))
        .unwrap();
    let result = retained_artifacts::retention::maintain_adopted(h.path())
        .unwrap()
        .unwrap();
    assert!(!h.path().join(p).exists());
    assert_eq!(result["removed_logical_bytes"], 2 * 1024 * 1024);
    assert_eq!(result["physical_bytes_reclaimed"], Value::Null);
    let l = ledger(h.path());
    assert_eq!(l["summaries"][0]["original_outcome"]["exit"], 7);
    assert_eq!(
        l["summaries"][0]["original_outcome"]["error"],
        "original failure"
    );
    assert_eq!(result["completed_work"], "not-claimed");
}
#[test]
fn no_finish_preserves_live_producer_then_recovers_unknown_partial_publication() {
    let h = host(256, 16);
    let p = tree(h.path(), "interrupted");
    let publication = inventory(h.path())
        .begin(
            "fixture",
            &p,
            json!({"operation":"build","status":"unknown"}),
        )
        .unwrap();
    fs::write(
        h.path().join(&p).join("partial"),
        b"not a successful result",
    )
    .unwrap();
    let id = publication.id().to_owned();
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    assert!(h.path().join(&p).exists());
    // Simulate the kernel capability closing without any finish/completion callback.
    drop(publication);
    let r = inventory(h.path()).maintain().unwrap();
    assert_eq!(r["remaining_outputs"], 0);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["completion"],
        "not-established"
    );
    assert!(inventory(h.path()).consume(&id).is_err());
}
#[test]
fn current_references_protect_closure_and_released_historical_cycles_are_not_roots() {
    let h = host(256, 16);
    let a = tree(h.path(), "a");
    let b = tree(h.path(), "b");
    let pa = inventory(h.path())
        .begin("fixture", &a, json!("a"))
        .unwrap();
    let aid = pa.id().to_owned();
    pa.complete(json!({"exit":0})).unwrap();
    let pb = inventory(h.path())
        .begin("fixture", &b, json!("b"))
        .unwrap();
    let bid = pb.id().to_owned();
    pb.complete(json!({"exit":9})).unwrap();
    let i = inventory(h.path());
    i.dependencies(&aid, &[bid.clone()]).unwrap();
    i.dependencies(&bid, &[aid.clone()]).unwrap();
    i.reference("pending-native-rerun", &[aid.clone()]).unwrap();
    assert_eq!(i.maintain().unwrap()["remaining_outputs"], 2);
    i.reference("pending-native-rerun", &[]).unwrap();
    assert_eq!(i.maintain().unwrap()["remaining_outputs"], 0);
    let s = ledger(h.path());
    let exits: Vec<_> = s["summaries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["original_outcome"]["exit"].as_i64().unwrap())
        .collect();
    assert!(exits.contains(&0) && exits.contains(&9));
}
#[test]
fn live_consumer_protects_references_even_after_retention_window_expires() {
    let h = host(256, 16);
    let a = tree(h.path(), "a");
    let b = tree(h.path(), "b");
    let pa = inventory(h.path())
        .begin("fixture", &a, json!(null))
        .unwrap();
    let aid = pa.id().to_owned();
    pa.complete(json!(0)).unwrap();
    let pb = inventory(h.path())
        .begin("fixture", &b, json!(null))
        .unwrap();
    let bid = pb.id().to_owned();
    pb.complete(json!(1)).unwrap();
    let i = inventory(h.path());
    i.dependencies(&aid, &[bid]).unwrap();
    let lease = i.consume(&aid).unwrap();
    assert_eq!(i.maintain().unwrap()["remaining_outputs"], 2);
    drop(lease);
    assert_eq!(i.maintain().unwrap()["remaining_outputs"], 0);
}
#[test]
fn identity_drift_and_changed_policy_preserve_original_evidence() {
    let h = host(256, 16);
    let p = tree(h.path(), "original");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    publication.complete(json!({"exit":4})).unwrap();
    fs::rename(h.path().join(&p), h.path().join("old-output")).unwrap();
    fs::create_dir(h.path().join(&p)).unwrap();
    fs::write(h.path().join(&p).join("new"), b"replacement").unwrap();
    let result = inventory(h.path()).maintain().unwrap();
    assert_eq!(result["remaining_outputs"], 1);
    assert!(result["protected"].to_string().contains("identity changed"));
    assert!(h.path().join(&p).join("new").exists());
    let i = inventory(h.path());
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["keep_seconds"] = json!(10);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    assert!(i.maintain().is_err());
    assert!(inventory(h.path()).maintain().is_err());
}
#[test]
fn partial_disposal_is_bounded_resumable_and_cannot_be_referenced_again() {
    let h = host(3, 2);
    let p = tree(h.path(), "many");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    let id = publication.id().to_owned();
    for n in 0..20 {
        fs::write(h.path().join(&p).join(n.to_string()), [7; 32]).unwrap();
    }
    publication.complete(json!({"exit":6})).unwrap();
    let i = inventory(h.path());
    let r = i.maintain().unwrap();
    assert!(r["nodes"].as_u64().unwrap() <= 3);
    assert_eq!(r["remaining_outputs"], 1);
    assert!(i.reference("too-late", &[id]).is_err());
    for _ in 0..80 {
        let r = i.maintain().unwrap();
        assert!(r["nodes"].as_u64().unwrap() <= 3);
        if r["remaining_outputs"] == 0 {
            break;
        }
    }
    assert!(!h.path().join(&p).exists());
    assert_eq!(
        ledger(h.path())["summaries"][0]["removed_logical_bytes"],
        640
    );
    for n in 0..8 {
        let p = tree(h.path(), &format!("more-{n}"));
        inventory(h.path())
            .begin("fixture", &p, json!(null))
            .unwrap()
            .complete(json!({"exit":n}))
            .unwrap();
        i.maintain().unwrap();
    }
    assert!(ledger(h.path())["summaries"].as_array().unwrap().len() <= 2);
}
#[test]
fn immutable_file_content_drift_is_refused_without_changing_original_outcome() {
    let h = host(256, 16);
    let p = ".chrono-harness/state/fixtures/original.json";
    fs::write(h.path().join(p), b"original failure").unwrap();
    inventory(h.path())
        .begin("fixture", p, json!(null))
        .unwrap()
        .complete(json!({"exit":17}))
        .unwrap();
    fs::write(h.path().join(p), b"changed failure").unwrap();
    let r = inventory(h.path()).maintain().unwrap();
    assert!(r["protected"].to_string().contains("content changed"));
    assert!(h.path().join(p).exists());
    assert_eq!(ledger(h.path())["outputs"][0]["outcome"]["exit"], 17);
}
#[test]
fn legacy_without_policy_and_unenrolled_outputs_remain_untouched() {
    let h = tempfile::tempdir().unwrap();
    assert!(Inventory::adopted(h.path()).unwrap().is_none());
    let h = host(256, 16);
    let p = tree(h.path(), "unknown-legacy");
    inventory(h.path()).maintain().unwrap();
    assert!(h.path().join(p).exists());
}
#[test]
fn verified_content_reuse_preserves_independent_operation_results() {
    let h = host(256, 16);
    let input = h.path().join("input");
    fs::write(&input, b"shared original failure bytes").unwrap();
    let a = retained_artifacts::stage(h.path(), &input, ".chrono-harness/state/fixtures/").unwrap();
    let b = retained_artifacts::stage(h.path(), &input, ".chrono-harness/state/fixtures/").unwrap();
    assert_eq!(a, b);
    assert_eq!(
        fs::read_dir(h.path().join(".chrono-harness/state/fixtures/blobs"))
            .unwrap()
            .count(),
        1
    );
    let blob = a["storage"].as_str().unwrap();
    fs::write(h.path().join(blob), b"tampered").unwrap();
    assert!(
        retained_artifacts::stage(h.path(), &input, ".chrono-harness/state/fixtures/").is_err()
    );
}

#[test]
fn killed_producer_is_joined_and_ordinary_entry_recovers_without_any_drop_callback() {
    use std::io::{BufRead, BufReader};
    let h = host(256, 16);
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
        .arg(h.path())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert!(!ready.trim().is_empty());
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    child.kill().unwrap();
    let joined = child.wait().unwrap();
    assert!(!joined.success());
    assert_eq!(
        retained_artifacts::retention::maintain_adopted(h.path())
            .unwrap()
            .unwrap()["remaining_outputs"],
        0
    );
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["original"]["original_exit"],
        Value::Null
    );
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["completion"],
        "not-established"
    );
}

#[cfg(unix)]
#[test]
fn bounded_disposal_unlinks_internal_symlink_without_touching_external_original() {
    let h = host(256, 16);
    let external = tempfile::tempdir().unwrap();
    fs::write(external.path().join("evidence"), b"external original").unwrap();
    let p = tree(h.path(), "links");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    std::os::unix::fs::symlink(external.path(), h.path().join(&p).join("outside")).unwrap();
    publication.complete(json!({"exit":5})).unwrap();
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        0
    );
    assert_eq!(
        fs::read(external.path().join("evidence")).unwrap(),
        b"external original"
    );
}

#[test]
fn reserved_publication_and_retired_capability_intents_recover_without_scanning() {
    let h = host(3, 16);
    let p = tree(h.path(), "reserved");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!({"original_exit":null}))
        .unwrap();
    let id = publication.id().to_owned();
    drop(publication);
    // Reproduce durable reservation followed by interruption before capability attachment.
    let mut state = ledger(h.path());
    state["outputs"][0]["phase"] = json!("reserving");
    state["outputs"][0]["lease_id"] = json!("");
    fs::remove_file(
        h.path()
            .join(".chrono-harness/state/output-retention-v1")
            .join(&id),
    )
    .unwrap();
    fs::write(
        h.path()
            .join(".chrono-harness/state/output-retention-v1/inventory.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    for _ in 0..8 {
        inventory(h.path()).maintain().unwrap();
    }
    let state = ledger(h.path());
    assert!(state["outputs"].as_array().unwrap().is_empty());
    assert!(state["retired_leases"].as_array().unwrap().is_empty());
    assert!(
        !h.path()
            .join(".chrono-harness/state/output-retention-v1")
            .join(id)
            .exists()
    );
    assert_eq!(
        state["summaries"][0]["original_outcome"]["completion"],
        "not-established"
    );
}

#[test]
fn protected_capacity_refuses_enrollment_and_bounded_replacement_recovers() {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_entries"] = json!(1);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let p = tree(h.path(), "live");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    let next = tree(h.path(), "next");
    assert!(
        inventory(h.path())
            .begin("fixture", &next, json!(null))
            .is_err()
    );
    assert!(h.path().join(&p).exists());
    fs::write(
        h.path()
            .join(".chrono-harness/state/output-retention-v1/inventory.pending"),
        b"interrupted replacement bytes",
    )
    .unwrap();
    let r = inventory(h.path()).maintain().unwrap();
    assert_eq!(r["inventory_capacity"], 1);
    assert_eq!(r["remaining_outputs"], 1);
    assert!(
        !h.path()
            .join(".chrono-harness/state/output-retention-v1/inventory.pending")
            .exists()
    );
    drop(publication);
}

fn report_host() -> tempfile::TempDir {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["producers"]["check-original"] = json!([".chrono-harness/state/preparation/"]);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    h
}
#[test]
fn actual_original_publisher_and_reader_reclaim_rotated_reports_but_preserve_shared_inputs() {
    use chrono_harness::prepared::{read_original, retain_original};
    use retained_artifacts::retention::{reference_report, settle_report};
    let h = report_host();
    let dir = ".chrono-harness/state/preparation/";
    let activity = inventory(h.path()).report_activity().unwrap();
    let shared = retain_original(h.path(), dir, "judge-stdout", b"same original stdout").unwrap();
    let old = retain_original(h.path(), dir, "report", br#"{"request":"one","exit":7}"#).unwrap();
    let old_lock = reference_report(
        h.path(),
        "unit/check.json",
        &old.path,
        &[shared.path.clone()],
        None,
    )
    .unwrap();
    settle_report(h.path(), "unit/check.json", &old.path).unwrap();
    drop(old_lock);
    drop(activity);
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        2
    );
    assert_eq!(
        read_original(h.path(), &shared, None).unwrap(),
        b"same original stdout"
    );
    let activity = inventory(h.path()).report_activity().unwrap();
    let reused = retain_original(h.path(), dir, "judge-stdout", b"same original stdout").unwrap();
    assert_eq!(shared, reused);
    let new = retain_original(h.path(), dir, "report", br#"{"request":"two","exit":0}"#).unwrap();
    let new_lock = reference_report(
        h.path(),
        "unit/check.json",
        &new.path,
        &[shared.path.clone()],
        None,
    )
    .unwrap();
    settle_report(h.path(), "unit/check.json", &new.path).unwrap();
    drop(new_lock);
    drop(activity);
    let result = inventory(h.path()).maintain().unwrap();
    assert_eq!(result["remaining_outputs"], 2);
    assert!(!h.path().join(old.path).exists());
    assert_eq!(
        read_original(h.path(), &new, None).unwrap(),
        br#"{"request":"two","exit":0}"#
    );
    assert_eq!(
        read_original(h.path(), &shared, None).unwrap(),
        b"same original stdout"
    );
    assert!(result["removed_logical_bytes"].as_u64().unwrap() > 0);
}
#[test]
fn interrupted_fixed_pointer_publication_keeps_both_originals_until_owner_settles() {
    use chrono_harness::prepared::retain_original;
    use retained_artifacts::retention::{reference_report, settle_report};
    let h = report_host();
    let dir = ".chrono-harness/state/preparation/";
    let first = retain_original(h.path(), dir, "report", b"first actual error").unwrap();
    drop(reference_report(h.path(), "check.json", &first.path, &[], None).unwrap());
    settle_report(h.path(), "check.json", &first.path).unwrap();
    let second = retain_original(h.path(), dir, "report", b"second actual error").unwrap();
    drop(reference_report(h.path(), "check.json", &second.path, &[], None).unwrap());
    // No pointer success/completion callback. A later ordinary entry must preserve both.
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        2
    );
    settle_report(h.path(), "check.json", &first.path).unwrap();
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    assert!(h.path().join(first.path).exists());
    assert!(!h.path().join(second.path).exists());
}
#[test]
fn report_activity_survives_expiry_and_abandoned_unrooted_originals_are_reclaimed() {
    use chrono_harness::prepared::retain_original;
    let h = report_host();
    let activity = retained_artifacts::retention::report_activity(h.path()).unwrap();
    let original = retain_original(
        h.path(),
        ".chrono-harness/state/preparation/",
        "acquisition",
        b"partial acquisition",
    )
    .unwrap();
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    drop(activity); // kernel close models an owner with no successful check result.
    assert_eq!(
        retained_artifacts::retention::maintain_adopted(h.path())
            .unwrap()
            .unwrap()["remaining_outputs"],
        0
    );
    assert!(!h.path().join(original.path).exists());
}
#[test]
fn pending_native_delivery_is_independent_of_latest_report_and_explicitly_released() {
    use chrono_harness::prepared::retain_original;
    use retained_artifacts::retention::{reference_report, settle_report};
    let h = report_host();
    let original = retain_original(
        h.path(),
        ".chrono-harness/state/preparation/",
        "report",
        b"native actual outcome",
    )
    .unwrap();
    drop(
        reference_report(
            h.path(),
            "check.json",
            &original.path,
            &[],
            Some("request-one"),
        )
        .unwrap(),
    );
    settle_report(h.path(), "check.json", &original.path).unwrap();
    let i = inventory(h.path());
    i.reference_paths(
        &format!("current-report:{}", chrono_harness::sha256(b"check.json")),
        &[],
    )
    .unwrap();
    assert_eq!(i.maintain().unwrap()["remaining_outputs"], 1);
    i.reference("native-delivery:request-one", &[]).unwrap();
    assert_eq!(i.maintain().unwrap()["remaining_outputs"], 0);
}
#[test]
fn addressed_legacy_original_is_verified_without_silent_enrollment_or_rewriting() {
    use chrono_harness::prepared::retain_original;
    let h = report_host();
    let dir = ".chrono-harness/state/preparation/";
    let path = format!("{dir}old-{}.json", chrono_harness::sha256(b"legacy"));
    fs::create_dir_all(h.path().join(dir)).unwrap();
    fs::write(h.path().join(&path), b"legacy").unwrap();
    assert_eq!(
        retain_original(h.path(), dir, "old", b"legacy")
            .unwrap()
            .path,
        path
    );
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        0
    );
    assert_eq!(fs::read(h.path().join(path)).unwrap(), b"legacy");
}

#[test]
fn explicit_policy_migration_preserves_originals_and_refuses_live_readers_and_missing_scopes() {
    let h = host(256, 16);
    let path = tree(h.path(), "migration");
    let publication = inventory(h.path())
        .begin("fixture", &path, json!({"exit":9}))
        .unwrap();
    let id = publication.id().to_owned();
    publication
        .complete(json!({"exit":9,"original_error":"keep"}))
        .unwrap();
    let original = ledger(h.path());
    let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["producers"]["check-original"] = json!([".chrono-harness/state/preparation/"]);
    let reader = inventory(h.path()).consume(&id).unwrap();
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    assert!(
        inventory(h.path())
            .maintain()
            .unwrap_err()
            .contains("migration required")
    );
    assert!(
        inventory(h.path())
            .migrate_policy(&previous)
            .unwrap_err()
            .contains("live output")
    );
    assert_eq!(ledger(h.path()), original);
    drop(reader);
    policy["producers"]
        .as_object_mut()
        .unwrap()
        .remove("fixture");
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    assert!(inventory(h.path()).migrate_policy(&previous).is_err());
    assert_eq!(ledger(h.path()), original);
    policy["producers"]["fixture"] = json!([".chrono-harness/state/fixtures/"]);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let receipt = inventory(h.path()).migrate_policy(&previous).unwrap();
    assert_eq!(receipt["outputs_preserved"], 1);
    let mut after = ledger(h.path());
    after["policy_sha256"] = original["policy_sha256"].clone();
    assert_eq!(after, original);
    assert!(h.path().join(path).exists());
}

#[test]
fn addressed_reuse_refuses_equal_bytes_at_changed_identity_or_partial_disposal() {
    use chrono_harness::prepared::retain_original;
    for replacement in [false, true] {
        let h = report_host();
        if !replacement {
            let mut policy: Value =
                serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
            policy["max_nodes_per_round"] = json!(1);
            fs::write(
                h.path().join(POLICY_PATH),
                serde_json::to_vec(&policy).unwrap(),
            )
            .unwrap();
        }
        let dir = ".chrono-harness/state/preparation/";
        let original = retain_original(h.path(), dir, "report", b"same bytes").unwrap();
        if replacement {
            fs::rename(
                h.path().join(&original.path),
                h.path().join("saved-original"),
            )
            .unwrap();
            fs::write(h.path().join(&original.path), b"same bytes").unwrap();
        } else {
            inventory(h.path()).maintain().unwrap();
            assert_eq!(ledger(h.path())["outputs"][0]["phase"], "disposing");
        }
        assert!(
            retain_original(h.path(), dir, "report", b"same bytes")
                .unwrap_err()
                .contains("unavailable")
        );
        assert_eq!(
            fs::read(h.path().join(&original.path)).unwrap(),
            b"same bytes"
        );
    }
}

#[test]
fn sealed_tree_rejects_replaced_descendants_and_preserves_the_replacement() {
    let h = host(256, 16);
    let p = tree(h.path(), "sealed");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    let file = h.path().join(&p).join("original");
    fs::write(&file, b"old failure").unwrap();
    publication.complete(json!({"exit":101})).unwrap();
    fs::rename(&file, h.path().join("retained-original")).unwrap();
    fs::write(&file, b"new source evidence").unwrap();
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["remaining_outputs"], 1);
    assert!(round["protected"].to_string().contains("sealed descendant"));
    assert_eq!(fs::read(file).unwrap(), b"new source evidence");
    assert_eq!(ledger(h.path())["outputs"][0]["outcome"]["exit"], 101);
}

#[test]
fn sealed_tree_never_enrolls_a_late_unknown_descendant_during_disposal() {
    let h = host(256, 16);
    let p = tree(h.path(), "unknown");
    inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap()
        .complete(json!({"exit":1}))
        .unwrap();
    fs::write(h.path().join(&p).join("unowned"), b"retain me").unwrap();
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["remaining_outputs"], 1);
    assert!(h.path().join(&p).join("unowned").exists());
    assert_eq!(round["removed_logical_bytes"], 0);
}

#[test]
fn total_capacity_reserves_live_writers_then_charges_sealed_tree_bytes() {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(1024);
    policy["max_total_bytes"] = json!(1024);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let a = tree(h.path(), "a");
    let b = tree(h.path(), "b");
    let first = inventory(h.path())
        .begin("fixture", &a, json!(null))
        .unwrap();
    assert!(
        matches!(inventory(h.path()).begin("fixture",&b,json!(null)),Err(error) if error.contains("total retained"))
    );
    fs::write(h.path().join(&a).join("bytes"), [5; 400]).unwrap();
    first.complete(json!({"exit":0})).unwrap();
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["removed_logical_bytes"],
        400
    );
    inventory(h.path())
        .begin("fixture", &b, json!(null))
        .unwrap()
        .complete(json!({"exit":2}))
        .unwrap();
}

#[test]
fn staged_source_added_after_publication_prevents_disposal() {
    let h = host(256, 16);
    let p = tree(h.path(), "source");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    fs::write(h.path().join(&p).join("source.rs"), b"source").unwrap();
    publication.complete(json!({"exit":0})).unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("/usr/bin/git")
            .args(args)
            .current_dir(h.path())
            .output()
            .unwrap()
    };
    assert!(git(&["init", "-q"]).status.success());
    assert!(git(&["add", &format!("{p}/source.rs")]).status.success());
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["remaining_outputs"], 1);
    assert_eq!(round["removed_logical_bytes"], 0);
    assert!(h.path().join(&p).join("source.rs").exists());
}

#[test]
fn historical_cli_requires_registered_producer_receipt_exclusion_and_release_evidence() {
    use chrono_harness::{artifact_disposal::identity, ownership::Lease, sha256};
    let h = host(256, 16);
    let p = tree(h.path(), "historical");
    fs::write(h.path().join(&p).join("failure"), [9; 2048]).unwrap();
    let source = b"registered fixture producer source";
    fs::write(h.path().join("producer.rs"), source).unwrap();
    let receipt = json!({"schema":"fixture-producer/v1","path":p,"outcome":{"exit":101,"error":"original failure"}});
    let raw = serde_json::to_vec(&receipt).unwrap();
    fs::write(h.path().join("receipt.json"), &raw).unwrap();
    let release = serde_json::to_vec(&json!({"recovery":"released"})).unwrap();
    fs::write(h.path().join("release.json"), &release).unwrap();
    let lease_path = ".chrono-harness/state/legacy-owner.lease";
    let lease = Lease::acquire(&h.path().join(lease_path), None, true, false, None)
        .unwrap()
        .unwrap();
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["historical"] = json!({"cohort":{"producer":"fixture","path":p,"producer_source":{"path":"producer.rs","sha256":sha256(source)},"receipt":{"path":"receipt.json","sha256":sha256(&raw)},"receipt_schema":"fixture-producer/v1","output_pointer":"/path","outcome_pointer":"/outcome","owner_lease":lease_path,"owner_lease_identity":lease.id(),"roots":["historical-recovery:cohort"],"release":{"receipt":{"path":"release.json","sha256":sha256(&release)},"pointer":"/recovery","expected":"released"}}});
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let call = |request: Value| {
        fs::write(
            h.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        chrono_harness::dispatch(&[
            "retention",
            "--host-root",
            h.path().to_str().unwrap(),
            "--request",
            "request.json",
        ])
    };
    let request = json!({"operation":"enroll","enrollment":"cohort","expected_identity":identity(&h.path().join(&p)).unwrap()});
    assert_eq!(call(request.clone()).exit_code, 2); // Actual live legacy owner.
    drop(lease);
    let original_receipt = policy["historical"]["cohort"]["receipt"].clone();
    let nested_receipt = format!("{p}/receipt.json");
    fs::write(h.path().join(&nested_receipt), &raw).unwrap();
    policy["historical"]["cohort"]["receipt"]["path"] = json!(nested_receipt);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let rejected = call(request.clone());
    assert_eq!(rejected.exit_code, 2);
    assert!(rejected.stderr.contains("outside the disposable output"));
    fs::remove_file(h.path().join(nested_receipt)).unwrap();
    policy["historical"]["cohort"]["receipt"] = original_receipt;
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let result = call(request);
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        1
    );
    assert_eq!(
        call(json!({"operation":"release","enrollment":"cohort"})).exit_code,
        0
    );
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["removed_logical_bytes"], 2048);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["original_outcome"]["exit"],
        101
    );
    assert_eq!(fs::read(h.path().join("receipt.json")).unwrap(), raw);
}

#[test]
fn native_delivery_acknowledgement_releases_only_its_exact_original_and_keeps_failed_outcome() {
    use retained_artifacts::retention::{publish_original, reference_report, settle_report};
    let h = report_host();
    let path = ".chrono-harness/state/preparation/native-delivery.json";
    let raw = serde_json::to_vec(
        &json!({"request":{"request_id":"native-1"},"response":{"status":"failed"}}),
    )
    .unwrap();
    publish_original(h.path(), path, &raw, json!({"exit":1})).unwrap();
    let slot = ".chrono-harness/state/current.json";
    let slot_owner = reference_report(h.path(), slot, path, &[], Some("native-1")).unwrap();
    settle_report(h.path(), slot, path).unwrap();
    drop(slot_owner);
    let inventory = inventory(h.path());
    assert!(
        inventory
            .acknowledge_delivery(path, &"0".repeat(64), 55, &"a".repeat(64))
            .is_err()
    );
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-1")
            .is_some()
    );
    let receipt = inventory
        .acknowledge_delivery(path, &chrono_harness::sha256(&raw), 55, &"a".repeat(64))
        .unwrap();
    assert_eq!(receipt["original_outcome"]["exit"], 1);
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-1")
            .is_none()
    );
    assert!(
        ledger(h.path())["roots"]
            .as_object()
            .unwrap()
            .keys()
            .any(|key| key.starts_with("current-report:"))
    );
    assert!(h.path().join(path).exists());
}

#[test]
fn unselected_producer_keeps_legacy_publication_semantics() {
    let h = host(256, 16);
    let path = tree(h.path(), "unselected");
    assert!(
        retained_artifacts::retention::begin_adopted(
            h.path(),
            "lifecycle-original",
            &h.path().join(&path),
            json!(null)
        )
        .unwrap()
        .is_none()
    );
    assert!(h.path().join(path).exists());
}

#[test]
fn oversized_tree_completion_keeps_the_actual_failed_producer_outcome() {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);
    policy["max_total_bytes"] = json!(256);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = tree(h.path(), "oversized");
    let publication = inventory(h.path())
        .begin("fixture", &path, json!({"status":"unknown"}))
        .unwrap();
    fs::write(h.path().join(&path).join("failure"), [1; 256]).unwrap();
    assert!(
        publication
            .complete(json!({"exit":101,"error":"original producer error"}))
            .is_err()
    );
    assert_eq!(ledger(h.path())["outputs"][0]["outcome"]["exit"], 101);
    assert_eq!(
        ledger(h.path())["outputs"][0]["outcome"]["error"],
        "original producer error"
    );
    assert!(h.path().join(path).join("failure").exists());
}

#[test]
fn native_upload_cli_consumes_current_reference_and_rejects_pointer_digest_drift() {
    use retained_artifacts::retention::{publish_original, reference_report, settle_report};
    let h = report_host();
    let original = ".chrono-harness/state/preparation/native-cli.json";
    let slot = ".chrono-harness/state/check.json";
    let raw = serde_json::to_vec(
        &json!({"request":{"request_id":"native-cli"},"response":{"status":"failed"}}),
    )
    .unwrap();
    publish_original(h.path(), original, &raw, json!({"exit":1})).unwrap();
    let publisher = reference_report(h.path(), slot, original, &[], Some("native-cli")).unwrap();
    settle_report(h.path(), slot, original).unwrap();
    drop(publisher);
    fs::write(
        h.path().join(".chrono-harness/check.json"),
        serde_json::to_vec(&json!({"report_path":slot})).unwrap(),
    )
    .unwrap();
    let call = |scope| {
        std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
            .args([
                "dispatch",
                "retention",
                "--host-root",
                h.path().to_str().unwrap(),
                "--delivered-check",
                ".chrono-harness/check.json",
                "--scope",
                scope,
                "--artifact-id-env",
                "TEST_ARTIFACT_ID",
                "--artifact-digest-env",
                "TEST_ARTIFACT_DIGEST",
            ])
            .env("TEST_ARTIFACT_ID", "1234")
            .env("TEST_ARTIFACT_DIGEST", format!("sha256:{}", "a".repeat(64)))
            .output()
            .unwrap()
    };
    let pointer = |digest: String| {
        serde_json::to_vec(&chrono_harness::units::ReportReference {
            schema: chrono_harness::units::REPORT_REFERENCE.into(),
            original: chrono_harness::prepared::Original {
                path: original.into(),
                sha256: digest,
            },
        })
        .unwrap()
    };
    fs::write(h.path().join(slot), pointer("0".repeat(64))).unwrap();
    assert_eq!(call("all").status.code(), Some(2));
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-cli")
            .is_some()
    );
    fs::write(h.path().join(slot), pointer(chrono_harness::sha256(&raw))).unwrap();
    let result = call("all");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["original_outcome"]["exit"], 1);
    assert_eq!(receipt["artifact_id"], 1234);
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-cli")
            .is_none()
    );
    assert_eq!(fs::read(h.path().join(original)).unwrap(), raw);
    inventory(h.path())
        .reference_paths("native-delivery:native-cli", &[original.into()])
        .unwrap();
    fs::write(
        h.path().join(".chrono-harness/check.json"),
        serde_json::to_vec(&json!({"policy":{"units":{"alpha":{"report_path":slot}}}})).unwrap(),
    )
    .unwrap();
    let unit = call("alpha");
    assert_eq!(
        unit.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&unit.stderr)
    );
}

#[test]
fn staged_blob_checks_capacity_before_copy_and_preserves_the_input() {
    let h = report_host();
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);
    policy["max_total_bytes"] = json!(256);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let source = h.path().join("original.bin");
    fs::write(&source, [7; 129]).unwrap();
    let dir = ".chrono-harness/state/preparation/";
    assert!(
        retained_artifacts::stage(h.path(), &source, dir)
            .unwrap_err()
            .contains("before writing")
    );
    assert_eq!(fs::read(&source).unwrap(), [7; 129]);
    assert_eq!(
        fs::read_dir(h.path().join(format!("{dir}blobs")))
            .unwrap()
            .count(),
        0
    );
    fs::write(&source, [7; 128]).unwrap();
    let descriptor = retained_artifacts::stage(h.path(), &source, dir).unwrap();
    assert_eq!(descriptor["length"], 128);
    let path = descriptor["storage"].as_str().unwrap();
    assert_eq!(fs::read(h.path().join(path)).unwrap(), [7; 128]);
    assert_eq!(ledger(h.path())["outputs"][0]["reserved_bytes"], 128);
    assert_eq!(
        retained_artifacts::stage(h.path(), &source, dir).unwrap(),
        descriptor
    );
    assert_eq!(ledger(h.path())["outputs"].as_array().unwrap().len(), 1);
}

#[test]
fn interrupted_lifecycle_original_stays_rooted_for_its_actual_recovery_reader() {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["producers"]["lifecycle-original"] = json!([".chrono-harness/state/fixtures/"]);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = ".chrono-harness/state/fixtures/result.json";
    fs::write(h.path().join(path), b"partial original").unwrap();
    let publication = retained_artifacts::retention::begin_adopted(
        h.path(),
        "lifecycle-original",
        &h.path().join(path),
        json!({"operation":"start","outcome":"not-established"}),
    )
    .unwrap()
    .unwrap();
    drop(publication);
    inventory(h.path()).maintain_at(u64::MAX).unwrap();
    assert_eq!(fs::read(h.path().join(path)).unwrap(), b"partial original");
    let reader = retained_artifacts::retention::read_guard(h.path(), path)
        .unwrap()
        .unwrap();
    inventory(h.path())
        .reference_paths(&format!("recovery:{path}"), &[])
        .unwrap();
    assert_eq!(
        inventory(h.path()).maintain_at(u64::MAX).unwrap()["remaining_outputs"],
        1
    );
    drop(reader);
    let final_round = inventory(h.path()).maintain_at(u64::MAX).unwrap();
    assert_eq!(final_round["remaining_outputs"], 0, "{final_round}");
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["completion"],
        "not-established"
    );
}

#[test]
fn explicit_small_reservations_admit_concurrent_outputs_and_enforce_each_bound() {
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(1024);
    policy["max_total_bytes"] = json!(2048);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let mut live = Vec::new();
    for n in 0..8 {
        let path = h
            .path()
            .join(format!(".chrono-harness/state/fixtures/small-{n}"));
        let p = retained_artifacts::retention::create_adopted_with_limit(
            h.path(),
            "fixture",
            &path,
            true,
            json!({"operation":n}),
            Some(256),
        )
        .unwrap()
        .unwrap();
        p.write_file(&path.join("data"), &[1; 128]).unwrap();
        assert!(p.replace_file(&path.join("data"), &[2; 129]).is_err());
        assert_eq!(fs::read(path.join("data")).unwrap(), [1; 128]);
        live.push(p);
    }
    let refused = h.path().join(".chrono-harness/state/fixtures/ninth");
    assert!(
        retained_artifacts::retention::create_adopted_with_limit(
            h.path(),
            "fixture",
            &refused,
            true,
            json!(null),
            Some(1),
        )
        .err()
        .unwrap()
        .contains("total retained byte capacity")
    );
    assert!(!refused.exists());
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        8
    );
    for (n, p) in live.into_iter().enumerate() {
        p.complete(json!({"exit":n})).unwrap();
    }
    let mut remaining = 8;
    for _ in 0..8 {
        let round = inventory(h.path()).maintain().unwrap();
        assert!(round["nodes"].as_u64().unwrap() <= 256);
        remaining = round["remaining_outputs"].as_u64().unwrap();
        if remaining == 0 {
            break;
        }
    }
    assert_eq!(remaining, 0);
    assert_eq!(ledger(h.path())["summaries"].as_array().unwrap().len(), 8);
}
#[test]
fn ordinary_maintenance_defers_busy_exclusion_without_spending_another_round() {
    let h = host(256, 16);
    let i = inventory(h.path());
    let gate = chrono_harness::ownership::Lease::acquire(
        &h.path()
            .join(".chrono-harness/state/output-retention-v1/admission.lease"),
        None,
        true,
        true,
        None,
    )
    .unwrap()
    .unwrap();
    let began = std::time::Instant::now();
    assert_eq!(
        i.maintain_available().unwrap()["effects"],
        "maintenance-deferred"
    );
    assert!(began.elapsed() < std::time::Duration::from_secs(1));
    drop(gate);
    assert_eq!(i.maintain_available().unwrap()["remaining_outputs"], 0);
}

#[test]
fn ordinary_concurrent_publications_admit_against_retained_inventory() {
    let h = host(256, 128);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["keep_seconds"] = json!(604800);
    policy["max_entries"] = json!(512);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    // Real completed producers supply the retained history; no fabricated ledger
    // state or enlarged admission/maintenance limit supplies the test's capacity.
    for n in 0..420 {
        let path = tree(h.path(), &format!("retained-{n}"));
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        publication
            .complete(json!({"exit":0,"original":"x".repeat(1024)}))
            .unwrap();
    }
    let start = std::sync::Barrier::new(28);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..28)
            .map(|n| {
                let start = &start;
                let root = h.path();
                scope.spawn(move || {
                    start.wait();
                    let path = root.join(format!(".chrono-harness/state/fixtures/current-{n}"));
                    let publication = retained_artifacts::retention::create_adopted_with_limit(
                        root,
                        "fixture",
                        &path,
                        true,
                        json!({"operation":n}),
                        Some(32 * 1024 * 1024),
                    )?
                    .ok_or("adoption missing")?;
                    publication.write_file(&path.join("original"), b"required original")?;
                    publication.complete(json!({"exit":0}))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    if results.iter().any(Result::is_err) {
        let original = h.keep();
        panic!("{results:?}; original fixture {}", original.display());
    }
    assert_eq!(ledger(h.path())["outputs"].as_array().unwrap().len(), 448);
    for n in 0..28 {
        assert_eq!(
            fs::read(h.path().join(format!(
                ".chrono-harness/state/fixtures/current-{n}/original"
            )))
            .unwrap(),
            b"required original"
        );
    }
}
