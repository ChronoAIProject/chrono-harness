#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_release as policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn require_frontier_expectation(actual: bool)
    requires actual,
{ if !actual { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn current_root_keeps_its_direct_original_boundary() {
    let actual = policy::original_expansion(true, true, None, "parent", 0, 1);
    match actual {
        policy::FinalizedExpansion::RootFrontier => {},
        _ => require_frontier_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn discovered_rootless_parent_can_retire_without_expanding_all_originals() {
    let actual = policy::original_expansion(false, true, Some("leaf"), "parent", 0, 0);
    match actual {
        policy::FinalizedExpansion::RetireFrontier => {},
        _ => require_frontier_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn chosen_parent_preserves_every_other_observed_original() {
    proof {
        reveal_strlit("parent"); reveal_strlit("leaf");
        assert("parent"@[0] != "leaf"@[0]);
    }
    require_frontier_expectation(!policy::preserve_frontier_original("parent", "parent"));
    require_frontier_expectation(policy::preserve_frontier_original("leaf", "parent"));
}


}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path};
#[cfg(not(verus_keep_ghost))]
#[path = "isolated_capability.rs"]
mod isolated_capability;
#[cfg(not(verus_keep_ghost))]
use isolated_capability::isolated_capability_test;
#[cfg(not(verus_keep_ghost))]
#[path = "retention_state.rs"]
mod retention_state;

#[cfg(not(verus_keep_ghost))]
fn host(nodes: usize, summaries: usize) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join(".chrono-harness/state/fixtures")).unwrap();
    fs::write(d.path().join(POLICY_PATH), serde_json::to_vec(&json!({"schema":"chrono-output-retention/v2","producers":{"fixture":[".chrono-harness/state/fixtures/"]},"keep_seconds":0,"max_related_outputs":512,"max_summaries":summaries,"max_nodes_per_round":nodes,"max_bytes_per_round":1048576,"max_millis_per_round":25})).unwrap()).unwrap();
    d
}
#[cfg(not(verus_keep_ghost))]
fn inventory(root: &Path) -> Inventory {
    Inventory::adopted(root).unwrap().unwrap()
}
#[cfg(not(verus_keep_ghost))]
fn tree(root: &Path, name: &str) -> String {
    let path = format!(".chrono-harness/state/fixtures/{name}");
    fs::create_dir(root.join(&path)).unwrap();
    path
}

#[test]
#[cfg(all(unix, not(verus_keep_ghost)))]
fn leaf_entry_preserves_rooted_wide_intermediate_with_256_descriptors() {
    if !isolated_capability_test(
        "retention_frontier::leaf_entry_preserves_rooted_wide_intermediate_with_256_descriptors",
    ) {
        return;
    }
    let h = host(256, 16);
    let mut originals = Vec::new();
    for n in 0..135 {
        let path = format!(".chrono-harness/state/fixtures/required-{n}");
        fs::write(h.path().join(&path), b"required original").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        publication.complete_sealed(json!({"exit": 7})).unwrap();
        originals.push((id, path));
    }
    let publish_report = |name: &str, dependencies: &[String]| {
        let path = format!(".chrono-harness/state/fixtures/{name}");
        fs::write(h.path().join(&path), b"required report").unwrap();
        let publication = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = publication.id().to_owned();
        inventory(h.path()).dependencies(&id, dependencies).unwrap();
        publication.complete_sealed(json!({"exit": 0})).unwrap();
        (id, path)
    };
    let ids: Vec<_> = originals.iter().map(|(id, _)| id.clone()).collect();
    let intermediate = publish_report("intermediate-report", &ids);
    let root = publish_report("current-report", &[intermediate.0.clone()]);
    inventory(h.path())
        .reference("current-report", &[root.0.clone()])
        .unwrap();
    let declarations: Vec<_> = originals
        .iter()
        .chain([&intermediate, &root])
        .map(|(id, path)| {
            let record = retention_state::object_path(h.path(), id);
            (
                record.clone(),
                fs::read(&record).unwrap(),
                path.clone(),
                fs::read(h.path().join(path)).unwrap(),
            )
        })
        .collect();
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
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let round = inventory(h.path())
            .maintain_originals(&[originals[0].0.clone()])
            .unwrap();
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert_eq!(round["removed_logical_bytes"], 0, "{round}");
        assert!(round["disposed"].as_array().unwrap().is_empty(), "{round}");
        for (record, before, path, original) in &declarations {
            let before: Value = serde_json::from_slice(before).unwrap();
            let mut after: Value = serde_json::from_slice(&fs::read(record).unwrap()).unwrap();
            // The ordinary maintenance cursor advances even when every output
            // stays rooted. All other declaration fields and original bytes
            // must remain exact, including every explicit dependency edge.
            after["cursor"] = before["cursor"].clone();
            assert_eq!(after, before);
            assert_eq!(&fs::read(h.path().join(path)).unwrap(), original);
        }
        inventory(h.path())
            .reference("current-report", &[])
            .unwrap();
        for (id, path) in [&root, &intermediate] {
            for _ in 0..135 + 16 {
                let record: Value = serde_json::from_slice(
                    &fs::read(retention_state::object_path(h.path(), id)).unwrap(),
                )
                .unwrap();
                if record["output"].is_null() {
                    break;
                }
                let round = inventory(h.path())
                    .maintain_originals(&[id.clone()])
                    .unwrap();
                assert!(
                    round["accounting_errors"].as_array().unwrap().is_empty(),
                    "{round}"
                );
                assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
                for (_, original_path) in &originals {
                    assert_eq!(
                        fs::read(h.path().join(original_path)).unwrap(),
                        b"required original"
                    );
                }
            }
            assert!(!h.path().join(path).exists(), "report not retired: {id}");
        }
    }));
    if let Err(failure) = verification {
        eprintln!("rooted wide intermediate fixture: {}", h.keep().display());
        std::panic::resume_unwind(failure);
    }
}

#[test]
#[cfg(all(unix, not(verus_keep_ghost)))]
fn publishing_recovery_root_preserves_326_originals_with_256_descriptors() {
    if !isolated_capability_test(
        "retention_frontier::publishing_recovery_root_preserves_326_originals_with_256_descriptors",
    ) {
        return;
    }
    let h = host(256, 16);
    let parent_path = tree(h.path(), "interrupted-parent");
    fs::write(
        h.path().join(&parent_path).join("failure"),
        b"parent failure",
    )
    .unwrap();
    let parent = inventory(h.path())
        .begin("fixture", &parent_path, json!(null))
        .unwrap();
    let parent_id = parent.id().to_owned();
    let mut children = Vec::new();
    for n in 0..326 {
        let path = format!(".chrono-harness/state/fixtures/original-{n}");
        fs::write(h.path().join(&path), b"required original").unwrap();
        let child = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = child.id().to_owned();
        child.complete_sealed(json!({"exit": 7})).unwrap();
        children.push((id, path));
    }
    let paths: Vec<_> = children.iter().map(|(_, path)| path.clone()).collect();
    inventory(h.path())
        .dependencies_paths(&parent_path, &paths)
        .unwrap();
    inventory(h.path())
        .reference("recovery:interrupted-parent", &[parent_id.clone()])
        .unwrap();
    drop(parent);
    let parent_record = retention_state::object_path(h.path(), &parent_id);
    let before = fs::read(&parent_record).unwrap();
    let declared: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(declared["output"]["phase"], "publishing");
    assert_eq!(
        declared["output"]["references"].as_array().unwrap().len(),
        326
    );
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
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let leaf = inventory(h.path())
            .maintain_originals(&[children[0].0.clone()])
            .unwrap();
        assert!(
            leaf["accounting_errors"].as_array().unwrap().is_empty(),
            "{leaf}"
        );
        assert_eq!(leaf["removed_logical_bytes"], 0, "{leaf}");
        assert_eq!(fs::read(&parent_record).unwrap(), before);
        // Selecting the unfinished producer still accounts its own bytes, while
        // the actual recovery root keeps publication and every dependency.
        for _ in 0..8 {
            let own = inventory(h.path())
                .maintain_originals(&[parent_id.clone()])
                .unwrap();
            assert!(
                own["accounting_errors"].as_array().unwrap().is_empty(),
                "{own}"
            );
            let state: Value = serde_json::from_slice(&fs::read(&parent_record).unwrap()).unwrap();
            assert_eq!(state["output"]["phase"], "sealing");
            assert_eq!(state["output"]["outcome"]["completion"], "not-established");
            if state["output"]["manifest"]["sealed"] == true {
                break;
            }
        }
        let accounted: Value = serde_json::from_slice(&fs::read(&parent_record).unwrap()).unwrap();
        assert_eq!(accounted["output"]["phase"], "sealing");
        assert_eq!(accounted["output"]["manifest"]["sealed"], true);
        assert_eq!(
            accounted["output"]["outcome"]["completion"],
            "not-established"
        );
        assert_eq!(
            accounted["output"]["references"],
            declared["output"]["references"]
        );
        assert_eq!(
            fs::read(h.path().join(&parent_path).join("failure")).unwrap(),
            b"parent failure"
        );
        for (_, path) in &children {
            assert_eq!(fs::read(h.path().join(path)).unwrap(), b"required original");
        }
        // A missing declared original must remain an error, even though the
        // protected publisher's complete width no longer consumes descriptors.
        fs::remove_file(retention_state::object_path(h.path(), &children[325].0)).unwrap();
        let missing = inventory(h.path())
            .maintain_originals(&[children[0].0.clone()])
            .unwrap();
        assert!(
            !missing["accounting_errors"].as_array().unwrap().is_empty(),
            "{missing}"
        );
        for (_, path) in &children {
            assert_eq!(fs::read(h.path().join(path)).unwrap(), b"required original");
        }
    }));
    if let Err(failure) = verification {
        eprintln!(
            "retained publishing-recovery fixture: {}",
            h.keep().display()
        );
        std::panic::resume_unwind(failure);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn empty_report_finishes_available_path_and_resumes_after_busy_path() {
    if !isolated_capability_test(
        "retention_frontier::empty_report_finishes_available_path_and_resumes_after_busy_path",
    ) {
        return;
    }
    for busy in [false, true] {
        let h = host(256, 16);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let original_path = ".chrono-harness/state/fixtures/required-original";
            fs::write(h.path().join(original_path), b"required failure").unwrap();
            let original = inventory(h.path())
                .begin("fixture", original_path, json!(null))
                .unwrap();
            let original_id = original.id().to_owned();
            original.complete_sealed(json!({"exit": 7})).unwrap();
            inventory(h.path())
                .reference("actual-recheck", &[original_id.clone()])
                .unwrap();
            let report_path = ".chrono-harness/state/fixtures/empty-report";
            fs::write(h.path().join(report_path), b"obsolete report").unwrap();
            let report = inventory(h.path())
                .begin("fixture", report_path, json!(null))
                .unwrap();
            let report_id = report.id().to_owned();
            inventory(h.path())
                .dependencies(&report_id, &[original_id.clone()])
                .unwrap();
            report.complete_sealed(json!({"exit": 17})).unwrap();
            // Fixed interrupted input: bytes were removed, but its original
            // dependency is still attached to the durable disposing report.
            let record_path = retention_state::object_path(h.path(), &report_id);
            let mut record: Value =
                serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
            record["output"]["phase"] = json!("disposing");
            record["output"]["manifest"]["entries"] = json!([]);
            record["output"]["removed_logical_bytes"] = json!(15);
            record["output"]["reserved_bytes"] = json!(0);
            fs::remove_file(h.path().join(report_path)).unwrap();
            fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
            let mut address = h
                .path()
                .join(".chrono-harness/state/output-retention-v1/paths");
            for component in report_path.split('/') {
                address.push(chrono_harness::sha256(component.as_bytes()));
            }
            let before = fs::read(address.join("address.json")).unwrap();
            let lease = busy.then(|| {
                chrono_harness::ownership::DirectoryLease::acquire(&address, true, None)
                    .unwrap()
                    .unwrap()
            });
            let mut detached = false;
            for _ in 0..4 {
                let round = inventory(h.path())
                    .maintain_originals(&[report_id.clone()])
                    .unwrap();
                assert!(
                    round["accounting_errors"].as_array().unwrap().is_empty(),
                    "{round}"
                );
                assert_eq!(round["metadata_inventory_entries"], 2, "{round}");
                let state: Value =
                    serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
                if state["output"].is_null() {
                    assert!(!busy, "busy path must retain final retirement");
                    detached = true;
                    break;
                }
                if state["output"]["references"].as_array().unwrap().is_empty() {
                    assert!(
                        busy,
                        "available path must finish within the observed component"
                    );
                    assert_eq!(fs::read(address.join("address.json")).unwrap(), before);
                    detached = true;
                    break;
                }
            }
            assert!(detached);
            drop(lease);
            if busy {
                for _ in 0..4 {
                    let round = inventory(h.path())
                        .maintain_originals(&[report_id.clone()])
                        .unwrap();
                    assert!(
                        round["accounting_errors"].as_array().unwrap().is_empty(),
                        "{round}"
                    );
                    let state: Value =
                        serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
                    if state["output"].is_null() {
                        break;
                    }
                }
            }
            let state: Value = serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
            assert!(state["output"].is_null());
            assert_eq!(state["summaries"][0]["original_outcome"]["exit"], 17);
            assert_eq!(
                fs::read(h.path().join(original_path)).unwrap(),
                b"required failure"
            );
            let original: Value = serde_json::from_slice(
                &fs::read(retention_state::object_path(h.path(), &original_id)).unwrap(),
            )
            .unwrap();
            assert_eq!(original["incoming"], json!([]));
            assert_eq!(original["output"]["outcome"]["exit"], 7);
        }));
        if let Err(failure) = result {
            eprintln!("retained empty-report fixture: {}", h.keep().display());
            std::panic::resume_unwind(failure);
        }
    }
}

#[test]
#[cfg(all(unix, not(verus_keep_ghost)))]
fn leaf_entry_retires_wide_parent_with_256_descriptors() {
    if isolated_capability_test(
        "retention_frontier::leaf_entry_retires_wide_parent_with_256_descriptors",
    ) {
        leaf_entry_retires_wide_parent(1, false);
    }
}

#[test]
#[cfg(all(unix, not(verus_keep_ghost)))]
fn two_hop_leaf_entry_retires_wide_parent_with_256_descriptors() {
    if isolated_capability_test(
        "retention_frontier::two_hop_leaf_entry_retires_wide_parent_with_256_descriptors",
    ) {
        leaf_entry_retires_wide_parent(2, false);
    }
}

#[test]
#[cfg(all(unix, not(verus_keep_ghost)))]
fn sealed_parent_retires_from_leaf_with_256_descriptors() {
    if isolated_capability_test(
        "retention_frontier::sealed_parent_retires_from_leaf_with_256_descriptors",
    ) {
        leaf_entry_retires_wide_parent(1, true);
    }
}

#[test]
#[cfg(all(unix, not(verus_keep_ghost)))]
fn sealed_parent_retires_from_two_hop_leaf_with_256_descriptors() {
    if isolated_capability_test(
        "retention_frontier::sealed_parent_retires_from_two_hop_leaf_with_256_descriptors",
    ) {
        leaf_entry_retires_wide_parent(2, true);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn leaf_entry_preserves_live_unknown_and_replaced_parent_originals() {
    if !isolated_capability_test(
        "retention_frontier::leaf_entry_preserves_live_unknown_and_replaced_parent_originals",
    ) {
        return;
    }
    let h = host(256, 16);
    let parent_path = ".chrono-harness/state/fixtures/protected-parent";
    let leaf_path = ".chrono-harness/state/fixtures/protected-leaf";
    fs::write(h.path().join(parent_path), b"parent failure").unwrap();
    fs::write(h.path().join(leaf_path), b"required original").unwrap();
    let parent = inventory(h.path())
        .begin("fixture", parent_path, json!(null))
        .unwrap();
    let parent_id = parent.id().to_owned();
    let leaf = inventory(h.path())
        .begin("fixture", leaf_path, json!(null))
        .unwrap();
    let leaf_id = leaf.id().to_owned();
    leaf.complete(json!({"exit": 7})).unwrap();
    inventory(h.path())
        .dependencies(&parent_id, &[leaf_id.clone()])
        .unwrap();
    parent.complete_sealed(json!({"exit": 17})).unwrap();
    let before: Value = serde_json::from_slice(
        &fs::read(retention_state::object_path(h.path(), &parent_id)).unwrap(),
    )
    .unwrap();
    let lease = h
        .path()
        .join(".chrono-harness/state/output-retention-v1")
        .join(before["output"]["lease"].as_str().unwrap());
    let missing = lease.with_extension("preserved");
    let reader = inventory(h.path()).consume(&parent_id).unwrap();
    let assert_preserved = |reason: &str| {
        let round = inventory(h.path())
            .maintain_originals(&[leaf_id.clone()])
            .unwrap();
        assert_eq!(round["removed_logical_bytes"], 0, "{round}");
        assert!(round["disposed"].as_array().unwrap().is_empty(), "{round}");
        assert!(
            round["protected"].as_array().unwrap().iter().any(|item| {
                item["id"] == parent_id && item["reason"].as_str().unwrap().contains(reason)
            }),
            "{round}"
        );
        assert_eq!(
            fs::read(h.path().join(leaf_path)).unwrap(),
            b"required original"
        );
    };
    assert_preserved("live producer/consumer descendants");
    assert_eq!(
        fs::read(h.path().join(parent_path)).unwrap(),
        b"parent failure"
    );
    drop(reader);
    fs::rename(&lease, &missing).unwrap();
    assert_preserved("(os error 2)");
    assert_eq!(
        fs::read(h.path().join(parent_path)).unwrap(),
        b"parent failure"
    );
    fs::rename(&missing, &lease).unwrap();
    let original = h.path().join("original-parent");
    fs::rename(h.path().join(parent_path), &original).unwrap();
    fs::write(h.path().join(parent_path), b"replacement").unwrap();
    assert_preserved("identity changed");
    assert_eq!(
        fs::read(h.path().join(parent_path)).unwrap(),
        b"replacement"
    );
    assert_eq!(fs::read(original).unwrap(), b"parent failure");
}

#[cfg(all(unix, not(verus_keep_ghost)))]
fn leaf_entry_retires_wide_parent(hops: usize, sealed: bool) {
    let h = host(2, 16);
    let mut policy: Value =
        serde_json::from_slice(&fs::read(h.path().join(POLICY_PATH)).unwrap()).unwrap();
    policy["max_related_outputs"] = json!(512);
    policy["max_millis_per_round"] = json!(25);
    fs::write(
        h.path().join(POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let parent_path = tree(h.path(), "wide-parent");
    let parent = inventory(h.path())
        .begin("fixture", &parent_path, json!(null))
        .unwrap();
    let parent_id = parent.id().to_owned();
    fs::write(
        h.path().join(&parent_path).join("failure"),
        b"parent failure",
    )
    .unwrap();
    let mut children = Vec::new();
    for n in 0..137 {
        let path = format!(".chrono-harness/state/fixtures/child-{n}");
        fs::write(h.path().join(&path), b"required original").unwrap();
        let child = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = child.id().to_owned();
        child.complete(json!({"exit": 7})).unwrap();
        children.push((id, path));
    }
    let child_ids: Vec<_> = children.iter().map(|(id, _)| id.clone()).collect();
    inventory(h.path())
        .dependencies_paths(
            &parent_path,
            &children
                .iter()
                .map(|(_, path)| path.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let (leaf, leaf_path) = if hops == 1 {
        children[0].clone()
    } else {
        let path = tree(h.path(), "leaf");
        let leaf = inventory(h.path())
            .begin("fixture", &path, json!(null))
            .unwrap();
        let id = leaf.id().to_owned();
        if sealed {
            leaf.complete_sealed(json!({"exit": 9})).unwrap();
        } else {
            leaf.complete(json!({"exit": 9})).unwrap();
        }
        inventory(h.path())
            .dependencies(&children[0].0, &[id.clone()])
            .unwrap();
        (id, path)
    };
    if sealed {
        parent.complete_sealed(json!({"exit": 17})).unwrap();
        let finalized: Value = serde_json::from_slice(
            &fs::read(retention_state::object_path(h.path(), &parent_id)).unwrap(),
        )
        .unwrap();
        assert_eq!(finalized["output"]["phase"], "released");
        assert_eq!(finalized["output"]["manifest"]["sealed"], true);
    } else {
        // Preserve the original failing publication method and its bounded work.
        parent.complete(json!({"exit": 17})).unwrap();
    }
    // The entry original is protected throughout this retirement. Its busy
    // physical path must not prevent the obsolete parent's bounded progress.
    let mut address = h
        .path()
        .join(".chrono-harness/state/output-retention-v1/paths");
    for component in leaf_path.split('/') {
        address.push(chrono_harness::sha256(component.as_bytes()));
    }
    let address_before = fs::read(address.join("address.json")).unwrap();
    let leaf_exclusion = chrono_harness::ownership::DirectoryLease::acquire(&address, true, None)
        .unwrap()
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
    let mut rounds = Vec::new();
    let verification = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let first = inventory(h.path())
            .maintain_originals(&[leaf.clone()])
            .unwrap();
        rounds.push(first);
        let first = rounds.last().unwrap();
        assert!(
            first["accounting_errors"].as_array().unwrap().is_empty(),
            "{first}"
        );
        assert_eq!(first["nodes"], 2, "{first}");
        assert_eq!(first["removed_logical_bytes"], 14, "{first}");
        assert_eq!(first["scope"]["complete"], false, "{first}");
        assert_eq!(first["remaining_outputs"], Value::Null, "{first}");
        let record = || -> Value {
            serde_json::from_slice(
                &fs::read(retention_state::object_path(h.path(), &parent_id)).unwrap(),
            )
            .unwrap()
        };
        assert_eq!(record()["output"]["phase"], "disposing");
        assert_eq!(record()["output"]["references"], json!(child_ids));
        for (_, path) in &children {
            assert_eq!(fs::read(h.path().join(path)).unwrap(), b"required original");
        }
        // Each call reopens the owner. Continue through the leaf while the
        // parent still references it, then resume by the parent's exact ID.
        for _ in 0..137 + 4 {
            let before = record();
            if before["output"].is_null() {
                break;
            }
            let entry = if before["output"]["references"]
                .as_array()
                .unwrap()
                .contains(&json!(children[0].0))
            {
                leaf.clone()
            } else {
                parent_id.clone()
            };
            let round = inventory(h.path()).maintain_originals(&[entry]).unwrap();
            assert!(
                round["accounting_errors"].as_array().unwrap().is_empty(),
                "{round}"
            );
            assert!(round["nodes"].as_u64().unwrap() <= 2, "{round}");
            rounds.push(round);
            for (_, path) in &children {
                assert_eq!(fs::read(h.path().join(path)).unwrap(), b"required original");
            }
        }
        assert!(record()["output"].is_null(), "{:?}", rounds.last());
        assert!(!h.path().join(&parent_path).exists());
        assert_eq!(record()["summaries"][0]["original_outcome"]["exit"], 17);
        assert_eq!(
            fs::read(address.join("address.json")).unwrap(),
            address_before
        );
    }));
    drop(leaf_exclusion);
    if let Err(failure) = verification {
        let path = h.keep();
        eprintln!(
            "leaf-entry rounds: {rounds:?}; retained fixture: {}",
            path.display()
        );
        std::panic::resume_unwind(failure);
    }
}
