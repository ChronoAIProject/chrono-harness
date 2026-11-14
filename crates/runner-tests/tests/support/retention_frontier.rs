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
    let actual = policy::finalized_expansion(true, None, "parent", 0, 1);
    match actual {
        policy::FinalizedExpansion::RootFrontier => {},
        _ => require_frontier_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn discovered_rootless_parent_can_retire_without_expanding_all_originals() {
    let actual = policy::finalized_expansion(false, Some("leaf"), "parent", 0, 0);
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
