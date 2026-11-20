#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
pub fn fixture_recovery_has_room(completed_rounds: usize) -> (room: bool)
    ensures room == (completed_rounds < 8),
{
    completed_rounds < 8
}

// 单报告 fixture 的首轮必须含原件，后续轮次按轮前持久引用精确计数。
pub fn report_component_inventory_matches(completed_rounds: usize,
    current_references: Option<usize>, observed: Option<u64>) -> (matches: bool)
    ensures matches == (completed_rounds < 4
        && ((current_references == Some(1usize) && observed == Some(2u64))
            || (completed_rounds > 0 && current_references == Some(0usize)
                && observed == Some(1u64)))),
{
    completed_rounds < 4
        && ((current_references == Some(1usize) && observed == Some(2u64))
            || (completed_rounds > 0 && current_references == Some(0usize)
                && observed == Some(1u64)))
}

// 原件还有独立入向引用时，只在本轮仍读取它的情况下报告未知剩余量。
pub fn shared_original_scope_matches(current_references: Option<usize>, retired: bool,
    complete: Option<bool>, remaining_is_null: bool, remaining: Option<u64>) -> (matches: bool)
    ensures matches == ((current_references == Some(1usize) && complete == Some(false)
        && remaining_is_null && remaining == None::<u64>)
        || (current_references == Some(0usize) && complete == Some(true)
            && !remaining_is_null && remaining == Some(if retired { 0u64 } else { 1u64 }))),
{
    (current_references == Some(1usize) && complete == Some(false)
        && remaining_is_null && remaining == None::<u64>)
        || (current_references == Some(0usize) && complete == Some(true)
            && !remaining_is_null && remaining == Some(if retired { 0u64 } else { 1u64 }))
}

fn expect_component(actual: bool) requires actual, {
    if !actual { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn component_inventory_requires_the_current_edge_and_original_four_round_limit() {
    expect_component(report_component_inventory_matches(0, Some(1), Some(2)));
    expect_component(!report_component_inventory_matches(0, Some(0), Some(1)));
    expect_component(report_component_inventory_matches(1, Some(1), Some(2)));
    expect_component(report_component_inventory_matches(1, Some(0), Some(1)));
    expect_component(report_component_inventory_matches(2, Some(0), Some(1)));
    expect_component(report_component_inventory_matches(3, Some(1), Some(2)));
    expect_component(report_component_inventory_matches(3, Some(0), Some(1)));
    expect_component(!report_component_inventory_matches(4, Some(1), Some(2)));
    expect_component(!report_component_inventory_matches(4, Some(0), Some(1)));
    expect_component(!report_component_inventory_matches(usize::MAX, Some(0), Some(1)));
    expect_component(!report_component_inventory_matches(1, Some(1), Some(1)));
    expect_component(!report_component_inventory_matches(1, Some(0), Some(2)));
    expect_component(!report_component_inventory_matches(1, Some(0), Some(0)));
    expect_component(!report_component_inventory_matches(1, Some(1), None));
    expect_component(!report_component_inventory_matches(1, None, Some(1)));
    expect_component(!report_component_inventory_matches(1, None, None));
    expect_component(!report_component_inventory_matches(1, Some(2), Some(3)));
    expect_component(!report_component_inventory_matches(1, Some(usize::MAX), Some(1)));
    expect_component(!report_component_inventory_matches(1, Some(1), Some(u64::MAX)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn component_scope_preserves_unknown_and_counts_only_the_remaining_report() {
    expect_component(shared_original_scope_matches(Some(1), false, Some(false), true, None));
    expect_component(shared_original_scope_matches(Some(1), true, Some(false), true, None));
    expect_component(!shared_original_scope_matches(Some(1), false, Some(true), true, None));
    expect_component(!shared_original_scope_matches(Some(1), false, Some(false), false, None));
    expect_component(!shared_original_scope_matches(Some(1), false, Some(false), true, Some(0)));
    expect_component(shared_original_scope_matches(Some(0), false, Some(true), false, Some(1)));
    expect_component(shared_original_scope_matches(Some(0), true, Some(true), false, Some(0)));
    expect_component(!shared_original_scope_matches(Some(0), false, Some(true), false, Some(0)));
    expect_component(!shared_original_scope_matches(Some(0), true, Some(true), false, Some(1)));
    expect_component(!shared_original_scope_matches(Some(0), true, Some(false), true, None));
    expect_component(!shared_original_scope_matches(Some(0), true, None, false, Some(0)));
    expect_component(!shared_original_scope_matches(Some(0), true, Some(true), true, Some(0)));
    expect_component(!shared_original_scope_matches(None, true, Some(true), false, Some(0)));
    expect_component(!shared_original_scope_matches(Some(2), true, Some(true), false, Some(0)));
    expect_component(!shared_original_scope_matches(Some(0), true, Some(true), false, Some(u64::MAX)));
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::{
    self,
    retention::{Inventory, POLICY_PATH},
};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path};
#[path = "support/isolated_capability.rs"]
#[cfg(not(verus_keep_ghost))]
mod isolated_capability;
#[cfg(not(verus_keep_ghost))]
use isolated_capability::isolated_capability_test;
#[path = "support/git_fixture.rs"]
#[cfg(not(verus_keep_ghost))]
mod git_fixture;
#[path = "support/native_executable.rs"]
#[cfg(not(verus_keep_ghost))]
mod native_executable;
#[cfg(not(verus_keep_ghost))]
fn host(nodes: usize, summaries: usize) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join(".chrono-harness/state/fixtures")).unwrap();
    fs::write(d.path().join(POLICY_PATH),serde_json::to_vec(&json!({"schema":"chrono-output-retention/v2","producers":{"fixture":[".chrono-harness/state/fixtures/"]},"keep_seconds":0,"max_related_outputs":32,"max_summaries":summaries,"max_nodes_per_round":nodes,"max_bytes_per_round":1048576,"max_millis_per_round":1000})).unwrap()).unwrap();
    d
}
#[cfg(not(verus_keep_ghost))]
fn inventory(root: &Path) -> Inventory {
    Inventory::adopted(root).unwrap().unwrap()
}

#[cfg(not(verus_keep_ghost))]
fn deadline_host() -> tempfile::TempDir {
    let h = host(256, 16);
    let path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    policy["max_lifetime_seconds"] = json!(60);
    policy["keep_seconds"] = json!(3600);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    h
}

#[test]
fn creation_deadline_survives_diagnostics_completion_and_consumer_handoffs() {
    if !isolated_capability_test(
        "creation_deadline_survives_diagnostics_completion_and_consumer_handoffs",
    ) {
        return;
    }
    let h = deadline_host();
    let path = ".chrono-harness/state/fixtures/deadline";
    let publication = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &h.path().join(path),
        false,
        json!({"completion":"pending"}),
    )
    .unwrap()
    .unwrap();
    fs::write(h.path().join(path), b"valuable original").unwrap();
    let initial = ledger(h.path())["outputs"][0].clone();
    let lifetime = initial["lifetime"].clone();
    let born = lifetime["created_at"].as_u64().unwrap();
    let deadline = lifetime["deadline"].as_u64().unwrap();
    assert_eq!(deadline - born, 60);
    assert_eq!(initial["expires"], deadline);
    let inv = inventory(h.path());
    inv.retain_diagnostic_path(path).unwrap();
    publication
        .complete_sealed(json!({"exit_code":17}))
        .unwrap();
    inv.reference_paths("actual-reader", &[path.into()])
        .unwrap();
    inv.retain_diagnostic_path(path).unwrap();
    let after = ledger(h.path())["outputs"][0].clone();
    assert_eq!(after["lifetime"], lifetime);
    assert_eq!(after["expires"], deadline);
    assert_eq!(after["outcome"]["exit_code"], 17);
}

#[test]
fn expired_original_rejects_new_readers_and_edges_but_allows_exact_release() {
    if !isolated_capability_test(
        "expired_original_rejects_new_readers_and_edges_but_allows_exact_release",
    ) {
        return;
    }
    let h = deadline_host();
    let path = ".chrono-harness/state/fixtures/expired";
    fs::write(h.path().join(path), b"original awaiting transfer").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit_code":0})).unwrap();
    let inv = inventory(h.path());
    inv.reference("existing-owner", std::slice::from_ref(&id))
        .unwrap();
    let mut output = ledger(h.path())["outputs"][0].clone();
    output["lifetime"] = json!({"created_at":1,"deadline":2});
    output["expires"] = json!(2);
    retention_state::replace_output(h.path(), &output);
    let retained = inv
        .maintain_originals_at(std::slice::from_ref(&id), 3)
        .unwrap();
    assert!(
        h.path().join(path).exists(),
        "expiry must not invent a release"
    );
    assert!(
        retained["protected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == id && entry["deadline_missed"] == true),
        "{retained}"
    );
    assert!(inv.consume(&id).err().unwrap().contains("deadline"));
    assert!(retained_artifacts::retention::read_guard(h.path(), path).is_err());
    assert!(
        inv.reference("new-reader", std::slice::from_ref(&id))
            .unwrap_err()
            .contains("deadline")
    );
    assert!(
        inv.reference_paths("new-path-reader", &[path.into()])
            .unwrap_err()
            .contains("deadline")
    );
    assert!(
        inv.retain_diagnostic_path(path)
            .unwrap_err()
            .contains("deadline")
    );
    let report_path = ".chrono-harness/state/fixtures/report";
    fs::write(h.path().join(report_path), b"new report").unwrap();
    let report = inventory(h.path())
        .begin("fixture", report_path, json!({}))
        .unwrap();
    inv.reference("existing-owner", &[id.clone(), report.id().into()])
        .unwrap();
    inv.reference("existing-owner", std::slice::from_ref(&id))
        .unwrap();
    assert!(
        inv.dependencies(report.id(), std::slice::from_ref(&id))
            .unwrap_err()
            .contains("deadline")
    );
    assert!(
        inv.dependencies_paths(report_path, &[path.into()])
            .unwrap_err()
            .contains("deadline")
    );
    drop(report);
    // Expiry reports a breach; it does not forge a consumer release or ignore
    // a live capability. The actual owner still performs its exact release.
    inv.reference("existing-owner", &[]).unwrap();
    let round = inv
        .maintain_originals_at(std::slice::from_ref(&id), 3)
        .unwrap();
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert!(!h.path().join(path).exists(), "{round}");
}

#[test]
fn output_metadata_retires_within_the_original_creation_window() {
    if !isolated_capability_test("output_metadata_retires_within_the_original_creation_window") {
        return;
    }
    let h = deadline_host();
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(86400);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let path = ".chrono-harness/state/fixtures/metadata-window";
    let physical = h.path().join(path);
    fs::write(&physical, b"short-lived original").unwrap();
    let owner = inventory(h.path());
    let publication = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit_code":0})).unwrap();
    let deadline = ledger(h.path())["outputs"][0]["lifetime"]["deadline"]
        .as_u64()
        .unwrap();
    let metadata = retention_state::object_path(h.path(), &id);

    let at_birth: Value = serde_json::from_slice(&fs::read(&metadata).unwrap()).unwrap();
    assert!(
        at_birth["metadata_expires"].as_u64().unwrap() <= deadline,
        "object metadata exceeded the creation window before retirement"
    );

    let active = owner.consume(&id).unwrap();
    let protected = owner
        .maintain_originals_at(std::slice::from_ref(&id), deadline)
        .unwrap();
    assert!(protected.get("retired_object_metadata").is_none());
    assert!(physical.is_file());
    assert!(metadata.is_file());
    drop(active);
    for _ in 0..8 {
        if !metadata.exists() {
            break;
        }
        let round = owner
            .maintain_originals_at(std::slice::from_ref(&id), deadline)
            .unwrap();
        if metadata.parent().unwrap().exists() {
            assert!(round.get("retired_object_metadata").is_none(), "{round}");
        } else {
            assert_eq!(round["retired_object_metadata"], json!([id]), "{round}");
        }
    }
    assert!(!physical.exists());
    assert!(
        !metadata.parent().unwrap().exists(),
        "retirement renewed output metadata after its original deadline"
    );
}

#[test]
fn longer_policy_keeps_the_short_object_metadata_deadline_at_retirement() {
    if !isolated_capability_test(
        "longer_policy_keeps_the_short_object_metadata_deadline_at_retirement",
    ) {
        return;
    }
    let h = deadline_host();
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(0);
    policy["keep_seconds"] = json!(0);
    let previous = serde_json::to_vec(&policy).unwrap();
    fs::write(&policy_path, &previous).unwrap();
    let path = ".chrono-harness/state/fixtures/short-object-metadata";
    let physical = h.path().join(path);
    fs::write(&physical, b"releasable original").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit_code":0})).unwrap();
    let metadata = retention_state::object_path(h.path(), &id);
    let record: Value = serde_json::from_slice(&fs::read(&metadata).unwrap()).unwrap();
    let observed = record["metadata_expires"]
        .as_u64()
        .unwrap()
        .max(record["output"]["expires"].as_u64().unwrap());

    policy["metadata_keep_seconds"] = json!(86400);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(h.path());
    owner
        .migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    for _ in 0..8 {
        if !metadata.exists() {
            break;
        }
        owner
            .maintain_originals_at(std::slice::from_ref(&id), observed)
            .unwrap();
    }
    assert!(!physical.exists());
    assert!(
        !metadata.parent().unwrap().exists(),
        "metadata window was extended"
    );
}

#[test]
fn lengthening_policy_does_not_extend_an_empty_path_address() {
    if !isolated_capability_test("lengthening_policy_does_not_extend_an_empty_path_address") {
        return;
    }
    let h = deadline_host();
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(0);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let path = "exports/stale";
    let guard = retained_artifacts::retention::external_removal_guard(h.path(), path)
        .unwrap()
        .unwrap();
    let address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths")
        .join(chrono_harness::sha256(b"exports"))
        .join(chrono_harness::sha256(b"stale"));

    inventory(h.path()).maintain_explicit().unwrap();
    guard.stable().unwrap();
    assert!(address.join("address.json").is_file());
    drop(guard);

    policy["metadata_keep_seconds"] = json!(86400);
    policy["max_lifetime_seconds"] = json!(86400);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let legacy = legacy_empty_address(h.path(), "previous/stale");
    let result = inventory(h.path()).maintain_explicit().unwrap();
    assert_eq!(result["observations"]["accounting_errors"], 0, "{result}");
    assert!(legacy.join("address.json").is_file());
    assert!(
        !address.exists(),
        "the new policy extended the original address deadline"
    );
}

#[test]
fn historical_object_metadata_uses_its_validated_directory_birth_after_body_retirement() {
    if !isolated_capability_test(
        "historical_object_metadata_uses_its_validated_directory_birth_after_body_retirement",
    ) {
        return;
    }
    let h = deadline_host();
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["keep_seconds"] = json!(0);
    policy["metadata_keep_seconds"] = json!(86400);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let path = ".chrono-harness/state/fixtures/legacy-object-metadata";
    fs::write(h.path().join(path), b"released").unwrap();
    let owner = inventory(h.path());
    let publication = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit_code":0})).unwrap();
    let record = retention_state::object_path(h.path(), &id);
    let directory = record.parent().unwrap();
    let published: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    let body_expiry = published["output"]["expires"].as_u64().unwrap();
    assert!(body_expiry < published["metadata_expires"].as_u64().unwrap());
    let birth = fs::metadata(directory)
        .unwrap()
        .created()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    for _ in 0..8 {
        owner
            .maintain_originals_at(std::slice::from_ref(&id), body_expiry)
            .unwrap();
        if !h.path().join(path).exists() {
            break;
        }
    }
    let mut retired: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert!(retired["output"].is_null());
    assert!(!h.path().join(path).exists());
    retired["metadata_expires"] = json!(birth + 86400);
    fs::write(&record, serde_json::to_vec(&retired).unwrap()).unwrap();
    owner
        .maintain_originals_at(std::slice::from_ref(&id), birth + 59)
        .unwrap();
    let bounded: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(bounded["metadata_expires"], birth + 60);
    let previous = fs::read(&policy_path).unwrap();
    policy["max_lifetime_seconds"] = json!(3600);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(h.path());
    owner
        .migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    owner
        .maintain_originals_at(std::slice::from_ref(&id), birth + 60)
        .unwrap();
    assert!(!directory.exists());
}

#[test]
fn empty_historical_role_keeps_the_window_of_its_directory_generation() {
    if !isolated_capability_test(
        "empty_historical_role_keeps_the_window_of_its_directory_generation",
    ) {
        return;
    }
    let h = deadline_host();
    let owner = inventory(h.path());
    let name = "consumer:previous-maintenance-program";
    owner.reference(name, &[]).unwrap();
    let record = retention_state::role_path(h.path(), name);
    let directory = record.parent().unwrap();
    let birth = fs::metadata(directory)
        .unwrap()
        .created()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut role: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    role["metadata_expires"] = json!(birth + 86400);
    fs::write(&record, serde_json::to_vec(&role).unwrap()).unwrap();

    owner.maintain_at(birth + 59).unwrap();
    let bounded: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(bounded["metadata_expires"], birth + 60);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_lifetime_seconds"] = json!(3600);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    inventory(h.path()).maintain_at(birth + 60).unwrap();
    assert!(!directory.exists());
}

#[test]
fn consumer_metadata_keeps_its_creation_deadline_until_release() {
    if !isolated_capability_test("consumer_metadata_keeps_its_creation_deadline_until_release") {
        return;
    }
    let h = deadline_host();
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_lifetime_seconds"] = json!(3600);
    policy["metadata_keep_seconds"] = json!(86400);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();

    let path = ".chrono-harness/state/fixtures/consumer-metadata-window";
    let physical = h.path().join(path);
    fs::write(&physical, b"origin with a consumer").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit_code":0})).unwrap();
    let owner = inventory(h.path());
    let consumer = "fixture-consumer-with-deadline";
    owner.reference(consumer, &[id]).unwrap();
    let record = retention_state::role_path(h.path(), consumer);
    let metadata_deadline = || {
        serde_json::from_slice::<Value>(&fs::read(&record).unwrap()).unwrap()["metadata_expires"]
            .as_u64()
            .unwrap()
    };
    let deadline = metadata_deadline();
    let observed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        deadline <= observed + 3600,
        "consumer exceeded its creation window"
    );
    owner.maintain_at(deadline).unwrap();
    assert!(record.is_file());
    assert!(physical.is_file());

    owner.reference(consumer, &[]).unwrap();
    assert_eq!(metadata_deadline(), deadline);
    for _ in 0..8 {
        if !record.exists() {
            break;
        }
        owner.maintain_at(deadline).unwrap();
    }
    assert!(!record.parent().unwrap().exists());
}

#[test]
fn existing_consumer_can_transfer_expired_originals_without_new_report_dependencies() {
    if !isolated_capability_test(
        "existing_consumer_can_transfer_expired_originals_without_new_report_dependencies",
    ) {
        return;
    }
    let h = deadline_host();
    let leaf = ".chrono-harness/state/fixtures/owned-leaf";
    let parent = ".chrono-harness/state/fixtures/owned-parent";
    let mut ids = Vec::new();
    for path in [leaf, parent] {
        fs::write(h.path().join(path), path.as_bytes()).unwrap();
        let output = inventory(h.path())
            .begin("fixture", path, json!({}))
            .unwrap();
        ids.push(output.id().to_owned());
        output.complete_sealed(json!({"exit_code":0})).unwrap();
    }
    let inv = inventory(h.path());
    inv.dependencies(&ids[1], &ids[..1]).unwrap();
    inv.reference("original-owner", &ids[1..]).unwrap();
    inv.reference("different-owner", &[]).unwrap();
    for mut output in ledger(h.path())["outputs"].as_array().unwrap().clone() {
        output["lifetime"] = json!({"created_at":1,"deadline":2});
        output["expires"] = json!(2);
        retention_state::replace_output(h.path(), &output);
    }
    let report_path = ".chrono-harness/state/fixtures/new-report";
    let report = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &h.path().join(report_path),
        false,
        json!({}),
    )
    .unwrap()
    .unwrap();
    fs::write(h.path().join(report_path), b"new observation").unwrap();
    let scope = retained_artifacts::retention::publication_read_scope(&report).unwrap();
    let before = ledger(h.path());
    for path in [leaf, parent] {
        assert!(retained_artifacts::retention::read_guard(h.path(), path).is_err());
        assert!(
            retained_artifacts::retention::consumer_read_guard(h.path(), path, "different-owner")
                .is_err()
        );
        assert!(
            retained_artifacts::retention::consumer_read_guard(h.path(), path, "missing-owner")
                .is_err()
        );
        let guard =
            retained_artifacts::retention::consumer_read_guard(h.path(), path, "original-owner")
                .unwrap()
                .unwrap();
        assert_eq!(fs::read(h.path().join(path)).unwrap(), path.as_bytes());
        drop(guard);
    }
    assert_eq!(
        ledger(h.path()),
        before,
        "recovery reads cannot renew any persistent retention state"
    );
    drop(scope);
    report.complete_sealed(json!({"exit_code":0})).unwrap();

    let guard =
        retained_artifacts::retention::consumer_read_guard(h.path(), leaf, "original-owner")
            .unwrap()
            .unwrap();
    inv.reference("original-owner", &[]).unwrap();
    inv.maintain_originals_at(&ids, 3).unwrap();
    assert!(
        h.path().join(leaf).exists(),
        "the returned capability still excludes disposal"
    );
    drop(guard);
    let mut rounds = Vec::new();
    for _ in 0..8 {
        let remaining: Vec<_> = ids
            .iter()
            .filter(|id| retention_state::object_path(h.path(), id).is_file())
            .cloned()
            .collect();
        if remaining.is_empty() {
            break;
        }
        rounds.push(inv.maintain_originals_at(&remaining, 3).unwrap());
    }
    assert!(
        !h.path().join(leaf).exists() && !h.path().join(parent).exists(),
        "{rounds:?}"
    );
}

#[test]
fn existing_consumer_read_rejects_stale_incoming_edges_and_unsealed_originals() {
    if !isolated_capability_test(
        "existing_consumer_read_rejects_stale_incoming_edges_and_unsealed_originals",
    ) {
        return;
    }
    let h = deadline_host();
    let mut ids = Vec::new();
    for path in [
        ".chrono-harness/state/fixtures/unowned",
        ".chrono-harness/state/fixtures/owned",
    ] {
        fs::write(h.path().join(path), path.as_bytes()).unwrap();
        let output = inventory(h.path())
            .begin("fixture", path, json!({}))
            .unwrap();
        ids.push(output.id().to_owned());
        output.complete_sealed(json!({"exit_code":0})).unwrap();
    }
    let inv = inventory(h.path());
    inv.reference("original-owner", &ids[1..]).unwrap();
    for mut output in ledger(h.path())["outputs"].as_array().unwrap().clone() {
        output["lifetime"] = json!({"created_at":1,"deadline":2});
        output["expires"] = json!(2);
        retention_state::replace_output(h.path(), &output);
    }
    let path = retention_state::object_path(h.path(), &ids[0]);
    let mut record: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    record["incoming"] = json!([ids[1]]);
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    let error = retained_artifacts::retention::consumer_read_guard(
        h.path(),
        ".chrono-harness/state/fixtures/unowned",
        "original-owner",
    )
    .err()
    .unwrap();
    assert!(error.contains("not protected"), "{error}");

    let mut output = ledger(h.path())["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["id"] == ids[1])
        .unwrap()
        .clone();
    output["phase"] = json!("publishing");
    output["manifest"]["sealed"] = json!(false);
    retention_state::replace_output(h.path(), &output);
    let error = retained_artifacts::retention::consumer_read_guard(
        h.path(),
        ".chrono-harness/state/fixtures/owned",
        "original-owner",
    )
    .err()
    .unwrap();
    assert!(error.contains("not a sealed released output"), "{error}");
}

#[test]
fn consumer_recovery_rejects_an_original_replaced_with_identical_bytes() {
    if !isolated_capability_test(
        "consumer_recovery_rejects_an_original_replaced_with_identical_bytes",
    ) {
        return;
    }
    let h = deadline_host();
    let path = ".chrono-harness/state/fixtures/replaced";
    fs::write(h.path().join(path), b"same bytes").unwrap();
    let output = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = output.id().to_owned();
    output.complete_sealed(json!({"exit_code":0})).unwrap();
    inventory(h.path())
        .reference("original-owner", &[id])
        .unwrap();
    let mut output = ledger(h.path())["outputs"][0].clone();
    output["lifetime"] = json!({"created_at":1,"deadline":2});
    output["expires"] = json!(2);
    retention_state::replace_output(h.path(), &output);
    fs::rename(h.path().join(path), h.path().join("saved-original")).unwrap();
    fs::write(h.path().join(path), b"same bytes").unwrap();
    assert!(
        retained_artifacts::retention::consumer_read_guard(h.path(), path, "original-owner")
            .err()
            .unwrap()
            .contains("identity changed")
    );
}

#[test]
fn consumer_recovery_requires_one_simultaneously_valid_reference_path() {
    if !isolated_capability_test(
        "consumer_recovery_requires_one_simultaneously_valid_reference_path",
    ) {
        return;
    }
    let h = deadline_host();
    let paths = [
        ".chrono-harness/state/fixtures/leaf",
        ".chrono-harness/state/fixtures/a",
        ".chrono-harness/state/fixtures/b",
        ".chrono-harness/state/fixtures/root",
    ];
    let inv = inventory(h.path());
    let mut ids = Vec::new();
    for path in paths {
        fs::write(h.path().join(path), path.as_bytes()).unwrap();
        let output = inventory(h.path())
            .begin("fixture", path, json!({}))
            .unwrap();
        ids.push(output.id().to_owned());
        output.complete_sealed(json!({"exit_code":0})).unwrap();
    }
    inv.dependencies_paths(paths[1], &[paths[0].into()])
        .unwrap();
    inv.dependencies_paths(paths[2], &[paths[1].into()])
        .unwrap();
    inv.reference("original-owner", &ids[3..]).unwrap();
    let before = ledger(h.path());
    for (source, target) in [(1, 0), (2, 1)] {
        assert_eq!(
            before["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|output| output["id"] == ids[source])
                .unwrap()["references"],
            json!([ids[target]]),
            "the fixture must install its actual reference edges"
        );
    }
    let mut leaf = ledger(h.path())["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["id"] == ids[0])
        .unwrap()
        .clone();
    leaf["lifetime"] = json!({"created_at":1,"deadline":2});
    leaf["expires"] = json!(2);
    retention_state::replace_output(h.path(), &leaf);

    let mut changed = false;
    let read = retained_artifacts::retention::consumer_read_observed(
        h.path(),
        paths[0],
        "original-owner",
        |id| {
            if id == ids[1] {
                assert!(!changed);
                // Break A -> leaf before connecting root -> B. At no point
                // does the owner have a complete path to the expired leaf.
                inv.dependencies_paths(paths[1], &[]).unwrap();
                inv.dependencies_paths(paths[3], &[paths[2].into()])
                    .unwrap();
                changed = true;
            }
        },
    );
    assert!(
        changed,
        "the interleaving did not execute: {:?}",
        read.as_ref().err()
    );
    let error = read
        .err()
        .expect("observations from different graph states cannot authorize recovery");
    assert!(error.contains("reference path changed"), "{error}");
    assert_eq!(
        ledger(h.path())["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|output| output["id"] == ids[0])
            .unwrap()["lifetime"],
        leaf["lifetime"]
    );
}

#[test]
fn explicit_lifetime_adoption_uses_physical_birth_and_never_renews_it() {
    if !isolated_capability_test(
        "explicit_lifetime_adoption_uses_physical_birth_and_never_renews_it",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/old-original";
    fs::write(h.path().join(path), b"preserved old output").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit_code":0})).unwrap();
    let policy_path = h.path().join(POLICY_PATH);
    let previous = fs::read(&policy_path).unwrap();
    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["max_lifetime_seconds"] = json!(60);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let inv = inventory(h.path());
    assert!(
        inv.consume(&id).is_err(),
        "ordinary entry must not invent a historical birth"
    );
    inv.migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    let migrated = ledger(h.path())["outputs"][0].clone();
    let physical_birth = fs::metadata(h.path().join(path))
        .unwrap()
        .created()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(migrated["lifetime"]["created_at"], physical_birth);
    assert_eq!(migrated["lifetime"]["deadline"], physical_birth + 60);
    assert_eq!(migrated["outcome"]["exit_code"], 0);
    let previous = fs::read(&policy_path).unwrap();
    policy["max_lifetime_seconds"] = json!(120);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    inventory(h.path())
        .migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    assert_eq!(
        ledger(h.path())["outputs"][0]["lifetime"],
        migrated["lifetime"]
    );
    assert_eq!(
        fs::read(h.path().join(path)).unwrap(),
        b"preserved old output"
    );
}

#[test]
fn terminal_policy_migration_uses_existing_expiry_after_original_is_absent() {
    if !isolated_capability_test(
        "terminal_policy_migration_uses_existing_expiry_after_original_is_absent",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/absent-terminal";
    fs::write(h.path().join(path), b"already disposed").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!({"exit": 0}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete_sealed(json!({"exit": 0})).unwrap();
    let mut output = ledger(h.path())["outputs"][0].clone();
    output["phase"] = json!("disposing");
    let expiry = output["expires"].as_u64().unwrap();
    retention_state::replace_output(h.path(), &output);
    fs::remove_file(h.path().join(path)).unwrap();

    let policy_path = h.path().join(POLICY_PATH);
    let previous = fs::read(&policy_path).unwrap();
    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["max_lifetime_seconds"] = json!(60);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();

    let migration = inventory(h.path())
        .migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    assert_eq!(migration["completion"], "not-established");
    let migrated = ledger(h.path())["outputs"][0].clone();
    assert_eq!(migrated["lifetime"]["deadline"], expiry);
    assert!(migrated["lifetime"]["created_at"].is_null());
    assert_eq!(migrated["expires"], expiry);

    let round = inventory(h.path())
        .maintain_originals_at(std::slice::from_ref(&id), u64::MAX)
        .unwrap();
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert!(
        !retention_state::object_path(h.path(), &id).exists() || output_retired(h.path(), &id),
        "{round}"
    );
}

#[test]
fn absent_terminal_migration_preserves_unknown_roots_and_excludes_readers() {
    if !isolated_capability_test(
        "absent_terminal_migration_preserves_unknown_roots_and_excludes_readers",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/unknown-absent";
    fs::write(h.path().join(path), b"unknown producer result").unwrap();
    let publication = inventory(h.path())
        .begin_rooted(
            "fixture",
            path,
            json!({"completion":"not-established"}),
            &["actual-recovery".into()],
        )
        .unwrap();
    let id = publication.id().to_owned();
    drop(publication);
    let mut output = ledger(h.path())["outputs"][0].clone();
    output["phase"] = json!("sealing");
    retention_state::replace_output(h.path(), &output);
    let reader = inventory(h.path()).consume(&id).unwrap();
    fs::remove_file(h.path().join(path)).unwrap();
    let before = ledger(h.path());
    let policy_path = h.path().join(POLICY_PATH);
    let previous = fs::read(&policy_path).unwrap();
    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["max_lifetime_seconds"] = json!(60);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    assert!(
        inventory(h.path())
            .migrate_policy(&previous, Some(std::slice::from_ref(&id)))
            .is_err()
    );
    assert_eq!(ledger(h.path())["outputs"], before["outputs"]);
    drop(reader);
    let inv = inventory(h.path());
    inv.migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    let migrated = ledger(h.path());
    assert_eq!(migrated["roots"], before["roots"]);
    assert_eq!(
        migrated["outputs"][0]["outcome"],
        before["outputs"][0]["outcome"]
    );
    assert_eq!(migrated["outputs"][0]["expires"], output["expires"]);
    assert_eq!(migrated["outputs"][0]["phase"], "sealing");
    assert!(migrated["outputs"][0]["lifetime"]["created_at"].is_null());
    assert!(inv.consume(&id).is_err());
    assert!(
        inv.reference("new-reader", std::slice::from_ref(&id))
            .is_err()
    );
    assert!(inv.retain_diagnostic_path(path).is_err());
    let round = inv
        .maintain_originals_at(std::slice::from_ref(&id), u64::MAX)
        .unwrap();
    assert_eq!(ledger(h.path())["roots"], before["roots"], "{round}");
    assert!(!output_retired(h.path(), &id), "{round}");
    inv.migrate_policy(&previous, Some(std::slice::from_ref(&id)))
        .unwrap();
    assert_eq!(ledger(h.path())["outputs"], migrated["outputs"]);
}

#[test]
fn lifetime_migration_rejects_missing_nonterminal_and_replaced_terminal_originals() {
    if !isolated_capability_test(
        "lifetime_migration_rejects_missing_nonterminal_and_replaced_terminal_originals",
    ) {
        return;
    }
    for terminal in [false, true] {
        let h = host(256, 16);
        let path = ".chrono-harness/state/fixtures/changed-original";
        fs::write(h.path().join(path), b"original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", path, json!({}))
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete_sealed(json!({"exit":0})).unwrap();
        if terminal {
            let mut output = ledger(h.path())["outputs"][0].clone();
            output["phase"] = json!("disposing");
            retention_state::replace_output(h.path(), &output);
        }
        let before = ledger(h.path());
        fs::rename(h.path().join(path), h.path().join("saved-original")).unwrap();
        if terminal {
            fs::write(h.path().join(path), b"replacement").unwrap();
        }
        let policy_path = h.path().join(POLICY_PATH);
        let previous = fs::read(&policy_path).unwrap();
        let mut policy: Value = serde_json::from_slice(&previous).unwrap();
        policy["max_lifetime_seconds"] = json!(60);
        fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        assert!(
            inventory(h.path())
                .migrate_policy(&previous, Some(std::slice::from_ref(&id)))
                .is_err()
        );
        assert_eq!(ledger(h.path())["outputs"], before["outputs"]);
        assert_eq!(
            fs::read(h.path().join("saved-original")).unwrap(),
            b"original"
        );
        if terminal {
            assert_eq!(fs::read(h.path().join(path)).unwrap(), b"replacement");
        }
    }
}

#[test]
fn exhausted_retirement_budget_reports_the_unattempted_original() {
    if !isolated_capability_test("exhausted_retirement_budget_reports_the_unattempted_original") {
        return;
    }
    let mut h = host(1, 16);
    h.disable_cleanup(true);
    eprintln!("retention budget fixture originals: {}", h.path().display());
    let path = ".chrono-harness/state/fixtures/interrupted-accounting";
    fs::write(h.path().join(path), b"original interrupted producer").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!({"completion":"unknown"}))
        .unwrap();
    let id = publication.id().to_owned();
    drop(publication);
    let round = inventory(h.path())
        .maintain_originals_at(std::slice::from_ref(&id), u64::MAX)
        .unwrap();
    assert_eq!(round["nodes"], 1, "{round}");
    assert_eq!(round["disposed"], json!([]), "{round}");
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert_eq!(
        fs::read(h.path().join(path)).unwrap(),
        b"original interrupted producer"
    );
    let budget = &round["protected"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["id"] == id
                && row["reason"] == "retirement round budget exhausted; original not attempted"
        })
        .unwrap_or_else(|| {
            panic!("the owning round must explain its unattempted original: {round}")
        })["budget"];
    assert_eq!(budget["nodes_used"], 1);
    assert_eq!(budget["node_limit"], 1);
    assert_eq!(budget["metadata_room"], true);
    let mut previous = 0;
    for field in [
        "entered_micros",
        "loaded_micros",
        "prepared_micros",
        "elapsed_micros",
    ] {
        let elapsed = budget[field].as_u64().unwrap();
        assert!(elapsed >= previous, "{budget}");
        previous = elapsed;
    }
    h.disable_cleanup(false);
}

#[test]
fn late_addressed_batch_retires_one_member_and_respects_a_live_reader() {
    if !isolated_capability_test(
        "late_addressed_batch_retires_one_member_and_respects_a_live_reader",
    ) {
        return;
    }
    let h = deadline_host();
    let owner = inventory(h.path());
    let path = ".chrono-harness/state/fixtures/batch";
    let root = h.path().join(path);
    let batch = inventory(h.path())
        .create_batch("fixture", path, json!({}))
        .unwrap();
    let id = batch.id().to_owned();
    fs::write(root.join("a"), b"a").unwrap();
    fs::write(root.join("b"), b"b").unwrap();
    batch.complete_sealed(json!({"status":"passed"})).unwrap();
    let reader = retained_artifacts::retention::read_guard(h.path(), path)
        .unwrap()
        .unwrap();
    let blocked = owner.maintain_late_original_at(&id, u64::MAX).unwrap();
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2, "{blocked}");
    drop(reader);
    for remaining in [1, 0] {
        let step = owner.maintain_late_original_at(&id, u64::MAX).unwrap();
        assert_eq!(step["accounting_errors"], json!([]), "{step}");
        assert_eq!(fs::read_dir(&root).unwrap().count(), remaining, "{step}");
        assert_eq!(step["removed_logical_bytes"], 1, "{step}");
    }
    let final_step = owner.maintain_late_original_at(&id, u64::MAX).unwrap();
    assert!(!root.try_exists().unwrap(), "{final_step}");
    assert_eq!(final_step["disposed"][0]["id"], id, "{final_step}");
}

#[test]
fn explicit_quota_observes_more_members_than_one_address_budget() {
    if !isolated_capability_test("explicit_quota_observes_more_members_than_one_address_budget") {
        return;
    }
    let d = host(32, 2);
    let path = d.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    policy["keep_seconds"] = json!(3600);
    policy["max_millis_per_round"] = json!(25);
    policy["terminal_limits"] = json!({"fixture": {
        "keep_seconds": 3600, "max_count": 2, "max_bytes": 1024,
        "metadata_keep_seconds": 0
    }});
    fs::write(&path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(d.path());
    let mut files = Vec::new();
    for i in 0..40 {
        let file = d
            .path()
            .join(format!(".chrono-harness/state/fixtures/quota-{i}"));
        let output = retained_artifacts::retention::create_adopted(
            d.path(),
            "fixture",
            &file,
            false,
            json!({}),
        )
        .unwrap()
        .unwrap();
        fs::write(&file, b"cache").unwrap();
        output.complete(json!({"status":"passed"})).unwrap();
        files.push(file);
    }
    let live = d.path().join(".chrono-harness/state/fixtures/live");
    let publication =
        retained_artifacts::retention::create_adopted(d.path(), "fixture", &live, false, json!({}))
            .unwrap()
            .unwrap();
    fs::write(&live, b"active producer").unwrap();
    let round = owner.maintain_explicit().unwrap();
    assert_eq!(
        round["terminal_quotas"]["fixture"]["scope_complete"], true,
        "{round}"
    );
    assert_eq!(
        round["terminal_quotas"]["fixture"]["errors"],
        json!([]),
        "{round}"
    );
    assert_eq!(
        files.iter().filter(|file| file.exists()).count(),
        2,
        "{round}"
    );
    assert_eq!(fs::read(&live).unwrap(), b"active producer");
    publication.complete(json!({"status":"passed"})).unwrap();
}

#[test]
fn unknown_quota_member_preserves_other_outputs_until_its_scope_is_known() {
    if !isolated_capability_test(
        "unknown_quota_member_preserves_other_outputs_until_its_scope_is_known",
    ) {
        return;
    }
    let d = host(32, 2);
    let path = d.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    policy["keep_seconds"] = json!(3600);
    policy["terminal_limits"] = json!({"fixture": {
        "keep_seconds": 3600, "max_count": 1, "max_bytes": 1024,
        "metadata_keep_seconds": 0
    }});
    fs::write(&path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(d.path());
    let mut files = Vec::new();
    let mut first = None;
    for i in 0..3 {
        let file = d
            .path()
            .join(format!(".chrono-harness/state/fixtures/unknown-quota-{i}"));
        let output = retained_artifacts::retention::create_adopted(
            d.path(),
            "fixture",
            &file,
            false,
            json!({}),
        )
        .unwrap()
        .unwrap();
        first.get_or_insert_with(|| output.id().to_owned());
        fs::write(&file, b"required original").unwrap();
        output.complete(json!({"status":"passed"})).unwrap();
        files.push(file);
    }
    let record = d
        .path()
        .join(".chrono-harness/state/output-retention-v1/objects")
        .join(first.unwrap())
        .join("record.json");
    let original = fs::read(&record).unwrap();
    fs::write(&record, b"unknown declaration").unwrap();
    let round = owner.maintain_explicit().unwrap();
    assert_eq!(
        round["terminal_quotas"]["fixture"]["scope_complete"], false,
        "{round}"
    );
    for file in &files {
        assert_eq!(fs::read(file).unwrap(), b"required original", "{round}");
    }
    fs::write(record, original).unwrap();
}

#[test]
fn reading_a_tree_member_leaves_only_its_owning_output_to_retire() {
    if !isolated_capability_test("reading_a_tree_member_leaves_only_its_owning_output_to_retire") {
        return;
    }
    let h = host(256, 16);
    let path = tree(h.path(), "read-member");
    let member = format!("{path}/result.json");
    fs::write(h.path().join(&member), b"returned").unwrap();
    let owner = inventory(h.path());
    let publication = inventory(h.path())
        .begin("fixture", &path, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete(json!({"exit_code":0})).unwrap();
    let mut address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for part in path.split('/') {
        address.push(chrono_harness::sha256(part.as_bytes()));
    }
    let reader = retained_artifacts::retention::read_guard(h.path(), &member)
        .unwrap()
        .unwrap();
    assert!(
        !address
            .join(chrono_harness::sha256(b"result.json"))
            .exists()
    );
    owner
        .maintain_originals_at(std::slice::from_ref(&id), u64::MAX)
        .unwrap();
    assert_eq!(fs::read(h.path().join(&member)).unwrap(), b"returned");
    drop(reader);
    for _ in 0..8 {
        if !retention_state::object_path(h.path(), &id).exists() {
            break;
        }
        owner
            .maintain_originals_at(std::slice::from_ref(&id), u64::MAX)
            .unwrap();
    }
    assert!(!h.path().join(path).exists());
    assert!(!address.exists());
    assert!(
        retained_artifacts::retention::read_guard(h.path(), &member)
            .unwrap()
            .is_none()
    );
    assert!(!address.exists(), "a lookup recreated the retired path");
}

#[test]
fn retired_metadata_recovers_a_path_unlinked_before_cursor_checkpoint() {
    if !isolated_capability_test(
        "retired_metadata_recovers_a_path_unlinked_before_cursor_checkpoint",
    ) {
        return;
    }
    let h = host(256, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(3600);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(h.path());
    let path = ".chrono-harness/state/fixtures/interrupted-path-prune";
    fs::write(h.path().join(path), b"rebuildable").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!(null))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete(json!({"exit":0})).unwrap();
    let round = owner.maintain_originals(std::slice::from_ref(&id)).unwrap();
    assert!(!h.path().join(path).exists(), "{round}");
    let object = retention_state::object_path(h.path(), &id);
    let before = fs::read(&object).unwrap();
    let declaration: Value = serde_json::from_slice(&before).unwrap();
    assert!(declaration["output"].is_null(), "{declaration}");
    assert_eq!(declaration["prune_path"], path);

    let sibling = ".chrono-harness/state/fixtures/current-path-sibling";
    fs::write(h.path().join(sibling), b"current original").unwrap();
    let current = owner.begin("fixture", sibling, json!(null)).unwrap();
    let mut address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for component in path.split('/') {
        address.push(chrono_harness::sha256(component.as_bytes()));
    }
    // Simulate interruption after the empty address was unlinked, before the
    // object cursor was checkpointed. Leave the actual old cursor unchanged.
    fs::remove_file(address.join("address.json")).unwrap();
    fs::remove_dir(&address).unwrap();
    assert_eq!(fs::read(&object).unwrap(), before);
    let parent = address.parent().unwrap();
    let sibling_reader = chrono_harness::ownership::DirectoryLease::acquire(parent, false, None)
        .unwrap()
        .unwrap();
    let round = inventory(h.path())
        .maintain_originals_at(std::slice::from_ref(&id), u64::MAX)
        .unwrap();
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert!(!object.parent().unwrap().exists(), "{round}");
    assert!(parent.join("address.json").is_file());
    assert_eq!(
        fs::read(h.path().join(sibling)).unwrap(),
        b"current original"
    );
    sibling_reader.stable().unwrap();
    drop(sibling_reader);
    current.complete(json!({"exit":0})).unwrap();
}

#[test]
fn retired_metadata_does_not_exclude_a_populated_parent_address() {
    if !isolated_capability_test("retired_metadata_does_not_exclude_a_populated_parent_address") {
        return;
    }
    let mut h = host(256, 16);
    h.disable_cleanup(true);
    eprintln!(
        "retention fixture originals (retired_metadata_does_not_exclude_a_populated_parent_address): {}",
        h.path().display()
    );
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(0);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(h.path());
    let obsolete = ".chrono-harness/state/fixtures/obsolete";
    fs::write(h.path().join(obsolete), b"rebuildable").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", obsolete, json!(null))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete(json!({"exit": 0})).unwrap();
    let sibling = ".chrono-harness/state/fixtures/current";
    fs::write(h.path().join(sibling), b"current original").unwrap();
    let current = inventory(h.path())
        .begin("fixture", sibling, json!(null))
        .unwrap();
    let mut parent_address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for component in ".chrono-harness/state/fixtures".split('/') {
        parent_address.push(chrono_harness::sha256(component.as_bytes()));
    }
    // A sibling reader shares this parent. Cleanup can retire obsolete leaf
    // metadata without requesting an exclusive lease on this populated parent.
    let parent_reader =
        chrono_harness::ownership::DirectoryLease::acquire(&parent_address, false, None)
            .unwrap()
            .unwrap();
    use chrono_diagnostics::logging::{Level, Logger};
    let logger = Logger::new("retention-fixture", Level::DEBUG, std::io::stderr());
    // 本例只核共享父路径下的退役，不依赖维护与元数据更新是否跨过秒边界。
    let round = logger
        .run(|| owner.maintain_originals_at(std::slice::from_ref(&id), u64::MAX))
        .unwrap();
    assert!(!h.path().join(obsolete).exists(), "{round}");
    let metadata = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/objects")
        .join(&id);
    // 原件处置可用尽本轮预算；只对仍存在的元数据续作，父路径始终保持共享。
    let continuation = if metadata.exists() {
        Some(
            logger
                .run(|| owner.maintain_originals_at(std::slice::from_ref(&id), u64::MAX))
                .unwrap(),
        )
    } else {
        None
    };
    logger.finish().expect("retired metadata diagnostics");
    assert!(
        !metadata.exists(),
        "obsolete metadata must retire while its populated parent is shared: {round}; continuation: {continuation:?}"
    );
    assert!(parent_address.join("address.json").is_file());
    assert_eq!(
        fs::read(h.path().join(sibling)).unwrap(),
        b"current original"
    );
    parent_reader.stable().unwrap();
    drop(parent_reader);
    current.complete(json!({"exit": 0})).unwrap();
    h.disable_cleanup(false);
}
#[cfg(not(verus_keep_ghost))]
fn maintenance(root: &Path) -> Value {
    let round = inventory(root).maintain().unwrap();
    if !round["accounting_errors"].as_array().unwrap().is_empty() {
        eprintln!("retention round errors: {round}");
    }
    round
}
#[cfg(not(verus_keep_ghost))]
fn diagnosed_maintenance_originals(root: &Path, id: &str) -> Value {
    use chrono_diagnostics::logging::{Level, Logger};
    // The existing operation owns and bounds stderr; no extra log family or
    // subscriber is installed in the parent test process.
    let logger = Logger::new("retention-fixture", Level::DEBUG, std::io::stderr());
    let round = logger.run(|| inventory(root).maintain_originals(&[id.to_owned()]));
    logger
        .finish()
        .expect("original retention stage diagnostics");
    round.unwrap()
}
// The fixture owns every address below its private root. This count observes
// those declarations after bounded work; it is never written into a producer report.
#[cfg(not(verus_keep_ghost))]
fn fixture_remaining(root: &Path, round: &Value) -> Value {
    let actual = json!(ledger(root)["outputs"].as_array().unwrap().len());
    if round["scope"]["complete"] == true {
        assert_eq!(round["remaining_outputs"], actual, "{round}");
    } else {
        assert_eq!(round["remaining_outputs"], Value::Null, "{round}");
    }
    actual
}
#[cfg(not(verus_keep_ghost))]
fn maintain_fixture_to(root: &Path, expected: u64) -> Value {
    let rounds = maintain_fixture_rounds_to(root, expected);
    fixture_remaining(root, rounds.last().unwrap())
}
#[cfg(not(verus_keep_ghost))]
fn maintain_fixture_rounds_to(root: &Path, expected: u64) -> Vec<Value> {
    maintain_fixture_rounds_until(
        root,
        &format!("{expected} remaining fixture outputs"),
        |round| fixture_remaining(root, round) == expected,
    )
}
#[cfg(not(verus_keep_ghost))]
fn maintain_fixture_rounds_until(
    root: &Path,
    expected: &str,
    mut complete: impl FnMut(&Value) -> bool,
) -> Vec<Value> {
    let policy: Value = serde_json::from_slice(&fs::read(root.join(POLICY_PATH)).unwrap()).unwrap();
    let mut rounds = Vec::new();
    let mut maintenance_micros = Vec::new();
    while fixture_recovery_has_room(rounds.len()) {
        let began = std::time::Instant::now();
        let round = maintenance(root);
        maintenance_micros.push(began.elapsed().as_micros());
        assert!(
            round["nodes"].as_u64().unwrap() <= policy["max_nodes_per_round"].as_u64().unwrap(),
            "{round}"
        );
        assert!(
            round["verified_bytes"].as_u64().unwrap()
                <= policy["max_bytes_per_round"].as_u64().unwrap(),
            "{round}"
        );
        let completed = complete(&round);
        rounds.push(round);
        if completed {
            return rounds;
        }
    }
    panic!(
        "bounded fixture recovery did not reach {expected}; rounds: {rounds:?}; maintenance_micros: {maintenance_micros:?}; policy: {policy}; remaining fixture state: {}",
        ledger(root)
    );
}
#[cfg(not(verus_keep_ghost))]
fn tree(root: &Path, name: &str) -> String {
    let path = format!(".chrono-harness/state/fixtures/{name}");
    fs::create_dir(root.join(&path)).unwrap();
    path
}
#[path = "support/retention_state.rs"]
#[cfg(not(verus_keep_ghost))]
mod retention_state;

#[path = "support/retention_frontier.rs"]
#[cfg(not(verus_keep_ghost))]
mod retention_frontier;
#[cfg(not(verus_keep_ghost))]
fn ledger(root: &Path) -> Value {
    retention_state::snapshot(root)
}
#[cfg(not(verus_keep_ghost))]
fn output_retired(root: &Path, id: &str) -> bool {
    let state: Value =
        serde_json::from_slice(&fs::read(retention_state::object_path(root, id)).unwrap()).unwrap();
    state["output"].is_null()
}

#[test]
fn bounded_fixture_recovery_has_exactly_eight_rounds() {
    assert!(fixture_recovery_has_room(0));
    assert!(fixture_recovery_has_room(7));
    assert!(!fixture_recovery_has_room(8));
    assert!(!fixture_recovery_has_room(usize::MAX));
}

#[test]
fn explicit_maintenance_reaches_expired_tail_after_retained_metadata_prefix() {
    if !isolated_capability_test(
        "explicit_maintenance_reaches_expired_tail_after_retained_metadata_prefix",
    ) {
        return;
    }
    let h = host(256, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_metadata_bytes_per_round"] = json!(24 * 1024 * 1024);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut outputs = std::collections::BTreeMap::new();
    for n in 0..512 {
        let path = format!(".chrono-harness/state/fixtures/large-metadata-{n}");
        fs::write(h.path().join(&path), b"required original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        publication
            .complete(json!({"diagnostic": "x".repeat(15 * 1024)}))
            .unwrap();
        outputs.insert(id, path);
    }
    // Choose the tail of the actual declaration iterator, not an assumed file
    // name order. Each declaration fits a round; their retained prefix does not.
    let addresses: Vec<_> = fs::read_dir(
        h.path()
            .join(".chrono-harness/state/output-retention-v1/objects"),
    )
    .unwrap()
    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
    .collect();
    let tail = addresses.last().unwrap();
    for (n, originals) in addresses[..addresses.len() - 1].chunks(32).enumerate() {
        inventory(h.path())
            .reference(&format!("current-inputs-{n}"), originals)
            .unwrap();
    }
    for _ in 0..3 {
        let round = inventory(h.path()).maintain().unwrap();
        assert_eq!(round["scope"]["complete"], false, "{round}");
        assert!(h.path().join(&outputs[tail]).exists(), "{round}");
    }
    let result = consumer_call(h.path(), json!({"operation":"maintain"}));
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    let sweep: Value = serde_json::from_str(&result.stdout).unwrap();
    assert!(
        !h.path().join(&outputs[tail]).exists(),
        "explicit maintenance did not reach the independent expired tail: {sweep}"
    );
    assert_eq!(sweep["schema"], "chrono-output-retention-sweep/v1");
    assert_eq!(sweep["removed_logical_bytes"], b"required original".len());
    assert_eq!(sweep["remaining_outputs"], Value::Null);
    assert!(sweep["samples"]["payload_bytes"].as_u64().unwrap() <= 65536);
    assert!(sweep["samples"]["omitted"].as_u64().unwrap() > 0);
    assert!(serde_json::to_vec(&sweep).unwrap().len() < 128 * 1024);
    for field in ["nodes", "metadata_bytes", "verified_bytes"] {
        assert!(
            sweep["round_maxima"][field].as_u64().unwrap()
                <= sweep["limits_per_round"][field].as_u64().unwrap(),
            "{sweep}"
        );
    }
    for id in &addresses[..addresses.len() - 1] {
        assert_eq!(
            fs::read(h.path().join(&outputs[id])).unwrap(),
            b"required original"
        );
    }
    // Complete members and raw outcomes have a real maintenance-report
    // consumer even when the unchanged compact sample omits observations.
    let mut observations = Vec::new();
    let sweep = inventory(h.path())
        .maintain_explicit_observed(|row| {
            observations.push(row.clone());
            Ok(())
        })
        .unwrap();
    let declared: std::collections::BTreeSet<_> = observations
        .iter()
        .filter(|row| row["kind"] == "declaration" && row["namespace"] == "objects")
        .map(|row| row["address"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        declared,
        addresses
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(serde_json::to_vec(&observations).unwrap().len() > 65536);
    assert!(sweep["samples"]["payload_bytes"].as_u64().unwrap() <= 65536);
    assert!(
        observations
            .iter()
            .any(|row| row["kind"] == "round" && row["namespace"] == "objects")
    );
}

#[test]
fn explicit_inventory_observer_keeps_damaged_live_and_unexecuted_members() {
    if !isolated_capability_test(
        "explicit_inventory_observer_keeps_damaged_live_and_unexecuted_members",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut ids = std::collections::BTreeMap::new();
    for name in ["expired", "retained", "damaged"] {
        let path = format!(".chrono-harness/state/fixtures/{name}");
        fs::write(h.path().join(&path), b"original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        ids.insert(name, publication.id().to_owned());
        publication.complete(json!({"exit":7})).unwrap();
    }
    inventory(h.path())
        .reference("actual-user", &[ids["retained"].clone()])
        .unwrap();
    let live_path = ".chrono-harness/state/fixtures/live";
    fs::write(h.path().join(live_path), b"live producer").unwrap();
    let live = inventory(h.path())
        .begin("fixture", live_path, json!(null))
        .unwrap();
    let damaged = retention_state::object_path(h.path(), &ids["damaged"]);
    fs::write(&damaged, b"original malformed declaration").unwrap();
    let mut rows = Vec::new();
    inventory(h.path())
        .maintain_explicit_observed(|row| {
            rows.push(row.clone());
            Ok(())
        })
        .unwrap();
    let declared: std::collections::BTreeSet<_> = rows
        .iter()
        .filter(|row| row["kind"] == "declaration" && row["namespace"] == "objects")
        .map(|row| row["address"].as_str().unwrap().to_owned())
        .collect();
    let expected: std::collections::BTreeSet<_> = ids
        .values()
        .cloned()
        .chain([live.id().to_owned()])
        .collect();
    assert_eq!(declared, expected);
    assert!(rows.iter().any(|row| {
        row["address"] == ids["damaged"]
            && row["kind"] == "round"
            && row["original"]["accounting_errors"]
                .as_array()
                .is_some_and(|errors| !errors.is_empty())
    }));
    assert_eq!(
        fs::read(&damaged).unwrap(),
        b"original malformed declaration"
    );
    assert_eq!(
        fs::read(h.path().join(live_path)).unwrap(),
        b"live producer"
    );
    assert_eq!(
        fs::read(h.path().join(".chrono-harness/state/fixtures/retained")).unwrap(),
        b"original"
    );
    let error = inventory(h.path())
        .maintain_explicit_observed(|_| Err("original report IO failure".into()))
        .unwrap_err();
    assert_eq!(error, "original report IO failure");
    // Missing declaration namespaces are reported as incomplete, never zero.
    let owner = inventory(h.path());
    fs::remove_dir_all(
        h.path()
            .join(".chrono-harness/state/output-retention-v1/roles"),
    )
    .unwrap();
    let mut rows = Vec::new();
    owner
        .maintain_explicit_observed(|row| {
            rows.push(row.clone());
            Ok(())
        })
        .unwrap();
    assert!(
        rows.iter()
            .any(|row| row["kind"] == "enumeration-error" && row["namespace"] == "roles")
    );
    drop(live);
}

#[cfg(not(verus_keep_ghost))]
fn legacy_empty_address(root: &Path, path: &str) -> std::path::PathBuf {
    let mut directory = root.join(".chrono-harness/state/output-retention-v1/paths");
    let mut logical = String::new();
    for part in path.split('/') {
        if !logical.is_empty() {
            logical.push('/');
        }
        logical.push_str(part);
        directory.push(chrono_harness::sha256(part.as_bytes()));
        fs::create_dir_all(&directory).unwrap();
        let marker = directory.join("address.json");
        if !marker.exists() {
            fs::write(
                marker,
                serde_json::to_vec(&json!({"path":logical,"output":null})).unwrap(),
            )
            .unwrap();
        }
    }
    directory
}

#[test]
fn expired_orphan_addresses_preserve_active_and_unknown_members_then_retire_bottom_up() {
    if !isolated_capability_test(
        "expired_orphan_addresses_preserve_active_and_unknown_members_then_retire_bottom_up",
    ) {
        return;
    }
    let h = host(256, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    let owner = inventory(h.path());
    let child = legacy_empty_address(h.path(), "archived/check/result.json");
    let sibling = legacy_empty_address(h.path(), "archived/shared");
    fs::write(sibling.join("address.pending"), b"unresolved publication").unwrap();
    owner.maintain_explicit().unwrap();
    assert!(
        child.join("address.json").is_file(),
        "inside its original window"
    );

    policy["metadata_keep_seconds"] = json!(0);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(h.path());
    let lease = chrono_harness::ownership::DirectoryLease::acquire_local(&child, false, None)
        .unwrap()
        .unwrap();
    owner.maintain_explicit().unwrap();
    assert!(child.join("address.json").is_file());
    drop(lease);
    owner.maintain_explicit().unwrap();
    assert!(
        !child.parent().unwrap().exists(),
        "parent must retire after the member"
    );
    assert!(sibling.parent().unwrap().join("address.json").is_file());
    assert_eq!(
        fs::read(sibling.join("address.pending")).unwrap(),
        b"unresolved publication"
    );
}

#[test]
fn retiring_object_preserves_an_unresolved_path_publication() {
    if !isolated_capability_test("retiring_object_preserves_an_unresolved_path_publication") {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/pending-path";
    fs::write(h.path().join(path), b"returned").unwrap();
    let original = inventory(h.path())
        .begin("fixture", path, json!({}))
        .unwrap();
    let id = original.id().to_owned();
    original.complete(json!({"exit":0})).unwrap();
    let owner = inventory(h.path());
    owner.maintain_originals(&[id.clone()]).unwrap();
    assert!(!h.path().join(path).exists());
    let address = legacy_empty_address(h.path(), path);
    let pending = address.join("address.pending");
    fs::write(&pending, b"interrupted mapping update").unwrap();
    owner.maintain_originals_at(&[id], u64::MAX).unwrap();
    assert_eq!(fs::read(pending).unwrap(), b"interrupted mapping update");
    assert!(address.join("address.json").is_file());
}

#[test]
fn interrupted_path_retirement_resumes_without_following_a_replacement() {
    if !isolated_capability_test(
        "interrupted_path_retirement_resumes_without_following_a_replacement",
    ) {
        return;
    }
    let h = host(1, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(0);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let owner = inventory(h.path());
    let old = legacy_empty_address(h.path(), "original");
    let stop = owner.maintain_explicit_observed(|row| {
        if row["namespace"] == "paths" && row["original"]["retirement_pending"] == true {
            return Err("interrupted after identity-qualified rename".into());
        }
        Ok(())
    });
    assert_eq!(
        stop.unwrap_err(),
        "interrupted after identity-qualified rename"
    );
    assert!(!old.exists());
    let index_root = old.parent().unwrap();
    let pending = fs::read_dir(index_root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("retiring-")
        })
        .unwrap();
    assert!(pending.join("address.json").is_file());
    fs::create_dir(&old).unwrap();
    let replacement = br#"{"path":"original","output":"new-owner"}"#;
    fs::write(old.join("address.json"), replacement).unwrap();
    let first = owner.maintain_explicit().unwrap();
    assert!(first["round_maxima"]["nodes"].as_u64().unwrap() <= 1);
    assert!(pending.is_dir());
    assert!(!pending.join("address.json").exists());
    owner.maintain_explicit().unwrap();
    assert!(!pending.exists());
    assert_eq!(
        fs::read_dir(index_root).unwrap().count(),
        1,
        "only the replacement remains"
    );
    assert_eq!(fs::read(old.join("address.json")).unwrap(), replacement);
}

#[test]
fn explicit_sweep_preserves_live_rooted_damaged_and_changed_originals() {
    if !isolated_capability_test(
        "explicit_sweep_preserves_live_rooted_damaged_and_changed_originals",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut ids = std::collections::BTreeMap::new();
    for name in ["expired", "required", "changed", "damaged"] {
        let path = format!(".chrono-harness/state/fixtures/{name}");
        fs::write(h.path().join(&path), b"original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        ids.insert(name, publication.id().to_owned());
        publication.complete(json!({"exit":7})).unwrap();
    }
    let live = ".chrono-harness/state/fixtures/live";
    fs::write(h.path().join(live), b"active writer").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", live, json!(null))
        .unwrap();
    inventory(h.path())
        .reference("actual-recheck", &[ids["required"].clone()])
        .unwrap();
    let changed = ".chrono-harness/state/fixtures/changed";
    fs::write(h.path().join(changed), b"new unaccepted bytes").unwrap();
    let damaged = retention_state::object_path(h.path(), &ids["damaged"]);
    fs::write(&damaged, b"unknown declaration bytes").unwrap();
    let unregistered = ".chrono-harness/state/fixtures/unregistered";
    fs::write(h.path().join(unregistered), b"unique unregistered work").unwrap();

    let result = consumer_call(h.path(), json!({"operation":"maintain"}));
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    let sweep: Value = serde_json::from_str(&result.stdout).unwrap();
    assert!(
        !h.path()
            .join(".chrono-harness/state/fixtures/expired")
            .exists(),
        "{sweep}"
    );
    assert_eq!(fs::read(h.path().join(live)).unwrap(), b"active writer");
    assert_eq!(
        fs::read(h.path().join(changed)).unwrap(),
        b"new unaccepted bytes"
    );
    assert_eq!(fs::read(&damaged).unwrap(), b"unknown declaration bytes");
    assert_eq!(
        fs::read(h.path().join(".chrono-harness/state/fixtures/damaged")).unwrap(),
        b"original"
    );
    assert_eq!(
        fs::read(h.path().join(".chrono-harness/state/fixtures/required")).unwrap(),
        b"original"
    );
    assert_eq!(
        fs::read(h.path().join(unregistered)).unwrap(),
        b"unique unregistered work"
    );
    assert!(
        sweep["observations"]["accounting_errors"].as_u64().unwrap() > 0,
        "{sweep}"
    );
    assert!(
        sweep["observations"]["protected"].as_u64().unwrap() > 0,
        "{sweep}"
    );
    assert_eq!(sweep["scope"]["complete"], false);
    drop(publication);
}

#[test]
fn explicit_sweep_reopens_each_partial_component_with_the_original_node_bound() {
    if !isolated_capability_test(
        "explicit_sweep_reopens_each_partial_component_with_the_original_node_bound",
    ) {
        return;
    }
    let h = host(3, 16);
    let mut paths = Vec::new();
    for name in ["first", "second"] {
        let path = tree(h.path(), name);
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        for n in 0..8 {
            fs::write(h.path().join(&path).join(n.to_string()), b"partial output").unwrap();
        }
        publication.complete(json!({"exit":7})).unwrap();
        paths.push(path);
    }
    let mut removed = 0;
    let mut rounds = Vec::new();
    for _ in 0..32 {
        let result = consumer_call(h.path(), json!({"operation":"maintain"}));
        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        let sweep: Value = serde_json::from_str(&result.stdout).unwrap();
        assert!(
            sweep["round_maxima"]["nodes"].as_u64().unwrap() <= 3,
            "{sweep}"
        );
        removed += sweep["removed_logical_bytes"].as_u64().unwrap();
        rounds.push(sweep);
        if paths.iter().all(|path| !h.path().join(path).exists()) {
            break;
        }
    }
    assert!(
        paths.iter().all(|path| !h.path().join(path).exists()),
        "{rounds:?}"
    );
    assert_eq!(removed, 16 * b"partial output".len() as u64, "{rounds:?}");
}

#[test]
#[cfg(unix)]
fn enrollment_original_membership_fits_the_native_256_descriptor_limit() {
    if !isolated_capability_test(
        "enrollment_original_membership_fits_the_native_256_descriptor_limit",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(128);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let mut paths = Vec::new();
    for n in 0..80 {
        let path = format!(".chrono-harness/state/fixtures/original-{n}.json");
        fs::write(h.path().join(&path), n.to_string()).unwrap();
        inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit":0}))
            .unwrap();
        paths.push(path);
    }
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    limit.rlim_cur = limit.rlim_cur.min(256);
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    inventory(h.path())
        .reference_paths("current-enrollment", &paths)
        .unwrap();
    let original = ledger(h.path());
    assert_eq!(
        original["roots"]["current-enrollment"]
            .as_array()
            .unwrap()
            .len(),
        80
    );
    inventory(h.path())
        .reference_paths("current-enrollment", &paths[40..])
        .unwrap();
    let released = ledger(h.path());
    let members = released["roots"]["current-enrollment"].as_array().unwrap();
    assert_eq!(members.len(), 40);
    for (n, path) in paths.iter().enumerate() {
        let output = released["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["path"] == *path)
            .unwrap();
        assert_eq!(members.contains(&output["id"]), n >= 40);
        assert_eq!(
            fs::read_to_string(h.path().join(path)).unwrap(),
            n.to_string()
        );
    }
}

#[test]
#[cfg(unix)]
fn actual_enrollment_width_preserves_370_originals_under_256_descriptors() {
    if !isolated_capability_test(
        "actual_enrollment_width_preserves_370_originals_under_256_descriptors",
    ) {
        return;
    }
    let h = host(256, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(512);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut paths = Vec::new();
    for n in 0..370 {
        let path = format!(".chrono-harness/state/fixtures/pending-use-{n}.json");
        fs::write(h.path().join(&path), n.to_string()).unwrap();
        inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit":0}))
            .unwrap();
        paths.push(path);
    }
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    limit.rlim_cur = limit.rlim_cur.min(256);
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    inventory(h.path())
        .reference_paths("actual-enrollment", &paths)
        .unwrap();
    let protected = ledger(h.path());
    assert_eq!(
        protected["roots"]["actual-enrollment"]
            .as_array()
            .unwrap()
            .len(),
        370
    );
    inventory(h.path())
        .reference_paths("actual-enrollment", &paths[185..])
        .unwrap();
    let released = ledger(h.path());
    let members = released["roots"]["actual-enrollment"].as_array().unwrap();
    assert_eq!(members.len(), 185);
    for (n, path) in paths.iter().enumerate() {
        let output = released["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|output| output["path"] == *path)
            .unwrap();
        assert_eq!(members.contains(&output["id"]), n >= 185);
        assert_eq!(
            fs::read_to_string(h.path().join(path)).unwrap(),
            n.to_string()
        );
    }
    inventory(h.path())
        .reference_paths("actual-enrollment", &[])
        .unwrap();
    assert!(ledger(h.path())["roots"].get("actual-enrollment").is_none());
}

#[test]
fn durable_consumer_shrink_prunes_stale_members_without_admitting_new_outputs() {
    if !isolated_capability_test(
        "durable_consumer_shrink_prunes_stale_members_without_admitting_new_outputs",
    ) {
        return;
    }
    let h = host(256, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(4);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let inventory = inventory(h.path());
    let mut paths = Vec::new();
    for n in 0..5 {
        let path = format!(".chrono-harness/state/fixtures/committed-handoff-{n}.json");
        fs::write(h.path().join(&path), n.to_string()).unwrap();
        Inventory::adopted(h.path())
            .unwrap()
            .unwrap()
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit":0}))
            .unwrap();
        paths.push(path);
    }
    let role = "durable-current-inputs";
    inventory.reference_paths(role, &paths[..4]).unwrap();
    let before = ledger(h.path());
    assert!(
        inventory
            .reference_paths(role, &paths[3..])
            .unwrap_err()
            .contains("related retention graph capacity")
    );
    assert_eq!(ledger(h.path())["roots"][role], before["roots"][role]);
    // The domain has durably stopped reading the first three originals.
    // Pruning cannot install the newly requested fifth output.
    inventory.prune_reference_paths(role, &paths[3..]).unwrap();
    let pruned = ledger(h.path());
    let fourth = pruned["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["path"] == paths[3])
        .unwrap()["id"]
        .clone();
    assert_eq!(pruned["roots"][role], json!([fourth]));
    // An interrupted caller resumes admission using the ordinary bounded API.
    inventory.reference_paths(role, &paths[3..]).unwrap();
    assert_eq!(ledger(h.path())["roots"][role].as_array().unwrap().len(), 2);
    inventory.prune_reference_paths(role, &paths[3..]).unwrap();
    assert_eq!(ledger(h.path())["roots"][role].as_array().unwrap().len(), 2);
    for path in paths {
        assert!(
            h.path().join(path).exists(),
            "membership updates never dispose originals"
        );
    }
}

#[test]
fn path_consumer_updates_preserve_dependencies_beyond_its_membership_capacity() {
    if !isolated_capability_test(
        "path_consumer_updates_preserve_dependencies_beyond_its_membership_capacity",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(4);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let publish = |name: &str| {
        let path = tree(h.path(), name);
        fs::write(h.path().join(&path).join("original"), name.as_bytes()).unwrap();
        inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit":0}))
            .unwrap();
        path
    };
    let mut sources = Vec::new();
    let mut paths = Vec::new();
    for group in 0..2 {
        let leaves = [
            publish(&format!("leaf-{group}-a")),
            publish(&format!("leaf-{group}-b")),
        ];
        let source = publish(&format!("report-{group}"));
        inventory(h.path())
            .dependencies_paths(&source, &leaves)
            .unwrap();
        paths.extend(leaves);
        paths.push(source.clone());
        sources.push(source);
    }
    let before = ledger(h.path());
    let edges = |snapshot: &Value| {
        snapshot["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|output| (output["id"].clone(), output["references"].clone()))
            .collect::<Vec<_>>()
    };
    // Two direct members fit, although their six-output closure does not.
    inventory(h.path())
        .reference_paths("current-enrollment", &sources)
        .unwrap();
    assert_eq!(edges(&ledger(h.path())), edges(&before));
    assert!(
        inventory(h.path())
            .reference_paths("current-enrollment", &paths)
            .is_err()
    );
    publish("independent-expired");
    assert_eq!(maintain_fixture_to(h.path(), 6), 6);
    for path in &paths {
        assert!(h.path().join(path).is_dir());
    }
    inventory(h.path())
        .reference_paths("current-enrollment", &sources[1..])
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 3), 3);
    assert!(!h.path().join(&sources[0]).exists());
    assert!(h.path().join(&sources[1]).exists());
    inventory(h.path())
        .reference_paths("current-enrollment", &[])
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
}

#[test]
#[cfg(unix)]
fn declarations_reject_special_files_without_blocking_or_disposing_originals() {
    use std::os::unix::fs::{MetadataExt, symlink};
    if !isolated_capability_test(
        "declarations_reject_special_files_without_blocking_or_disposing_originals",
    ) {
        return;
    }
    for (namespace, kind) in ["path", "object", "consumer"]
        .into_iter()
        .flat_map(|namespace| {
            [
                "fifo",
                "symlink",
                "dangling",
                "symlink-fifo",
                "directory",
                "oversized",
            ]
            .into_iter()
            .map(move |kind| (namespace, kind))
        })
    {
        let h = host(256, 16);
        let path = tree(h.path(), "original");
        let original = h.path().join(&path).join("failure");
        fs::write(&original, b"necessary original failure").unwrap();
        let publication = inventory(h.path())
            .begin_rooted("fixture", &path, json!(null), &["current".into()])
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete(json!({"exit":7})).unwrap();
        let mut directory = h
            .path()
            .join(".chrono-harness/state/output-retention-v1/paths");
        for component in path.split('/') {
            directory.push(chrono_harness::sha256(component.as_bytes()));
        }
        let marker = match namespace {
            "path" => directory.join("address.json"),
            "object" => retention_state::object_path(h.path(), &id),
            "consumer" => retention_state::role_path(h.path(), "current"),
            _ => unreachable!(),
        };
        let directory = marker.parent().unwrap();
        let saved = directory.join("saved-address.json");
        let target = directory.join("fifo-target");
        fs::rename(&marker, &saved).unwrap();
        let fifo = |path: &Path| {
            use std::os::unix::ffi::OsStrExt;
            let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        };
        match kind {
            "fifo" => fifo(&marker),
            "symlink" => symlink(&saved, &marker).unwrap(),
            "dangling" => symlink(directory.join("absent"), &marker).unwrap(),
            "symlink-fifo" => {
                fifo(&target);
                symlink(&target, &marker).unwrap();
            }
            "directory" => fs::create_dir(&marker).unwrap(),
            "oversized" => fs::File::create(&marker)
                .unwrap()
                .set_len(8 * 1024 * 1024 + 1)
                .unwrap(),
            _ => unreachable!(),
        }
        let before = fs::symlink_metadata(&marker).unwrap();
        // Explicit selection rejects a non-file object declaration before its
        // reader runs. Global maintenance exercises that reader's refusal.
        let request = if namespace == "object" {
            json!({"operation":"maintain"})
        } else {
            json!({"operation":"maintain", "originals":[id]})
        };
        fs::write(
            h.path().join("maintain.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        // The managed real child is joined even if an unsafe FIFO open blocks.
        let result = chrono_harness::run_process(
            h.path(),
            &chrono_harness::CommandSpec {
                program: env!("CARGO_BIN_EXE_chrono-test-retention-child").into(),
                args: vec![
                    "dispatch".into(),
                    "retention".into(),
                    "--host-root".into(),
                    h.path().to_str().unwrap().into(),
                    "--request".into(),
                    "maintain.json".into(),
                ],
                env: Default::default(),
                timeout_seconds: 5,
                output_limit_bytes: 65536,
            },
            b"",
        )
        .unwrap();
        assert!(result.failure.is_none(), "{namespace}/{kind}: {result:?}");
        assert_eq!(result.exit_code, 0, "{namespace}/{kind}: {result:?}");
        let round: Value = serde_json::from_str(&result.stdout).unwrap();
        if namespace == "object" {
            assert_eq!(round["schema"], "chrono-output-retention-sweep/v1");
            assert!(
                round["observations"]["accounting_errors"].as_u64().unwrap() > 0,
                "{namespace}/{kind}: {round}"
            );
            assert!(
                round["samples"]["items"].as_array().unwrap().iter().any(
                    |sample| sample["namespace"] == "objects"
                        && sample["address"] == id
                        && sample["kind"] == "accounting_errors"
                ),
                "{namespace}/{kind}: {round}"
            );
        } else {
            assert!(
                !round["accounting_errors"].as_array().unwrap().is_empty(),
                "{namespace}/{kind}: {round}"
            );
        }
        assert_eq!(
            round["removed_logical_bytes"], 0,
            "{namespace}/{kind}: {round}"
        );
        assert_eq!(fs::read(&original).unwrap(), b"necessary original failure");
        let after = fs::symlink_metadata(&marker).unwrap();
        assert_eq!(
            (after.dev(), after.ino(), after.mode(), after.len()),
            (before.dev(), before.ino(), before.mode(), before.len()),
            "{namespace}/{kind}"
        );
        if kind == "directory" {
            fs::remove_dir(&marker).unwrap();
        } else {
            fs::remove_file(&marker).unwrap();
        }
        if kind == "symlink-fifo" {
            fs::remove_file(&target).unwrap();
        }
        fs::rename(&saved, &marker).unwrap();
        inventory(h.path()).reference("current", &[]).unwrap();
        assert_eq!(maintain_fixture_to(h.path(), 0), 0, "{namespace}/{kind}");
        assert!(!original.exists(), "{namespace}/{kind}");
    }
}

#[test]
fn executable_output_requires_an_explicit_subdirectory_and_preserves_selected_binary() {
    if !isolated_capability_test(
        "executable_output_requires_an_explicit_subdirectory_and_preserves_selected_binary",
    ) {
        return;
    }
    let h = host(256, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    for scope in [".chrono-harness/bin/", ".chrono-harness/", "crates/"] {
        policy["producers"]["fixture"] = json!([scope]);
        fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        assert!(Inventory::adopted(h.path()).is_err(), "{scope}");
    }
    policy["producers"]["fixture"] = json!([".chrono-harness/bin/owner-stages/"]);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let parent = h.path().join(".chrono-harness/bin/owner-stages");
    fs::create_dir_all(&parent).unwrap();
    let selected = h.path().join(".chrono-harness/bin/chrono-worktree");
    fs::write(&selected, b"current installed executable").unwrap();
    let stage = parent.join("stage-fixture");
    let publication =
        retained_artifacts::retention::create_adopted(h.path(), "fixture", &stage, true, json!({}))
            .unwrap()
            .unwrap();
    publication
        .write_file(&stage.join("candidate"), b"candidate executable")
        .unwrap();
    publication.complete(json!({"status":"joined"})).unwrap();
    maintain_fixture_to(h.path(), 0);
    assert!(!stage.exists());
    assert_eq!(
        fs::read(&selected).unwrap(),
        b"current installed executable"
    );
}

#[test]
fn maintenance_locks_ancestors_without_consuming_their_unrelated_address_records() {
    if !isolated_capability_test(
        "maintenance_locks_ancestors_without_consuming_their_unrelated_address_records",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/exclusion-only-original";
    fs::write(h.path().join(path), b"original failure bytes").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!(null))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete(json!({"exit":7})).unwrap();
    let ancestor = ".chrono-harness/state/fixtures";
    let mut directory = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for component in ancestor.split('/') {
        directory.push(chrono_harness::sha256(component.as_bytes()));
    }
    let marker = directory.join("address.json");
    let original_marker = fs::read(&marker).unwrap();
    let excluded = chrono_harness::ownership::DirectoryLease::acquire(&directory, true, None)
        .unwrap()
        .unwrap();
    let busy = inventory(h.path())
        .maintain_originals(&[id.clone()])
        .unwrap();
    assert!(
        busy["accounting_errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["reason"]
                .as_str()
                .unwrap()
                .contains("retention path is busy")),
        "{busy}"
    );
    assert_eq!(busy["removed_logical_bytes"], 0, "{busy}");
    assert_eq!(
        fs::read(h.path().join(path)).unwrap(),
        b"original failure bytes"
    );
    drop(excluded);

    // The original's own address remains valid. This ancestor record is not an
    // input to its physical disposal, but the directory must still be excluded.
    fs::write(&marker, b"unrelated malformed ancestor address").unwrap();
    let round = inventory(h.path()).maintain_originals(&[id]).unwrap();
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert_eq!(round["removed_logical_bytes"], 22, "{round}");
    assert!(!h.path().join(path).exists());
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"unrelated malformed ancestor address"
    );

    // Publication still consumes ancestor mappings to detect overlapping
    // reservations. Maintenance must not weaken that separate activity.
    let next = ".chrono-harness/state/fixtures/new-original";
    fs::write(h.path().join(next), b"next original").unwrap();
    assert!(
        inventory(h.path())
            .begin("fixture", next, json!(null))
            .is_err()
    );
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"unrelated malformed ancestor address"
    );
    fs::write(&marker, &original_marker).unwrap();
    inventory(h.path())
        .begin("fixture", next, json!(null))
        .unwrap()
        .complete(json!({"exit":9}))
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
}

#[test]
fn unfinished_directory_resumes_after_accounting_and_seals_only_after_actual_publication() {
    if !isolated_capability_test(
        "unfinished_directory_resumes_after_accounting_and_seals_only_after_actual_publication",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/two-stage-output";
    let publication = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &h.path().join(path),
        true,
        json!({"stage":"constructed"}),
    )
    .unwrap()
    .unwrap();
    publication
        .write_file(&h.path().join(path).join("candidate"), b"candidate bytes")
        .unwrap();
    let id = publication.id().to_owned();
    inventory(h.path())
        .reference_paths("actual-stage-report", &[path.into()])
        .unwrap();
    assert!(
        inventory(h.path())
            .resume_unfinished_directory("fixture", path)
            .is_err()
    );
    drop(publication);
    maintenance(h.path());
    let interrupted = ledger(h.path())["outputs"][0]["outcome"].clone();
    assert_eq!(interrupted["completion"], "not-established");
    let publication = inventory(h.path())
        .resume_unfinished_directory("fixture", path)
        .unwrap()
        .unwrap();
    assert_eq!(publication.id(), id);
    publication
        .write_file(
            &h.path().join(path).join("publication-result"),
            b"published bytes",
        )
        .unwrap();
    publication.complete(json!({"status":"published"})).unwrap();
    assert!(
        inventory(h.path())
            .resume_unfinished_directory("fixture", path)
            .is_err()
    );
    maintenance(h.path());
    assert_eq!(
        fs::read(h.path().join(path).join("candidate")).unwrap(),
        b"candidate bytes"
    );
    assert_eq!(
        fs::read(h.path().join(path).join("publication-result")).unwrap(),
        b"published bytes"
    );
    inventory(h.path())
        .reference("actual-stage-report", &[])
        .unwrap();
    maintain_fixture_to(h.path(), 0);
    assert!(!h.path().join(path).exists());
}

#[test]
fn unfinished_directory_recovery_accounts_beyond_one_round_without_publishing_completion() {
    if !isolated_capability_test(
        "unfinished_directory_recovery_accounts_beyond_one_round_without_publishing_completion",
    ) {
        return;
    }
    let h = host(2, 16);
    let path = ".chrono-harness/state/fixtures/recovered-rounds";
    let publication = inventory(h.path())
        .plan_rooted(
            "fixture",
            path,
            json!({"completion":"not-established"}),
            "domain-recovery",
        )
        .unwrap();
    let id = publication.id().to_owned();
    publication.materialize().unwrap();
    for name in ["first", "second", "third"] {
        publication
            .write_file(&h.path().join(path).join(name), name.as_bytes())
            .unwrap();
    }
    drop(publication);
    let repaired = std::cell::Cell::new(0);
    let result = inventory(h.path()).recover_unfinished_directory("fixture", path, || {
        repaired.set(repaired.get() + 1);
        fs::write(h.path().join(path).join("recovery-result"), b"repaired")
            .map_err(|error| error.to_string())
    });
    let publication = match result {
        Ok(Some(publication)) => publication,
        Ok(None) => panic!("the original publication must remain adopted"),
        Err(error) => {
            let original = h.keep();
            panic!("bounded recovery must continue accounting: {error}; originals: {original:?}");
        }
    };
    assert_eq!(publication.id(), id);
    assert_eq!(
        repaired.get(),
        1,
        "accounting must not repeat domain repair"
    );
    let before = ledger(h.path());
    let original = before["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["id"] == id)
        .unwrap();
    assert_eq!(original["phase"], "publishing");
    assert_eq!(original["outcome"]["completion"], "not-established");
    assert_eq!(
        inventory(h.path())
            .rooted_original("fixture", "domain-recovery")
            .unwrap()
            .unwrap()
            .0,
        id
    );
    publication
        .write_file(&h.path().join(path).join("final"), b"published")
        .unwrap();
    publication
        .complete_sealed(json!({"publication":"committed"}))
        .unwrap();
    for (name, expected) in [
        ("first", b"first".as_slice()),
        ("second", b"second".as_slice()),
        ("third", b"third".as_slice()),
        ("recovery-result", b"repaired".as_slice()),
        ("final", b"published".as_slice()),
    ] {
        assert_eq!(fs::read(h.path().join(path).join(name)).unwrap(), expected);
    }
    let after = ledger(h.path());
    let original = after["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["id"] == id)
        .unwrap();
    assert_eq!(original["phase"], "released");
    assert_eq!(original["manifest"]["sealed"], true);
    assert_eq!(original["outcome"]["publication"], "committed");
}

#[test]
fn unfinished_directory_recovery_keeps_original_byte_and_manifest_bounds() {
    if !isolated_capability_test(
        "unfinished_directory_recovery_keeps_original_byte_and_manifest_bounds",
    ) {
        return;
    }
    for (kind, byte_limit, node_limit, expected_error) in [
        ("bytes", 4, 16, "output byte capacity exceeded"),
        ("nodes", 1024, 2, "capacity; preserve original"),
    ] {
        let h = host(2, 16);
        let policy_path = h.path().join(POLICY_PATH);
        let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
        policy["max_output_bytes"] = json!(byte_limit);
        policy["max_manifest_nodes"] = json!(node_limit);
        fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        let path = format!(".chrono-harness/state/fixtures/recovery-{kind}");
        let publication = inventory(h.path())
            .plan_rooted(
                "fixture",
                &path,
                json!({"completion":"not-established"}),
                "domain-recovery",
            )
            .unwrap();
        let id = publication.id().to_owned();
        publication.materialize().unwrap();
        drop(publication);
        let result = inventory(h.path()).recover_unfinished_directory("fixture", &path, || {
            fs::write(h.path().join(&path).join("first"), b"12345")
                .and_then(|()| fs::write(h.path().join(&path).join("second"), b"second"))
                .map_err(|error| error.to_string())
        });
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("recovery must preserve the original {kind} bound"),
        };
        assert!(error.contains(expected_error), "{kind}: {error}");
        assert_eq!(
            fs::read(h.path().join(&path).join("first")).unwrap(),
            b"12345"
        );
        assert_eq!(
            fs::read(h.path().join(&path).join("second")).unwrap(),
            b"second"
        );
        let after = ledger(h.path());
        let original = after["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|output| output["id"] == id)
            .unwrap();
        assert_eq!(original["phase"], "publishing");
        assert_eq!(original["outcome"]["completion"], "not-established");
        assert_eq!(
            inventory(h.path())
                .rooted_original("fixture", "domain-recovery")
                .unwrap()
                .unwrap()
                .0,
            id
        );
    }
}

#[test]
fn unfinished_directory_resume_preserves_live_readers_replacements_and_completed_originals() {
    if !isolated_capability_test(
        "unfinished_directory_resume_preserves_live_readers_replacements_and_completed_originals",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/unpublished";
    let publication = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &h.path().join(path),
        true,
        json!({"original_exit":null}),
    )
    .unwrap()
    .unwrap();
    drop(publication);
    let reader = retained_artifacts::retention::read_guard(h.path(), path)
        .unwrap()
        .unwrap();
    assert!(
        inventory(h.path())
            .resume_unfinished_directory("fixture", path)
            .is_err()
    );
    drop(reader);
    fs::rename(
        h.path().join(path),
        h.path().join(format!("{path}-original")),
    )
    .unwrap();
    fs::create_dir(h.path().join(path)).unwrap();
    assert!(
        inventory(h.path())
            .resume_unfinished_directory("fixture", path)
            .is_err()
    );
    assert!(h.path().join(format!("{path}-original")).is_dir());
    assert!(h.path().join(path).is_dir());
    let complete = ".chrono-harness/state/fixtures/complete";
    let publication = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &h.path().join(complete),
        true,
        json!({}),
    )
    .unwrap()
    .unwrap();
    publication.complete(json!({"status":"joined"})).unwrap();
    assert!(
        inventory(h.path())
            .resume_unfinished_directory("fixture", complete)
            .is_err()
    );
}

#[test]
fn live_publication_accepts_exact_caller_without_leaving_a_durable_root() {
    if !isolated_capability_test(
        "live_publication_accepts_exact_caller_without_leaving_a_durable_root",
    ) {
        return;
    }
    use retained_artifacts::retention::{Consumer, create_adopted};
    let h = host(256, 16);
    let source = ".chrono-harness/state/fixtures/caller-original";
    let destination = ".chrono-harness/state/fixtures/caller-output";
    let original = create_adopted(
        h.path(),
        "fixture",
        &h.path().join(source),
        false,
        json!({}),
    )
    .unwrap()
    .unwrap();
    original
        .replace_file(&h.path().join(source), b"actual original")
        .unwrap();
    let identity = original
        .protect_consumer("live-publication-caller")
        .unwrap();
    original.complete(json!({"status":"joined"})).unwrap();
    let consumer = Consumer {
        schema: "chrono-retention-consumer/v1".into(),
        consumer: "live-publication-caller".into(),
        purpose: "Consume the exact original in a live parent".into(),
        originals: vec![identity],
    };
    let parent = create_adopted(
        h.path(),
        "fixture",
        &h.path().join(destination),
        true,
        json!({}),
    )
    .unwrap()
    .unwrap();
    // A path alone must not acquire the live producer's authority.
    assert!(
        inventory(h.path())
            .transfer_consumer(&consumer, destination)
            .is_err()
    );
    let mut changed = consumer.clone();
    changed.originals[0].identity = "different-original".into();
    assert!(parent.accept_consumer(&changed).is_err());
    maintenance(h.path());
    assert_eq!(fs::read(h.path().join(source)).unwrap(), b"actual original");
    parent.accept_consumer(&consumer).unwrap();
    assert!(ledger(h.path())["roots"].as_object().unwrap().is_empty());
    maintenance(h.path());
    assert!(h.path().join(source).exists());
    parent.complete(json!({"status":"joined"})).unwrap();
    maintain_fixture_to(h.path(), 0);
    assert!(!h.path().join(source).exists());
    assert!(!h.path().join(destination).exists());
}

#[test]
fn materialized_originals_extend_the_owners_reference_before_content_and_release_by_scope() {
    if !isolated_capability_test(
        "materialized_originals_extend_the_owners_reference_before_content_and_release_by_scope",
    ) {
        return;
    }
    let h = host(256, 16);
    let role = "fixture-domain:address";
    let first = ".chrono-harness/state/fixtures/first";
    let second = ".chrono-harness/state/fixtures/second";
    for path in [first, second] {
        let publication = inventory(h.path())
            .plan("fixture", path, false, json!({"status":"unknown"}))
            .unwrap();
        publication.materialize_for_reference(role).unwrap();
        let state = ledger(h.path());
        assert!(
            state["roots"][role]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id == publication.id())
        );
        publication
            .replace_file(&h.path().join(path), path.as_bytes())
            .unwrap();
        assert!(
            !inventory(h.path())
                .published_original(
                    path,
                    &chrono_harness::sha256(path.as_bytes()),
                    path.len() as u64
                )
                .unwrap()
        );
        publication.complete(json!({"status":"published"})).unwrap();
        assert!(
            inventory(h.path())
                .published_original(
                    path,
                    &chrono_harness::sha256(path.as_bytes()),
                    path.len() as u64
                )
                .unwrap()
        );
        assert!(
            !inventory(h.path())
                .published_original(path, &chrono_harness::sha256(b"changed"), 7)
                .unwrap()
        );
    }
    assert_eq!(ledger(h.path())["roots"][role].as_array().unwrap().len(), 2);
    inventory(h.path()).maintain().unwrap();
    assert_eq!(fs::read(h.path().join(first)).unwrap(), first.as_bytes());
    assert_eq!(fs::read(h.path().join(second)).unwrap(), second.as_bytes());
    inventory(h.path())
        .reference_paths(role, &[second.into()])
        .unwrap();
    let activity = retained_artifacts::retention::read_scope(h.path()).unwrap();
    drop(retained_artifacts::retention::read_guard(h.path(), first).unwrap());
    inventory(h.path()).maintain().unwrap();
    assert!(
        h.path().join(first).is_file(),
        "returning caller still consumes the released original"
    );
    drop(activity);
    // Disposal can stop at the policy's round limit; the remaining rooted
    // original must stay protected throughout ordinary continuation rounds.
    let released = maintain_fixture_to(h.path(), 1);
    assert!(!h.path().join(first).exists(), "{released}");
    assert!(h.path().join(second).is_file());
    inventory(h.path()).reference_paths(role, &[]).unwrap();
    let unreferenced = maintain_fixture_to(h.path(), 0);
    assert!(!h.path().join(second).exists(), "{unreferenced}");
}

#[test]
fn diagnostic_window_expires_without_releasing_an_actual_consumer() {
    if !isolated_capability_test("diagnostic_window_expires_without_releasing_an_actual_consumer") {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/failed-original";
    fs::write(h.path().join(path), b"original failed output\0\xff").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", path, json!(null))
        .unwrap();
    let inventory = inventory(h.path());
    let id = publication.id().to_owned();
    inventory.retain_diagnostic_path(path).unwrap();
    let publishing = ledger(h.path())["outputs"][0].clone();
    assert_eq!(publishing["phase"], "publishing");
    assert_eq!(publishing["outcome"], Value::Null);
    assert!(publishing["expires"].as_u64().unwrap() > 0);
    inventory.maintain().unwrap();
    assert_eq!(
        fs::read(h.path().join(path)).unwrap(),
        b"original failed output\0\xff"
    );
    publication.complete(json!({"exit_code":17})).unwrap();
    inventory.retain_diagnostic_path(path).unwrap();
    inventory.maintain().unwrap();
    assert_eq!(
        fs::read(h.path().join(path)).unwrap(),
        b"original failed output\0\xff"
    );
    inventory
        .reference_paths("actual-reader", &[path.into()])
        .unwrap();
    let mut output = ledger(h.path())["outputs"][0].clone();
    assert_eq!(output["id"], id);
    assert_eq!(output["outcome"]["exit_code"], 17);
    output["expires"] = json!(0);
    retention_state::replace_output(h.path(), &output);
    inventory.maintain().unwrap();
    assert!(h.path().join(path).exists());
    inventory.reference_paths("actual-reader", &[]).unwrap();
    maintain_fixture_to(h.path(), 0);
    assert!(!h.path().join(path).exists());
    assert!(inventory.retain_diagnostic_path(path).is_err());
}

#[test]
fn busy_or_damaged_output_metadata_does_not_block_an_independent_publisher() {
    if !isolated_capability_test(
        "busy_or_damaged_output_metadata_does_not_block_an_independent_publisher",
    ) {
        return;
    }
    use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
    for damaged in [false, true] {
        let h = host(256, 16);
        let first_path = tree(h.path(), "independent-a");
        fs::write(h.path().join(&first_path).join("body"), b"required A input").unwrap();
        let first = inventory(h.path())
            .begin("fixture", &first_path, json!(null))
            .unwrap();
        let id = first.id().to_owned();
        first.complete(json!({"exit": 7})).unwrap();
        let directory = h
            .path()
            .join(".chrono-harness/state/output-retention-v1/objects")
            .join(&id);
        let record = directory.join("record.json");
        assert!(
            record.is_file(),
            "each output must own its addressed declaration"
        );
        let original = fs::read(&record).unwrap();
        let guard = if damaged {
            fs::write(&record, b"damaged independent metadata").unwrap();
            None
        } else {
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
                .open(&directory)
                .unwrap();
            assert_eq!(
                unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
                0
            );
            Some(file)
        };
        let second_path = tree(h.path(), "independent-b");
        let began = std::time::Instant::now();
        let publisher = inventory(h.path());
        let adopted = began.elapsed();
        let second = publisher
            .begin("fixture", &second_path, json!(null))
            .unwrap();
        let reserved = began.elapsed();
        second.complete(json!({"exit": 101})).unwrap();
        let completed = began.elapsed();
        assert!(
            completed < std::time::Duration::from_secs(5),
            "damaged={damaged}; adopt={adopted:?}; reserve={:?}; complete={:?}; total={completed:?}",
            reserved - adopted,
            completed - reserved
        );
        let rounds =
            maintain_fixture_rounds_until(h.path(), "independent physical disposal", |_| {
                !h.path().join(&second_path).exists()
            });
        assert!(
            !h.path().join(&second_path).exists(),
            "independent disposal must progress: {rounds:?}"
        );
        assert_eq!(
            fs::read(h.path().join(&first_path).join("body")).unwrap(),
            b"required A input"
        );
        assert_eq!(
            fs::read(&record).unwrap(),
            if damaged {
                b"damaged independent metadata".to_vec()
            } else {
                original.clone()
            }
        );
        drop(guard);
        if damaged {
            fs::write(&record, original).unwrap();
        }
        assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    }
}
#[test]
fn one_node_round_retires_empty_role_without_touching_its_replacement() {
    if !isolated_capability_test(
        "one_node_round_retires_empty_role_without_touching_its_replacement",
    ) {
        return;
    }
    let h = host(1, 16);
    let owner = inventory(h.path());
    let name = "consumer:one-node-retirement";
    owner.reference(name, &[]).unwrap();
    let record = retention_state::role_path(h.path(), name);
    let original = record.parent().unwrap();
    let roles = original.parent().unwrap();
    let mut role: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    role["metadata_expires"] = json!(1);
    fs::write(&record, serde_json::to_vec(&role).unwrap()).unwrap();

    let first = owner.maintain_at(1).unwrap();
    assert_eq!(first["nodes"], 1, "{first}");
    assert!(
        !original.exists(),
        "expired role must move under its original exclusion"
    );
    let retiring = fs::read_dir(roles).unwrap().next().unwrap().unwrap().path();
    assert!(retiring.join("record.json").is_file());

    owner.reference(name, &[]).unwrap();
    let replacement = fs::read(&record).unwrap();
    for directory_present in [true, false] {
        let round = inventory(h.path()).maintain_explicit().unwrap();
        assert!(round["round_maxima"]["nodes"].as_u64().unwrap() <= 1);
        assert!(!retiring.join("record.json").exists());
        assert_eq!(retiring.exists(), directory_present);
        assert_eq!(fs::read(&record).unwrap(), replacement);
    }
    assert!(!retiring.exists());
}

#[test]
fn interrupted_empty_role_birth_recovers_original_deadline_then_retires() {
    if !isolated_capability_test(
        "interrupted_empty_role_birth_recovers_original_deadline_then_retires",
    ) {
        return;
    }
    for expires in [1u64, u64::MAX] {
        let h = host(256, 16);
        let owner = inventory(h.path());
        let name = "consumer:interrupted-birth";
        owner.reference(name, &[]).unwrap();
        let record = retention_state::role_path(h.path(), name);
        let pending = record.with_extension("pending");
        let mut role: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
        role["metadata_expires"] = json!(expires);
        let raw = serde_json::to_vec(&role).unwrap();
        fs::write(&record, &raw).unwrap();
        fs::rename(&record, &pending).unwrap();
        let round = owner.maintain_explicit().unwrap();
        if expires == 1 {
            assert!(
                !record.parent().unwrap().exists(),
                "expired empty birth must retire: {round}"
            );
        } else {
            assert_eq!(
                fs::read(&record).unwrap(),
                raw,
                "resume must not renew or reconstruct the declaration: {round}"
            );
            assert!(!pending.exists());
        }
    }
}

#[test]
fn interrupted_role_birth_preserves_unknown_busy_and_authoritative_records() {
    if !isolated_capability_test(
        "interrupted_role_birth_preserves_unknown_busy_and_authoritative_records",
    ) {
        return;
    }
    for fault in [
        "missing",
        "empty",
        "partial",
        "identity",
        "role",
        "schema",
        "members",
        "attached",
        "summaries",
        "observation",
        "link",
        "busy",
        "published",
    ] {
        let h = host(256, 16);
        let owner = inventory(h.path());
        let name = "consumer:interrupted-birth-boundary";
        owner.reference(name, &[]).unwrap();
        let record = retention_state::role_path(h.path(), name);
        let pending = record.with_extension("pending");
        let mut role: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
        role["metadata_expires"] = json!(u64::MAX);
        let authoritative = serde_json::to_vec(&role).unwrap();
        fs::write(&record, &authoritative).unwrap();
        fs::rename(&record, &pending).unwrap();
        match fault {
            "missing" => fs::remove_file(&pending).unwrap(),
            "empty" => fs::write(&pending, b"").unwrap(),
            "partial" => fs::write(&pending, b"{\"schema\":").unwrap(),
            "identity" => {
                role["directory_identity"] = json!("changed");
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "role" => {
                role["role"] = json!("consumer:other-address");
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "schema" => {
                role["schema"] = json!("unknown");
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "members" => {
                role["ids"] = json!(["output-unresolved"]);
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "attached" => {
                role["attached"] = json!(["output-unresolved"]);
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "summaries" => {
                role["summaries"] = json!([{"unresolved":"original"}]);
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "observation" => {
                role["observation"] = json!({});
                fs::write(&pending, serde_json::to_vec(&role).unwrap()).unwrap();
            }
            "link" => {
                fs::rename(&pending, h.path().join("other-original")).unwrap();
                std::os::unix::fs::symlink(h.path().join("other-original"), &pending).unwrap();
            }
            "published" => {
                fs::write(&record, &authoritative).unwrap();
                fs::write(&pending, b"partial replacement").unwrap();
            }
            _ => (),
        }
        let original_pending = fs::read(&pending).ok();
        let gate = (fault == "busy").then(|| {
            chrono_harness::ownership::DirectoryLease::acquire(record.parent().unwrap(), true, None)
                .unwrap()
                .unwrap()
        });
        let round = owner.maintain_explicit().unwrap();
        assert!(
            record.parent().unwrap().exists(),
            "unknown or protected birth must remain: {fault}: {round}"
        );
        assert_eq!(
            fs::read(&pending).ok(),
            original_pending,
            "{fault}: {round}"
        );
        if fault == "published" {
            assert_eq!(fs::read(&record).unwrap(), authoritative);
        } else {
            assert!(
                !record.exists(),
                "must not publish an unqualified pending: {fault}: {round}"
            );
        }
        if let Some(gate) = gate {
            gate.stable().unwrap();
        }
    }
}

#[test]
fn interrupted_consumer_attachment_recovers_and_expired_addressed_metadata_retires() {
    if !isolated_capability_test(
        "interrupted_consumer_attachment_recovers_and_expired_addressed_metadata_retires",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(0);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let path = tree(h.path(), "interrupted-consumer");
    let p = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    let id = p.id().to_owned();
    p.complete(json!({"exit":101})).unwrap();
    inventory(h.path())
        .reference("consumer:interrupted", &[id.clone()])
        .unwrap();
    let role = retention_state::role_path(h.path(), "consumer:interrupted");
    let mut interrupted: Value = serde_json::from_slice(&fs::read(&role).unwrap()).unwrap();
    // The source commit is durable; inbound attachment cleanup was interrupted.
    interrupted["ids"] = json!([]);
    fs::write(&role, serde_json::to_vec(&interrupted).unwrap()).unwrap();
    let sibling_path = tree(h.path(), "still-consumed");
    let sibling = inventory(h.path())
        .begin("fixture", &sibling_path, json!(null))
        .unwrap();
    for _ in 0..4 {
        let r = maintenance(h.path());
        assert!(r["nodes"].as_u64().unwrap() <= 256, "{r}");
    }
    assert!(!h.path().join(path).exists());
    assert!(
        !retention_state::object_path(h.path(), &id)
            .parent()
            .unwrap()
            .exists()
    );
    assert!(!role.parent().unwrap().exists());
    assert!(h.path().join(&sibling_path).exists());
    assert!(retention_state::object_path(h.path(), sibling.id()).is_file());
    sibling.complete(json!({"exit":7})).unwrap();
}

#[test]
fn independent_components_share_the_existing_round_verification_byte_budget() {
    if !isolated_capability_test(
        "independent_components_share_the_existing_round_verification_byte_budget",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_bytes_per_round"] = json!(8);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    for n in 0..2 {
        let path = format!(".chrono-harness/state/fixtures/budget-{n}");
        fs::write(h.path().join(&path), [n as u8; 8]).unwrap();
        inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit":n}))
            .unwrap();
    }
    assert_eq!(ledger(h.path())["outputs"].as_array().unwrap().len(), 2);
    // A round may stop at its existing time or node bound before disposal.
    // Keep the byte bound shared across both components in every real round.
    let rounds = maintain_fixture_rounds_to(h.path(), 0);
    assert!(rounds.len() >= 2, "{rounds:?}");
    assert_eq!(
        rounds
            .iter()
            .map(|round| round["verified_bytes"].as_u64().unwrap())
            .sum::<u64>(),
        16,
        "{rounds:?}"
    );
    let mut disposed = 0;
    for round in &rounds {
        let count = round["disposed"].as_array().unwrap().len();
        assert!(count <= 1, "{round}");
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        disposed += count;
    }
    assert_eq!(disposed, 2, "{rounds:?}");
    for n in 0..2 {
        assert!(
            !h.path()
                .join(format!(".chrono-harness/state/fixtures/budget-{n}"))
                .exists(),
            "{rounds:?}"
        );
    }
}

#[test]
fn isolated_capability_driver_rejects_a_successful_zero_test_process() {
    if !isolated_capability_test(
        "isolated_capability_driver_rejects_a_successful_zero_test_process",
    ) {
        return;
    }
    let failure =
        std::panic::catch_unwind(|| isolated_capability_test("__no_declared_retention_test__"))
            .unwrap_err();
    let message = failure.downcast_ref::<String>().unwrap();
    assert!(message.contains("exit status: 0"), "{message}");
    assert!(
        message.contains("test result: ok. 0 passed; 0 failed;"),
        "{message}"
    );
}
#[test]
fn creation_intent_survives_no_finish_and_admission_creates_nothing() {
    if !isolated_capability_test("creation_intent_survives_no_finish_and_admission_creates_nothing")
    {
        return;
    }
    let mut h = host(256, 16);
    h.disable_cleanup(true);
    eprintln!(
        "retention fixture originals (creation_intent_survives_no_finish_and_admission_creates_nothing): {}",
        h.path().display()
    );
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
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    drop(publication);
    let round = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &round), 0, "{round}");
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["original"]["operation"],
        "create"
    );
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(1);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    inventory(h.path()).migrate_policy(&serde_json::to_vec(&json!({"schema":"chrono-output-retention/v2","producers":{"fixture":[".chrono-harness/state/fixtures/"]},"keep_seconds":0,"max_related_outputs":32,"max_summaries":16,"max_nodes_per_round":256,"max_bytes_per_round":1048576,"max_millis_per_round":1000})).unwrap(), None).unwrap();
    let live = inventory(h.path())
        .plan("fixture", path, true, json!(null))
        .unwrap();
    let refused = h.path().join(".chrono-harness/state/fixtures/refused");
    let independent = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &refused,
        true,
        json!(null),
    )
    .unwrap()
    .unwrap();
    assert!(refused.is_dir());
    independent.complete(json!({"exit":0})).unwrap();
    drop(live);
    h.disable_cleanup(false);
}

#[test]
fn controlled_fixture_writes_refuse_before_copy_and_keep_original_outcomes() {
    if !isolated_capability_test(
        "controlled_fixture_writes_refuse_before_copy_and_keep_original_outcomes",
    ) {
        return;
    }
    let h = host(3, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);

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
    let mut rounds = Vec::new();
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut removed = 0;
        for _ in 0..8 {
            let round = maintenance(h.path());
            assert!(round["nodes"].as_u64().unwrap() <= 3, "{round}");
            removed += round["removed_logical_bytes"].as_u64().unwrap();
            let completed = round["remaining_outputs"] == 0;
            rounds.push(round);
            if completed {
                break;
            }
        }
        assert_eq!(removed, 128, "{rounds:?}");
        assert_eq!(rounds.last().unwrap()["remaining_outputs"], 0);
        assert!(!path.exists());
        let state = ledger(h.path());
        let summaries = state["summaries"].as_array().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0]["original_outcome"]["exit"], 101);
    }));
    if let Err(failure) = verification {
        let original = ledger(h.path());
        let path = h.keep();
        eprintln!(
            "controlled fixture retirement failed; rounds: {rounds:?}; inventory: {original}; retained fixture: {}",
            path.display()
        );
        std::panic::resume_unwind(failure);
    }
}

#[test]
fn released_byte_overflow_is_accounted_and_reclaimed_under_unchanged_policy() {
    if !isolated_capability_test(
        "released_byte_overflow_is_accounted_and_reclaimed_under_unchanged_policy",
    ) {
        return;
    }
    let h = host(8, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);

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
    let accounted = maintenance(h.path());
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
        let round = maintenance(h.path());
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
    if !isolated_capability_test(
        "released_original_larger_than_verification_round_is_not_permanently_blocked",
    ) {
        return;
    }
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
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    drop(reader);
    let round = maintenance(h.path());
    if round["remaining_outputs"] != 0 {
        let original = ledger(h.path());
        let path = h.keep();
        panic!(
            "released original was not retired: {round}; inventory: {original}; retained fixture: {}",
            path.display()
        );
    }
    assert_eq!(fixture_remaining(h.path(), &round), 0, "{round}");
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
fn returned_metadata_exclusion_is_released_while_an_unrelated_fork_waits_for_exec() {
    if !isolated_capability_test(
        "returned_metadata_exclusion_is_released_while_an_unrelated_fork_waits_for_exec",
    ) {
        return;
    }
    use std::{
        io::{Read, Write},
        os::{fd::AsRawFd, unix::net::UnixStream, unix::process::CommandExt},
    };
    let h = host(256, 16);
    let path = tree(h.path(), "synchronous-metadata");
    let owner = inventory(h.path());
    let publication = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    let id = publication.id().to_owned();
    publication.complete(json!({"exit":7})).unwrap();
    let role = "current-synchronous-metadata";
    owner.reference(role, &[id.clone()]).unwrap();
    let role_record = retention_state::role_path(h.path(), role);
    let (mut parent, child) = UnixStream::pair().unwrap();
    let parent_fd = parent.as_raw_fd();
    let child_fd = child.as_raw_fd();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"));
    command.arg("exit-success");
    unsafe {
        command.pre_exec(move || {
            libc::close(parent_fd);
            let mut byte = 1u8;
            if libc::write(child_fd, (&byte as *const u8).cast(), 1) != 1
                || libc::read(child_fd, (&mut byte as *mut u8).cast(), 1) != 1
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let (excluded, signaled, during, snapshot, exact, role_available, resumed, joined) =
        std::thread::scope(|scope| {
            let mut launched = None;
            let mut signaled = None;
            let mut during = None;
            let excluded = owner.metadata_observed(&path, &[role.into()], || {
                launched = Some(scope.spawn(|| command.output()));
                signaled = Some(parent.read_exact(&mut [0]));
                during = Some(owner.producer_status("fixture", Some(&path)));
            });
            // Keep the unrelated child before exec until all observations have
            // returned. No retry, process probe or sleep can close its fd copies.
            let snapshot = owner.producer_status("fixture", None);
            let exact = owner.producer_status("fixture", Some(&path));
            let role_available = chrono_harness::ownership::DirectoryLease::acquire(
                role_record.parent().unwrap(),
                true,
                None,
            )
            .map(|lease| lease.is_some());
            let resumed = parent.write_all(&[1]);
            let joined = launched.map(|child| child.join());
            (
                excluded,
                signaled,
                during,
                snapshot,
                exact,
                role_available,
                resumed,
                joined,
            )
        });
    excluded.unwrap();
    signaled.unwrap().unwrap();
    resumed.unwrap();
    assert!(joined.unwrap().unwrap().unwrap().status.success());
    assert!(during.unwrap().unwrap_err().contains("busy"));
    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot["scope_complete"], true, "{snapshot}");
    assert_eq!(snapshot["errors"], json!([]), "{snapshot}");
    assert_eq!(snapshot["records"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["records"][0]["id"], id);
    let exact = exact.unwrap();
    assert_eq!(exact["scope_complete"], true, "{exact}");
    assert_eq!(exact["records"][0]["path"], path);
    assert!(role_available.unwrap());
}

#[test]
fn unrelated_fork_retains_a_reader_until_exec_closes_its_capability() {
    if !isolated_capability_test("unrelated_fork_retains_a_reader_until_exec_closes_its_capability")
    {
        return;
    }
    use std::{
        io::{Read, Write},
        os::{fd::AsRawFd, unix::net::UnixStream, unix::process::CommandExt},
    };
    let h = host(8, 4);
    let path = ".chrono-harness/state/fixtures/fork-original";
    fs::write(h.path().join(path), [9; 4096]).unwrap();
    let p = inventory(h.path())
        .begin("fixture", path, json!(null))
        .unwrap();
    let id = p.id().to_string();
    p.complete(json!({"exit":101})).unwrap();
    let reader = inventory(h.path()).consume(&id).unwrap();
    let (mut parent, child) = UnixStream::pair().unwrap();
    let parent_fd = parent.as_raw_fd();
    let child_fd = child.as_raw_fd();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"));
    command.arg("exit-success");
    unsafe {
        command.pre_exec(move || {
            libc::close(parent_fd);
            let mut byte = 1u8;
            if libc::write(child_fd, (&byte as *const u8).cast(), 1) != 1
                || libc::read(child_fd, (&mut byte as *mut u8).cast(), 1) != 1
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let result = std::thread::scope(|scope| {
        let launched = scope.spawn(|| command.output());
        let signaled = parent.read_exact(&mut [0]);
        drop(reader);
        let protected = inventory(h.path()).maintain();
        let resumed = parent.write_all(&[1]);
        let joined = launched.join();
        (signaled, protected, resumed, joined)
    });
    result.0.unwrap();
    let protected = result.1.unwrap();
    result.2.unwrap();
    assert!(result.3.unwrap().unwrap().status.success());
    assert_eq!(protected["remaining_outputs"], 1, "{protected}");
    assert!(
        protected["protected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| {
                row["id"] == id && row["reason"] == "live producer/consumer descendants"
            }),
        "the inherited capability must be the recorded protection reason: {protected}"
    );
    assert_eq!(fs::read(h.path().join(path)).unwrap(), [9; 4096]);
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["exit"],
        101
    );
}

#[test]
fn killed_creation_intent_recovers_on_ordinary_entry_without_a_callback() {
    if !isolated_capability_test(
        "killed_creation_intent_recovers_on_ordinary_entry_without_a_callback",
    ) {
        return;
    }
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
    let live = maintenance(h.path());
    assert_eq!(live["remaining_outputs"], 1);
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    for _ in 0..4 {
        if maintenance(h.path())["remaining_outputs"] == 0 {
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
    if !isolated_capability_test(
        "released_manifest_overflow_makes_bounded_progress_without_policy_growth",
    ) {
        return;
    }
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
    maintenance(h.path());
    assert_eq!(fs::read_dir(h.path().join(&path)).unwrap().count(), 20);
    inventory(h.path()).reference("build-reader", &[]).unwrap();
    let mut removed = 0;
    for _ in 0..64 {
        let round = maintenance(h.path());
        assert!(round["nodes"].as_u64().unwrap() <= 4);
        assert!(
            round["metadata_bytes"].as_u64().unwrap()
                <= policy["max_metadata_bytes_per_round"]
                    .as_u64()
                    .unwrap_or(64 * 1024 * 1024)
        );
        removed += round["removed_logical_bytes"].as_u64().unwrap();
        if round["remaining_outputs"] == 0 && round["remaining_capability_retirements"] == 0 {
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
    if !isolated_capability_test(
        "creation_destination_replacement_is_preserved_and_controlled_nodes_are_refused",
    ) {
        return;
    }
    let h = host(8, 4);
    let path = ".chrono-harness/state/fixtures/planned";
    let p = inventory(h.path())
        .plan("fixture", path, false, json!({"exit":null}))
        .unwrap();
    fs::write(h.path().join(path), b"unrelated replacement").unwrap();
    assert!(p.materialize().is_err());
    drop(p);
    let round = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &round), 1);
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
    if !isolated_capability_test(
        "preparation_write_refusal_leaves_owner_intent_and_ordinary_recovery",
    ) {
        return;
    }
    let h = report_host();
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);

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
    maintenance(h.path());
    assert!(h.path().join(path).exists());
    assert_eq!(fs::metadata(h.path().join(path)).unwrap().len(), 0);
    assert_eq!(
        ledger(h.path())["outputs"][0]["outcome"]["completion"],
        "not-established"
    );
    assert!(
        ledger(h.path())["roots"]
            .get(format!(
                "consumer:publication-recheck:{}",
                before["outputs"][0]["id"].as_str().unwrap()
            ))
            .is_some()
    );
}

#[test]
fn ordinary_entry_reclaims_completed_nested_outputs_with_real_savings() {
    if !isolated_capability_test(
        "ordinary_entry_reclaims_completed_nested_outputs_with_real_savings",
    ) {
        return;
    }
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
    let mut removed = 0;
    let mut result = Value::Null;
    for _ in 0..8 {
        result = maintenance(h.path());
        removed += result["removed_logical_bytes"].as_u64().unwrap();
        if result["remaining_outputs"] == 0 {
            break;
        }
    }
    assert!(!h.path().join(p).exists());
    assert_eq!(removed, 2 * 1024 * 1024);
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
    if !isolated_capability_test(
        "no_finish_preserves_live_producer_then_recovers_unknown_partial_publication",
    ) {
        return;
    }
    // Exercise the owner's bounded continuation instead of relying on all
    // manifest and disposal IO finishing within a single maintenance round.
    let h = host(3, 16);
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
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(h.path().join(&p).exists());
    // Simulate the kernel capability closing without any finish/completion callback.
    drop(publication);
    let mut r = maintenance(h.path());
    assert!(r["nodes"].as_u64().unwrap() <= 3, "{r}");
    assert_eq!(fixture_remaining(h.path(), &r), 1, "{r}");
    for _ in 0..8 {
        r = maintenance(h.path());
        assert!(r["nodes"].as_u64().unwrap() <= 3, "{r}");
        if r["remaining_outputs"] == 0 {
            break;
        }
    }
    if r["remaining_outputs"] != 0 {
        let original = ledger(h.path());
        let path = h.keep();
        panic!(
            "released partial publication was not retired: {r}; inventory: {original}; retained fixture: {}",
            path.display()
        );
    }
    assert_eq!(fixture_remaining(h.path(), &r), 0, "{r}");
    assert!(!h.path().join(&p).exists());
    assert_eq!(
        ledger(h.path())["summaries"][0]["original_outcome"]["completion"],
        "not-established"
    );
    assert!(inventory(h.path()).consume(&id).is_err());
}
#[test]
fn current_references_protect_closure_and_released_historical_cycles_are_not_roots() {
    if !isolated_capability_test(
        "current_references_protect_closure_and_released_historical_cycles_are_not_roots",
    ) {
        return;
    }
    let h = host(3, 16);
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
    let mut rounds = Vec::new();
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        i.reference("pending-native-rerun", &[aid.clone()]).unwrap();
        // Selecting the rooted member must also work when its cyclic original
        // is intentionally unvisited. Directory enumeration order must not be
        // what makes the rooted frontier valid.
        let selected = i.maintain_originals(&[aid.clone()]).unwrap();
        assert!(
            selected["accounting_errors"].as_array().unwrap().is_empty(),
            "{selected}"
        );
        assert_eq!(selected["metadata_inventory_entries"], 1, "{selected}");
        assert_eq!(selected["scope"]["complete"], false, "{selected}");
        assert_eq!(selected["remaining_outputs"], Value::Null, "{selected}");
        assert!(h.path().join(&a).is_dir() && h.path().join(&b).is_dir());
        rounds.push(i.maintain().unwrap());
        assert_eq!(rounds[0]["remaining_outputs"], 2);
        assert!(rounds[0]["nodes"].as_u64().unwrap() <= 3);
        assert!(h.path().join(&a).is_dir() && h.path().join(&b).is_dir());
        i.reference("pending-native-rerun", &[]).unwrap();
        // Maintenance does actual bounded work on each call. Releasing the
        // reference makes the cycle reclaimable, without promising that all
        // manifest and disposal IO fits in a single round.
        for _ in 0..8 {
            let round = maintenance(h.path());
            assert!(round["nodes"].as_u64().unwrap() <= 3, "{round}");
            let completed = round["remaining_outputs"] == 0;
            rounds.push(round);
            if completed {
                break;
            }
        }
        assert_ne!(rounds[1]["remaining_outputs"], 0, "{rounds:?}");
        assert_eq!(rounds.last().unwrap()["remaining_outputs"], 0, "{rounds:?}");
        assert!(!h.path().join(&a).exists() && !h.path().join(&b).exists());
        let s = ledger(h.path());
        let exits: Vec<_> = s["summaries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["original_outcome"]["exit"].as_i64().unwrap())
            .collect();
        assert_eq!(exits.len(), 2);
        assert!(exits.contains(&0) && exits.contains(&9));
    }));
    if let Err(failure) = verification {
        let original = ledger(h.path());
        let path = h.keep();
        eprintln!(
            "historical cycle verification failed; rounds: {rounds:?}; inventory: {original}; retained fixture: {}",
            path.display()
        );
        std::panic::resume_unwind(failure);
    }
}
#[test]
fn live_consumer_protects_references_even_after_retention_window_expires() {
    if !isolated_capability_test(
        "live_consumer_protects_references_even_after_retention_window_expires",
    ) {
        return;
    }
    let h = host(3, 16);
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
    i.dependencies(&aid, &[bid.clone()]).unwrap();
    let lease = i.consume(&aid).unwrap();
    let mut rounds = Vec::new();
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Visit both outputs under the small node budget. A live consumer must
        // protect the referenced original as well as its own output.
        for _ in 0..2 {
            rounds.push(i.maintain().unwrap());
            let round = rounds.last().unwrap();
            assert!(round["nodes"].as_u64().unwrap() <= 3, "{round}");
            assert_eq!(fixture_remaining(h.path(), &round), 2, "{round}");
            assert!(h.path().join(&a).is_dir() && h.path().join(&b).is_dir());
        }
        drop(lease);
        // Release ends protection; retirement still obeys the same per-round
        // bounds and may resume through subsequent ordinary entries.
        for _ in 0..8 {
            let round = maintenance(h.path());
            assert!(round["nodes"].as_u64().unwrap() <= 3, "{round}");
            let completed = round["remaining_outputs"] == 0;
            rounds.push(round);
            if completed {
                break;
            }
        }
        assert_eq!(rounds.last().unwrap()["remaining_outputs"], 0, "{rounds:?}");
        assert!(!h.path().join(&a).exists() && !h.path().join(&b).exists());
        let state = ledger(h.path());
        let summaries = state["summaries"].as_array().unwrap();
        assert_eq!(summaries.len(), 2);
        for (id, outcome) in [(&aid, 0), (&bid, 1)] {
            let original = summaries.iter().find(|row| row["id"] == *id).unwrap();
            assert_eq!(original["original_outcome"], outcome);
        }
    }));
    if let Err(failure) = verification {
        let original = ledger(h.path());
        let path = h.keep();
        eprintln!(
            "live consumer verification failed; rounds: {rounds:?}; inventory: {original}; retained fixture: {}",
            path.display()
        );
        std::panic::resume_unwind(failure);
    }
}

#[test]
fn publication_transfers_consumer_before_acquisition_and_release_reclaims_closure() {
    if !isolated_capability_test(
        "publication_transfers_consumer_before_acquisition_and_release_reclaims_closure",
    ) {
        return;
    }
    let h = host(256, 16);
    let receipt = tree(h.path(), "receipt");
    let p = inventory(h.path())
        .begin("fixture", &receipt, json!({"exit":0}))
        .unwrap();
    let receipt_id = p.id().to_owned();
    p.complete(json!({"exit":0})).unwrap();
    let report = tree(h.path(), "report");
    let p = inventory(h.path())
        .begin("fixture", &report, json!(null))
        .unwrap();
    inventory(h.path())
        .dependencies(p.id(), &[receipt_id])
        .unwrap();
    let original = p.protect_consumer("assertions").unwrap();
    let declaration = consumer_input(h.path(), "assertions", json!([original]));
    p.complete(json!({"exit":0})).unwrap();
    // The producer is gone and the caller has not acquired/read the original.
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    let reader = inventory(h.path()).consume(&original.id).unwrap();
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release.clone()).exit_code, 2);
    drop(reader);
    assert_eq!(consumer_call(h.path(), release).exit_code, 0);
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(!h.path().join(report).exists());
    assert!(!h.path().join(receipt).exists());
}

#[test]
fn retained_capacity_round_retires_unrelated_outputs_without_probing_all_leases() {
    if !isolated_capability_test(
        "retained_capacity_round_retires_unrelated_outputs_without_probing_all_leases",
    ) {
        return;
    }
    let h = host(256, 128);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(512);
    policy["max_millis_per_round"] = json!(25);
    policy["max_bytes_per_round"] = json!(67108864);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let mut ids = Vec::new();
    for n in 0..512 {
        let path = format!(".chrono-harness/state/fixtures/retained-{n}");
        fs::write(h.path().join(&path), b"original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        ids.push(publication.id().to_owned());
        publication.complete(json!({"exit": n % 2})).unwrap();
    }
    let i = inventory(h.path());
    i.reference("required-failed-recheck", &[ids[1].clone()])
        .unwrap();
    let reader = i.consume(&ids[0]).unwrap();
    assert_eq!(ledger(h.path())["outputs"].as_array().unwrap().len(), 512);
    let began = std::time::Instant::now();
    let mut rounds = Vec::new();
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // A round may durably stop inside one disposal. Continue actual bounded
        // work through ordinary entries; no clock or capacity is enlarged.
        for _ in 0..8 {
            let round = i.maintain().unwrap();
            assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
            assert!(
                round["verified_bytes"].as_u64().unwrap() <= 67108864,
                "{round}"
            );
            assert_eq!(round["related_output_capacity"], 512);
            for n in 0..2 {
                assert_eq!(
                    fs::read(
                        h.path()
                            .join(format!(".chrono-harness/state/fixtures/retained-{n}"))
                    )
                    .unwrap(),
                    b"original"
                );
            }
            let admitted_room = ledger(h.path())["outputs"].as_array().unwrap().len() < 512;
            rounds.push(round);
            if admitted_room {
                break;
            }
        }
        assert!(
            ledger(h.path())["outputs"].as_array().unwrap().len() < 512,
            "{rounds:?}"
        );
    }));
    if let Err(failure) = verification {
        let original = ledger(h.path());
        let path = h.keep();
        eprintln!(
            "capacity retirement failed; rounds: {rounds:?}; inventory: {original}; retained fixture: {}",
            path.display()
        );
        std::panic::resume_unwind(failure);
    }
    let elapsed = began.elapsed();
    let round = rounds.last().unwrap();
    let path = ".chrono-harness/state/fixtures/next";
    fs::write(h.path().join(path), b"next").unwrap();
    inventory(h.path())
        .begin("fixture", path, json!(null))
        .unwrap()
        .complete(json!({"exit": 0}))
        .unwrap();
    println!(
        "MEASURE retained-capacity outputs=512 policy_millis=25 rounds={} elapsed_micros={} remaining={} nodes={} removed_bytes={}",
        rounds.len(),
        elapsed.as_micros(),
        round["remaining_outputs"],
        round["nodes"],
        round["removed_logical_bytes"]
    );
    drop(reader);
}

#[test]
fn retirement_candidate_preserves_transitive_live_and_unknown_consumers_in_a_cycle() {
    if !isolated_capability_test(
        "retirement_candidate_preserves_transitive_live_and_unknown_consumers_in_a_cycle",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut ids = Vec::new();
    for name in ["input", "middle", "consumer", "unrelated"] {
        let path = tree(h.path(), name);
        let p = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        ids.push(p.id().to_owned());
        p.complete(json!({"exit": 9})).unwrap();
    }
    let i = inventory(h.path());
    i.dependencies(&ids[2], &[ids[1].clone()]).unwrap();
    i.dependencies(&ids[1], &[ids[0].clone()]).unwrap();
    i.dependencies(&ids[0], &[ids[2].clone()]).unwrap();
    let reader = i.consume(&ids[2]).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 3), 3);
    for name in ["input", "middle", "consumer"] {
        assert!(
            h.path()
                .join(format!(".chrono-harness/state/fixtures/{name}"))
                .exists()
        );
    }
    drop(reader);
    let original_lease = h.path().join(format!(
        ".chrono-harness/state/output-retention-v1/{}",
        ids[2]
    ));
    let changed_lease = original_lease.with_extension("preserved");
    fs::rename(&original_lease, &changed_lease).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 3), 3);
    fs::rename(&changed_lease, &original_lease).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(
        ledger(h.path())["summaries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["original_outcome"]["exit"] == 9)
    );
}
#[test]
fn identity_drift_and_changed_policy_preserve_original_evidence() {
    if !isolated_capability_test("identity_drift_and_changed_policy_preserve_original_evidence") {
        return;
    }
    let h = host(256, 16);
    let p = tree(h.path(), "original");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    publication.complete(json!({"exit":4})).unwrap();
    fs::rename(h.path().join(&p), h.path().join("old-output")).unwrap();
    fs::create_dir(h.path().join(&p)).unwrap();
    fs::write(h.path().join(&p).join("new"), b"replacement").unwrap();
    let result = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &result), 1);
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
    let stale = i.maintain().unwrap();
    assert!(
        stale["accounting_errors"]
            .to_string()
            .contains("policy changed"),
        "{stale}"
    );
    let current = maintenance(h.path());
    assert!(
        current["accounting_errors"]
            .to_string()
            .contains("migration required"),
        "{current}"
    );
}
#[test]
fn component_discovery_releases_metadata_when_a_consumer_or_path_is_busy() {
    if !isolated_capability_test(
        "component_discovery_releases_metadata_when_a_consumer_or_path_is_busy",
    ) {
        return;
    }
    use chrono_harness::ownership::DirectoryLease;
    for busy in ["consumer", "path"] {
        let h = host(256, 16);
        let input_path = tree(h.path(), "current-input");
        let input = inventory(h.path())
            .begin_rooted("fixture", &input_path, json!(null), &["current".into()])
            .unwrap();
        let input_id = input.id().to_owned();
        // Refusal must concern the actual report consumer/path. Its unfinished
        // input and that input's consumer remain independently protected.
        let report_path = tree(h.path(), "obsolete-report");
        let report = inventory(h.path())
            .begin_rooted(
                "fixture",
                &report_path,
                json!(null),
                &["report-current".into()],
            )
            .unwrap();
        let report_id = report.id().to_owned();
        inventory(h.path())
            .dependencies(&report_id, &[input_id.clone()])
            .unwrap();
        report.complete(json!({"exit": 17})).unwrap();
        let other_path = tree(h.path(), "independent");
        let other = inventory(h.path())
            .begin("fixture", &other_path, json!(null))
            .unwrap();
        let other_id = other.id().to_owned();
        other.complete(json!({"exit": 0})).unwrap();
        let busy_directory = if busy == "consumer" {
            retention_state::role_path(h.path(), "report-current")
                .parent()
                .unwrap()
                .to_path_buf()
        } else {
            let mut directory = h
                .path()
                .join(".chrono-harness/state/output-retention-v1/paths");
            for component in report_path.split('/') {
                directory.push(chrono_harness::sha256(component.as_bytes()));
            }
            directory
        };
        let holder = DirectoryLease::acquire(&busy_directory, true, None)
            .unwrap()
            .unwrap();
        let began = std::time::Instant::now();
        let round = inventory(h.path())
            .maintain_originals(&[report_id.clone()])
            .unwrap();
        assert!(
            began.elapsed() < std::time::Duration::from_secs(1),
            "{round}"
        );
        assert_eq!(round["scope"]["complete"], false, "{round}");
        assert_eq!(round["remaining_outputs"], Value::Null, "{round}");
        assert_eq!(round["removed_logical_bytes"], 0, "{round}");
        assert_eq!(round["accounting_errors"][0]["id"], report_id, "{round}");
        assert!(h.path().join(&report_path).is_dir());
        for id in [&input_id, &report_id] {
            let directory = retention_state::object_path(h.path(), id)
                .parent()
                .unwrap()
                .to_path_buf();
            let released = DirectoryLease::acquire(&directory, true, None)
                .unwrap()
                .expect("refused maintenance must release every acquired metadata lease");
            drop(released);
        }
        let independent = inventory(h.path()).maintain_originals(&[other_id]).unwrap();
        assert!(!h.path().join(&other_path).exists(), "{independent}");
        drop(holder);
        input.complete(json!({"exit": 7})).unwrap();
        inventory(h.path())
            .reference("report-current", &[])
            .unwrap();
        inventory(h.path())
            .maintain_originals(&[report_id])
            .unwrap();
        assert!(!h.path().join(&report_path).exists());
        assert!(h.path().join(&input_path).is_dir());
    }
}

#[test]
fn bounded_metadata_capacity_still_allows_reference_component_disposal() {
    if !isolated_capability_test(
        "bounded_metadata_capacity_still_allows_reference_component_disposal",
    ) {
        return;
    }
    let h = host(2, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(512);
    policy["max_metadata_bytes_per_round"] = json!(24 * 1024 * 1024);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let report_path = tree(h.path(), "obsolete-report");
    let report = inventory(h.path())
        .begin("fixture", &report_path, json!(null))
        .unwrap();
    let report_id = report.id().to_owned();
    for n in 0..4 {
        fs::write(h.path().join(&report_path).join(n.to_string()), [7; 32]).unwrap();
    }
    let outcome = json!({"exit": 7, "diagnostic": "x".repeat(16300)});
    let mut originals = Vec::new();
    for n in 0..256 {
        let path = format!(".chrono-harness/state/fixtures/required-{n}");
        fs::write(h.path().join(&path), b"current original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete(outcome.clone()).unwrap();
        originals.push((id, path));
    }
    let ids: Vec<_> = originals.iter().map(|(id, _)| id.clone()).collect();
    inventory(h.path())
        .reference("current-recheck", &ids)
        .unwrap();
    inventory(h.path()).dependencies(&report_id, &ids).unwrap();
    report.complete(json!({"exit": 17})).unwrap();
    let role_path = retention_state::role_path(h.path(), "current-recheck");
    let role_before = fs::read(&role_path).unwrap();

    // Every declaration and outcome comes from the real producer. With the
    // smallest valid metadata capacity, preparing this closed component must
    // leave enough capacity to carry out the requested two disposal steps.
    let round = inventory(h.path())
        .maintain_originals(&[report_id.clone()])
        .unwrap();
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert_eq!(round["nodes"], 2, "{round}");
    assert_eq!(round["removed_logical_bytes"], 32, "{round}");
    assert!(round["metadata_bytes"].as_u64().unwrap() <= 24 * 1024 * 1024);
    assert!(h.path().join(&report_path).is_dir());
    assert_eq!(fs::read(role_path).unwrap(), role_before);
    for (id, path) in originals {
        assert_eq!(fs::read(h.path().join(path)).unwrap(), b"current original");
        let record: Value =
            serde_json::from_slice(&fs::read(retention_state::object_path(h.path(), &id)).unwrap())
                .unwrap();
        assert_eq!(record["output"]["outcome"], outcome);
        assert_eq!(record["roles"], json!(["current-recheck"]));
    }
    let report: Value = serde_json::from_slice(
        &fs::read(retention_state::object_path(h.path(), &report_id)).unwrap(),
    )
    .unwrap();
    assert_eq!(report["output"]["outcome"], json!({"exit": 17}));
}

#[test]
fn bounded_disposal_does_not_rewrite_unchanged_dependency_declarations() {
    if !isolated_capability_test(
        "bounded_disposal_does_not_rewrite_unchanged_dependency_declarations",
    ) {
        return;
    }
    let h = host(2, 16);
    let report_path = tree(h.path(), "obsolete-report");
    let report = inventory(h.path())
        .begin("fixture", &report_path, json!(null))
        .unwrap();
    let report_id = report.id().to_owned();
    for n in 0..4 {
        fs::write(h.path().join(&report_path).join(n.to_string()), [7; 32]).unwrap();
    }
    let mut originals = Vec::new();
    for n in 0..8 {
        let path = format!(".chrono-harness/state/fixtures/required-{n}");
        fs::write(h.path().join(&path), b"current original").unwrap();
        let publication = inventory(h.path())
            .begin_rooted("fixture", &path, json!(null), &[format!("current-{n}")])
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete(json!({"exit": 7})).unwrap();
        originals.push((id, path));
    }
    inventory(h.path())
        .dependencies(
            &report_id,
            &originals
                .iter()
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    report.complete(json!({"exit": 17})).unwrap();
    let declarations: Vec<_> = originals
        .iter()
        .map(|(id, _)| {
            let path = retention_state::object_path(h.path(), id);
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();

    let round = inventory(h.path())
        .maintain_originals(&[report_id.clone()])
        .unwrap();
    assert!(
        round["accounting_errors"].as_array().unwrap().is_empty(),
        "{round}"
    );
    assert_eq!(round["nodes"], 2, "{round}");
    assert_eq!(round["removed_logical_bytes"], 32, "{round}");
    assert!(round["disposed"].as_array().unwrap().is_empty(), "{round}");
    assert!(h.path().join(&report_path).is_dir());
    for (path, before) in &declarations {
        assert_eq!(fs::read(path).unwrap(), *before, "{}", path.display());
    }
    let partial: Value = serde_json::from_slice(
        &fs::read(retention_state::object_path(h.path(), &report_id)).unwrap(),
    )
    .unwrap();
    assert_eq!(partial["output"]["phase"], "disposing");
    assert_eq!(partial["output"]["outcome"]["exit"], 17);

    // Five tree entries and eight edges advance in distinct bounded steps.
    for _ in 0..5 + originals.len() + 2 {
        let round = inventory(h.path())
            .maintain_originals(&[report_id.clone()])
            .unwrap();
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 2, "{round}");
        if output_retired(h.path(), &report_id) {
            break;
        }
    }
    assert!(!h.path().join(&report_path).exists());
    assert!(output_retired(h.path(), &report_id));
    for (id, path) in originals {
        assert_eq!(fs::read(h.path().join(path)).unwrap(), b"current original");
        let record: Value =
            serde_json::from_slice(&fs::read(retention_state::object_path(h.path(), &id)).unwrap())
                .unwrap();
        assert_eq!(record["incoming"], json!([]));
        assert_eq!(record["output"]["outcome"]["exit"], 7);
        assert_eq!(record["roles"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn partial_disposal_is_bounded_resumable_and_cannot_be_referenced_again() {
    if !isolated_capability_test(
        "partial_disposal_is_bounded_resumable_and_cannot_be_referenced_again",
    ) {
        return;
    }
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
    assert_eq!(fixture_remaining(h.path(), &r), 1);
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
    for entry in fs::read_dir(
        h.path()
            .join(".chrono-harness/state/output-retention-v1/objects"),
    )
    .unwrap()
    {
        let record: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("record.json")).unwrap())
                .unwrap();
        assert!(record["summaries"].as_array().unwrap().len() <= 2);
    }
}
#[test]
fn immutable_file_content_drift_is_refused_without_changing_original_outcome() {
    if !isolated_capability_test(
        "immutable_file_content_drift_is_refused_without_changing_original_outcome",
    ) {
        return;
    }
    let h = host(256, 16);
    let p = ".chrono-harness/state/fixtures/original.json";
    fs::write(h.path().join(p), b"original failure").unwrap();
    inventory(h.path())
        .begin("fixture", p, json!(null))
        .unwrap()
        .complete(json!({"exit":17}))
        .unwrap();
    fs::write(h.path().join(p), b"changed failure").unwrap();
    let mut rounds = Vec::new();
    let mut detected = false;
    for _ in 0..8 {
        let r = maintenance(h.path());
        detected = r["protected"].to_string().contains("content changed");
        assert_eq!(fixture_remaining(h.path(), &r), 1, "{r}");
        assert_eq!(fs::read(h.path().join(p)).unwrap(), b"changed failure");
        assert_eq!(ledger(h.path())["outputs"][0]["outcome"]["exit"], 17);
        rounds.push(r);
        if detected {
            break;
        }
    }
    assert!(detected, "content drift was not detected: {rounds:?}");
}
#[test]
fn legacy_without_policy_and_unenrolled_outputs_remain_untouched() {
    if !isolated_capability_test("legacy_without_policy_and_unenrolled_outputs_remain_untouched") {
        return;
    }
    let h = tempfile::tempdir().unwrap();
    assert!(Inventory::adopted(h.path()).unwrap().is_none());
    let h = host(256, 16);
    let p = tree(h.path(), "unknown-legacy");
    maintenance(h.path());
    assert!(h.path().join(p).exists());
}
#[test]
fn verified_content_reuse_preserves_independent_operation_results() {
    if !isolated_capability_test("verified_content_reuse_preserves_independent_operation_results") {
        return;
    }
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
    if !isolated_capability_test(
        "killed_producer_is_joined_and_ordinary_entry_recovers_without_any_drop_callback",
    ) {
        return;
    }
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
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    child.kill().unwrap();
    let joined = child.wait().unwrap();
    assert!(!joined.success());
    let original_path = ledger(h.path())["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut rounds = Vec::new();
    for _ in 0..8 {
        let r = maintenance(h.path());
        let completed = r["remaining_outputs"] == 0;
        rounds.push(r);
        if completed {
            break;
        }
    }
    assert_eq!(
        rounds.last().unwrap()["remaining_outputs"],
        0,
        "joined producer was not reclaimed: {rounds:?}"
    );
    assert!(!h.path().join(original_path).exists());
    assert_eq!(ledger(h.path())["summaries"].as_array().unwrap().len(), 1);
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
    if !isolated_capability_test(
        "bounded_disposal_unlinks_internal_symlink_without_touching_external_original",
    ) {
        return;
    }
    let h = host(256, 16);
    let external = tempfile::tempdir().unwrap();
    fs::write(external.path().join("evidence"), b"external original").unwrap();
    let p = tree(h.path(), "links");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    std::os::unix::fs::symlink(external.path(), h.path().join(&p).join("outside")).unwrap();
    publication.complete(json!({"exit":5})).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert_eq!(
        fs::read(external.path().join("evidence")).unwrap(),
        b"external original"
    );
}

#[test]
fn reserved_publication_and_retired_capability_intents_recover_without_scanning() {
    if !isolated_capability_test(
        "reserved_publication_and_retired_capability_intents_recover_without_scanning",
    ) {
        return;
    }
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
    retention_state::replace_output(h.path(), &state["outputs"][0]);
    for _ in 0..8 {
        maintenance(h.path());
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
fn per_relation_capacity_preserves_independent_outputs_and_recovers_pending_record() {
    if !isolated_capability_test(
        "per_relation_capacity_preserves_independent_outputs_and_recovers_pending_record",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(1);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let first_path = tree(h.path(), "live");
    let first = inventory(h.path())
        .begin("fixture", &first_path, json!(null))
        .unwrap();
    let second_path = tree(h.path(), "independent");
    let second = inventory(h.path())
        .begin("fixture", &second_path, json!(null))
        .unwrap();
    assert!(
        inventory(h.path())
            .reference("too-many", &[first.id().into(), second.id().into()])
            .is_err()
    );
    let pending = retention_state::object_path(h.path(), second.id()).with_extension("pending");
    fs::write(&pending, b"interrupted record replacement").unwrap();
    second.complete(json!({"exit":7})).unwrap();
    assert!(!pending.exists());
    let rounds = maintain_fixture_rounds_to(h.path(), 1);
    for round in &rounds {
        assert_eq!(round["related_output_capacity"], 1);
    }
    assert!(h.path().join(&first_path).exists());
    assert!(!h.path().join(&second_path).exists());
    drop(first);
}

#[cfg(not(verus_keep_ghost))]
fn report_host() -> tempfile::TempDir {
    let h = host(256, 16);
    fs::create_dir_all(h.path().join(".chrono-harness/state/preparation")).unwrap();
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
    if !isolated_capability_test(
        "actual_original_publisher_and_reader_reclaim_rotated_reports_but_preserve_shared_inputs",
    ) {
        return;
    }
    use chrono_harness::prepared::{read_original, retain_original};
    use retained_artifacts::retention::{reference_report, settle_report};
    let h = report_host();
    let dir = ".chrono-harness/state/preparation/";
    let activity = inventory(h.path()).read_scope().unwrap();
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
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    assert_eq!(
        read_original(h.path(), &shared, None).unwrap(),
        b"same original stdout"
    );
    let activity = inventory(h.path()).read_scope().unwrap();
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
    let rounds = maintain_fixture_rounds_to(h.path(), 2);
    assert!(!h.path().join(old.path).exists());
    assert_eq!(
        read_original(h.path(), &new, None).unwrap(),
        br#"{"request":"two","exit":0}"#
    );
    assert_eq!(
        read_original(h.path(), &shared, None).unwrap(),
        b"same original stdout"
    );
    assert!(
        rounds
            .iter()
            .map(|round| round["removed_logical_bytes"].as_u64().unwrap())
            .sum::<u64>()
            > 0,
        "{rounds:?}"
    );
}
#[test]
fn interrupted_fixed_pointer_publication_keeps_both_originals_until_owner_settles() {
    if !isolated_capability_test(
        "interrupted_fixed_pointer_publication_keeps_both_originals_until_owner_settles",
    ) {
        return;
    }
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
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    settle_report(h.path(), "check.json", &first.path).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    assert!(h.path().join(&first.path).exists());
    assert_eq!(
        fs::read(h.path().join(&second.path)).unwrap(),
        b"second actual error"
    );
    // The actual interrupted-publication consumer accounts its exact bytes before release.
    let output = ledger(h.path())["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == second.path)
        .unwrap()
        .clone();
    let declaration = consumer_input(
        h.path(),
        &format!("publication-recheck:{}", output["id"].as_str().unwrap()),
        json!([original_identity(
            h.path(),
            output["id"].as_str().unwrap(),
            &second.path
        )]),
    );
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release).exit_code, 0);
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(!h.path().join(second.path).exists());
}
#[test]
fn read_scope_and_pending_publication_preserve_interrupted_originals() {
    if !isolated_capability_test(
        "read_scope_and_pending_publication_preserve_interrupted_originals",
    ) {
        return;
    }
    use chrono_harness::prepared::retain_original;
    let h = report_host();
    let activity = retained_artifacts::retention::read_scope(h.path()).unwrap();
    let original = retain_original(
        h.path(),
        ".chrono-harness/state/preparation/",
        "acquisition",
        b"partial acquisition",
    )
    .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    drop(activity); // kernel close models an owner with no successful check result.
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert_eq!(
        fs::read(h.path().join(original.path)).unwrap(),
        b"partial acquisition"
    );
}

#[test]
fn caller_read_scope_protects_only_consumed_originals_and_forwards_their_capabilities() {
    if !isolated_capability_test(
        "caller_read_scope_protects_only_consumed_originals_and_forwards_their_capabilities",
    ) {
        return;
    }
    let h = host(256, 16);
    for name in ["required-input", "unrelated-old-output"] {
        let path = tree(h.path(), name);
        let p = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        p.complete(json!({"exit":7})).unwrap();
    }
    let caller = retained_artifacts::retention::read_scope(h.path()).unwrap();
    let nested = retained_artifacts::retention::read_scope(h.path()).unwrap();
    drop(
        retained_artifacts::retention::read_guard(
            h.path(),
            ".chrono-harness/state/fixtures/required-input",
        )
        .unwrap(),
    );
    drop(nested);
    let before = ledger(h.path());
    let original = before["outputs"][0].clone();
    let unrelated = before["outputs"][1]["lease_id"].as_str().unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(
        !h.path()
            .join(".chrono-harness/state/fixtures/unrelated-old-output")
            .exists()
    );
    let child = env!("CARGO_BIN_EXE_chrono-test-retention-child");
    let result = chrono_harness::run_process(
        h.path(),
        &chrono_harness::CommandSpec {
            program: child.into(),
            args: vec![
                h.path().to_str().unwrap().into(),
                "scope-input".into(),
                original["lease_id"].as_str().unwrap().into(),
                unrelated.into(),
            ],
            env: Default::default(),
            timeout_seconds: 5,
            output_limit_bytes: 4096,
        },
        b"",
    )
    .unwrap();
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(result.stdout.trim(), "exact input capability inherited");
    assert!(
        !h.path()
            .join(".chrono-harness/state/output-retention-v1/reports.lease")
            .exists()
    );
    assert_eq!(
        fs::read_dir(
            h.path()
                .join(".chrono-harness/state/fixtures/required-input")
        )
        .unwrap()
        .count(),
        0
    );
    drop(caller);
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
}
#[test]
fn failed_dependency_replacement_keeps_old_and_already_accepted_originals() {
    if !isolated_capability_test(
        "failed_dependency_replacement_keeps_old_and_already_accepted_originals",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut paths = Vec::new();
    for name in ["old", "new", "unavailable", "report"] {
        let path = tree(h.path(), name);
        inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit": 0}))
            .unwrap();
        paths.push(path);
    }
    inventory(h.path())
        .dependencies_paths(&paths[3], &paths[..1])
        .unwrap();
    let before = ledger(h.path());
    let unavailable = before["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == paths[2])
        .unwrap();
    let record_path = retention_state::object_path(h.path(), unavailable["id"].as_str().unwrap());
    let original = fs::read(&record_path).unwrap();
    let mut record: Value = serde_json::from_slice(&original).unwrap();
    record["output"]["phase"] = json!("disposing");
    fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
    let error = inventory(h.path())
        .dependencies_paths(&paths[3], &paths[1..3])
        .unwrap_err();
    assert!(error.contains("disposal has started"), "{error}");
    fs::write(&record_path, original).unwrap();
    let partial = ledger(h.path());
    let references = &partial["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == paths[3])
        .unwrap()["references"];
    for path in &paths[..2] {
        let output = partial["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["path"] == *path)
            .unwrap();
        assert!(references.as_array().unwrap().contains(&output["id"]));
        assert!(h.path().join(path).is_dir());
    }
    inventory(h.path())
        .dependencies_paths(&paths[3], &paths[1..3])
        .unwrap();
    let settled = ledger(h.path());
    let references = &settled["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == paths[3])
        .unwrap()["references"];
    assert_eq!(references.as_array().unwrap().len(), 2);
    for (n, path) in paths[..3].iter().enumerate() {
        let output = settled["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["path"] == *path)
            .unwrap();
        assert_eq!(
            references.as_array().unwrap().contains(&output["id"]),
            n > 0
        );
    }
}

#[test]
#[cfg(unix)]
fn parent_publication_accepts_and_settles_many_children_with_256_descriptors() {
    if !isolated_capability_test(
        "parent_publication_accepts_and_settles_many_children_with_256_descriptors",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(256);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    limit.rlim_cur = limit.rlim_cur.min(256);
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    let parent_path = ".chrono-harness/state/fixtures/parent-report.json".to_owned();
    let parent_bytes = b"original parent failure";
    fs::write(h.path().join(&parent_path), parent_bytes).unwrap();
    let parent = inventory(h.path())
        .begin("fixture", &parent_path, json!(null))
        .unwrap();
    let caller = retained_artifacts::retention::published_output_scope(&parent).unwrap();
    let mut paths = Vec::new();
    for n in 0..64 {
        let path = format!(".chrono-harness/state/fixtures/child-{n}.json");
        fs::write(h.path().join(&path), n.to_string()).unwrap();
        inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap()
            .complete(json!({"exit": n % 2}))
            .unwrap();
        paths.push(path);
    }
    let accepted = ledger(h.path());
    let output = accepted["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == parent_path)
        .unwrap();
    assert_eq!(output["references"].as_array().unwrap().len(), 64);
    // Keep ordinary caller files open too. Reading a report and retaining its
    // diagnostic only need that member; opening its 64 dependencies would
    // exhaust this same native budget. These are private Rust-owned files.
    inventory(h.path())
        .dependencies_paths(&parent_path, &paths[32..])
        .unwrap();
    let owner = "existing-lifecycle-owner";
    inventory(h.path())
        .reference_paths(owner, std::slice::from_ref(&parent_path))
        .unwrap();
    let pending_path = ".chrono-harness/state/fixtures/next-owner-report.json";
    let pending = inventory(h.path())
        .plan("fixture", pending_path, false, json!(null))
        .unwrap();
    let held: Vec<_> = (0..190)
        .map(|_| fs::File::open("/dev/null").unwrap())
        .collect();
    let input_caller = retained_artifacts::retention::publication_read_scope(&parent).unwrap();
    // Recovery may validate a long chain of previous receipts before its next
    // process call. Keep all actual inputs through the report's capability.
    for path in &paths {
        let input = retained_artifacts::retention::read_guard(h.path(), path)
            .unwrap()
            .unwrap();
        assert!(fs::read(h.path().join(path)).unwrap().len() > 0);
        drop(input);
    }
    // The owner already references the parent and its 64 inputs. Publishing
    // one more original must not open every unchanged outward dependency.
    pending.materialize_for_reference(owner).unwrap();
    assert_eq!(fs::read(h.path().join(pending_path)).unwrap(), b"");
    parent.complete(json!({"exit": 2})).unwrap();
    let retained = retained_artifacts::retention::read_guard(h.path(), &parent_path)
        .unwrap()
        .unwrap();
    let consumed = inventory(h.path())
        .consume(output["id"].as_str().unwrap())
        .unwrap();
    assert!(
        inventory(h.path())
            .published_original(
                &parent_path,
                &chrono_harness::sha256(parent_bytes),
                parent_bytes.len() as u64,
            )
            .unwrap()
    );
    inventory(h.path())
        .retain_diagnostic_path(&parent_path)
        .unwrap();
    assert_eq!(fs::read(h.path().join(&parent_path)).unwrap(), parent_bytes);
    drop((retained, consumed, held));
    let protected = ledger(h.path());
    assert_eq!(
        protected["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["path"] == parent_path)
            .unwrap()["references"]
            .as_array()
            .unwrap()
            .len(),
        64
    );
    drop(input_caller);
    pending.complete(json!({"exit": 0})).unwrap();
    inventory(h.path())
        .dependencies_paths(&parent_path, &paths[32..])
        .unwrap();
    let settled = ledger(h.path());
    let output = settled["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == parent_path)
        .unwrap();
    assert_eq!(output["phase"], "released");
    assert_eq!(output["outcome"]["exit"], 2);
    let references = output["references"].as_array().unwrap();
    assert_eq!(references.len(), 32);
    for (n, path) in paths.iter().enumerate() {
        let child = settled["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["path"] == *path)
            .unwrap();
        assert_eq!(references.contains(&child["id"]), n >= 32);
        assert_eq!(child["outcome"]["exit"], n % 2);
        assert_eq!(
            fs::read_to_string(h.path().join(path)).unwrap(),
            n.to_string()
        );
    }
    drop(caller);
}

#[test]
fn published_output_scope_keeps_child_reports_without_accumulating_their_inputs() {
    if !isolated_capability_test(
        "published_output_scope_keeps_child_reports_without_accumulating_their_inputs",
    ) {
        return;
    }
    let mut h = host(256, 16);
    h.disable_cleanup(true);
    eprintln!(
        "retention fixture originals (published_output_scope_keeps_child_reports_without_accumulating_their_inputs): {}",
        h.path().display()
    );
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(8);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let publish = |name: &str| {
        let path = tree(h.path(), name);
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        publication.complete(json!({"exit":0})).unwrap();
        path
    };
    let groups: Vec<Vec<String>> = (0..3)
        .map(|group| {
            let paths = (0..3)
                .map(|input| publish(&format!("input-{group}-{input}")))
                .collect::<Vec<_>>();
            inventory(h.path())
                .reference_paths(&format!("domain-owner-{group}"), &paths)
                .unwrap();
            paths
        })
        .collect();
    let unknown = publish("unknown-domain-input");
    inventory(h.path())
        .reference_paths("unfinished-domain-owner", &[unknown.clone()])
        .unwrap();
    let unrelated = publish("unrelated-expired");
    let parent_path = tree(h.path(), "parent-report");
    let parent = inventory(h.path())
        .begin("fixture", &parent_path, json!(null))
        .unwrap();
    let caller = retained_artifacts::retention::published_output_scope(&parent).unwrap();
    let mut reports = Vec::new();
    for (group, paths) in groups.iter().enumerate() {
        let _child = retained_artifacts::retention::read_scope(h.path()).unwrap();
        for path in paths {
            drop(retained_artifacts::retention::read_guard(h.path(), path).unwrap());
        }
        let path = tree(h.path(), &format!("child-report-{group}"));
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        inventory(h.path())
            .dependencies_paths(&path, paths)
            .unwrap();
        publication.complete(json!({"exit":0})).unwrap();
        inventory(h.path())
            .reference_paths(&format!("domain-owner-{group}"), &[])
            .unwrap();
        reports.push(path);
    }
    // Each child has four actual capabilities. The parent holds one live
    // report capability, with three durable edges to the child reports.
    use chrono_diagnostics::logging::{Level, Logger};
    let logger = Logger::new("retention-fixture", Level::DEBUG, std::io::stderr());
    assert_eq!(logger.run(|| maintain_fixture_to(h.path(), 14)), 14);
    assert!(!h.path().join(unrelated).exists());
    for path in groups.iter().flatten().chain(reports.iter()) {
        assert!(h.path().join(path).is_dir());
    }
    parent.complete(json!({"exit": 0})).unwrap();
    drop(caller);
    assert_eq!(logger.run(|| maintain_fixture_to(h.path(), 1)), 1);
    assert!(h.path().join(&unknown).is_dir());
    inventory(h.path())
        .reference_paths("unfinished-domain-owner", &[])
        .unwrap();
    assert_eq!(logger.run(|| maintain_fixture_to(h.path(), 0)), 0);
    logger
        .finish()
        .expect("published output retention diagnostics");
    h.disable_cleanup(false);
}

#[test]
fn completed_child_original_transfers_to_parent_before_its_success_root_is_released() {
    if !isolated_capability_test(
        "completed_child_original_transfers_to_parent_before_its_success_root_is_released",
    ) {
        return;
    }
    let h = host(256, 16);
    let unrelated = tree(h.path(), "unrelated-old");
    inventory(h.path())
        .begin("fixture", &unrelated, json!(null))
        .unwrap()
        .complete(json!({"exit":7}))
        .unwrap();
    let parent_scope = retained_artifacts::retention::read_scope(h.path()).unwrap();
    let child_path = tree(h.path(), "child-original");
    let child_id = {
        let _child_scope = retained_artifacts::retention::read_scope(h.path()).unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &child_path, json!(null))
            .unwrap();
        publication
            .write_file(
                &h.path().join(&child_path).join("stdout"),
                b"exact child result",
            )
            .unwrap();
        let id = publication.id().to_owned();
        publication
            .protect_consumer("child-success-handoff")
            .unwrap();
        publication
            .complete_releasing_consumer(json!({"exit":0}), "child-success-handoff")
            .unwrap();
        id
    };
    assert!(ledger(h.path())["roots"].as_object().unwrap().is_empty());
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(!h.path().join(unrelated).exists());
    assert_eq!(
        fs::read(h.path().join(&child_path).join("stdout")).unwrap(),
        b"exact child result"
    );
    let parent_path = tree(h.path(), "parent-original");
    let publication = inventory(h.path())
        .begin("fixture", &parent_path, json!(null))
        .unwrap();
    let parent_id = publication.id().to_owned();
    inventory(h.path())
        .dependencies(&parent_id, &[child_id])
        .unwrap();
    inventory(h.path())
        .reference("current-parent", &[parent_id])
        .unwrap();
    publication.complete(json!({"exit":3})).unwrap();
    drop(parent_scope);
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    inventory(h.path())
        .reference("current-parent", &[])
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    let summaries = ledger(h.path())["summaries"].as_array().unwrap().clone();
    assert_eq!(
        summaries
            .iter()
            .find(|row| row["path"] == child_path)
            .unwrap()["original_outcome"]["exit"],
        0
    );
    assert_eq!(
        summaries
            .iter()
            .find(|row| row["path"] == parent_path)
            .unwrap()["original_outcome"]["exit"],
        3
    );
}

#[test]
fn pending_native_delivery_is_independent_of_latest_report_and_explicitly_released() {
    if !isolated_capability_test(
        "pending_native_delivery_is_independent_of_latest_report_and_explicitly_released",
    ) {
        return;
    }
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
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    i.reference("native-delivery:request-one", &[]).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
}
#[test]
fn addressed_legacy_original_is_verified_without_silent_enrollment_or_rewriting() {
    if !isolated_capability_test(
        "addressed_legacy_original_is_verified_without_silent_enrollment_or_rewriting",
    ) {
        return;
    }
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
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert_eq!(fs::read(h.path().join(path)).unwrap(), b"legacy");
}

#[test]
fn unfinished_creation_policy_migration_preserves_state_and_reaches_existing_recovery() {
    if !isolated_capability_test(
        "unfinished_creation_policy_migration_preserves_state_and_reaches_existing_recovery",
    ) {
        return;
    }
    // These are interrupted producer states, not evidence of inactivity. Each
    // fixture closes its producer and leaves the real owner guards in place.
    for (phase, capability_exists, attached_inode) in [
        ("creating", true, false),
        ("creating", true, true),
        ("reserving", true, false),
        ("reserving", false, false),
    ] {
        for domain in [false, true] {
            let mut h = host(256, 16);
            h.disable_cleanup(true);
            eprintln!(
                "creation migration originals ({phase}, capability={capability_exists}, inode={attached_inode}, domain={domain}): {}",
                h.path().display()
            );
            let path = ".chrono-harness/state/fixtures/interrupted-creation";
            let role = "fixture-creation-recovery";
            let outcome = if domain && attached_inode {
                json!({"exit":9,"completion":"not-established","recovery_role":role,"error":"original domain failure"})
            } else if domain {
                json!({"exit":null,"completion":"unknown","recovery_role":role})
            } else {
                json!({"exit":9,"completion":"unknown","error":"original producer failure"})
            };
            let publication = if domain {
                inventory(h.path()).plan_rooted("fixture", path, outcome.clone(), role)
            } else {
                inventory(h.path()).plan("fixture", path, true, outcome.clone())
            }
            .unwrap();
            let id = publication.id().to_owned();
            drop(publication);
            let directory = h.path().join(".chrono-harness/state/output-retention-v1");
            let capability = directory.join(&id);
            let staged = directory.join(format!("creation-{id}"));
            let staged_identity = attached_inode.then(|| {
                fs::create_dir(&staged).unwrap();
                let identity = chrono_harness::artifact_disposal::identity(&staged).unwrap();
                let mut output = ledger(h.path())["outputs"][0].clone();
                // Reproduce the durable inode attachment before destination
                // publication, without materializing through a new attempt.
                output["identity"] = json!(identity);
                retention_state::replace_output(h.path(), &output);
                identity
            });
            if phase == "reserving" {
                let mut output = ledger(h.path())["outputs"][0].clone();
                output["phase"] = json!(phase);
                output["lease_id"] = json!("");
                retention_state::replace_output(h.path(), &output);
            }
            if !capability_exists {
                fs::remove_file(&capability).unwrap();
            }
            let capability_before = capability_exists.then(|| {
                (
                    chrono_harness::ownership::identity(&capability).unwrap(),
                    fs::read(&capability).unwrap(),
                )
            });
            let record = retention_state::object_path(h.path(), &id);
            let before: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
            let role_before = domain.then(|| {
                let address = retention_state::role_path(h.path(), role);
                (address.clone(), fs::read(address).unwrap())
            });
            let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
            fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
            let mut policy: Value = serde_json::from_slice(&previous).unwrap();
            policy["keep_seconds"] = json!(3600);
            let current = serde_json::to_vec(&policy).unwrap();
            fs::write(h.path().join(POLICY_PATH), &current).unwrap();
            let request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[id]});

            // A formal request must unlock recovery by changing only the
            // binding, including when the original capability ID is empty.
            let migrated = consumer_call(h.path(), request.clone());
            assert_eq!(
                migrated.exit_code, 0,
                "stdout: {}; stderr: {}",
                migrated.stdout, migrated.stderr
            );
            let receipt: Value = serde_json::from_str(&migrated.stdout).unwrap();
            assert_eq!(receipt["effects"], "metadata-binding-only");
            assert_eq!(receipt["completion"], "not-established");
            assert_eq!(receipt["outputs_preserved"], 1);
            assert_eq!(
                receipt["roots_preserved"],
                json!(if domain { 1 } else { 0 })
            );
            assert_eq!(receipt["scope"]["complete"], false);
            let mut expected = before.clone();
            expected["policy_sha256"] = json!(chrono_harness::sha256(&current));
            assert_eq!(
                serde_json::from_slice::<Value>(&fs::read(&record).unwrap()).unwrap(),
                expected
            );
            assert!(
                !h.path().join(path).exists(),
                "migration must not materialize the original"
            );
            if let Some(identity) = staged_identity {
                assert_eq!(
                    chrono_harness::artifact_disposal::identity(&staged).unwrap(),
                    identity
                );
                assert!(fs::read_dir(&staged).unwrap().next().is_none());
            }
            if let Some((identity, bytes)) = capability_before {
                assert_eq!(
                    chrono_harness::ownership::identity(&capability).unwrap(),
                    identity
                );
                assert_eq!(fs::read(&capability).unwrap(), bytes);
            } else {
                assert_eq!(
                    fs::symlink_metadata(&capability).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                );
            }
            if let Some((address, bytes)) = &role_before {
                assert_eq!(fs::read(address).unwrap(), *bytes);
            }
            let migrated_bytes = fs::read(&record).unwrap();
            let repeated = consumer_call(h.path(), request);
            assert_eq!(
                repeated.exit_code, 0,
                "stdout: {}; stderr: {}",
                repeated.stdout, repeated.stderr
            );
            assert_eq!(fs::read(&record).unwrap(), migrated_bytes);
            if !capability_exists {
                assert_eq!(
                    fs::symlink_metadata(&capability).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                );
            }

            if domain {
                // Ordinary maintenance does not retire the domain's failed or
                // unknown result after expiry. Its existing recovery owner can
                // then materialize the same object without creating an attempt.
                let round = maintenance(h.path());
                assert_eq!(fixture_remaining(h.path(), &round), 1);
                let original = inventory(h.path())
                    .rooted_original("fixture", role)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    original,
                    (id.clone(), path.into(), "creating".into(), outcome.clone())
                );
                assert!(!h.path().join(path).exists());
                assert!(ledger(h.path())["summaries"].as_array().unwrap().is_empty());
                inventory(h.path())
                    .materialize_rooted("fixture", role)
                    .unwrap();
                assert!(h.path().join(path).is_dir());
                let resumed = inventory(h.path())
                    .rooted_original("fixture", role)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    resumed,
                    (
                        id.clone(),
                        path.into(),
                        "publishing".into(),
                        outcome.clone()
                    )
                );
                assert_eq!(maintain_fixture_to(h.path(), 1), 1);
                assert_eq!(ledger(h.path())["outputs"][0]["outcome"], outcome);
                assert_eq!(ledger(h.path())["roots"][role], json!([id]));
                let (address, bytes) = role_before.unwrap();
                assert_eq!(fs::read(address).unwrap(), bytes);
            } else {
                assert_eq!(maintain_fixture_to(h.path(), 0), 0);
                let state = ledger(h.path());
                let summary = state["summaries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == id)
                    .unwrap();
                assert_eq!(summary["original_outcome"]["original"], outcome);
                assert_eq!(
                    summary["original_outcome"]["publication"],
                    "interrupted-creation"
                );
                assert_eq!(summary["original_outcome"]["completion"], "not-established");
            }
            h.disable_cleanup(false);
        }
    }
}

#[test]
fn unfinished_creation_policy_migration_excludes_original_producers_and_readers() {
    if !isolated_capability_test(
        "unfinished_creation_policy_migration_excludes_original_producers_and_readers",
    ) {
        return;
    }
    for phase in ["creating", "reserving"] {
        let mut h = host(256, 16);
        h.disable_cleanup(true);
        eprintln!(
            "live creation migration originals ({phase}): {}",
            h.path().display()
        );
        let publication = inventory(h.path())
            .plan(
                "fixture",
                ".chrono-harness/state/fixtures/live-creation",
                true,
                json!({"completion":"unknown"}),
            )
            .unwrap();
        let id = publication.id().to_owned();
        if phase == "reserving" {
            // The producer still owns the real capability while its durable
            // reservation has not attached an identity yet.
            let mut output = ledger(h.path())["outputs"][0].clone();
            output["phase"] = json!(phase);
            output["lease_id"] = json!("");
            retention_state::replace_output(h.path(), &output);
        }
        let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
        fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
        let mut policy: Value = serde_json::from_slice(&previous).unwrap();
        policy["keep_seconds"] = json!(3600);
        fs::write(
            h.path().join(POLICY_PATH),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        let request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[id]});
        let record = retention_state::object_path(h.path(), &id);
        let before = fs::read(&record).unwrap();
        let blocked = consumer_call(h.path(), request.clone());
        assert_eq!(
            blocked.exit_code, 2,
            "stdout: {}; stderr: {}",
            blocked.stdout, blocked.stderr
        );
        assert_eq!(fs::read(&record).unwrap(), before);
        drop(publication);

        let migrated = consumer_call(h.path(), request.clone());
        assert_eq!(
            migrated.exit_code, 0,
            "stdout: {}; stderr: {}",
            migrated.stdout, migrated.stderr
        );
        let bound = fs::read(&record).unwrap();
        let capability = h
            .path()
            .join(".chrono-harness/state/output-retention-v1")
            .join(&id);
        let reader =
            chrono_harness::ownership::Lease::acquire(&capability, None, false, false, None)
                .unwrap()
                .unwrap();
        // Reentry must exclude a current-binding reader too, even if the
        // record still has the original empty reservation capability ID.
        let blocked = consumer_call(h.path(), request.clone());
        assert_eq!(
            blocked.exit_code, 2,
            "stdout: {}; stderr: {}",
            blocked.stdout, blocked.stderr
        );
        assert_eq!(fs::read(&record).unwrap(), bound);
        drop(reader);
        let repeated = consumer_call(h.path(), request);
        assert_eq!(
            repeated.exit_code, 0,
            "stdout: {}; stderr: {}",
            repeated.stdout, repeated.stderr
        );
        assert_eq!(fs::read(&record).unwrap(), bound);
        h.disable_cleanup(false);
    }
}

#[test]
fn unfinished_creation_policy_migration_preserves_missing_changed_and_unknown_custody() {
    if !isolated_capability_test(
        "unfinished_creation_policy_migration_preserves_missing_changed_and_unknown_custody",
    ) {
        return;
    }
    for custody in [
        "missing-attached",
        "changed-attached",
        "directory-reservation",
        "symlink-reservation",
        "excluded-reservation",
        "unbound-creating-present",
        "unbound-creating-missing",
    ] {
        let mut h = host(256, 16);
        h.disable_cleanup(true);
        eprintln!(
            "invalid creation migration originals ({custody}): {}",
            h.path().display()
        );
        let publication = inventory(h.path())
            .plan(
                "fixture",
                ".chrono-harness/state/fixtures/unknown-creation",
                true,
                json!({"exit":null}),
            )
            .unwrap();
        let id = publication.id().to_owned();
        drop(publication);
        let capability = h
            .path()
            .join(".chrono-harness/state/output-retention-v1")
            .join(&id);
        // Rename, rather than unlink, so an attached replacement cannot reuse
        // the original inode and the fixture retains the original capability.
        let parked = capability.with_extension("original");
        fs::rename(&capability, &parked).unwrap();
        if custody.ends_with("reservation") {
            let mut output = ledger(h.path())["outputs"][0].clone();
            output["phase"] = json!("reserving");
            output["lease_id"] = json!("");
            retention_state::replace_output(h.path(), &output);
        }
        if custody.starts_with("unbound-creating") {
            let mut output = ledger(h.path())["outputs"][0].clone();
            output["lease_id"] = json!("");
            retention_state::replace_output(h.path(), &output);
        }
        match custody {
            "changed-attached" | "unbound-creating-present" => {
                fs::write(&capability, b"unknown replacement capability").unwrap()
            }
            "directory-reservation" => fs::create_dir(&capability).unwrap(),
            "symlink-reservation" => std::os::unix::fs::symlink(&parked, &capability).unwrap(),
            _ => (),
        }
        let record = retention_state::object_path(h.path(), &id);
        let exclusion = (custody == "excluded-reservation").then(|| {
            chrono_harness::ownership::DirectoryLease::acquire(record.parent().unwrap(), true, None)
                .unwrap()
                .unwrap()
        });
        let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
        fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
        let mut policy: Value = serde_json::from_slice(&previous).unwrap();
        policy["keep_seconds"] = json!(3600);
        fs::write(
            h.path().join(POLICY_PATH),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        let before = fs::read(&record).unwrap();
        let original_identity = chrono_harness::ownership::identity(&parked).unwrap();
        let request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[id]});
        let blocked = consumer_call(h.path(), request);
        assert_eq!(
            blocked.exit_code, 2,
            "stdout: {}; stderr: {}",
            blocked.stdout, blocked.stderr
        );
        assert_eq!(fs::read(&record).unwrap(), before);
        assert_eq!(
            chrono_harness::ownership::identity(&parked).unwrap(),
            original_identity
        );
        assert_eq!(fs::read(&parked).unwrap(), b"");
        match custody {
            "changed-attached" | "unbound-creating-present" => assert_eq!(
                fs::read(&capability).unwrap(),
                b"unknown replacement capability"
            ),
            "directory-reservation" => assert!(capability.is_dir()),
            "symlink-reservation" => assert_eq!(fs::read_link(&capability).unwrap(), parked),
            _ => assert!(!capability.exists()),
        }
        drop(exclusion);
        h.disable_cleanup(false);
    }
}

#[test]
fn unfinished_creation_policy_migration_preserves_dependencies_and_mixed_binding_retry() {
    if !isolated_capability_test(
        "unfinished_creation_policy_migration_preserves_dependencies_and_mixed_binding_retry",
    ) {
        return;
    }
    let mut h = host(256, 16);
    h.disable_cleanup(true);
    eprintln!("mixed creation migration originals: {}", h.path().display());
    let dependency_path = tree(h.path(), "required-input");
    fs::write(
        h.path().join(&dependency_path).join("original"),
        b"original failure input",
    )
    .unwrap();
    let dependency = inventory(h.path())
        .begin("fixture", &dependency_path, json!({"exit":9}))
        .unwrap();
    let dependency_id = dependency.id().to_owned();
    dependency.complete(json!({"exit":9})).unwrap();
    let mut ids = Vec::new();
    for phase in ["reserving", "creating"] {
        let path = format!(".chrono-harness/state/fixtures/{phase}");
        let publication = inventory(h.path())
            .plan("fixture", &path, true, json!({"completion":"unknown"}))
            .unwrap();
        let id = publication.id().to_owned();
        drop(publication);
        if phase == "reserving" {
            let state = ledger(h.path());
            let mut output = state["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["id"] == id)
                .unwrap()
                .clone();
            output["phase"] = json!(phase);
            output["lease_id"] = json!("");
            retention_state::replace_output(h.path(), &output);
        }
        ids.push(id);
    }
    inventory(h.path())
        .dependencies(&ids[0], &[dependency_id.clone()])
        .unwrap();
    let reader = inventory(h.path()).consume(&dependency_id).unwrap();
    let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
    fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["keep_seconds"] = json!(3600);
    let current = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &current).unwrap();
    let records: Vec<_> = ids
        .iter()
        .chain(std::iter::once(&dependency_id))
        .map(|id| {
            let path = retention_state::object_path(h.path(), id);
            (path.clone(), fs::read(path).unwrap())
        })
        .collect();
    let mut request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[ids[0]]});
    let blocked = consumer_call(h.path(), request.clone());
    assert_eq!(
        blocked.exit_code, 2,
        "stdout: {}; stderr: {}",
        blocked.stdout, blocked.stderr
    );
    for (path, bytes) in &records {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    drop(reader);
    // An unfinished source does not waive the existing dependency contract.
    // Keep each invalid fixture input and prove refusal before restoring it.
    let (dependency_record, dependency_bytes) = &records[2];
    let parked = dependency_record.with_extension("original");
    for condition in ["unknown-binding", "missing-declaration"] {
        if condition == "unknown-binding" {
            let mut dependency: Value = serde_json::from_slice(dependency_bytes).unwrap();
            let mut third_policy = policy.clone();
            third_policy["keep_seconds"] = json!(7200);
            dependency["policy_sha256"] = json!(chrono_harness::sha256(
                &serde_json::to_vec(&third_policy).unwrap()
            ));
            fs::write(dependency_record, serde_json::to_vec(&dependency).unwrap()).unwrap();
        } else {
            fs::rename(dependency_record, &parked).unwrap();
        }
        let original_path = if condition == "unknown-binding" {
            dependency_record
        } else {
            &parked
        };
        let invalid_bytes = fs::read(original_path).unwrap();
        let blocked = consumer_call(h.path(), request.clone());
        assert_eq!(
            blocked.exit_code, 2,
            "{condition}; stdout: {}; stderr: {}",
            blocked.stdout, blocked.stderr
        );
        assert_eq!(fs::read(original_path).unwrap(), invalid_bytes);
        for (path, bytes) in &records[..2] {
            assert_eq!(fs::read(path).unwrap(), *bytes);
        }
        if condition == "unknown-binding" {
            fs::write(dependency_record, dependency_bytes).unwrap();
        } else {
            assert!(!dependency_record.exists());
            fs::rename(&parked, dependency_record).unwrap();
        }
    }
    let migrated = consumer_call(h.path(), request.clone());
    assert_eq!(
        migrated.exit_code, 0,
        "stdout: {}; stderr: {}",
        migrated.stdout, migrated.stderr
    );
    let receipt: Value = serde_json::from_str(&migrated.stdout).unwrap();
    assert_eq!(receipt["outputs_preserved"], 2);
    assert_eq!(fs::read(&records[1].0).unwrap(), records[1].1);
    // A completed subset leaves a mixed old/current cohort. The same prior
    // policy and full original list must finish and then safely repeat.
    request["originals"] = json!(ids);
    for _ in 0..2 {
        let migrated = consumer_call(h.path(), request.clone());
        assert_eq!(
            migrated.exit_code, 0,
            "stdout: {}; stderr: {}",
            migrated.stdout, migrated.stderr
        );
        let receipt: Value = serde_json::from_str(&migrated.stdout).unwrap();
        assert_eq!(receipt["outputs_preserved"], 3);
        for (path, bytes) in &records {
            let mut expected: Value = serde_json::from_slice(bytes).unwrap();
            expected["policy_sha256"] = json!(chrono_harness::sha256(&current));
            assert_eq!(
                serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap(),
                expected
            );
        }
        assert_eq!(
            fs::read(h.path().join(&dependency_path).join("original")).unwrap(),
            b"original failure input"
        );
    }
    h.disable_cleanup(false);
}

#[test]
fn unfinished_creation_policy_binding_keeps_existing_recovery_identity_guards() {
    if !isolated_capability_test(
        "unfinished_creation_policy_binding_keeps_existing_recovery_identity_guards",
    ) {
        return;
    }
    for domain in [false, true] {
        for original in ["destination", "staging"] {
            let mut h = host(256, 16);
            h.disable_cleanup(true);
            eprintln!(
                "creation recovery guard originals ({original}, domain={domain}): {}",
                h.path().display()
            );
            let path = ".chrono-harness/state/fixtures/replaced-creation";
            let role = "fixture-recovery-guard";
            let outcome = json!({"exit":null,"completion":"unknown","recovery_role":role});
            let publication = if domain {
                inventory(h.path()).plan_rooted("fixture", path, outcome.clone(), role)
            } else {
                inventory(h.path()).plan("fixture", path, true, outcome.clone())
            }
            .unwrap();
            let id = publication.id().to_owned();
            drop(publication);
            let original_path = if original == "destination" {
                h.path().join(path)
            } else {
                h.path()
                    .join(".chrono-harness/state/output-retention-v1")
                    .join(format!("creation-{id}"))
            };
            fs::create_dir(&original_path).unwrap();
            fs::write(original_path.join("original"), b"unknown required original").unwrap();
            let original_identity =
                chrono_harness::artifact_disposal::identity(&original_path).unwrap();
            let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
            fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
            let mut policy: Value = serde_json::from_slice(&previous).unwrap();
            policy["keep_seconds"] = json!(3600);
            fs::write(
                h.path().join(POLICY_PATH),
                serde_json::to_vec(&policy).unwrap(),
            )
            .unwrap();
            let before = ledger(h.path());
            let request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[id]});
            let migrated = consumer_call(h.path(), request);
            assert_eq!(
                migrated.exit_code, 0,
                "stdout: {}; stderr: {}",
                migrated.stdout, migrated.stderr
            );
            if domain {
                maintenance(h.path());
                assert!(
                    inventory(h.path())
                        .materialize_rooted("fixture", role)
                        .is_err()
                );
            } else {
                let round = maintenance(h.path());
                assert!(
                    !round["accounting_errors"].as_array().unwrap().is_empty(),
                    "{round}"
                );
            }
            assert_eq!(
                chrono_harness::artifact_disposal::identity(&original_path).unwrap(),
                original_identity
            );
            assert_eq!(
                fs::read(original_path.join("original")).unwrap(),
                b"unknown required original"
            );
            let after = ledger(h.path());
            assert_eq!(after["outputs"], before["outputs"]);
            assert_eq!(after["roots"], before["roots"]);
            assert_eq!(after["summaries"], before["summaries"]);
            h.disable_cleanup(false);
        }
    }
}

#[test]
fn policy_migration_recovers_interrupted_transactions_and_preserves_live_owners() {
    if !isolated_capability_test(
        "policy_migration_recovers_interrupted_transactions_and_preserves_live_owners",
    ) {
        return;
    }
    for phase in ["reserving", "retiring"] {
        for capability_exists in [false, true] {
            let h = report_host();
            let path = ".chrono-harness/state/preparation/interrupted-migration.json";
            retained_artifacts::retention::publish_owned_original(
                h.path(),
                "check-original",
                path,
                b"original failure bytes",
                json!({"exit":9}),
            )
            .unwrap();
            inventory(h.path())
                .reference_paths("current-delivery", &[path.into()])
                .unwrap();
            let original = ledger(h.path());
            let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
            let directory = h.path().join(".chrono-harness/state/output-retention-v1");
            let key = format!("writer-{}", chrono_harness::sha256(path.as_bytes()));
            let declaration = directory.join(format!("transaction-{key}.json"));
            let mut transaction: Value =
                serde_json::from_slice(&fs::read(&declaration).unwrap()).unwrap();
            transaction["phase"] = json!(phase);
            if phase == "reserving" {
                transaction["lease_id"] = json!("");
            }
            let interrupted = serde_json::to_vec(&transaction).unwrap();
            fs::write(&declaration, &interrupted).unwrap();
            if !capability_exists {
                fs::remove_file(directory.join(&key)).unwrap();
            }
            let mut policy: Value = serde_json::from_slice(&previous).unwrap();
            policy["keep_seconds"] = json!(3600);
            let current = serde_json::to_vec(&policy).unwrap();
            fs::write(h.path().join(POLICY_PATH), &current).unwrap();

            // An admitted publisher may still be between intent and capability
            // attachment. Only its original address exclusion proves it left.
            let address = directory.join("transaction-gates").join(&key);
            let live = chrono_harness::ownership::DirectoryLease::acquire(&address, true, None)
                .unwrap()
                .unwrap();
            assert!(inventory(h.path()).migrate_policy(&previous, None).is_err());
            assert_eq!(fs::read(&declaration).unwrap(), interrupted);
            assert_eq!(ledger(h.path()), original);
            drop(live);

            if capability_exists {
                let live = chrono_harness::ownership::Lease::acquire(
                    &directory.join(&key),
                    None,
                    false,
                    false,
                    None,
                )
                .unwrap()
                .unwrap();
                assert!(inventory(h.path()).migrate_policy(&previous, None).is_err());
                assert_eq!(fs::read(&declaration).unwrap(), interrupted);
                assert_eq!(ledger(h.path()), original);
                drop(live);
            }

            let receipt = inventory(h.path()).migrate_policy(&previous, None).unwrap();
            assert_eq!(receipt["outputs_preserved"], 1);
            assert_eq!(receipt["effects"], "metadata-binding-only");
            assert_eq!(receipt["completion"], "not-established");
            assert!(!declaration.exists());
            assert!(!directory.join(&key).exists());
            assert!(!address.exists());
            let mut after = ledger(h.path());
            after["policy_sha256"] = original["policy_sha256"].clone();
            assert_eq!(after, original);
            assert_eq!(
                fs::read(h.path().join(path)).unwrap(),
                b"original failure bytes"
            );
            inventory(h.path()).migrate_policy(&previous, None).unwrap();
        }
    }
}

#[test]
fn policy_migration_recovers_empty_transaction_addresses_but_preserves_unknown_residue() {
    if !isolated_capability_test(
        "policy_migration_recovers_empty_transaction_addresses_but_preserves_unknown_residue",
    ) {
        return;
    }
    for residue in ["empty", "capability", "member", "unknown-key"] {
        let h = report_host();
        let path = ".chrono-harness/state/preparation/retired-address.json";
        retained_artifacts::retention::publish_owned_original(
            h.path(),
            "check-original",
            path,
            b"preserved original",
            json!({"exit":7}),
        )
        .unwrap();
        let original = ledger(h.path());
        let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
        let directory = h.path().join(".chrono-harness/state/output-retention-v1");
        let key = format!("writer-{}", chrono_harness::sha256(path.as_bytes()));
        fs::remove_file(directory.join(format!("transaction-{key}.json"))).unwrap();
        if residue != "capability" {
            fs::remove_file(directory.join(&key)).unwrap();
        }
        let mut address = directory.join("transaction-gates").join(&key);
        if residue == "member" {
            fs::write(address.join("unknown"), b"keep unknown bytes").unwrap();
        } else if residue == "unknown-key" {
            let unknown = directory.join("transaction-gates/unknown-address");
            fs::rename(&address, &unknown).unwrap();
            address = unknown;
        }
        let mut policy: Value = serde_json::from_slice(&previous).unwrap();
        policy["keep_seconds"] = json!(3600);
        fs::write(
            h.path().join(POLICY_PATH),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        let live = chrono_harness::ownership::DirectoryLease::acquire(&address, true, None)
            .unwrap()
            .unwrap();
        assert!(inventory(h.path()).migrate_policy(&previous, None).is_err());
        assert!(address.is_dir());
        assert_eq!(ledger(h.path()), original);
        drop(live);

        if residue == "empty" {
            let receipt = inventory(h.path()).migrate_policy(&previous, None).unwrap();
            assert_eq!(receipt["outputs_preserved"], 1);
            assert!(!address.exists());
            let mut after = ledger(h.path());
            after["policy_sha256"] = original["policy_sha256"].clone();
            assert_eq!(after, original);
        } else {
            assert!(inventory(h.path()).migrate_policy(&previous, None).is_err());
            assert!(address.is_dir());
            assert_eq!(ledger(h.path()), original);
            if residue == "capability" {
                assert!(directory.join(&key).exists());
            } else if residue == "member" {
                assert_eq!(
                    fs::read(address.join("unknown")).unwrap(),
                    b"keep unknown bytes"
                );
            }
        }
        assert_eq!(
            fs::read(h.path().join(path)).unwrap(),
            b"preserved original"
        );
    }
}

#[test]
fn policy_migration_does_not_hold_descriptors_for_unrelated_outputs() {
    if !isolated_capability_test("policy_migration_does_not_hold_descriptors_for_unrelated_outputs")
    {
        return;
    }
    let h = host(256, 16);
    let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
    for n in 0..80 {
        let path = tree(h.path(), &format!("independent-{n}"));
        inventory(h.path())
            .begin("fixture", &path, json!({"exit":0}))
            .unwrap()
            .complete(json!({"exit":0}))
            .unwrap();
    }
    let original = ledger(h.path());
    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["keep_seconds"] = json!(3600);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    // Ownership capabilities deliberately use descriptors starting at 200.
    limit.rlim_cur = limit.rlim_cur.min(256);
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    let receipt = inventory(h.path()).migrate_policy(&previous, None).unwrap();
    assert_eq!(receipt["outputs_preserved"], 80);
    let mut after = ledger(h.path());
    after["policy_sha256"] = original["policy_sha256"].clone();
    assert_eq!(after, original);
}

#[test]
fn policy_migration_preserves_a_large_reverse_graph_with_bounded_forward_dependencies() {
    if !isolated_capability_test(
        "policy_migration_preserves_a_large_reverse_graph_with_bounded_forward_dependencies",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(4);
    let previous = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &previous).unwrap();
    let shared_path = tree(h.path(), "shared-original");
    fs::write(h.path().join(&shared_path).join("original"), b"preserved").unwrap();
    let shared = inventory(h.path())
        .begin_rooted(
            "fixture",
            &shared_path,
            json!({"exit":9}),
            &["keep-shared".into()],
        )
        .unwrap();
    let shared_id = shared.id().to_owned();
    shared.complete(json!({"exit":9})).unwrap();
    let mut paths = vec![shared_path];
    for n in 0..4 {
        let path = tree(h.path(), &format!("referencing-{n}"));
        fs::write(h.path().join(&path).join("original"), b"preserved").unwrap();
        let publication = inventory(h.path())
            .begin_rooted("fixture", &path, json!({"exit":0}), &[format!("keep-{n}")])
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete(json!({"exit":0})).unwrap();
        inventory(h.path())
            .dependencies(&id, &[shared_id.clone()])
            .unwrap();
        paths.push(path);
    }
    let original = ledger(h.path());
    let mut objects = Vec::new();
    for (cursor, output) in original["outputs"].as_array().unwrap().iter().enumerate() {
        let path = retention_state::object_path(h.path(), output["id"].as_str().unwrap());
        let mut record: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        record["cursor"] = json!(cursor + 1);
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        objects.push((path, record));
    }
    let roots: Vec<_> = original["roots"]
        .as_object()
        .unwrap()
        .keys()
        .map(|role| {
            let path = retention_state::role_path(h.path(), role);
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    policy["producers"]["check-original"] = json!([".chrono-harness/state/preparation/"]);
    let current = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &current).unwrap();
    let receipt = inventory(h.path()).migrate_policy(&previous, None).unwrap();
    assert_eq!(receipt["outputs_preserved"], 5);
    assert_eq!(receipt["roots_preserved"], 5);
    assert_eq!(receipt["policy_sha256"], chrono_harness::sha256(&current));
    let mut after = ledger(h.path());
    after["policy_sha256"] = original["policy_sha256"].clone();
    assert_eq!(after, original);
    for (path, mut expected) in objects {
        expected["policy_sha256"] = json!(chrono_harness::sha256(&current));
        let actual: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }
    for (path, bytes) in roots {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    for path in paths {
        assert_eq!(
            fs::read(h.path().join(path).join("original")).unwrap(),
            b"preserved"
        );
    }
}

#[test]
fn explicit_policy_migration_preserves_originals_and_refuses_live_readers_and_missing_scopes() {
    if !isolated_capability_test(
        "explicit_policy_migration_preserves_originals_and_refuses_live_readers_and_missing_scopes",
    ) {
        return;
    }
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
        maintenance(h.path())["accounting_errors"]
            .to_string()
            .contains("migration required")
    );
    assert!(
        inventory(h.path())
            .migrate_policy(&previous, None)
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
    assert!(inventory(h.path()).migrate_policy(&previous, None).is_err());
    assert_eq!(ledger(h.path()), original);
    policy["producers"]["fixture"] = json!([".chrono-harness/state/fixtures/"]);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let receipt = inventory(h.path()).migrate_policy(&previous, None).unwrap();
    assert_eq!(receipt["outputs_preserved"], 1);
    let mut after = ledger(h.path());
    after["policy_sha256"] = original["policy_sha256"].clone();
    assert_eq!(after, original);
    assert!(h.path().join(path).exists());
}

#[test]
fn declared_policy_migration_isolates_other_bindings_and_protects_forward_dependencies() {
    if !isolated_capability_test(
        "declared_policy_migration_isolates_other_bindings_and_protects_forward_dependencies",
    ) {
        return;
    }
    let h = host(256, 16);
    let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
    fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
    let mut originals = Vec::new();
    for name in ["dependency", "selected"] {
        let path = tree(h.path(), name);
        fs::write(h.path().join(&path).join("original"), name.as_bytes()).unwrap();
        let publication = inventory(h.path())
            .begin_rooted("fixture", &path, json!({"exit":9}), &[name.into()])
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete(json!({"exit":9})).unwrap();
        originals.push((id, path));
    }
    inventory(h.path())
        .dependencies(&originals[1].0, &[originals[0].0.clone()])
        .unwrap();
    let dependency_reader = inventory(h.path()).consume(&originals[0].0).unwrap();

    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["keep_seconds"] = json!(3600);
    let other_policy = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &other_policy).unwrap();
    let unrelated_path = tree(h.path(), "other-old-policy");
    let unrelated = inventory(h.path())
        .begin("fixture", &unrelated_path, json!({"exit":0}))
        .unwrap();
    let unrelated_id = unrelated.id().to_owned();
    unrelated.complete(json!({"exit":0})).unwrap();
    let unrelated_reader = inventory(h.path()).consume(&unrelated_id).unwrap();
    policy["keep_seconds"] = json!(7200);
    let current = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &current).unwrap();
    let before = ledger(h.path());
    let mut request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[originals[1].0]});

    let blocked = consumer_call(h.path(), request.clone());
    assert_eq!(blocked.exit_code, 2);
    assert!(blocked.stderr.contains("live output"), "{}", blocked.stderr);
    assert_eq!(ledger(h.path()), before);
    drop(dependency_reader);
    let mut unscoped = request.clone();
    unscoped.as_object_mut().unwrap().remove("originals");
    let blocked = consumer_call(h.path(), unscoped);
    assert_eq!(blocked.exit_code, 2);
    assert!(
        blocked.stderr.contains("migration required"),
        "{}",
        blocked.stderr
    );
    assert_eq!(ledger(h.path()), before);
    for invalid in [
        json!([]),
        json!([originals[1].0, originals[1].0]),
        json!(["missing"]),
        json!((0..33).map(|n| format!("missing-{n}")).collect::<Vec<_>>()),
    ] {
        request["originals"] = invalid;
        assert_eq!(consumer_call(h.path(), request.clone()).exit_code, 2);
        assert_eq!(ledger(h.path()), before);
    }
    request["originals"] = json!([originals[1].0]);
    let unrelated_record = retention_state::object_path(h.path(), &unrelated_id);
    let unrelated_bytes = fs::read(&unrelated_record).unwrap();
    let mut selected_records = Vec::new();
    for (id, _) in &originals {
        let path = retention_state::object_path(h.path(), id);
        let record: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        selected_records.push((path, record));
    }
    let roles: Vec<_> = before["roots"]
        .as_object()
        .unwrap()
        .keys()
        .map(|role| {
            let path = retention_state::role_path(h.path(), role);
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    let migrated = consumer_call(h.path(), request.clone());
    assert_eq!(migrated.exit_code, 0, "{}", migrated.stderr);
    let receipt: Value = serde_json::from_str(&migrated.stdout).unwrap();
    assert_eq!(receipt["outputs_preserved"], 2);
    assert_eq!(receipt["scope"]["kind"], "declared-components");
    assert_eq!(receipt["scope"]["complete"], false);
    assert_eq!(fs::read(&unrelated_record).unwrap(), unrelated_bytes);
    for (path, mut expected) in selected_records {
        expected["policy_sha256"] = json!(chrono_harness::sha256(&current));
        let actual: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }
    for (path, bytes) in roles {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    for (_, path) in &originals {
        assert_eq!(
            fs::read(h.path().join(path).join("original")).unwrap(),
            path.rsplit('/').next().unwrap().as_bytes()
        );
    }
    assert_eq!(consumer_call(h.path(), request.clone()).exit_code, 0);
    let current_reader = inventory(h.path()).consume(&originals[0].0).unwrap();
    let blocked = consumer_call(h.path(), request.clone());
    assert_eq!(blocked.exit_code, 2);
    assert!(blocked.stderr.contains("live output"), "{}", blocked.stderr);
    drop(current_reader);

    // A third binding inside the selected forward dependency set still fails.
    let dependency_record = retention_state::object_path(h.path(), &originals[0].0);
    let mut dependency: Value =
        serde_json::from_slice(&fs::read(&dependency_record).unwrap()).unwrap();
    dependency["policy_sha256"] = json!(chrono_harness::sha256(&other_policy));
    fs::write(&dependency_record, serde_json::to_vec(&dependency).unwrap()).unwrap();
    let blocked = consumer_call(h.path(), request);
    assert_eq!(blocked.exit_code, 2);
    assert!(
        blocked.stderr.contains("migration required"),
        "{}",
        blocked.stderr
    );
    assert_eq!(fs::read(&unrelated_record).unwrap(), unrelated_bytes);
    drop(unrelated_reader);
}

#[test]
fn declared_policy_migration_preserves_unrelated_live_and_interrupted_publishers() {
    if !isolated_capability_test(
        "declared_policy_migration_preserves_unrelated_live_and_interrupted_publishers",
    ) {
        return;
    }
    for phase in ["active", "reserving", "retiring"] {
        let h = report_host();
        let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
        fs::write(h.path().join("previous-policy.json"), &previous).unwrap();
        let selected_path = tree(h.path(), "selected");
        fs::write(h.path().join(&selected_path).join("original"), b"selected").unwrap();
        let selected = inventory(h.path())
            .begin("fixture", &selected_path, json!({"exit":9}))
            .unwrap();
        let selected_id = selected.id().to_owned();
        selected.complete(json!({"exit":9})).unwrap();
        let unrelated_path = ".chrono-harness/state/preparation/unrelated.json";
        retained_artifacts::retention::publish_owned_original(
            h.path(),
            "check-original",
            unrelated_path,
            b"unrelated original failure",
            json!({"exit":7}),
        )
        .unwrap();
        let slot = ".chrono-harness/state/unrelated-slot.json";
        let publisher = retained_artifacts::retention::reference_report(
            h.path(),
            slot,
            unrelated_path,
            &[],
            None,
        )
        .unwrap()
        .unwrap();
        let transaction_path = h.path().join(format!(
            ".chrono-harness/state/output-retention-v1/transaction-slot-{}.json",
            chrono_harness::sha256(slot.as_bytes())
        ));
        let mut transaction: Value =
            serde_json::from_slice(&fs::read(&transaction_path).unwrap()).unwrap();
        transaction["phase"] = json!(phase);
        if phase == "reserving" {
            transaction["lease_id"] = json!("");
        }
        let transaction_bytes = serde_json::to_vec(&transaction).unwrap();
        fs::write(&transaction_path, &transaction_bytes).unwrap();
        let before = ledger(h.path());
        let role_paths: Vec<_> = before["roots"]
            .as_object()
            .unwrap()
            .keys()
            .map(|role| {
                let path = retention_state::role_path(h.path(), role);
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        let records: Vec<_> = before["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|output| {
                let id = output["id"].as_str().unwrap();
                let path = retention_state::object_path(h.path(), id);
                let bytes = fs::read(&path).unwrap();
                (id.to_owned(), path, bytes)
            })
            .collect();
        let mut policy: Value = serde_json::from_slice(&previous).unwrap();
        policy["keep_seconds"] = json!(3600);
        let current = serde_json::to_vec(&policy).unwrap();
        fs::write(h.path().join(POLICY_PATH), &current).unwrap();
        let mut request = json!({"operation":"migrate-policy","previous_policy":{"path":"previous-policy.json","sha256":chrono_harness::sha256(&previous)},"originals":[selected_id]});

        let migrated = consumer_call(h.path(), request.clone());
        assert_eq!(migrated.exit_code, 0, "{phase}: {}", migrated.stderr);
        let receipt: Value = serde_json::from_str(&migrated.stdout).unwrap();
        assert_eq!(receipt["outputs_preserved"], 1);
        assert_eq!(receipt["scope"]["kind"], "declared-components");
        assert_eq!(receipt["scope"]["complete"], false);
        for (id, path, bytes) in records {
            let actual = fs::read(path).unwrap();
            if id == selected_id {
                let mut expected: Value = serde_json::from_slice(&bytes).unwrap();
                expected["policy_sha256"] = json!(chrono_harness::sha256(&current));
                assert_eq!(
                    serde_json::from_slice::<Value>(&actual).unwrap(),
                    expected,
                    "{phase}: {id}"
                );
            } else {
                assert_eq!(actual, bytes, "{phase}: {id}");
            }
        }
        for (path, bytes) in role_paths {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        let migrated_state = ledger(h.path());
        request.as_object_mut().unwrap().remove("originals");
        let blocked = consumer_call(h.path(), request);
        assert_eq!(blocked.exit_code, 2, "{phase}");
        assert!(
            blocked.stderr.contains(if phase == "active" {
                "live addressed publisher"
            } else {
                "still owned"
            }),
            "{phase}: {}",
            blocked.stderr
        );
        assert_eq!(ledger(h.path()), migrated_state);
        assert_eq!(fs::read(&transaction_path).unwrap(), transaction_bytes);
        assert_eq!(
            fs::read(h.path().join(unrelated_path)).unwrap(),
            b"unrelated original failure"
        );
        assert_eq!(
            fs::read(h.path().join(&selected_path).join("original")).unwrap(),
            b"selected"
        );
        drop(publisher);
    }
}

#[test]
fn explicit_policy_migration_accepts_mixed_bindings_and_preserves_exclusion() {
    if !isolated_capability_test(
        "explicit_policy_migration_accepts_mixed_bindings_and_preserves_exclusion",
    ) {
        return;
    }
    let h = host(256, 16);
    let previous = fs::read(h.path().join(POLICY_PATH)).unwrap();
    let old_path = tree(h.path(), "prior-policy");
    fs::write(h.path().join(&old_path).join("original"), b"old failure").unwrap();
    let old = inventory(h.path())
        .begin_rooted(
            "fixture",
            &old_path,
            json!({"exit":9}),
            &["old-reader".into()],
        )
        .unwrap();
    let old_id = old.id().to_owned();
    old.complete(json!({"exit":9})).unwrap();

    let mut policy: Value = serde_json::from_slice(&previous).unwrap();
    policy["keep_seconds"] = json!(3600);
    let current = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &current).unwrap();
    let new_path = tree(h.path(), "current-policy");
    fs::write(h.path().join(&new_path).join("original"), b"new success").unwrap();
    let new = inventory(h.path())
        .begin_rooted(
            "fixture",
            &new_path,
            json!({"exit":0}),
            &["new-reader".into()],
        )
        .unwrap();
    let new_id = new.id().to_owned();
    new.complete(json!({"exit":0})).unwrap();
    let original = ledger(h.path());

    // Ordinary access still refuses the old binding, while independent current
    // publications remain usable before the explicit owner migration.
    assert!(inventory(h.path()).consume(&old_id).is_err());
    let reader = inventory(h.path()).consume(&new_id).unwrap();
    let blocked = inventory(h.path())
        .migrate_policy(&previous, None)
        .unwrap_err();
    assert!(blocked.contains("live output"), "{blocked}");
    assert_eq!(ledger(h.path()), original);
    drop(reader);

    let mut unknown: Value = serde_json::from_slice(&previous).unwrap();
    unknown["keep_seconds"] = json!(7200);
    let unknown_binding = inventory(h.path())
        .migrate_policy(&serde_json::to_vec(&unknown).unwrap(), None)
        .unwrap_err();
    assert!(
        unknown_binding.contains("migration required"),
        "{unknown_binding}"
    );
    assert_eq!(ledger(h.path()), original);

    let receipt = inventory(h.path()).migrate_policy(&previous, None).unwrap();
    assert_eq!(receipt["outputs_preserved"], 2);
    assert_eq!(receipt["roots_preserved"], 2);
    assert_eq!(
        receipt["previous_policy_sha256"],
        chrono_harness::sha256(&previous)
    );
    assert_eq!(receipt["policy_sha256"], chrono_harness::sha256(&current));
    assert_eq!(receipt["effects"], "metadata-binding-only");
    let mut after = ledger(h.path());
    after["policy_sha256"] = original["policy_sha256"].clone();
    assert_eq!(after, original);
    for (id, path, bytes) in [
        (&old_id, &old_path, b"old failure".as_slice()),
        (&new_id, &new_path, b"new success".as_slice()),
    ] {
        assert_eq!(
            fs::read(h.path().join(path).join("original")).unwrap(),
            bytes
        );
        let record: Value =
            serde_json::from_slice(&fs::read(retention_state::object_path(h.path(), id)).unwrap())
                .unwrap();
        assert_eq!(record["policy_sha256"], chrono_harness::sha256(&current));
        drop(inventory(h.path()).consume(id).unwrap());
    }
    inventory(h.path()).migrate_policy(&previous, None).unwrap();
    let mut repeated = ledger(h.path());
    repeated["policy_sha256"] = original["policy_sha256"].clone();
    assert_eq!(repeated, original);
}

#[test]
fn addressed_reuse_refuses_equal_bytes_at_changed_identity_or_partial_disposal() {
    if !isolated_capability_test(
        "addressed_reuse_refuses_equal_bytes_at_changed_identity_or_partial_disposal",
    ) {
        return;
    }
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
            let _slot = retained_artifacts::retention::reference_report(
                h.path(),
                "check.json",
                &original.path,
                &[],
                None,
            )
            .unwrap();
            retained_artifacts::retention::settle_report(h.path(), "check.json", &original.path)
                .unwrap();
            drop(_slot);
            let newer = retain_original(h.path(), dir, "report", b"new current report").unwrap();
            let _slot = retained_artifacts::retention::reference_report(
                h.path(),
                "check.json",
                &newer.path,
                &[],
                None,
            )
            .unwrap();
            retained_artifacts::retention::settle_report(h.path(), "check.json", &newer.path)
                .unwrap();
            drop(_slot);
            // Each addressed transaction retirement consumes this fixture's
            // single metadata step. Account those declared effects before the
            // separate output-disposal step; never enlarge the round budget.
            let transactions =
                fs::read_dir(h.path().join(".chrono-harness/state/output-retention-v1"))
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|name| name.starts_with("transaction-") && name.ends_with(".json"))
                    .count();
            for _ in 0..transactions {
                let round = maintenance(h.path());
                assert_eq!(round["nodes"], 1);
                assert_eq!(ledger(h.path())["outputs"][0]["phase"], "released");
            }
            maintenance(h.path());
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
    if !isolated_capability_test(
        "sealed_tree_rejects_replaced_descendants_and_preserves_the_replacement",
    ) {
        return;
    }
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
    let round = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &round), 1);
    assert!(round["protected"].to_string().contains("sealed descendant"));
    assert_eq!(fs::read(file).unwrap(), b"new source evidence");
    assert_eq!(ledger(h.path())["outputs"][0]["outcome"]["exit"], 101);
}

#[test]
fn sealed_tree_never_enrolls_a_late_unknown_descendant_during_disposal() {
    if !isolated_capability_test(
        "sealed_tree_never_enrolls_a_late_unknown_descendant_during_disposal",
    ) {
        return;
    }
    let h = host(256, 16);
    let p = tree(h.path(), "unknown");
    inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap()
        .complete(json!({"exit":1}))
        .unwrap();
    fs::write(h.path().join(&p).join("unowned"), b"retain me").unwrap();
    let round = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &round), 1);
    assert!(h.path().join(&p).join("unowned").exists());
    assert_eq!(round["removed_logical_bytes"], 0);
}

#[test]
fn independent_live_writers_keep_their_own_sealed_tree_accounting() {
    if !isolated_capability_test("independent_live_writers_keep_their_own_sealed_tree_accounting") {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(1024);

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
    let second = inventory(h.path())
        .begin("fixture", &b, json!(null))
        .unwrap();
    fs::write(h.path().join(&a).join("bytes"), [5; 400]).unwrap();
    first.complete(json!({"exit":0})).unwrap();
    let rounds = maintain_fixture_rounds_to(h.path(), 1);
    assert_eq!(
        rounds
            .iter()
            .map(|round| round["removed_logical_bytes"].as_u64().unwrap())
            .sum::<u64>(),
        400,
        "{rounds:?}"
    );
    assert!(h.path().join(&b).exists());
    second.complete(json!({"exit":2})).unwrap();
}

#[test]
fn source_protection_accepts_unborn_outputs_and_preserves_fixture_git_sources() {
    if !git_fixture::isolated(
        "source_protection_accepts_unborn_outputs_and_preserves_fixture_git_sources",
    ) {
        return;
    }
    source_protection_boundaries(&git_fixture::program());
}

#[cfg(not(verus_keep_ghost))]
fn source_protection_boundaries(program: &Path) {
    use chrono_harness::artifact_disposal::source_safe;
    let h = host(256, 16);
    let output = "build[1]";
    let source = "build[1]/source.rs";
    fs::create_dir(h.path().join(output)).unwrap();
    fs::write(h.path().join(output).join("untracked"), b"generated").unwrap();
    assert!(source_safe(h.path(), output).is_ok());
    let git = |args: &[&str]| {
        let result = std::process::Command::new(program)
            .args([
                "-c",
                "user.name=Source protection fixture",
                "-c",
                "user.email=source-protection@example.invalid",
                "-c",
                "commit.gpgSign=false",
            ])
            .args(args)
            .current_dir(h.path())
            .output()
            .unwrap();
        assert!(result.status.success(), "{args:?}: {result:?}");
        result.stdout
    };
    git(&["init", "-q"]);
    assert_eq!(
        git(&[
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--no-renames",
            "--ignore-submodules=none",
            "--diff-filter=D",
            "--",
            output
        ]),
        b""
    );
    source_safe(h.path(), output).expect("an unborn repository has no committed source");

    fs::write(h.path().join(source), b"source").unwrap();
    git(&["add", "--", source]);
    assert!(
        source_safe(h.path(), output)
            .unwrap_err()
            .contains("committed/index source")
    );
    git(&["commit", "-qm", "source"]);
    assert!(
        source_safe(h.path(), output)
            .unwrap_err()
            .contains("committed/index source")
    );

    git(&["update-index", "--force-remove", "--", source]);
    assert_eq!(
        git(&["--literal-pathspecs", "ls-files", "-z", "--", output]),
        b""
    );
    assert_eq!(
        git(&[
            "--literal-pathspecs",
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--no-renames",
            "--ignore-submodules=none",
            "--diff-filter=D",
            "--",
            output
        ]),
        format!("{source}\0").as_bytes()
    );
    assert!(
        source_safe(h.path(), output)
            .unwrap_err()
            .contains("committed/index source")
    );
    assert_eq!(fs::read(h.path().join(source)).unwrap(), b"source");

    git(&["add", "--", source]);
    git(&["mv", source, "renamed.rs"]);
    assert!(
        source_safe(h.path(), output)
            .unwrap_err()
            .contains("committed/index source")
    );
    assert_eq!(fs::read(h.path().join("renamed.rs")).unwrap(), b"source");
    git(&["commit", "-qm", "move source out of output"]);
    fs::create_dir_all(h.path().join("build1")).unwrap();
    fs::write(h.path().join("build1/source.rs"), b"unrelated source").unwrap();
    git(&["add", "--", "build1/source.rs"]);
    git(&[
        "commit",
        "-qm",
        "unrelated path must not match literal output",
    ]);
    source_safe(h.path(), output).expect("committed deletions leave only the generated output");
    let commit = String::from_utf8(git(&["rev-parse", "HEAD"])).unwrap();
    fs::create_dir(h.path().join("module-output")).unwrap();
    git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{},module-output", commit.trim()),
    ]);
    git(&["commit", "-qm", "tracked gitlink"]);
    assert!(
        source_safe(h.path(), "module-output")
            .unwrap_err()
            .contains("committed/index source")
    );
    git(&["config", "diff.ignoreSubmodules", "all"]);
    git(&["update-index", "--force-remove", "--", "module-output"]);
    assert!(
        source_safe(h.path(), "module-output")
            .unwrap_err()
            .contains("committed/index source")
    );
    assert_eq!(
        fs::read(h.path().join(output).join("untracked")).unwrap(),
        b"generated"
    );
    fs::write(
        h.path().join(".git/HEAD"),
        b"ffffffffffffffffffffffffffffffffffffffff\n",
    )
    .unwrap();
    let failure = std::process::Command::new(program)
        .args([
            "--literal-pathspecs",
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--no-renames",
            "--ignore-submodules=none",
            "--diff-filter=D",
            "--",
            output,
        ])
        .current_dir(h.path())
        .output()
        .unwrap();
    assert!(!failure.status.success(), "{failure:?}");
    let original_error = String::from_utf8(failure.stderr).unwrap();
    assert!(!original_error.trim().is_empty());
    let error = source_safe(h.path(), output).unwrap_err();
    assert!(error.contains(original_error.trim()), "{error}");
}

#[test]
fn staged_source_added_after_publication_prevents_disposal() {
    if !git_fixture::isolated("staged_source_added_after_publication_prevents_disposal") {
        return;
    }
    let h = host(256, 16);
    let p = tree(h.path(), "source");
    let publication = inventory(h.path())
        .begin("fixture", &p, json!(null))
        .unwrap();
    fs::write(h.path().join(&p).join("source.rs"), b"source").unwrap();
    publication.complete(json!({"exit":0})).unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new(git_fixture::program())
            .args(args)
            .current_dir(h.path())
            .output()
            .unwrap()
    };
    assert!(git(&["init", "-q"]).status.success());
    assert!(git(&["add", &format!("{p}/source.rs")]).status.success());
    let round = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &round), 1);
    assert_eq!(round["removed_logical_bytes"], 0);
    assert!(h.path().join(&p).join("source.rs").exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn retired_historical_inputs_preserve_unregistered_originals() {
    let h = host(256, 16);
    let path = tree(h.path(), "unregistered-original");
    let original = h.path().join(&path).join("failure");
    let bytes = [9; 2048];
    fs::write(&original, bytes).unwrap();
    for request in [
        json!({"operation":"enroll","enrollment":"cohort","expected_identity":chrono_harness::artifact_disposal::identity(&h.path().join(&path)).unwrap()}),
        json!({"operation":"release","enrollment":"cohort"}),
    ] {
        fs::write(
            h.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        let result = chrono_harness::dispatch(&[
            "retention",
            "--host-root",
            h.path().to_str().unwrap(),
            "--request",
            "request.json",
        ]);
        assert_eq!(result.exit_code, 2, "{}", result.stderr);
        assert!(
            result.stderr.contains("unknown variant"),
            "{}",
            result.stderr
        );
        assert_eq!(fs::read(&original).unwrap(), bytes);
    }
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["historical"] = json!({});
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    assert!(matches!(Inventory::adopted(h.path()), Err(error) if error.contains("unknown field")));
    assert_eq!(fs::read(&original).unwrap(), bytes);
}

#[test]
fn native_delivery_acknowledgement_releases_only_its_exact_original_and_keeps_failed_outcome() {
    if !isolated_capability_test(
        "native_delivery_acknowledgement_releases_only_its_exact_original_and_keeps_failed_outcome",
    ) {
        return;
    }
    use retained_artifacts::retention::{publish_owned_original, reference_report, settle_report};
    let h = report_host();
    let path = ".chrono-harness/state/preparation/native-delivery.json";
    let raw = serde_json::to_vec(
        &json!({"request":{"request_id":"native-1"},"response":{"status":"failed"}}),
    )
    .unwrap();
    publish_owned_original(h.path(), "check-original", path, &raw, json!({"exit":1})).unwrap();
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
    if !isolated_capability_test("unselected_producer_keeps_legacy_publication_semantics") {
        return;
    }
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
    if !isolated_capability_test(
        "oversized_tree_completion_keeps_the_actual_failed_producer_outcome",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);

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
    if !isolated_capability_test(
        "native_upload_cli_consumes_current_reference_and_rejects_pointer_digest_drift",
    ) {
        return;
    }
    use retained_artifacts::retention::{publish_owned_original, reference_report, settle_report};
    let h = report_host();
    let original = ".chrono-harness/state/preparation/native-cli.json";
    let slot = ".chrono-harness/state/check.json";
    let raw = serde_json::to_vec(
        &json!({"request":{"request_id":"native-cli"},"response":{"status":"failed"}}),
    )
    .unwrap();
    publish_owned_original(
        h.path(),
        "check-original",
        original,
        &raw,
        json!({"exit":1}),
    )
    .unwrap();
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
    // Ordinary preparation failure produces an acquisition original but no
    // final report slot. Upload success cannot stand in for that report.
    let absent = call("all");
    assert_eq!(absent.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&absent.stderr).contains(slot),
        "{absent:?}"
    );
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-cli")
            .is_some()
    );
    fs::write(h.path().join(slot), pointer("0".repeat(64))).unwrap();
    assert_eq!(call("all").status.code(), Some(2));
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-cli")
            .is_some()
    );
    fs::write(h.path().join(slot), pointer(chrono_harness::sha256(&raw))).unwrap();
    let saved = h.path().join(format!("{original}.fixture-original"));
    fs::rename(h.path().join(original), &saved).unwrap();
    let missing = call("all");
    fs::rename(saved, h.path().join(original)).unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains(original),
        "{missing:?}"
    );
    assert!(
        ledger(h.path())["roots"]
            .get("native-delivery:native-cli")
            .is_some()
    );
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
    if !isolated_capability_test("staged_blob_checks_capacity_before_copy_and_preserves_the_input")
    {
        return;
    }
    let h = report_host();
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(128);

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
    if !isolated_capability_test(
        "interrupted_lifecycle_original_stays_rooted_for_its_actual_recovery_reader",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["producers"]["lifecycle-original"] = json!([".chrono-harness/state/fixtures/"]);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
        .args([h.path().to_str().unwrap(), "lifecycle-recovery"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn explicit_small_reservations_admit_concurrent_outputs_and_enforce_each_bound() {
    if !isolated_capability_test(
        "explicit_small_reservations_admit_concurrent_outputs_and_enforce_each_bound",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_output_bytes"] = json!(1024);

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
    let ninth = h.path().join(".chrono-harness/state/fixtures/ninth");
    let independent = retained_artifacts::retention::create_adopted_with_limit(
        h.path(),
        "fixture",
        &ninth,
        true,
        json!(null),
        Some(1),
    )
    .unwrap()
    .unwrap();
    assert!(ninth.is_dir());
    independent.complete(json!({"exit":9})).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 8), 8);
    assert!(!ninth.exists());
    for (n, p) in live.into_iter().enumerate() {
        p.complete(json!({"exit":n})).unwrap();
    }
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert_eq!(ledger(h.path())["summaries"].as_array().unwrap().len(), 9);
}
#[test]
fn ordinary_maintenance_defers_only_busy_metadata_without_spending_another_round() {
    if !isolated_capability_test(
        "ordinary_maintenance_defers_only_busy_metadata_without_spending_another_round",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = tree(h.path(), "busy");
    let publication = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    let directory = retention_state::object_path(h.path(), publication.id())
        .parent()
        .unwrap()
        .to_path_buf();
    publication.complete(json!({"exit":7})).unwrap();
    let gate = chrono_harness::ownership::DirectoryLease::acquire(&directory, true, None)
        .unwrap()
        .unwrap();
    let began = std::time::Instant::now();
    let round = inventory(h.path()).maintain().unwrap();
    assert_eq!(round["remaining_outputs"], Value::Null);
    assert!(!round["accounting_errors"].as_array().unwrap().is_empty());
    assert!(began.elapsed() < std::time::Duration::from_secs(1));
    assert!(h.path().join(&path).exists());
    drop(gate);
    assert_eq!(
        inventory(h.path()).maintain().unwrap()["remaining_outputs"],
        0
    );
}

#[test]
fn independent_report_content_writers_progress_and_retire_their_transaction_metadata() {
    if !isolated_capability_test(
        "independent_report_content_writers_progress_and_retire_their_transaction_metadata",
    ) {
        return;
    }
    use retained_artifacts::retention::{publish_owned_observed, publish_owned_original};
    let h = report_host();
    let _activity = inventory(h.path()).read_scope().unwrap();
    let first = ".chrono-harness/state/preparation/first-writer.json";
    let second = ".chrono-harness/state/preparation/second-writer.json";
    publish_owned_observed(
        h.path(),
        "check-original",
        first,
        b"first original",
        json!({"exit":7}),
        || {
            publish_owned_original(
                h.path(),
                "check-original",
                second,
                b"second original",
                json!({"exit":3}),
            )
            .unwrap();
            assert_eq!(fs::read(h.path().join(second)).unwrap(), b"second original");
            drop(retained_artifacts::retention::read_guard(h.path(), second).unwrap());
            // The first writer is still inside its original content operation.
            assert_eq!(fs::read(h.path().join(first)).unwrap(), b"");
        },
    )
    .unwrap();
    assert_eq!(fs::read(h.path().join(first)).unwrap(), b"first original");
    drop(retained_artifacts::retention::read_guard(h.path(), first).unwrap());
    let state = ledger(h.path());
    let directory = h.path().join(".chrono-harness/state/output-retention-v1");
    let declarations = || {
        fs::read_dir(&directory)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("transaction-") && n.ends_with(".json"))
            .collect::<Vec<_>>()
    };
    assert_eq!(declarations().len(), 2);
    assert!(state.get("transactions").is_none());
    let originals = state["outputs"].as_array().unwrap();
    assert_eq!(
        originals.iter().find(|o| o["path"] == first).unwrap()["outcome"]["exit"],
        7
    );
    assert_eq!(
        originals.iter().find(|o| o["path"] == second).unwrap()["outcome"]["exit"],
        3
    );
    let first_key = format!("writer-{}", chrono_harness::sha256(first.as_bytes()));
    let first_record = directory.join(format!("transaction-{first_key}.json"));
    let original_record = fs::read(&first_record).unwrap();
    let mut changed: Value = serde_json::from_slice(&original_record).unwrap();
    changed["phase"] = json!("unknown independent state");
    let changed_record = serde_json::to_vec(&changed).unwrap();
    fs::write(&first_record, &changed_record).unwrap();
    let partial = maintenance(h.path());
    assert!(
        partial["accounting_errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["transaction"] == first_key)
    );
    assert_eq!(fs::read(&first_record).unwrap(), changed_record);
    assert_eq!(
        declarations().len(),
        1,
        "the unrelated declared writer must retire"
    );
    assert!(directory.join(&first_key).exists());
    assert_eq!(fs::read(h.path().join(first)).unwrap(), b"first original");
    assert_eq!(fs::read(h.path().join(second)).unwrap(), b"second original");
    // Restore only the corruption introduced by this test, then resume the
    // actual owner's same bounded cleanup without releasing report consumers.
    fs::write(first_record, original_record).unwrap();
    maintenance(h.path());
    assert!(declarations().is_empty());
    for path in [first, second] {
        assert!(
            !directory
                .join(format!(
                    "writer-{}",
                    chrono_harness::sha256(path.as_bytes())
                ))
                .exists()
        );
    }
    assert!(!directory.join("report-writer.lease").exists());
    assert!(!directory.join("report-slot.lease").exists());
}

#[test]
fn independent_report_slots_publish_while_another_slot_is_held() {
    if !isolated_capability_test("independent_report_slots_publish_while_another_slot_is_held") {
        return;
    }
    use retained_artifacts::retention::{publish_owned_original, reference_report, settle_report};
    let h = report_host();
    let first = ".chrono-harness/state/preparation/slot-first.json";
    let second = ".chrono-harness/state/preparation/slot-second.json";
    let _activity = inventory(h.path()).read_scope().unwrap();
    for (path, content) in [
        (first, b"first original".as_slice()),
        (second, b"second original".as_slice()),
    ] {
        publish_owned_original(h.path(), "check-original", path, content, json!({"exit":0}))
            .unwrap();
        drop(retained_artifacts::retention::read_guard(h.path(), path).unwrap());
    }
    let first_slot = reference_report(h.path(), "first/check.json", first, &[], None).unwrap();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = std::sync::mpsc::channel();
    let root = h.path().to_owned();
    let competing = std::thread::spawn(move || {
        entered_tx.send(()).unwrap();
        let guard = reference_report(&root, "first/check.json", first, &[], None).unwrap();
        finished_tx.send(()).unwrap();
        drop(guard);
    });
    entered_rx.recv().unwrap();
    let second_slot = reference_report(h.path(), "second/check.json", second, &[], None).unwrap();
    assert_eq!(
        finished_rx.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    );
    settle_report(h.path(), "second/check.json", second).unwrap();
    settle_report(h.path(), "first/check.json", first).unwrap();
    drop(second_slot);
    drop(first_slot);
    competing.join().unwrap();
    finished_rx.recv().unwrap();
    assert_eq!(fs::read(h.path().join(first)).unwrap(), b"first original");
    assert_eq!(fs::read(h.path().join(second)).unwrap(), b"second original");
}

#[test]
fn synchronous_completion_seals_across_exhausted_rounds_before_handoff() {
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/synchronous-result";
    let publication = inventory(h.path())
        .plan_rooted(
            "fixture",
            path,
            json!({"request":"original"}),
            "domain-recovery",
        )
        .unwrap();
    let id = publication.id().to_owned();
    publication.materialize().unwrap();
    let bytes = b"exact producer result";
    fs::write(h.path().join(path).join("result.json"), bytes).unwrap();
    let mut rounds = 0;
    publication
        .complete_sealed_with_round_budget(json!({"publication":"committed"}), |steps| {
            if steps == 0 {
                rounds += 1;
                assert_eq!(
                    inventory(h.path())
                        .rooted_original("fixture", "domain-recovery")
                        .unwrap()
                        .unwrap()
                        .0,
                    id
                );
            }
            steps >= 1
        })
        .unwrap();
    assert!(
        rounds > 1,
        "the clock must force multiple actual sealing rounds"
    );
    let state = ledger(h.path());
    let output = state["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap();
    assert_eq!(output["phase"], "released", "{output}");
    assert_eq!(output["manifest"]["sealed"], true, "{output}");
    assert_eq!(output["outcome"]["publication"], "committed");
    let _guard = retained_artifacts::retention::read_guard(h.path(), path)
        .unwrap()
        .unwrap();
    assert!(
        inventory(h.path())
            .published_tree_member(
                "fixture",
                path,
                &format!("{path}/result.json"),
                &chrono_harness::sha256(bytes),
                bytes.len() as u64
            )
            .unwrap()
    );
}

#[test]
fn synchronous_sealing_preserves_recovery_and_actual_error_on_changed_tree() {
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/changed-synchronous-result";
    let publication = inventory(h.path())
        .plan_rooted(
            "fixture",
            path,
            json!({"request":"original"}),
            "domain-recovery",
        )
        .unwrap();
    let id = publication.id().to_owned();
    publication.materialize().unwrap();
    fs::write(h.path().join(path).join("result.json"), b"original").unwrap();
    let mut rounds = 0;
    let error = publication
        .complete_sealed_with_round_budget(json!({"publication":"committed"}), |steps| {
            if steps == 0 {
                rounds += 1;
                if rounds == 2 {
                    fs::write(h.path().join(path).join("unexpected"), b"changed").unwrap();
                }
            }
            steps >= 1
        })
        .unwrap_err();
    assert!(error.contains("tree changed while sealing"), "{error}");
    let original = inventory(h.path())
        .rooted_original("fixture", "domain-recovery")
        .unwrap()
        .unwrap();
    assert_eq!(original.0, id);
    assert_eq!(original.2, "sealing");
    assert_eq!(
        fs::read(h.path().join(path).join("result.json")).unwrap(),
        b"original"
    );
}

#[test]
fn completion_io_allows_independent_publication_and_preserves_new_references() {
    if !isolated_capability_test(
        "completion_io_allows_independent_publication_and_preserves_new_references",
    ) {
        return;
    }
    let h = host(256, 16);
    let original = tree(h.path(), "completion-original");
    let parent = tree(h.path(), "completion-parent");
    let first = inventory(h.path())
        .begin("fixture", &original, json!(null))
        .unwrap();
    let first_id = first.id().to_owned();
    first.complete(json!({"exit":7})).unwrap();
    let publication = inventory(h.path())
        .begin("fixture", &parent, json!(null))
        .unwrap();
    let id = publication.id().to_owned();
    publication
        .complete_observed(json!({"exit":101}), || {
            // Content IO must have no global admission held. The original owner
            // capability still prevents this output's disposal at the boundary.
            assert!(
                !h.path()
                    .join(".chrono-harness/state/output-retention-v1/admission.lease")
                    .exists()
            );
            inventory(h.path())
                .dependencies(&id, &[first_id.clone()])
                .unwrap();
            inventory(h.path())
                .reference("current-completion", &[id.clone()])
                .unwrap();
            let sibling = tree(h.path(), "completion-sibling");
            inventory(h.path())
                .begin("fixture", &sibling, json!(null))
                .unwrap()
                .complete(json!({"exit":3}))
                .unwrap();
        })
        .unwrap();
    let state = ledger(h.path());
    let output = state["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap();
    assert_eq!(output["outcome"]["exit"], 101);
    assert_eq!(output["references"], json!([first_id]));
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    assert!(h.path().join(original).exists());
    assert!(h.path().join(parent).exists());
    inventory(h.path())
        .reference("current-completion", &[])
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
}

#[test]
fn ordinary_concurrent_publications_admit_against_retained_inventory() {
    if !isolated_capability_test(
        "ordinary_concurrent_publications_admit_against_retained_inventory",
    ) {
        return;
    }
    let h = host(256, 128);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["keep_seconds"] = json!(604800);
    policy["max_related_outputs"] = json!(512);
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
                    let publication = retained_artifacts::retention::create_adopted_for_consumer(
                        root,
                        "fixture",
                        &path,
                        true,
                        json!({"operation":n}),
                        Some(32 * 1024 * 1024),
                        "fixture-recheck",
                    )?
                    .ok_or("adoption missing")?;
                    publication.write_file(&path.join("original"), b"required original")?;
                    let consumer = format!("fixture-recheck:{}", publication.id());
                    publication.complete_releasing_consumer(json!({"exit":0}), &consumer)
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

#[cfg(not(verus_keep_ghost))]
fn consumer_input(root: &Path, consumer: &str, originals: Value) -> Value {
    let raw = serde_json::to_vec(&json!({"schema":"chrono-retention-consumer/v1","consumer":consumer,"purpose":"Repair and recheck required original failure inputs; interruption is not release","originals":originals})).unwrap();
    fs::write(root.join("consumer.json"), &raw).unwrap();
    json!({"path":"consumer.json","sha256":chrono_harness::sha256(&raw)})
}
#[cfg(not(verus_keep_ghost))]
fn consumer_call(root: &Path, request: Value) -> chrono_harness::CliOutput {
    fs::write(
        root.join("consumer-request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    chrono_harness::dispatch(&[
        "retention",
        "--host-root",
        root.to_str().unwrap(),
        "--request",
        "consumer-request.json",
    ])
}
#[cfg(not(verus_keep_ghost))]
fn original_identity(root: &Path, id: &str, path: &str) -> Value {
    json!({"id":id,"path":path,"identity":chrono_harness::artifact_disposal::identity(&root.join(path)).unwrap()})
}
#[cfg(not(verus_keep_ghost))]
fn consumer_release(root: &Path, declaration: &Value, state: &str) -> Value {
    // This fixture's responsible repair consumer produces its actual domain state.
    let raw = serde_json::to_vec(&json!({"repair":{"consumer":serde_json::from_slice::<Value>(&fs::read(root.join("consumer.json")).unwrap()).unwrap()["consumer"],"declaration_sha256":declaration["sha256"],"state":state}})).unwrap();
    fs::write(root.join("repair-result.json"), &raw).unwrap();
    json!({"operation":"release-consumer","declaration":declaration,"receipt":{"path":"repair-result.json","sha256":chrono_harness::sha256(&raw)},"pointer":"/repair"})
}

#[test]
fn preserved_original_release_requires_exact_archive_roles_and_closed_users() {
    if !isolated_capability_test(
        "preserved_original_release_requires_exact_archive_roles_and_closed_users",
    ) {
        return;
    }
    let h = host(256, 16);
    let survivor = tempfile::tempdir().unwrap();
    let path = ".chrono-harness/state/fixtures/interrupted.json";
    let raw = b"{\"exit\":101,\"completion\":\"unknown\"}\n";
    fs::write(h.path().join(path), raw).unwrap();
    fs::write(survivor.path().join("original.json"), raw).unwrap();
    let role = format!("recovery:{path}");
    let publication = inventory(h.path())
        .begin_rooted(
            "fixture",
            path,
            json!({"completion":"unknown"}),
            &[role.clone()],
        )
        .unwrap();
    let id = publication.id().to_owned();
    let declaration = json!({
        "operation":"release-preserved",
        "original":original_identity(h.path(), &id, path),
        "roles":[role],
        "survivor_root":fs::canonicalize(survivor.path()).unwrap(),
        "survivor_path":"original.json",
        "sha256":chrono_harness::sha256(raw),
        "length":raw.len(),
        "current_consumers_released":true,
        "reason":"The responsible caller joined the interrupted producer and preserves its exact unknown result outside the retired checkout"
    });
    let live = consumer_call(h.path(), declaration.clone());
    assert_eq!(live.exit_code, 2, "{}", live.stdout);
    drop(publication);
    let reader = inventory(h.path()).consume(&id).unwrap();
    assert_eq!(consumer_call(h.path(), declaration.clone()).exit_code, 2);
    drop(reader);
    let before = ledger(h.path());
    for field in [
        "sha256",
        "roles",
        "current_consumers_released",
        "survivor_path",
    ] {
        let mut rejected = declaration.clone();
        rejected[field] = match field {
            "sha256" => json!("0".repeat(64)),
            "roles" => json!([]),
            "current_consumers_released" => json!(false),
            _ => json!("missing.json"),
        };
        assert_eq!(consumer_call(h.path(), rejected).exit_code, 2);
        assert_eq!(ledger(h.path())["outputs"], before["outputs"]);
        assert_eq!(ledger(h.path())["roots"], before["roots"]);
    }
    // An address outside the checkout is insufficient: the archive must not
    // alias the original's physical file, even when every byte matches.
    let alias = survivor.path().join("hardlink.json");
    fs::hard_link(h.path().join(path), &alias).unwrap();
    let mut linked = declaration.clone();
    linked["survivor_path"] = json!("hardlink.json");
    let rejected = consumer_call(h.path(), linked);
    assert_eq!(rejected.exit_code, 2, "{}", rejected.stdout);
    assert_eq!(ledger(h.path())["outputs"], before["outputs"]);
    assert_eq!(ledger(h.path())["roots"], before["roots"]);
    assert_eq!(fs::read(&alias).unwrap(), raw);
    fs::remove_file(alias).unwrap();
    let result = consumer_call(h.path(), declaration.clone());
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    let output = &ledger(h.path())["outputs"][0];
    assert_eq!(output["outcome"], json!({"completion":"unknown"}));
    assert_eq!(output["phase"], "publishing");
    assert_eq!(output["expires"], before["outputs"][0]["expires"]);
    assert!(ledger(h.path())["roots"].as_object().unwrap().is_empty());
    assert_eq!(consumer_call(h.path(), declaration.clone()).exit_code, 0);
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert_eq!(
        fs::read(survivor.path().join("original.json")).unwrap(),
        raw
    );
    assert_eq!(consumer_call(h.path(), declaration).exit_code, 0);
}

#[test]
fn archived_interrupted_report_retires_when_its_transitive_inputs_exceed_the_round_limit() {
    if !isolated_capability_test(
        "archived_interrupted_report_retires_when_its_transitive_inputs_exceed_the_round_limit",
    ) {
        return;
    }
    let h = host(256, 16);
    let root = h.path();
    let config = root.join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(4);
    fs::write(config, serde_json::to_vec(&policy).unwrap()).unwrap();
    let archive = tempfile::tempdir().unwrap();
    let inv = inventory(root);
    let path = ".chrono-harness/state/fixtures/interrupted-parent";
    let role = format!("recovery:{path}");
    fs::write(root.join(path), b"").unwrap();
    fs::write(archive.path().join("parent"), b"").unwrap();
    let parent = inventory(root)
        .begin_rooted("fixture", path, json!(null), std::slice::from_ref(&role))
        .unwrap();
    let id = parent.id().to_owned();
    let publish = |name: &str| {
        let path = format!(".chrono-harness/state/fixtures/{name}");
        fs::write(root.join(&path), b"independent input").unwrap();
        let output = inventory(root)
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = output.id().to_owned();
        output.complete_sealed(json!({"exit":7})).unwrap();
        (id, path)
    };
    let children = [publish("first-child"), publish("second-child")];
    let child_ids = children
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    inv.dependencies(&id, &child_ids).unwrap();
    drop(parent);
    let record_path = retention_state::object_path(root, &id);
    let record = || serde_json::from_slice::<Value>(&fs::read(&record_path).unwrap()).unwrap();
    let round = inv.maintain_originals(std::slice::from_ref(&id)).unwrap();
    assert_eq!(round["accounting_errors"], json!([]));
    let interrupted = record()["output"].clone();
    assert_eq!(interrupted["phase"], "sealing");
    assert_eq!(interrupted["manifest"]["sealed"], true);
    assert_eq!(interrupted["outcome"]["publication"], "interrupted");
    let moved = consumer_call(
        root,
        json!({
            "operation":"release-preserved","original":original_identity(root,&id,path),"roles":[role],
            "survivor_root":fs::canonicalize(archive.path()).unwrap(),"survivor_path":"parent",
            "sha256":chrono_harness::sha256(b""),"length":0,"current_consumers_released":true,
            "reason":"recovery read back the interrupted parent from its independent copy"
        }),
    );
    assert_eq!(moved.exit_code, 0, "{}", moved.stderr);
    // Each independent child adds its inputs after the parent has been handed off.
    // Every update fits the limit, while expanding the whole graph would require seven objects.
    let mut originals = Vec::new();
    for (index, (child_id, child_path)) in children.into_iter().enumerate() {
        let (first, first_path) = publish(&format!("{index}-a"));
        let (second, second_path) = publish(&format!("{index}-b"));
        inv.dependencies(&child_id, &[first, second]).unwrap();
        originals.extend([child_path, first_path, second_path]);
    }
    let mut disposed = None;
    for _ in 0..12 {
        let round = inv.maintain_originals(std::slice::from_ref(&id)).unwrap();
        assert_eq!(round["accounting_errors"], json!([]), "{round}");
        if let Some(summary) = round["disposed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
        {
            disposed = Some(summary.clone());
            break;
        }
    }
    assert_eq!(
        disposed.unwrap()["original_outcome"],
        interrupted["outcome"]
    );
    assert!(!root.join(path).exists());
    assert_eq!(fs::read(archive.path().join("parent")).unwrap(), b"");
    for original in originals {
        assert_eq!(fs::read(root.join(original)).unwrap(), b"independent input");
    }
}

#[test]
fn archived_run_requires_all_members_and_releases_the_exact_expired_slot() {
    if !isolated_capability_test(
        "archived_run_requires_all_members_and_releases_the_exact_expired_slot",
    ) {
        return;
    }
    archived_install_run_handoff(None);
    archived_install_run_handoff(Some(0));
    archived_install_run_handoff(Some(1));
    archived_install_run_handoff(Some(86400));
}

#[cfg(not(verus_keep_ghost))]
fn archived_install_run_handoff(diagnostic_window: Option<u64>) {
    let host = deadline_host();
    let root = host.path();
    let archive = tempfile::tempdir().unwrap();
    let survivor = fs::canonicalize(archive.path()).unwrap();
    let run = ".chrono-harness/state/fixtures/install-run";
    let slot = ".chrono-harness/state/installed.json";
    let report = format!("{run}/report.json");
    let stream = format!("{run}/stdout");
    let raw = b"successful install";
    let policy_file = root.join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_file).unwrap()).unwrap();
    policy["producers"]["check-original"] = json!([".chrono-harness/state/fixtures/"]);
    if let Some(seconds) = diagnostic_window {
        policy
            .as_object_mut()
            .unwrap()
            .remove("max_lifetime_seconds");
        policy["diagnostic_keep_seconds"] = json!(seconds);
    }
    fs::write(policy_file, serde_json::to_vec(&policy).unwrap()).unwrap();
    for base in [root, survivor.as_path()] {
        fs::create_dir_all(base.join(run)).unwrap();
        fs::write(base.join(slot), raw).unwrap();
        fs::write(base.join(&report), raw).unwrap();
    }
    fs::write(root.join(&stream), b"download complete").unwrap();
    let inv = inventory(root);
    let publication = inventory(root)
        .begin("check-original", run, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication
        .complete_sealed(json!({"status":"passed"}))
        .unwrap();
    let role = retained_artifacts::retention::current_report_role(slot);
    inv.reference(&role, std::slice::from_ref(&id)).unwrap();
    let request = json!({"operation":"release-preserved","original":original_identity(root,&id,run),
        "roles":[role],"survivor_root":survivor,"survivor_path":report,"sha256":chrono_harness::sha256(raw),
        "length":raw.len(),"archived_slot":{"path":slot,"identity":chrono_harness::artifact_disposal::identity(&root.join(slot)).unwrap()},
        "reason":"the installer diagnostics are read from the archive","current_consumers_released":true});
    let reject = || {
        let result = consumer_call(root, request.clone());
        assert_eq!(result.exit_code, 2, "{}", result.stdout);
        assert!(root.join(slot).exists());
        assert_eq!(ledger(root)["roots"][&role], json!([id]));
    };
    fs::copy(root.join(&stream), survivor.join(&stream)).unwrap();
    reject();
    let reader = inv.consume(&id).unwrap();
    let mut output = ledger(root)["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap()
        .clone();
    if diagnostic_window.is_none() {
        output["lifetime"] = json!({"created_at":1,"deadline":2});
    } else {
        assert!(output["lifetime"].is_null());
    }
    output["expires"] = json!(2);
    retention_state::replace_output(root, &output);
    reject();
    drop(reader);
    if diagnostic_window == Some(86400) {
        reject();
        return;
    }
    if diagnostic_window == Some(1) {
        let born = fs::metadata(root.join(run)).unwrap().created().unwrap();
        let due = born + std::time::Duration::from_secs(1);
        if let Ok(remaining) = due.duration_since(std::time::SystemTime::now()) {
            std::thread::sleep(remaining);
        }
    }
    fs::remove_file(survivor.join(&stream)).unwrap();
    reject();
    fs::write(survivor.join(&stream), b"changed contents!").unwrap();
    reject();
    fs::copy(root.join(&stream), survivor.join(&stream)).unwrap();
    fs::remove_file(survivor.join(&report)).unwrap();
    fs::hard_link(root.join(slot), survivor.join(&report)).unwrap();
    reject();
    fs::remove_file(survivor.join(&report)).unwrap();
    fs::copy(root.join(&report), survivor.join(&report)).unwrap();
    let result = consumer_call(root, request.clone());
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert!(!root.join(slot).exists());
    if let Some(seconds) = diagnostic_window {
        let born = fs::metadata(root.join(run))
            .unwrap()
            .created()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let saved = ledger(root);
        let run = saved["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["id"] == id)
            .unwrap();
        assert_eq!(
            run["lifetime"],
            json!({"created_at":born,"deadline":born+seconds})
        );
    }
    assert!(ledger(root)["roots"].get(&role).is_none());
    assert_eq!(consumer_call(root, request).exit_code, 0);
    assert_eq!(maintain_fixture_to(root, 0), 0);
    assert!(!root.join(run).exists());
    assert_eq!(
        fs::read(survivor.join(&stream)).unwrap(),
        b"download complete"
    );
}

#[test]
fn archived_check_requires_its_slot_and_every_sealed_input_before_release() {
    if !isolated_capability_test(
        "archived_check_requires_its_slot_and_every_sealed_input_before_release",
    ) {
        return;
    }
    let host = deadline_host();
    let root = host.path();
    let archive = tempfile::tempdir().unwrap();
    let survivor = fs::canonicalize(archive.path()).unwrap();
    let directory = ".chrono-harness/state/fixtures";
    fs::create_dir_all(survivor.join(directory)).unwrap();
    let report = format!("{directory}/report.json");
    let input = format!("{directory}/result");
    let member = format!("{input}/stdout");
    let slot = ".chrono-harness/state/check.json";
    let policy_path = root.join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["producers"]["check-original"] = json!([format!("{directory}/")]);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let inv = inventory(root);
    for base in [root, survivor.as_path()] {
        fs::create_dir(base.join(&input)).unwrap();
    }
    fs::write(root.join(&member), b"judge output").unwrap();
    let child = inventory(root)
        .begin("check-original", &input, json!({}))
        .unwrap();
    let child_id = child.id().to_owned();
    child.complete_sealed(json!({"exit_code":0})).unwrap();
    inv.reference("shared-input", std::slice::from_ref(&child_id))
        .unwrap();
    let raw = serde_json::to_vec(&json!({"schema":"chrono-check-report/v2","retained_report":report,
        "request":{"host_root":fs::canonicalize(root).unwrap(),"request_id":"checked",
        "protocol":"chrono-ci-judge/v2","config_path":".chrono-harness/ci/check.json","config_sha256":"a".repeat(64),
        "base":null,"candidate":"c".repeat(40),"initial":true},"publication":{"schema":"chrono-report-publication/v1",
        "slot":slot,"previous_sha256":null,"snapshot_failure":null,"originals":[member],
        "native_delivery":null,"recovery_root":null,"exit_code":0}})).unwrap();
    let digest = chrono_harness::sha256(&raw);
    let pointer = serde_json::to_vec(
        &json!({"schema":"chrono-check-reference/v1","original":{"path":report,"sha256":digest}}),
    )
    .unwrap();
    for base in [root, survivor.as_path()] {
        fs::write(base.join(&report), &raw).unwrap();
        fs::write(base.join(slot), &pointer).unwrap();
    }
    let publication = inventory(root)
        .begin("check-original", &report, json!({}))
        .unwrap();
    let id = publication.id().to_owned();
    publication
        .complete_sealed(json!({"request_id":"checked","exit_code":0}))
        .unwrap();
    inv.dependencies(&id, std::slice::from_ref(&child_id))
        .unwrap();
    let role = retained_artifacts::retention::current_report_role(slot);
    inv.reference(&role, std::slice::from_ref(&id)).unwrap();
    let mut output = ledger(root)["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap()
        .clone();
    output["lifetime"] = json!({"created_at":1,"deadline":2});
    output["expires"] = json!(2);
    retention_state::replace_output(root, &output);
    let mut request = json!({"operation":"release-preserved","original":original_identity(root,&id,&report),
        "roles":[role],"survivor_root":survivor,"survivor_path":report,"sha256":digest,"length":raw.len(),
        "reason":"The maintainer now reads this check and its inputs from the archive","current_consumers_released":true});
    let roots = ledger(root)["roots"].clone();
    let reject = |request: &Value| {
        let result = consumer_call(root, request.clone());
        assert_eq!(result.exit_code, 2, "{}", result.stdout);
        assert_eq!(ledger(root)["roots"], roots);
        assert!(root.join(slot).is_file());
    };
    reject(&request);
    let slot_identity = chrono_harness::artifact_disposal::identity(&root.join(slot)).unwrap();
    request["archived_slot"] = json!({"path":slot,"identity":slot_identity});
    reject(&request);
    fs::copy(root.join(&member), survivor.join(&member)).unwrap();
    let reader = inv.consume(&child_id).unwrap();
    reject(&request);
    drop(reader);
    fs::write(survivor.join(&member), b"wrong bytes").unwrap();
    reject(&request);
    fs::copy(root.join(&member), survivor.join(&member)).unwrap();
    fs::write(root.join(slot), b"new publication").unwrap();
    reject(&request);
    fs::write(root.join(slot), &pointer).unwrap();
    let mut stale = request.clone();
    stale["archived_slot"]["identity"] = json!("replaced");
    reject(&stale);
    let result = consumer_call(root, request.clone());
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert!(!root.join(slot).exists());
    assert!(ledger(root)["roots"].get(&role).is_none());
    assert_eq!(ledger(root)["roots"]["shared-input"], json!([child_id]));
    let mut slot_address = root.join(".chrono-harness/state/output-retention-v1/paths");
    for component in slot.split('/') {
        slot_address.push(chrono_harness::sha256(component.as_bytes()));
    }
    let marker: Value =
        serde_json::from_slice(&fs::read(slot_address.join("address.json")).unwrap()).unwrap();
    assert_eq!(marker["metadata_expires"], 2, "{marker}");
    assert_eq!(consumer_call(root, request).exit_code, 0);
    inv.maintain_explicit().unwrap();
    assert!(!slot_address.exists());
    assert_eq!(maintain_fixture_to(root, 1), 1);
    assert!(!root.join(&report).exists());
    assert!(root.join(&member).is_file());
    assert_eq!(fs::read(survivor.join(slot)).unwrap(), pointer);
    assert_eq!(fs::read(survivor.join(&member)).unwrap(), b"judge output");
}

#[test]
fn preserved_original_release_keeps_other_members_of_the_same_consumer() {
    if !isolated_capability_test(
        "preserved_original_release_keeps_other_members_of_the_same_consumer",
    ) {
        return;
    }
    let h = host(256, 16);
    let survivor = tempfile::tempdir().unwrap();
    let mut ids = Vec::new();
    for name in ["archived", "still-required"] {
        let path = format!(".chrono-harness/state/fixtures/{name}.json");
        fs::write(h.path().join(&path), b"original").unwrap();
        let p = inventory(h.path())
            .begin("fixture", &path, json!({"exit":7}))
            .unwrap();
        ids.push(p.id().to_owned());
        p.complete(json!({"exit":7})).unwrap();
    }
    inventory(h.path())
        .reference("shared-recheck", &ids)
        .unwrap();
    let path = ".chrono-harness/state/fixtures/archived.json";
    fs::write(survivor.path().join("original.json"), b"original").unwrap();
    let mut request = json!({"operation":"release-preserved",
        "original":original_identity(h.path(), &ids[0], path),"roles":["shared-recheck"],
        "survivor_root":h.path(),"survivor_path":path,
        "sha256":chrono_harness::sha256(b"original"),"length":8,
        "current_consumers_released":true,"reason":"Only the first member has been preserved and released"});
    assert_eq!(consumer_call(h.path(), request.clone()).exit_code, 2);
    request["survivor_root"] = json!(fs::canonicalize(survivor.path()).unwrap());
    request["survivor_path"] = json!("original.json");
    fs::write(survivor.path().join("original.json"), b"modified").unwrap();
    assert_eq!(consumer_call(h.path(), request.clone()).exit_code, 2);
    fs::write(survivor.path().join("original.json"), b"original").unwrap();
    let result = consumer_call(h.path(), request);
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(ledger(h.path())["roots"]["shared-recheck"], json!([ids[1]]));
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(
        h.path()
            .join(".chrono-harness/state/fixtures/still-required.json")
            .is_file()
    );
    assert_eq!(
        fs::read(survivor.path().join("original.json")).unwrap(),
        b"original"
    );
}

#[test]
fn maintenance_keeps_shared_role_members_outside_the_reference_component_unvisited() {
    if !isolated_capability_test(
        "maintenance_keeps_shared_role_members_outside_the_reference_component_unvisited",
    ) {
        return;
    }
    let h = host(256, 128);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(512);
    policy["max_millis_per_round"] = json!(25);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut ids = Vec::new();
    let mut originals = Vec::new();
    for n in 0..65 {
        let path = format!(".chrono-harness/state/fixtures/original-{n}");
        fs::write(h.path().join(&path), b"required original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        ids.push(publication.id().to_owned());
        publication.complete(json!({"exit": 7})).unwrap();
        originals.push(path);
    }
    let report = ".chrono-harness/state/fixtures/obsolete-report";
    fs::write(h.path().join(report), b"obsolete report").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", report, json!(null))
        .unwrap();
    let report_id = publication.id().to_owned();
    inventory(h.path())
        .dependencies(&report_id, &[ids[0].clone()])
        .unwrap();
    publication.complete(json!({"exit": 0})).unwrap();
    inventory(h.path())
        .reference("current-recheck", &ids[..64])
        .unwrap();

    // An interrupted root transition may have installed conservative attachment
    // protection before publishing its membership. Neither that attachment nor
    // current siblings belongs to the selected report's reference component.
    let role_path = retention_state::role_path(h.path(), "current-recheck");
    let mut role: Value = serde_json::from_slice(&fs::read(&role_path).unwrap()).unwrap();
    role["attached"]
        .as_array_mut()
        .unwrap()
        .push(json!(ids[64]));
    fs::write(&role_path, serde_json::to_vec(&role).unwrap()).unwrap();
    let role_before = fs::read(&role_path).unwrap();
    let interrupted_path = retention_state::object_path(h.path(), &ids[64]);
    let mut interrupted: Value =
        serde_json::from_slice(&fs::read(&interrupted_path).unwrap()).unwrap();
    interrupted["roles"] = json!(["current-recheck"]);
    fs::write(&interrupted_path, serde_json::to_vec(&interrupted).unwrap()).unwrap();
    let sibling_path = retention_state::object_path(h.path(), &ids[1]);
    fs::write(&sibling_path, b"unrelated damaged declaration").unwrap();
    let untouched: Vec<_> = ids[1..]
        .iter()
        .map(|id| {
            let path = retention_state::object_path(h.path(), id);
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    let mut rounds = Vec::new();
    for _ in 0..4 {
        let report_before: Value = serde_json::from_slice(
            &fs::read(retention_state::object_path(h.path(), &report_id)).unwrap(),
        )
        .unwrap();
        let references = report_before["output"]["references"]
            .as_array()
            .map(Vec::len);
        let round = diagnosed_maintenance_originals(h.path(), &report_id);
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(
            report_component_inventory_matches(
                rounds.len(),
                references,
                round["metadata_inventory_entries"].as_u64()
            ),
            "before: {report_before}; round: {round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
        rounds.push(round);
        if output_retired(h.path(), &report_id) {
            break;
        }
    }
    assert!(
        !h.path().join(report).exists(),
        "{rounds:?}; policy: {policy}; report declaration: {}; original declaration: {}",
        fs::read_to_string(retention_state::object_path(h.path(), &report_id)).unwrap(),
        fs::read_to_string(retention_state::object_path(h.path(), &ids[0])).unwrap()
    );
    assert_eq!(fs::read(role_path).unwrap(), role_before);
    for (path, bytes) in untouched {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    for path in originals {
        assert_eq!(fs::read(h.path().join(path)).unwrap(), b"required original");
    }
    let retained: Value =
        serde_json::from_slice(&fs::read(retention_state::object_path(h.path(), &ids[0])).unwrap())
            .unwrap();
    assert_eq!(retained["roles"], json!(["current-recheck"]));
    assert_eq!(retained["incoming"], json!([]));
}

#[test]
fn released_report_does_not_consume_sibling_reports_of_its_finalized_original() {
    if !isolated_capability_test(
        "released_report_does_not_consume_sibling_reports_of_its_finalized_original",
    ) {
        return;
    }
    let h = host(256, 128);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_millis_per_round"] = json!(25);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let original = ".chrono-harness/state/fixtures/shared-original";
    fs::write(h.path().join(original), b"required failure original").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", original, json!(null))
        .unwrap();
    let original_id = publication.id().to_owned();
    publication.complete(json!({"exit": 7})).unwrap();
    inventory(h.path())
        .reference("actual-recheck", &[original_id.clone()])
        .unwrap();
    let mut reports = Vec::new();
    for n in 0..17 {
        let path = format!(".chrono-harness/state/fixtures/report-{n}");
        fs::write(h.path().join(&path), b"obsolete report").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        inventory(h.path())
            .dependencies(&id, &[original_id.clone()])
            .unwrap();
        publication.complete(json!({"exit": 0})).unwrap();
        reports.push((id, path));
    }
    let (selected, path) = &reports[0];
    let original_record = retention_state::object_path(h.path(), &original_id);
    let original_before: Value =
        serde_json::from_slice(&fs::read(&original_record).unwrap()).unwrap();
    let exclusion = chrono_harness::ownership::DirectoryLease::acquire(
        original_record.parent().unwrap(),
        true,
        None,
    )
    .unwrap()
    .unwrap();
    let refused = inventory(h.path())
        .maintain_originals(&[selected.clone()])
        .unwrap();
    assert!(!refused["accounting_errors"].as_array().unwrap().is_empty());
    assert_eq!(fs::read(h.path().join(path)).unwrap(), b"obsolete report");
    drop(exclusion);

    let broken = retention_state::object_path(h.path(), &reports[1].0);
    fs::write(&broken, b"unrelated sibling declaration is damaged").unwrap();
    let sibling = retention_state::object_path(h.path(), &reports[2].0);
    let sibling_exclusion =
        chrono_harness::ownership::DirectoryLease::acquire(sibling.parent().unwrap(), true, None)
            .unwrap()
            .unwrap();
    let untouched: Vec<_> = reports[1..]
        .iter()
        .map(|(id, _)| {
            let p = retention_state::object_path(h.path(), id);
            let bytes = fs::read(&p).unwrap();
            (p, bytes)
        })
        .collect();
    let role = retention_state::role_path(h.path(), "actual-recheck");
    let role_before = fs::read(&role).unwrap();
    let mut rounds = Vec::new();
    for _ in 0..4 {
        let report_before: Value = serde_json::from_slice(
            &fs::read(retention_state::object_path(h.path(), selected)).unwrap(),
        )
        .unwrap();
        let references = report_before["output"]["references"]
            .as_array()
            .map(Vec::len);
        let round = inventory(h.path())
            .maintain_originals(&[selected.clone()])
            .unwrap();
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(
            report_component_inventory_matches(
                rounds.len(),
                references,
                round["metadata_inventory_entries"].as_u64()
            ),
            "before: {report_before}; round: {round}"
        );
        let retired = output_retired(h.path(), selected);
        assert!(
            shared_original_scope_matches(
                references,
                retired,
                round["scope"]["complete"].as_bool(),
                round["remaining_outputs"].is_null(),
                round["remaining_outputs"].as_u64()
            ),
            "before: {report_before}; retired: {retired}; round: {round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
        rounds.push(round);
        if retired {
            break;
        }
    }
    assert!(!h.path().join(path).exists(), "{rounds:?}");
    drop(sibling_exclusion);
    for (p, bytes) in untouched {
        assert_eq!(fs::read(p).unwrap(), bytes);
    }
    for (_, path) in &reports[1..] {
        assert_eq!(fs::read(h.path().join(path)).unwrap(), b"obsolete report");
    }
    assert_eq!(
        fs::read(h.path().join(original)).unwrap(),
        b"required failure original"
    );
    assert_eq!(fs::read(role).unwrap(), role_before);
    let retained: Value = serde_json::from_slice(&fs::read(original_record).unwrap()).unwrap();
    let expected: Vec<_> = original_before["incoming"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|id| id.as_str() != Some(selected.as_str()))
        .cloned()
        .collect();
    assert_eq!(retained["incoming"], json!(expected));
    assert_eq!(retained["output"], original_before["output"]);
    assert_eq!(retained["roles"], original_before["roles"]);
}

#[cfg(not(verus_keep_ghost))]
fn released_report_with_busy_retained_original(test: &str, consumer: bool) {
    if !isolated_capability_test(test) {
        return;
    }
    let h = host(256, 128);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_millis_per_round"] = json!(25);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let original = ".chrono-harness/state/fixtures/required-original";
    fs::write(
        h.path().join(original),
        b"original still required by its consumer",
    )
    .unwrap();
    let publication = inventory(h.path())
        .begin("fixture", original, json!(null))
        .unwrap();
    let original_id = publication.id().to_owned();
    publication.complete(json!({"exit": 7})).unwrap();
    inventory(h.path())
        .reference("actual-original-consumer", &[original_id.clone()])
        .unwrap();
    let report = ".chrono-harness/state/fixtures/obsolete-report";
    fs::write(h.path().join(report), b"obsolete report").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", report, json!(null))
        .unwrap();
    let report_id = publication.id().to_owned();
    inventory(h.path())
        .dependencies(&report_id, &[original_id.clone()])
        .unwrap();
    publication.complete(json!({"exit": 0})).unwrap();

    let role = retention_state::role_path(h.path(), "actual-original-consumer");
    let mut address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for component in original.split('/') {
        address.push(chrono_harness::sha256(component.as_bytes()));
    }
    address.push("address.json");
    let role_before = fs::read(&role).unwrap();
    let address_before = fs::read(&address).unwrap();
    let original_record = retention_state::object_path(h.path(), &original_id);
    let original_before: Value =
        serde_json::from_slice(&fs::read(&original_record).unwrap()).unwrap();
    let busy = if consumer { &role } else { &address };
    let exclusion =
        chrono_harness::ownership::DirectoryLease::acquire(busy.parent().unwrap(), true, None)
            .unwrap()
            .unwrap();
    let mut rounds = Vec::new();
    for _ in 0..4 {
        let report_before: Value = serde_json::from_slice(
            &fs::read(retention_state::object_path(h.path(), &report_id)).unwrap(),
        )
        .unwrap();
        let references = report_before["output"]["references"]
            .as_array()
            .map(Vec::len);
        let round = diagnosed_maintenance_originals(h.path(), &report_id);
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(
            report_component_inventory_matches(
                rounds.len(),
                references,
                round["metadata_inventory_entries"].as_u64()
            ),
            "before: {report_before}; round: {round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
        rounds.push(round);
        if output_retired(h.path(), &report_id) {
            break;
        }
    }
    let report_state: Value = serde_json::from_slice(
        &fs::read(retention_state::object_path(h.path(), &report_id)).unwrap(),
    )
    .unwrap();
    assert!(
        !h.path().join(report).exists(),
        "consumer metadata exclusion: {consumer}; rounds: {rounds:?}; policy: {policy}; report declaration: {report_state}"
    );
    assert_eq!(
        fs::read(h.path().join(original)).unwrap(),
        b"original still required by its consumer"
    );
    assert_eq!(fs::read(&role).unwrap(), role_before);
    assert_eq!(fs::read(&address).unwrap(), address_before);
    let retained: Value = serde_json::from_slice(&fs::read(original_record).unwrap()).unwrap();
    assert_eq!(retained["incoming"], json!([]));
    assert_eq!(retained["output"], original_before["output"]);
    assert_eq!(retained["roles"], original_before["roles"]);
    drop(exclusion);
}

#[test]
fn finalized_original_consumer_lease_does_not_block_released_report() {
    released_report_with_busy_retained_original(
        "finalized_original_consumer_lease_does_not_block_released_report",
        true,
    );
}

#[test]
fn released_report_detaches_many_originals_with_bounded_durable_progress() {
    if !isolated_capability_test(
        "released_report_detaches_many_originals_with_bounded_durable_progress",
    ) {
        return;
    }
    let h = host(8, 128);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(64);
    policy["max_millis_per_round"] = json!(25);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut originals = Vec::new();
    for n in 0..52 {
        let path = format!(".chrono-harness/state/fixtures/required-{n}");
        let bytes = format!("original failure {n}");
        fs::write(h.path().join(&path), &bytes).unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete(json!({"exit": 7})).unwrap();
        originals.push((id, path, bytes));
    }
    let ids: Vec<_> = originals.iter().map(|(id, _, _)| id.clone()).collect();
    inventory(h.path())
        .reference("actual-recheck", &ids)
        .unwrap();
    let role = retention_state::role_path(h.path(), "actual-recheck");
    let role_before = fs::read(&role).unwrap();
    let report = ".chrono-harness/state/fixtures/many-originals-report";
    let bytes = b"expired report for 52 required originals";
    fs::write(h.path().join(report), bytes).unwrap();
    let publication = inventory(h.path())
        .begin("fixture", report, json!(null))
        .unwrap();
    let report_id = publication.id().to_owned();
    inventory(h.path()).dependencies(&report_id, &ids).unwrap();
    publication.complete(json!({"exit": 0})).unwrap();
    let record = retention_state::object_path(h.path(), &report_id);
    let report_reader = inventory(h.path()).consume(&report_id).unwrap();
    let live = inventory(h.path())
        .maintain_originals(&[report_id.clone()])
        .unwrap();
    assert!(h.path().join(report).exists(), "{live}");
    assert!(live["disposed"].as_array().unwrap().is_empty(), "{live}");
    assert!(
        live["metadata_inventory_entries"].as_u64().unwrap() <= 2,
        "{live}"
    );
    drop(report_reader);

    let mut rounds = Vec::new();
    let mut removed = 0;
    let mut previous_references = ids.len();
    let mut checked_replacement = false;
    for _ in 0..ids.len() + 8 {
        // Reopen the owner on every call: progress must survive process loss.
        let round = diagnosed_maintenance_originals(h.path(), &report_id);
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(
            round["metadata_inventory_entries"].as_u64().unwrap() <= 2,
            "{round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 8, "{round}");
        assert!(
            round["verified_bytes"].as_u64().unwrap() <= 1048576,
            "{round}"
        );
        removed += round["removed_logical_bytes"].as_u64().unwrap();
        let state: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
        if state["output"].is_null() {
            rounds.push(round);
            break;
        }
        let references = state["output"]["references"].as_array().unwrap().len();
        assert!(references <= previous_references, "{state}");
        if references < previous_references {
            assert!(
                !h.path().join(report).exists(),
                "edges released before report bytes"
            );
            assert_eq!(state["output"]["phase"], "disposing");
            assert!(inventory(h.path()).consume(&report_id).is_err());
            assert!(
                inventory(h.path())
                    .reference("late-reader", &[report_id.clone()])
                    .is_err()
            );
            if !checked_replacement {
                fs::write(
                    h.path().join(report),
                    b"replacement is not the retired report",
                )
                .unwrap();
                let replacement = inventory(h.path())
                    .maintain_originals(&[report_id.clone()])
                    .unwrap();
                assert_eq!(replacement["removed_logical_bytes"], 0, "{replacement}");
                let preserved: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
                assert_eq!(
                    preserved["output"]["references"],
                    state["output"]["references"]
                );
                assert_eq!(
                    fs::read(h.path().join(report)).unwrap(),
                    b"replacement is not the retired report"
                );
                fs::remove_file(h.path().join(report)).unwrap();
                checked_replacement = true;
            }
        }
        previous_references = references;
        rounds.push(round);
    }
    let final_state: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert!(
        final_state["output"].is_null(),
        "{rounds:?}; policy: {policy}; report declaration: {final_state}"
    );
    assert!(checked_replacement);
    assert_eq!(removed, bytes.len() as u64, "{rounds:?}");
    assert!(!h.path().join(report).exists());
    assert_eq!(fs::read(role).unwrap(), role_before);
    for (id, path, bytes) in originals {
        assert_eq!(fs::read(h.path().join(path)).unwrap(), bytes.as_bytes());
        let original: Value =
            serde_json::from_slice(&fs::read(retention_state::object_path(h.path(), &id)).unwrap())
                .unwrap();
        assert_eq!(original["incoming"], json!([]));
        assert_eq!(original["output"]["outcome"]["exit"], 7);
        assert_eq!(original["roles"], json!(["actual-recheck"]));
    }
}

#[test]
fn finalized_original_path_lease_does_not_block_released_report() {
    released_report_with_busy_retained_original(
        "finalized_original_path_lease_does_not_block_released_report",
        false,
    );
}

#[test]
fn retiring_report_detaches_without_reacquiring_its_absent_path() {
    if !isolated_capability_test("retiring_report_detaches_without_reacquiring_its_absent_path") {
        return;
    }
    // The first two-node round removes bytes but cannot also detach an edge.
    let h = host(2, 128);
    let original = ".chrono-harness/state/fixtures/required-original";
    fs::write(
        h.path().join(original),
        b"original required by its consumer",
    )
    .unwrap();
    let publication = inventory(h.path())
        .begin("fixture", original, json!(null))
        .unwrap();
    let original_id = publication.id().to_owned();
    publication.complete(json!({"exit": 7})).unwrap();
    inventory(h.path())
        .reference("actual-original-consumer", &[original_id.clone()])
        .unwrap();
    let role = retention_state::role_path(h.path(), "actual-original-consumer");
    let role_before = fs::read(&role).unwrap();
    let report = ".chrono-harness/state/fixtures/retiring-report";
    fs::write(h.path().join(report), b"obsolete report").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", report, json!(null))
        .unwrap();
    let report_id = publication.id().to_owned();
    inventory(h.path())
        .dependencies(&report_id, &[original_id.clone()])
        .unwrap();
    publication.complete(json!({"exit": 0})).unwrap();
    let record = retention_state::object_path(h.path(), &report_id);
    let first = diagnosed_maintenance_originals(h.path(), &report_id);
    assert!(!h.path().join(report).exists(), "{first}");
    let pending: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(pending["output"]["phase"], "disposing", "{pending}");
    assert_eq!(
        pending["output"]["references"],
        json!([original_id]),
        "{pending}"
    );

    // An observed replacement is preserved before any outgoing protection ends.
    fs::write(
        h.path().join(report),
        b"replacement has no retirement authority",
    )
    .unwrap();
    let replacement = diagnosed_maintenance_originals(h.path(), &report_id);
    let preserved: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(
        preserved["output"]["references"], pending["output"]["references"],
        "{replacement}"
    );
    assert_eq!(
        fs::read(h.path().join(report)).unwrap(),
        b"replacement has no retirement authority"
    );
    fs::remove_file(h.path().join(report)).unwrap();

    let mut address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for component in report.split('/') {
        address.push(chrono_harness::sha256(component.as_bytes()));
    }
    address.push("address.json");
    let exclusion =
        chrono_harness::ownership::DirectoryLease::acquire(address.parent().unwrap(), true, None)
            .unwrap()
            .unwrap();
    let detached = diagnosed_maintenance_originals(h.path(), &report_id);
    assert!(
        detached["accounting_errors"].as_array().unwrap().is_empty(),
        "{detached}"
    );
    let waiting: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(
        waiting["output"]["references"],
        json!([]),
        "{detached}; {waiting}"
    );
    assert_eq!(waiting["output"]["phase"], "disposing");
    let original_record = retention_state::object_path(h.path(), &original_id);
    let retained: Value = serde_json::from_slice(&fs::read(&original_record).unwrap()).unwrap();
    assert_eq!(retained["incoming"], json!([]));

    // Final path/address retirement still waits for the actual path capability.
    let blocked = diagnosed_maintenance_originals(h.path(), &report_id);
    assert!(
        !blocked["accounting_errors"].as_array().unwrap().is_empty(),
        "{blocked}"
    );
    let still_waiting: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(still_waiting["output"], waiting["output"]);
    drop(exclusion);
    let retired = diagnosed_maintenance_originals(h.path(), &report_id);
    let done: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert!(done["output"].is_null(), "{retired}; {done}");
    let path_state: Value = serde_json::from_slice(&fs::read(address).unwrap()).unwrap();
    assert!(path_state["output"].is_null(), "{path_state}");
    assert!(!h.path().join(report).exists());
    assert_eq!(
        fs::read(h.path().join(original)).unwrap(),
        b"original required by its consumer"
    );
    assert_eq!(fs::read(role).unwrap(), role_before);
    let retained: Value = serde_json::from_slice(&fs::read(original_record).unwrap()).unwrap();
    assert_eq!(retained["output"]["outcome"]["exit"], 7);
    assert_eq!(retained["roles"], json!(["actual-original-consumer"]));
    assert_eq!(retained["incoming"], json!([]));
}

#[test]
fn report_retirement_preserves_unfinished_originals_and_their_busy_consumers() {
    if !isolated_capability_test(
        "report_retirement_preserves_unfinished_originals_and_their_busy_consumers",
    ) {
        return;
    }
    let h = host(256, 128);
    let original = ".chrono-harness/state/fixtures/finalized-original";
    fs::write(h.path().join(original), b"required original").unwrap();
    let publication = inventory(h.path())
        .begin("fixture", original, json!(null))
        .unwrap();
    let original_id = publication.id().to_owned();
    publication.complete(json!({"exit": 7})).unwrap();
    inventory(h.path())
        .reference("actual-original-consumer", &[original_id.clone()])
        .unwrap();
    let unfinished = ".chrono-harness/state/fixtures/unfinished-original";
    fs::write(
        h.path().join(unfinished),
        b"producer still owns these bytes",
    )
    .unwrap();
    let publication = inventory(h.path())
        .begin("fixture", unfinished, json!(null))
        .unwrap();
    let unfinished_id = publication.id().to_owned();
    let report = ".chrono-harness/state/fixtures/obsolete-report";
    fs::write(h.path().join(report), b"obsolete report").unwrap();
    let result = inventory(h.path())
        .begin("fixture", report, json!(null))
        .unwrap();
    let report_id = result.id().to_owned();
    inventory(h.path())
        .dependencies(&report_id, &[original_id.clone(), unfinished_id.clone()])
        .unwrap();
    result.complete(json!({"exit": 0})).unwrap();
    let original_outputs: Vec<_> = [&original_id, &unfinished_id]
        .into_iter()
        .map(|id| {
            let record = retention_state::object_path(h.path(), id);
            let state: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
            (record, state["output"].clone())
        })
        .collect();
    let role = retention_state::role_path(h.path(), "actual-original-consumer");
    let role_before = fs::read(&role).unwrap();
    let exclusion =
        chrono_harness::ownership::DirectoryLease::acquire(role.parent().unwrap(), true, None)
            .unwrap()
            .unwrap();
    let mut rounds = Vec::new();
    for _ in 0..4 {
        let round = inventory(h.path())
            .maintain_originals(&[report_id.clone()])
            .unwrap();
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(
            round["metadata_inventory_entries"].as_u64().unwrap() <= 2,
            "{round}"
        );
        rounds.push(round);
        let state: Value = serde_json::from_slice(
            &fs::read(retention_state::object_path(h.path(), &report_id)).unwrap(),
        )
        .unwrap();
        if state["output"].is_null() {
            break;
        }
    }
    assert!(!h.path().join(report).exists(), "{rounds:?}");
    assert!(output_retired(h.path(), &report_id), "{rounds:?}");
    for (path, output) in original_outputs {
        let state: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(state["output"], output);
        assert_eq!(state["incoming"], json!([]));
    }
    assert_eq!(
        fs::read(h.path().join(original)).unwrap(),
        b"required original"
    );
    assert_eq!(
        fs::read(h.path().join(unfinished)).unwrap(),
        b"producer still owns these bytes"
    );
    assert_eq!(fs::read(role).unwrap(), role_before);
    drop(exclusion);
    publication.complete(json!({"exit": 0})).unwrap();
}

#[test]
fn several_retirements_count_their_shared_finalized_original_once() {
    if !isolated_capability_test("several_retirements_count_their_shared_finalized_original_once") {
        return;
    }
    let h = host(256, 128);
    let original = ".chrono-harness/state/fixtures/shared-count-original";
    let bytes = b"required original for a real recheck";
    fs::write(h.path().join(original), bytes).unwrap();
    let publication = inventory(h.path())
        .begin("fixture", original, json!(null))
        .unwrap();
    let original_id = publication.id().to_owned();
    publication.complete(json!({"exit": 7})).unwrap();
    inventory(h.path())
        .reference("actual-count-recheck", &[original_id.clone()])
        .unwrap();
    let mut ids = Vec::new();
    let mut paths = Vec::new();
    for n in 0..3 {
        let path = format!(".chrono-harness/state/fixtures/shared-count-report-{n}");
        fs::write(h.path().join(&path), b"obsolete report").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        inventory(h.path())
            .dependencies(&id, &[original_id.clone()])
            .unwrap();
        publication.complete(json!({"exit": 0})).unwrap();
        ids.push(id);
        paths.push(path);
    }
    let role = retention_state::role_path(h.path(), "actual-count-recheck");
    let role_before = fs::read(&role).unwrap();
    let mut rounds = Vec::new();
    let mut removed = 0;
    for _ in 0..4 {
        // Deleted bytes can still have a disposing declaration whose
        // references need another bounded round before retirement completes.
        let observed: Vec<_> = ids
            .iter()
            .map(|id| (id.clone(), output_retired(h.path(), id)))
            .collect();
        let pending = chrono_harness::verified_release::pending_retirements(&observed);
        if pending.is_empty() {
            break;
        }
        let round = inventory(h.path()).maintain_originals(&pending).unwrap();
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        let remaining_reports = paths
            .iter()
            .filter(|path| h.path().join(path).exists())
            .count();
        // Even an incomplete round must not count the shared original once
        // for every target it visited. Unvisited reports may remain unknown.
        assert!(
            round["known_retained_file_logical_bytes"].as_u64().unwrap()
                <= (bytes.len() + remaining_reports * b"obsolete report".len()) as u64,
            "{round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
        removed += round["removed_logical_bytes"].as_u64().unwrap();
        rounds.push(round);
    }
    let round = rounds.last().unwrap();
    assert_eq!(round["scope"]["complete"], true, "{round}");
    assert_eq!(round["remaining_outputs"], 1, "{round}");
    assert_eq!(
        round["known_retained_file_logical_bytes"],
        bytes.len(),
        "{round}"
    );
    assert_eq!(round["remaining_capability_retirements"], 0, "{round}");
    assert_eq!(removed, (3 * b"obsolete report".len()) as u64, "{rounds:?}");
    for path in paths {
        assert!(!h.path().join(path).exists(), "{round}");
    }
    assert_eq!(fs::read(h.path().join(original)).unwrap(), bytes);
    assert_eq!(fs::read(role).unwrap(), role_before);
    let record: Value = serde_json::from_slice(
        &fs::read(retention_state::object_path(h.path(), &original_id)).unwrap(),
    )
    .unwrap();
    assert_eq!(record["incoming"], json!([]));
}

#[test]
fn explicit_maintenance_visits_only_the_named_originals() {
    if !isolated_capability_test("explicit_maintenance_visits_only_the_named_originals") {
        return;
    }
    let h = host(256, 16);
    let mut ids = Vec::new();
    for name in ["selected", "unrelated"] {
        let path = tree(h.path(), name);
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        ids.push(publication.id().to_owned());
        publication.complete(json!({"exit":7})).unwrap();
    }
    let unrelated = retention_state::object_path(h.path(), &ids[1]);
    fs::write(&unrelated, b"unrelated damaged metadata").unwrap();
    for originals in [json!([]), json!([ids[0], ids[0]]), json!(["unknown"])] {
        assert_eq!(
            consumer_call(
                h.path(),
                json!({"operation":"maintain","originals":originals})
            )
            .exit_code,
            2
        );
        assert!(
            h.path()
                .join(".chrono-harness/state/fixtures/selected")
                .exists()
        );
    }
    let result = consumer_call(
        h.path(),
        json!({"operation":"maintain","originals":[ids[0]]}),
    );
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert!(
        !h.path()
            .join(".chrono-harness/state/fixtures/selected")
            .exists()
    );
    assert!(
        h.path()
            .join(".chrono-harness/state/fixtures/unrelated")
            .exists()
    );
    assert_eq!(fs::read(unrelated).unwrap(), b"unrelated damaged metadata");
    let reply: Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(reply["scope"]["kind"], "declared-components");
    assert_eq!(reply["scope"]["originals"], json!([ids[0]]));
}
#[test]
fn declared_consumer_cli_protects_existing_original_and_requires_real_release() {
    if !isolated_capability_test(
        "declared_consumer_cli_protects_existing_original_and_requires_real_release",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = tree(h.path(), "required-failure");
    fs::write(h.path().join(&path).join("stderr"), [3; 2048]).unwrap();
    let p = inventory(h.path())
        .begin("fixture", &path, json!({"exit":101}))
        .unwrap();
    let id = p.id().to_owned();
    p.complete(json!({"exit":101,"error":"original failure"}))
        .unwrap();
    let declaration = consumer_input(
        h.path(),
        "required-recheck",
        json!([original_identity(h.path(), &id, &path)]),
    );
    let request = json!({"operation":"protect-consumer","declaration":declaration});
    let protected = consumer_call(h.path(), request.clone());
    assert_eq!(protected.exit_code, 0, "{}", protected.stderr);
    assert_eq!(consumer_call(h.path(), request).exit_code, 0); // interruption/reentry
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    let not_released = consumer_release(h.path(), &declaration, "unresolved");
    assert_eq!(consumer_call(h.path(), not_released).exit_code, 2);
    let reader = inventory(h.path()).consume(&id).unwrap();
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release.clone()).exit_code, 2);
    drop(reader);
    let result = consumer_call(h.path(), release.clone());
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(consumer_call(h.path(), release.clone()).exit_code, 0);
    let round = maintenance(h.path());
    let rounds = maintain_fixture_rounds_to(h.path(), 0);
    assert_eq!(
        round["removed_logical_bytes"].as_u64().unwrap()
            + rounds
                .iter()
                .map(|r| r["removed_logical_bytes"].as_u64().unwrap())
                .sum::<u64>(),
        2048,
        "{round}; {rounds:?}"
    );
    assert!(!h.path().join(&path).exists());
    assert!(
        ledger(h.path())["summaries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["original_outcome"]["exit"] == 101)
    );
    assert_eq!(consumer_call(h.path(), release).exit_code, 0); // durable receipt retry after reclamation
}

#[test]
fn declared_consumer_rejects_partial_binding_missing_and_replaced_originals() {
    if !isolated_capability_test(
        "declared_consumer_rejects_partial_binding_missing_and_replaced_originals",
    ) {
        return;
    }
    let h = host(256, 16);
    let a = tree(h.path(), "first");
    let b = tree(h.path(), "second");
    let p = inventory(h.path())
        .begin("fixture", &a, json!(null))
        .unwrap();
    let aid = p.id().to_owned();
    p.complete(json!({"exit":101})).unwrap();
    let p = inventory(h.path())
        .begin("fixture", &b, json!(null))
        .unwrap();
    let bid = p.id().to_owned();
    p.complete(json!({"exit":0})).unwrap();
    let originals = json!([
        original_identity(h.path(), &aid, &a),
        original_identity(h.path(), &bid, &b)
    ]);
    let declaration = consumer_input(h.path(), "required-recheck", originals.clone());
    let mut bad = originals.clone();
    bad[1]["id"] = json!("not-enrolled");
    let bad_declaration = consumer_input(h.path(), "required-recheck", bad);
    assert_eq!(
        consumer_call(
            h.path(),
            json!({"operation":"protect-consumer","declaration":bad_declaration})
        )
        .exit_code,
        2
    );
    assert!(ledger(h.path())["roots"].as_object().unwrap().is_empty()); // atomic: no partial acquisition
    consumer_input(h.path(), "required-recheck", originals.clone());
    assert_eq!(
        consumer_call(
            h.path(),
            json!({"operation":"protect-consumer","declaration":declaration})
        )
        .exit_code,
        0
    );
    let partial = consumer_input(h.path(), "required-recheck", json!([originals[0]]));
    assert_eq!(
        consumer_call(
            h.path(),
            json!({"operation":"protect-consumer","declaration":partial})
        )
        .exit_code,
        2
    );
    let partial_release = consumer_release(h.path(), &partial, "released");
    assert_eq!(consumer_call(h.path(), partial_release).exit_code, 2);
    consumer_input(h.path(), "required-recheck", originals);
    fs::rename(h.path().join(&b), h.path().join("saved-original")).unwrap();
    fs::create_dir(h.path().join(&b)).unwrap();
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release).exit_code, 2);
    assert_eq!(
        ledger(h.path())["roots"]["consumer:required-recheck"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    inventory(h.path())
        .reference("consumer:required-recheck", &[])
        .unwrap();
    let mut rounds = Vec::new();
    for _ in 0..8 {
        let round = maintenance(h.path());
        let observed = round["protected"].to_string().contains("identity");
        rounds.push(round);
        if observed {
            break;
        }
    }
    assert!(h.path().join(&b).exists()); // even released roots never authorize replacement disposal
    assert!(
        rounds
            .iter()
            .any(|r| r["protected"].to_string().contains("identity")),
        "{rounds:?}"
    );
}

#[test]
fn declared_consumer_survives_killed_real_reader_and_ordinary_reentry() {
    if !isolated_capability_test(
        "declared_consumer_survives_killed_real_reader_and_ordinary_reentry",
    ) {
        return;
    }
    use std::io::{BufRead, BufReader};
    let h = host(256, 16);
    let path = tree(h.path(), "interrupted-repair");
    fs::write(h.path().join(&path).join("stderr"), b"required error").unwrap();
    let p = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    let id = p.id().to_owned();
    p.complete(json!({"exit":101})).unwrap();
    let declaration = consumer_input(
        h.path(),
        "required-recheck",
        json!([original_identity(h.path(), &id, &path)]),
    );
    fs::write(
        h.path().join("consumer-request.json"),
        serde_json::to_vec(&json!({"operation":"protect-consumer","declaration":declaration}))
            .unwrap(),
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
        .args([
            "dispatch",
            "retention",
            "--host-root",
            h.path().to_str().unwrap(),
            "--request",
            "consumer-request.json",
        ])
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
        .args([h.path().to_str().unwrap(), "consumer", &id])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line.trim(), id);
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success()); // all owned children joined, no finish
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release).exit_code, 0);
    maintenance(h.path());
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(!h.path().join(path).exists());
}

#[test]
fn consumer_adoption_keeps_old_inventory_expiry_and_capacity_while_future_entries_reclaim() {
    if !isolated_capability_test(
        "consumer_adoption_keeps_old_inventory_expiry_and_capacity_while_future_entries_reclaim",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(2);
    policy["keep_seconds"] = json!(604800);
    let previous = serde_json::to_vec(&policy).unwrap();
    fs::write(h.path().join(POLICY_PATH), &previous).unwrap();
    let path = tree(h.path(), "old-required");
    let p = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    let id = p.id().to_owned();
    p.complete(json!({"exit":101})).unwrap();
    let absent = tree(h.path(), "old-already-absent");
    inventory(h.path())
        .begin("fixture", &absent, json!(null))
        .unwrap()
        .complete(json!({"exit":0}))
        .unwrap();
    let next = tree(h.path(), "future");
    inventory(h.path())
        .begin("fixture", &next, json!(null))
        .unwrap()
        .complete(json!({"exit":0}))
        .unwrap();
    // Existing outcomes and expiry remain authoritative across explicit policy migration.
    fs::remove_dir(h.path().join(&next)).unwrap();
    maintenance(h.path());
    let original = ledger(h.path());
    let declaration = consumer_input(
        h.path(),
        "required-recheck",
        json!([original_identity(h.path(), &id, &path)]),
    );
    assert_eq!(
        consumer_call(
            h.path(),
            json!({"operation":"protect-consumer","declaration":declaration})
        )
        .exit_code,
        0
    );
    fs::remove_dir(h.path().join(&absent)).unwrap();
    maintenance(h.path()); // honest absent metadata recovery before old expiry
    policy["keep_seconds"] = json!(0); // explicit host transition, never backdate old outputs
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let migration = inventory(h.path()).migrate_policy(&previous, None).unwrap();
    assert_eq!(migration["effects"], "metadata-binding-only");
    assert_eq!(
        ledger(h.path())["outputs"][0]["expires"],
        original["outputs"][0]["expires"]
    );
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    fs::create_dir(h.path().join(&next)).unwrap();
    inventory(h.path())
        .begin("fixture", &next, json!(null))
        .unwrap()
        .complete(json!({"exit":0}))
        .unwrap();
    let round = maintenance(h.path());
    assert_eq!(fixture_remaining(h.path(), &round), 1);
    assert!(h.path().join(&path).exists());
    assert!(!h.path().join(&next).exists());
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release).exit_code, 0);
    assert_eq!(maintain_fixture_to(h.path(), 1), 1); // released old original still has its window
}

#[test]
fn producer_consumer_retains_interrupted_capture_and_releases_only_accounted_success() {
    if !isolated_capability_test(
        "producer_consumer_retains_interrupted_capture_and_releases_only_accounted_success",
    ) {
        return;
    }
    use std::io::{BufRead, BufReader};
    let h = host(256, 16);
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
        .args([h.path().to_str().unwrap(), "fixture-consumer"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut id = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut id)
        .unwrap();
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let path = ".chrono-harness/state/fixtures/interrupted-capture";
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert_eq!(
        ledger(h.path())["outputs"][0]["outcome"]["exit"],
        Value::Null
    );
    let declaration = consumer_input(
        h.path(),
        &format!("fixture-recheck:{}", id.trim()),
        json!([original_identity(h.path(), id.trim(), path)]),
    );
    let release = consumer_release(h.path(), &declaration, "released");
    assert_eq!(consumer_call(h.path(), release).exit_code, 0);
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(!h.path().join(path).exists());
    let next = h
        .path()
        .join(".chrono-harness/state/fixtures/accounted-success");
    let p = retained_artifacts::retention::create_adopted_for_consumer(
        h.path(),
        "fixture",
        &next,
        true,
        json!({"exit":null}),
        None,
        "fixture-recheck",
    )
    .unwrap()
    .unwrap();
    let consumer = format!("fixture-recheck:{}", p.id());
    p.write_file(&next.join("stdout"), b"success").unwrap();
    p.complete_releasing_consumer(json!({"exit":0}), &consumer)
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(!next.exists());
    assert!(ledger(h.path())["roots"].as_object().unwrap().is_empty());
}

#[test]
fn addressed_records_observe_changed_rows_roots_and_dependency_refusals() {
    if !isolated_capability_test(
        "addressed_records_observe_changed_rows_roots_and_dependency_refusals",
    ) {
        return;
    }
    let h = host(256, 16);
    let first = tree(h.path(), "changed");
    let second = tree(h.path(), "unchanged");
    let mut ids = Vec::new();
    for (path, exit) in [(&first, 101), (&second, 7)] {
        let p = inventory(h.path())
            .begin("fixture", path, json!(null))
            .unwrap();
        ids.push(p.id().to_owned());
        p.complete(json!({"exit":exit})).unwrap();
    }
    inventory(h.path())
        .reference("current", &[ids[0].clone()])
        .unwrap();
    inventory(h.path())
        .dependencies(&ids[0], &[ids[1].clone()])
        .unwrap();
    let file = retention_state::object_path(h.path(), &ids[0]);
    let mut changed: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    changed["output"]["outcome"] = json!({"exit":202});
    let valid = serde_json::to_vec(&changed).unwrap();
    fs::write(&file, &valid).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    assert!(h.path().join(&first).exists());
    assert!(h.path().join(&second).exists());
    changed["output"]["references"] = json!(["output-missing"]);
    let invalid = serde_json::to_vec(&changed).unwrap();
    fs::write(&file, &invalid).unwrap();
    let round = maintenance(h.path());
    assert!(!round["accounting_errors"].as_array().unwrap().is_empty());
    assert_eq!(fs::read(&file).unwrap(), invalid);
    assert!(h.path().join(&first).exists());
    assert!(h.path().join(&second).exists());
    fs::write(&file, valid).unwrap();
    inventory(h.path())
        .reference("current", &[ids[1].clone()])
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(!h.path().join(&first).exists());
    assert!(h.path().join(&second).exists());
    inventory(h.path()).reference("current", &[]).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(!h.path().join(&second).exists());
    let state = ledger(h.path());
    for (id, exit) in [(&ids[0], 202), (&ids[1], 7)] {
        assert!(
            state["summaries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["id"] == *id && s["original_outcome"]["exit"] == exit)
        );
    }
}

#[test]
fn addressed_decoder_keeps_duplicate_unknown_and_opaque_json_refusals() {
    if !isolated_capability_test(
        "addressed_decoder_keeps_duplicate_unknown_and_opaque_json_refusals",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = tree(h.path(), "strict-original");
    let p = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .unwrap();
    let id = p.id().to_owned();
    p.complete(json!({"nested":{"exit":101}})).unwrap();
    let file = retention_state::object_path(h.path(), &id);
    let original = fs::read_to_string(&file).unwrap();
    let cases = [
        original.replacen("\"cursor\":", "\"cursor\":0,\"cursor\":", 1),
        original.replacen("\"phase\":", "\"phase\":\"released\",\"phase\":", 1),
        original.replacen("\"exit\":101", "\"exit\":0,\"exit\":101", 1),
        original.replacen(
            "\"summaries\":[]",
            "\"summaries\":[{\"exit\":0,\"exit\":101}]",
            1,
        ),
        original.replacen("\"cursor\":", "\"unknown\":true,\"cursor\":", 1),
    ];
    for raw in cases {
        assert_ne!(raw, original);
        fs::write(&file, &raw).unwrap();
        let round = maintenance(h.path());
        assert!(!round["accounting_errors"].as_array().unwrap().is_empty());
        assert_eq!(fs::read_to_string(&file).unwrap(), raw);
        assert!(h.path().join(&path).exists());
    }
    fs::write(file, original).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    assert!(!h.path().join(path).exists());
}

#[test]
fn settling_a_report_keeps_an_unfinished_failed_dependency_handoff() {
    if !isolated_capability_test("settling_a_report_keeps_an_unfinished_failed_dependency_handoff")
    {
        return;
    }
    use retained_artifacts::retention::{publish_final_original, reference_report, settle_report};
    let h = report_host();
    let failed = ".chrono-harness/state/preparation/unsettled-failure.json";
    let current = ".chrono-harness/state/preparation/settled-report.json";
    let activity = inventory(h.path()).read_scope().unwrap();
    publish_final_original(
        h.path(),
        "check-original",
        failed,
        b"failed dependency",
        json!({"exit_code":9}),
        false,
    )
    .unwrap();
    publish_final_original(
        h.path(),
        "check-original",
        current,
        b"current report",
        json!({"exit_code":0}),
        true,
    )
    .unwrap();
    let _slot = reference_report(h.path(), "check.json", current, &[failed.into()], None).unwrap();
    fs::write(h.path().join("check.json"), b"current report").unwrap();
    settle_report(h.path(), "check.json", current).unwrap();
    let output = ledger(h.path())["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["path"] == failed)
        .unwrap()
        .clone();
    assert!(
        ledger(h.path())["roots"]
            .get(format!(
                "consumer:report-recheck:{}",
                output["id"].as_str().unwrap()
            ))
            .is_some()
    );
    drop(activity);
    inventory(h.path())
        .reference_paths(
            &format!("current-report:{}", chrono_harness::sha256(b"check.json")),
            &[],
        )
        .unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert_eq!(
        fs::read(h.path().join(failed)).unwrap(),
        b"failed dependency"
    );
}

#[test]
fn failed_final_original_rolls_after_diagnostics_and_a_real_exact_consumer() {
    if !isolated_capability_test(
        "failed_final_original_rolls_after_diagnostics_and_a_real_exact_consumer",
    ) {
        return;
    }
    use retained_artifacts::retention::{publish_final_original, reference_report, settle_report};
    let h = report_host();
    let failed = ".chrono-harness/state/preparation/failed-final.json";
    let passed = ".chrono-harness/state/preparation/passed-final.json";
    let failed_bytes = br#"{"request":{"request_id":"failed-original"},"response":{"status":"failed"},"exit_code":1}"#;
    let publish = |path, bytes: &[u8], exit| {
        let _activity = inventory(h.path()).read_scope().unwrap();
        publish_final_original(
            h.path(),
            "check-original",
            path,
            bytes,
            json!({"exit_code":exit}),
            exit == 0,
        )
        .unwrap();
        assert_eq!(
            maintain_fixture_to(h.path(), if exit == 0 { 2 } else { 1 }),
            if exit == 0 { 2 } else { 1 }
        );
        let _slot = reference_report(h.path(), "check.json", path, &[], None).unwrap();
        fs::write(h.path().join("check.json"), bytes).unwrap();
        settle_report(h.path(), "check.json", path).unwrap();
    };
    publish(failed, failed_bytes, 1);
    let failed_record = ledger(h.path())["outputs"][0].clone();
    assert!(failed_record["expires"].as_u64().unwrap() > 0);
    assert!(
        ledger(h.path())["roots"]
            .get(format!(
                "consumer:report-recheck:{}",
                failed_record["id"].as_str().unwrap()
            ))
            .is_none()
    );
    publish(
        passed,
        br#"{"request":{"request_id":"passed-original"},"response":{"status":"passed"}}"#,
        0,
    );
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    let consumer = "actual-failed-original-recheck";
    let declaration = consumer_input(
        h.path(),
        &consumer,
        json!([original_identity(
            h.path(),
            failed_record["id"].as_str().unwrap(),
            failed
        )]),
    );
    let protected = consumer_call(
        h.path(),
        json!({"operation":"protect-consumer", "declaration":declaration}),
    );
    assert_eq!(protected.exit_code, 0, "{}", protected.stderr);
    let mut expired = failed_record.clone();
    expired["expires"] = json!(0);
    retention_state::replace_output(h.path(), &expired);
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    fs::write(
        h.path().join("recheck-expectation.json"),
        serde_json::to_vec(
            &json!({"path":failed,"sha256":chrono_harness::sha256(failed_bytes),"exit_code":1}),
        )
        .unwrap(),
    )
    .unwrap();
    let call = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"))
            .args([
                h.path().to_str().unwrap(),
                "recheck",
                "consumer.json",
                "recheck-expectation.json",
            ])
            .output()
            .unwrap()
    };
    let original_guard = retained_artifacts::retention::read_guard(h.path(), failed).unwrap();
    assert_eq!(call().status.code(), Some(2)); // completed read cannot release another live reader
    assert!(
        ledger(h.path())["roots"]
            .get(format!("consumer:{consumer}"))
            .is_some()
    );
    drop(original_guard);
    let result = call();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["original_count"], 1);
    assert_eq!(result["declaration_sha256"], declaration["sha256"]);
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    assert!(!h.path().join(failed).exists());
    assert_eq!(failed_record["outcome"]["exit_code"], 1);
    assert_eq!(
        ledger(h.path())["summaries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["path"] == failed)
            .unwrap()["original_outcome"]["exit_code"],
        1
    );
}

#[test]
fn external_removal_exclusion_survives_handoff_to_its_native_child() {
    if !isolated_capability_test("external_removal_exclusion_survives_handoff_to_its_native_child")
    {
        return;
    }
    use std::{
        io::{Read, Write},
        os::{fd::AsRawFd, unix::net::UnixStream, unix::process::CommandExt},
    };
    let h = host(256, 16);
    let path = tree(h.path(), "external-removal-effect");
    let guard = retained_artifacts::retention::external_removal_guard(h.path(), &path)
        .unwrap()
        .unwrap();
    let (mut parent, child) = UnixStream::pair().unwrap();
    let parent_fd = parent.as_raw_fd();
    let child_fd = child.as_raw_fd();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"));
    command.arg("exit-success");
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    unsafe {
        command.pre_exec(move || {
            libc::close(parent_fd);
            let mut byte = 1u8;
            if libc::write(child_fd, (&byte as *const u8).cast(), 1) != 1
                || libc::read(child_fd, (&mut byte as *mut u8).cast(), 1) != 1
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    // This is an explicit native handoff, unlike an unrelated fork of a
    // synchronous metadata operation. The prepared command owns the copies.
    drop(guard);
    let (signaled, conflicting, resumed, joined) = std::thread::scope(|scope| {
        let launched = scope.spawn(move || command.output());
        let signaled = parent.read_exact(&mut [0]);
        let conflicting = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .and_then(|publication| publication.complete(json!({"exit": 0})));
        let resumed = parent.write_all(&[1]);
        let joined = launched.join();
        (signaled, conflicting, resumed, joined)
    });
    signaled.unwrap();
    resumed.unwrap();
    assert!(joined.unwrap().unwrap().status.success());
    assert!(
        conflicting.is_err(),
        "an explicitly forwarded removal capability must exclude overlapping publication"
    );
    let publication = inventory(h.path())
        .begin("fixture", &path, json!(null))
        .expect("the joined child releases its removal capability");
    publication.complete(json!({"exit": 0})).unwrap();
}

#[test]
fn fixture_effect_exclusion_holds_exact_final_original_members_until_owner_retirement() {
    if !isolated_capability_test(
        "fixture_effect_exclusion_holds_exact_final_original_members_until_owner_retirement",
    ) {
        return;
    }
    use retained_artifacts::retention::{external_removal_guards, publish_owned_original};
    let h = report_host();
    let path = ".chrono-harness/state/preparation/nested-original";
    publish_owned_original(
        h.path(),
        "check-original",
        path,
        b"exact final bytes",
        json!({"exit_code":0}),
    )
    .unwrap();
    assert!(
        external_removal_guards(
            h.path(),
            &[".chrono-harness/state/preparation".into(), path.into()]
        )
        .is_err()
    );
    let reader = retained_artifacts::retention::read_guard(h.path(), path).unwrap();
    assert_eq!(maintain_fixture_to(h.path(), 1), 1);
    drop(reader);
    assert_eq!(maintain_fixture_to(h.path(), 0), 0);
    let guard = external_removal_guards(
        h.path(),
        &[
            path.into(),
            ".chrono-harness/state/preparation/another".into(),
        ],
    )
    .unwrap();
    assert!(guard.is_some());
    guard.as_ref().unwrap().stable().unwrap();
    assert!(
        !h.path()
            .join(".chrono-harness/state/output-retention-v1/admission.lease")
            .exists()
    );
    let independent = ".chrono-harness/state/preparation/independent-original";
    publish_owned_original(
        h.path(),
        "check-original",
        independent,
        b"independent bytes",
        json!({"exit_code":3}),
    )
    .unwrap();
    assert_eq!(
        fs::read(h.path().join(independent)).unwrap(),
        b"independent bytes"
    );
    assert!(
        publish_owned_original(
            h.path(),
            "check-original",
            path,
            b"conflicting bytes",
            json!({"exit_code":0})
        )
        .is_err()
    );
    assert!(!h.path().join(path).exists());
    guard.as_ref().unwrap().stable().unwrap();
    let mut policy: Value =
        chrono_harness::json(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["keep_seconds"] = json!(1);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    assert!(guard.as_ref().unwrap().stable().is_err());
    drop(guard);
}

#[test]
fn adopted_process_streams_and_launch_keep_original_bytes_and_joined_failure() {
    if !isolated_capability_test(
        "adopted_process_streams_and_launch_keep_original_bytes_and_joined_failure",
    ) {
        return;
    }
    let h = report_host();
    let dir = h.path().join(".chrono-harness/state/preparation");
    fs::create_dir_all(&dir).unwrap();
    let stdout = dir.join("actual-stdout");
    let stderr = dir.join("actual-stderr");
    let launch = dir.join("actual-launch.json");
    let program = Path::new(env!("CARGO_BIN_EXE_chrono-test-retention-child"));
    let process = chrono_harness::run_process_observed_retained(
        h.path(),
        &chrono_harness::CommandSpec {
            program: program.to_str().unwrap().into(),
            args: vec!["emit-original-failure".into()],
            env: Default::default(),
            timeout_seconds: 10,
            output_limit_bytes: 4096,
        },
        b"exact stdin\0",
        &chrono_harness::file_identity(program).unwrap().0,
        &chrono_harness::ProcessEvidence {
            root: h.path(),
            stdout: &stdout,
            stderr: &stderr,
            launch: &launch,
            publication: None,
        },
    )
    .unwrap();
    assert_eq!(process.exit_code, 7);
    assert!(process.failure.is_none());
    assert_eq!(fs::read(&stdout).unwrap(), b"original output\0\xff");
    assert_eq!(fs::read(&stderr).unwrap(), b"original failure\n");
    let report = chrono_harness::scoped_report::retain_process_evidence(
        h.path(),
        &process,
        ".chrono-harness/state/preparation/actual-stdout",
        ".chrono-harness/state/preparation/actual-stderr",
        ".chrono-harness/state/preparation/actual-launch.json",
    )
    .unwrap();
    assert_eq!(
        report["stdin_sha256"],
        chrono_harness::sha256(b"exact stdin\0")
    );
    assert_eq!(maintain_fixture_to(h.path(), 3), 3);
    assert_eq!(
        ledger(h.path())["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["outcome"]["exit_code"] == 7)
            .count(),
        2
    );
    assert_eq!(ledger(h.path())["roots"].as_object().unwrap().len(), 3);
}

#[test]
fn final_report_dependencies_include_exact_nested_acquisition_members() {
    if !isolated_capability_test(
        "final_report_dependencies_include_exact_nested_acquisition_members",
    ) {
        return;
    }
    let h = report_host();
    let path = tree(h.path(), "acquired");
    fs::write(
        h.path().join(&path).join("unit-original.json"),
        b"original unit bytes",
    )
    .unwrap();
    let acquisition = inventory(h.path())
        .begin("fixture", &path, json!({"status":"downloaded-unjudged"}))
        .unwrap();
    let id = acquisition.id().to_owned();
    acquisition
        .complete(json!({"status":"downloaded-unjudged"}))
        .unwrap();
    let acquisition_use = inventory(h.path()).consume(&id).unwrap();
    let activity = inventory(h.path()).read_scope().unwrap();
    let report = chrono_harness::prepared::retain_original(
        h.path(),
        ".chrono-harness/state/preparation/",
        "collection",
        b"actual collector",
    )
    .unwrap();
    let publisher = retained_artifacts::retention::reference_report(
        h.path(),
        "collection.json",
        &report.path,
        &[format!("{path}/unit-original.json")],
        None,
    )
    .unwrap();
    retained_artifacts::retention::settle_report(h.path(), "collection.json", &report.path)
        .unwrap();
    drop(publisher);
    drop(activity);
    drop(acquisition_use);
    let inventory_state = ledger(h.path());
    assert!(
        inventory_state["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["path"] == report.path)
            .unwrap()["references"]
            .as_array()
            .unwrap()
            .contains(&json!(id))
    );
    assert_eq!(maintain_fixture_to(h.path(), 2), 2);
    assert_eq!(
        fs::read(h.path().join(path).join("unit-original.json")).unwrap(),
        b"original unit bytes"
    );
}

#[test]
fn sealed_tree_member_qualification_requires_exact_producer_identity_and_bytes() {
    if !isolated_capability_test(
        "sealed_tree_member_qualification_requires_exact_producer_identity_and_bytes",
    ) {
        return;
    }
    let h = host(256, 16);
    let path = ".chrono-harness/state/fixtures/qualified-tree";
    let member = format!("{path}/result.json");
    let publication = retained_artifacts::retention::create_adopted(
        h.path(),
        "fixture",
        &h.path().join(path),
        true,
        json!({}),
    )
    .unwrap()
    .unwrap();
    publication
        .write_file(&h.path().join(&member), b"original")
        .unwrap();
    inventory(h.path())
        .reference_paths("actual-consumer", &[path.into()])
        .unwrap();
    let digest = chrono_harness::sha256(b"original");
    assert!(
        !inventory(h.path())
            .published_tree_member("fixture", path, &member, &digest, 8)
            .unwrap(),
        "live intent is not a completed original"
    );
    publication.complete(json!({"exit_code":7})).unwrap();
    let _reader = retained_artifacts::retention::read_guard(h.path(), path)
        .unwrap()
        .unwrap();
    assert!(
        inventory(h.path())
            .published_tree_member("fixture", path, &member, &digest, 8)
            .unwrap()
    );
    assert!(
        !inventory(h.path())
            .published_tree_member("other-producer", path, &member, &digest, 8)
            .unwrap()
    );
    assert!(
        !inventory(h.path())
            .published_tree_member(
                "fixture",
                path,
                &member,
                &chrono_harness::sha256(b"different"),
                8
            )
            .unwrap()
    );
    assert!(
        !inventory(h.path())
            .published_tree_member("fixture", path, &member, &digest, 9)
            .unwrap()
    );
    assert!(
        !inventory(h.path())
            .published_tree_member("fixture", path, &format!("{path}/missing"), &digest, 8)
            .unwrap()
    );
    assert!(
        inventory(h.path())
            .published_tree_member(
                "fixture",
                path,
                ".chrono-harness/state/fixtures/elsewhere",
                &digest,
                8
            )
            .is_err()
    );
    // Same bytes in a different inode do not have the producer's identity.
    fs::rename(
        h.path().join(&member),
        h.path().join(format!("{path}/original-result")),
    )
    .unwrap();
    fs::write(h.path().join(&member), b"original").unwrap();
    assert!(
        inventory(h.path())
            .published_tree_member("fixture", path, &member, &digest, 8)
            .is_err()
    );
}

#[cfg(not(verus_keep_ghost))]
fn observation_publication(
    root: &Path,
    name: &str,
) -> (String, retained_artifacts::retention::Publication) {
    let path = format!(".chrono-harness/state/fixtures/{name}");
    let publication = inventory(root)
        .plan("fixture", &path, true, json!({"state":"intent"}))
        .unwrap();
    publication.materialize().unwrap();
    publication
        .write_file(
            &root.join(&path).join("result.json"),
            b"actual producer result",
        )
        .unwrap();
    (path, publication)
}

#[test]
fn observation_membership_does_not_exclude_another_publishers_dependencies() {
    if !isolated_capability_test(
        "observation_membership_does_not_exclude_another_publishers_dependencies",
    ) {
        return;
    }
    let h = host(512, 16);
    let owner = inventory(h.path());
    let (input, input_publication) = observation_publication(h.path(), "shared-input");
    let input_id = input_publication.id().to_owned();
    input_publication.complete(json!({})).unwrap();
    let (older, older_publication) = observation_publication(h.path(), "pending-command");
    let older_id = older_publication.id().to_owned();
    let (newer, newer_publication) = observation_publication(h.path(), "later-query");
    owner.dependencies_paths(&older, &[input.clone()]).unwrap();
    let first = owner.reserve_observation("pr42", &older, 1).unwrap();
    let input_record = retention_state::object_path(h.path(), &input_id);
    let before_input = fs::read(&input_record).unwrap();
    let before_older = fs::read(retention_state::object_path(h.path(), &older_id)).unwrap();
    // Another activity owns this dependency's metadata. Booking and settling
    // the later query only change the PR's membership, not that dependency.
    let excluded = chrono_harness::ownership::DirectoryLease::acquire(
        input_record.parent().unwrap(),
        true,
        None,
    )
    .unwrap()
    .unwrap();
    let later = owner
        .reserve_observation("pr42", &newer, 1)
        .expect("a query must not wait for another publisher's dependencies");
    assert!(later.serial > first.serial);
    newer_publication.complete(json!({})).unwrap();
    owner
        .settle_observation(&later, Some((&later, true)))
        .expect("sealing the later query does not consume the pending command's inputs");
    assert_eq!(fs::read(&input_record).unwrap(), before_input);
    assert_eq!(
        fs::read(retention_state::object_path(h.path(), &older_id)).unwrap(),
        before_older,
        "the still-pending operation keeps its exact output and dependency edges"
    );
    let retained = ledger(h.path());
    assert_eq!(retained["roots"]["pr42"].as_array().unwrap().len(), 2);
    assert!(h.path().join(&older).exists());
    assert!(h.path().join(&input).exists());
    drop(excluded);
    older_publication.complete(json!({})).unwrap();
    owner
        .settle_observation(&first, Some((&first, true)))
        .unwrap();
    assert_eq!(
        ledger(h.path())["roots"]["pr42"],
        json!([later.original.id])
    );
}

#[test]
fn observation_order_keeps_newer_facts_pending_unknown_and_independent_objects() {
    if !isolated_capability_test(
        "observation_order_keeps_newer_facts_pending_unknown_and_independent_objects",
    ) {
        return;
    }
    let h = host(512, 16);
    let (a, pa) = observation_publication(h.path(), "older");
    let (b, pb) = observation_publication(h.path(), "newer");
    let (other, po) = observation_publication(h.path(), "other-object");
    let owner = inventory(h.path());
    let first = owner.reserve_observation("pr42", &a, 1).unwrap();
    let newer = owner.reserve_observation("pr42", &b, 1).unwrap();
    let independent = owner.reserve_observation("pr43", &other, 1).unwrap();
    let failed_final = owner.reserve_observation("pr42", &a, 3).unwrap();
    assert!(first.serial < newer.serial && newer.serial < failed_final.serial);
    assert_eq!(independent.serial, 1);
    assert_ne!(first.epoch, independent.epoch);
    pb.complete(json!({})).unwrap();
    po.complete(json!({})).unwrap();
    pa.complete(json!({})).unwrap();
    owner
        .settle_observation(&newer, Some((&newer, true)))
        .unwrap();
    owner
        .settle_observation(&independent, Some((&independent, true)))
        .unwrap();
    // Command a returned last and issued the latest failed request. Only its
    // earlier valid fact participates in acceptance, so b still owns this PR.
    owner
        .settle_observation(&failed_final, Some((&first, true)))
        .unwrap();
    let (unknown, pu) = observation_publication(h.path(), "unknown");
    owner.reserve_observation("pr42", &unknown, 1).unwrap();
    drop(pu); // Missing domain completion remains unknown, never manufactured.
    for _ in 0..8 {
        let round = maintenance(h.path());
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
    }
    assert!(!h.path().join(a).exists());
    for path in [&b, &other, &unknown] {
        assert!(h.path().join(path).exists());
    }
    assert!(
        owner
            .reference_paths("pr42", &[])
            .unwrap_err()
            .contains("observation")
    );
    assert_eq!(
        ledger(h.path())["roots"]["pr42"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn observation_handoff_rejects_unsealed_replaced_or_mismatched_bindings() {
    if !isolated_capability_test(
        "observation_handoff_rejects_unsealed_replaced_or_mismatched_bindings",
    ) {
        return;
    }
    let h = host(512, 16);
    let (path, publication) = observation_publication(h.path(), "qualified");
    let owner = inventory(h.path());
    let token = owner.reserve_observation("pr42", &path, 1).unwrap();
    assert!(
        owner
            .settle_observation(&token, Some((&token, true)))
            .unwrap_err()
            .contains("unsealed")
    );
    publication.complete(json!({})).unwrap();
    for fault in ["epoch", "serial", "path", "identity", "observation"] {
        let mut bad = token.clone();
        match fault {
            "epoch" => bad.epoch = "wrong epoch".into(),
            "serial" => bad.serial += 1,
            "path" => bad.original.path.push_str("-other"),
            "identity" => bad.original.identity.push_str("-other"),
            "observation" => bad.attempt += 1,
            _ => unreachable!(),
        }
        let rejected = if fault == "observation" {
            owner.settle_observation(&token, Some((&bad, true)))
        } else {
            owner.settle_observation(&bad, Some((&bad, true)))
        };
        assert!(rejected.is_err(), "{fault}");
    }
    let actual = h.path().join(&path);
    let saved = actual.with_extension("saved-original");
    fs::rename(&actual, &saved).unwrap();
    fs::create_dir(&actual).unwrap();
    assert!(owner.settle_observation(&token, None).is_err());
    fs::remove_dir(&actual).unwrap();
    fs::rename(saved, actual).unwrap();
    owner
        .settle_observation(&token, Some((&token, true)))
        .unwrap();
    owner
        .settle_observation(&token, Some((&token, true)))
        .unwrap();
    assert_eq!(
        ledger(h.path())["roots"]["pr42"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn observation_metadata_survives_pending_work_then_expires_after_terminal_settlement() {
    if !isolated_capability_test(
        "observation_metadata_survives_pending_work_then_expires_after_terminal_settlement",
    ) {
        return;
    }
    let h = host(512, 16);
    let policy_path = h.path().join(POLICY_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["metadata_keep_seconds"] = json!(0);
    fs::write(&policy_path, policy.to_string()).unwrap();
    let (path, publication) = observation_publication(h.path(), "pending");
    let owner = inventory(h.path());
    let first = owner.reserve_observation("pr42", &path, 1).unwrap();
    for _ in 0..8 {
        let round = maintenance(h.path());
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
    }
    let final_request = owner.reserve_observation("pr42", &path, 2).unwrap();
    assert_eq!(first.epoch, final_request.epoch);
    assert_eq!(final_request.serial, first.serial + 1);
    publication.complete(json!({})).unwrap();
    owner
        .settle_observation(&final_request, Some((&final_request, false)))
        .unwrap();
    owner
        .settle_observation(&final_request, Some((&final_request, false)))
        .unwrap();
    let old_reader = retained_artifacts::retention::read_guard(h.path(), &path)
        .unwrap()
        .unwrap();
    for _ in 0..8 {
        let round = maintenance(h.path());
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
    }
    let role_path = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/roles")
        .join(chrono_harness::sha256(b"pr42"));
    assert!(
        h.path().join(&path).exists(),
        "actual reader keeps the old sealed output"
    );
    assert!(!role_path.exists());
    let (next, publication) = observation_publication(h.path(), "next-epoch");
    let newer = owner.reserve_observation("pr42", &next, 1).unwrap();
    assert_ne!(newer.epoch, first.epoch);
    assert_eq!(newer.serial, 1);
    publication.complete(json!({})).unwrap();
    owner.settle_observation(&final_request, Some((&final_request, false))).expect("already completed handoff is a no-op after the object's empty metadata expires and a new epoch begins");
    assert_eq!(
        ledger(h.path())["roots"]["pr42"],
        json!([newer.original.id])
    );
    owner.settle_observation(&newer, None).unwrap();
    drop(old_reader);
    for _ in 0..8 {
        let round = maintenance(h.path());
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
    }
    assert!(!h.path().join(path).exists());
}

#[test]
fn observation_reservation_cannot_claim_an_ordinary_nonempty_consumer() {
    if !isolated_capability_test(
        "observation_reservation_cannot_claim_an_ordinary_nonempty_consumer",
    ) {
        return;
    }
    let h = host(512, 16);
    let (path, publication) = observation_publication(h.path(), "ordinary");
    let owner = inventory(h.path());
    owner
        .reference_paths("existing-consumer", &[path.clone()])
        .unwrap();
    assert!(
        owner
            .reserve_observation("existing-consumer", &path, 1)
            .unwrap_err()
            .contains("ordinary")
    );
    publication.complete(json!({})).unwrap();
    for _ in 0..8 {
        let round = maintenance(h.path());
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
    }
    assert!(h.path().join(path).exists());
}
