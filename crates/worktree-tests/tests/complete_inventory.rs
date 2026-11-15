#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy;
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
fn repeated_actions_keep_every_original_index_in_order() {
    let indices = policy::append_action_index(Vec::new(), true, 0);
    let indices = policy::append_action_index(indices, false, 1);
    let indices = policy::append_action_index(indices, true, 2);
    if indices.len() != 2 || indices[0] != 0 || indices[1] != 2 { panic!(); }
}
}
// Original inventory acceptance remains alongside the independent pointer expectation.
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeSet;

#[cfg(not(verus_keep_ghost))]
fn configure_inventory(h: &EnrollmentOriginalsFixture, paths: &[PathBuf], error: Option<&str>) {
    let path = h.root.join("git-fixture.json");
    let mut config = json(&fs::read(&path).unwrap()).unwrap();
    config["inventory_paths"] = value!(paths);
    config["inventory_error"] = value!(error);
    fs::write(path, serde_json::to_vec(&config).unwrap()).unwrap();
}

#[cfg(not(verus_keep_ghost))]
fn original(h: &EnrollmentOriginalsFixture, report: &Value) -> Vec<Value> {
    original_at(&h.root, report)
}

#[cfg(not(verus_keep_ghost))]
fn original_at(root: &Path, report: &Value) -> Vec<Value> {
    let descriptor = &report["complete_inventory"];
    let path = descriptor["path"].as_str().unwrap();
    let bytes = fs::read(root.join(path)).unwrap();
    assert_eq!(descriptor["receipt"]["sha256"], sha256(&bytes));
    assert_eq!(descriptor["byte_length"], bytes.len());
    bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| json(line).unwrap())
        .collect()
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn explicit_inventory_keeps_the_whole_union_beyond_the_sample_ceiling() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::imported();
    let paths: Vec<_> = (0..300)
        .map(|i| h.root.join(format!("git-only-{i}")))
        .collect();
    configure_inventory(&h, &paths, None);
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    let rows = original(&h, &report);
    let members: Vec<_> = rows
        .iter()
        .filter(|r| {
            r["kind"] == "worktree" && r["phase"] == "before" && r["state"]["record_kind"].is_null()
        })
        .collect();
    let actual: BTreeSet<_> = members
        .iter()
        .map(|r| PathBuf::from(r["state"]["path"].as_str().unwrap()))
        .collect();
    let expected: BTreeSet<_> = paths
        .into_iter()
        .chain([h.root.clone(), h.target.clone()])
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(members.len(), expected.len());
    assert!(serde_json::to_vec(&rows).unwrap().len() > 65536);
    assert!(
        members.iter().any(
            |r| r["state"]["path"] == value!(h.target) && r["state"]["git_registered"] == false
        )
    );
    assert!(rows.iter().any(|r| r["kind"] == "unknown-declarations"));
    assert_eq!(
        report["complete_inventory"]["member_enumeration_complete"],
        false
    );
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
    let state = retention_state::snapshot(&h.root);
    let parent = state["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == report["report_path"])
        .unwrap();
    let child = state["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == report["complete_inventory"]["path"])
        .unwrap();
    assert!(
        parent["references"]
            .as_array()
            .unwrap()
            .contains(&child["id"])
    );
    let recovery = format!(
        "recovery:{}",
        report["complete_inventory"]["path"].as_str().unwrap()
    );
    assert!(state["roots"].get(recovery).is_none(), "{state}");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn measurement_failure_is_unknown_and_keeps_the_original_failure() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::imported();
    let external = tempfile::tempdir().unwrap();
    fs::write(external.path().join("source"), b"unique work").unwrap();
    let link = h.root.join("unknown-link");
    std::os::unix::fs::symlink(external.path(), &link).unwrap();
    configure_inventory(&h, &[link.clone()], None);
    let (code, report) = h.invoke("maintain", false);
    assert_ne!(code, 0, "{report}");
    let rows = original(&h, &report);
    let row = rows
        .iter()
        .find(|r| {
            r["kind"] == "worktree" && r["phase"] == "before" && r["state"]["path"] == value!(link)
        })
        .unwrap();
    assert_eq!(row["measurement"]["freshness"], "unknown");
    assert_eq!(row["measurement"]["footprint"], Value::Null);
    assert!(
        row["measurement"]["error"]
            .as_str()
            .unwrap()
            .contains("symbolic link")
    );
    assert_eq!(report["complete_inventory"]["measurements_complete"], false);
    assert_eq!(
        fs::read(external.path().join("source")).unwrap(),
        b"unique work"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn adopted_policy_lists_retained_and_unselected_artifacts_with_real_measurements() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    let config = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["artifacts"] = value!([
        {"path":".chrono-harness/state/","disposition":"evidence-retain"},
        {"path":".chrono-harness/bin/","disposition":"retain"}
    ]);
    h.adopt_retained_policy(config, policy);
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    let rows = original(&h, &report);
    let artifacts: Vec<_> = rows
        .iter()
        .filter(|row| row["kind"] == "worktree-artifact" && row["worktree"] == value!(h.target))
        .collect();
    assert_eq!(artifacts.len(), 2);
    let dispositions: BTreeSet<_> = artifacts
        .iter()
        .map(|row| row["disposition"].as_str().unwrap())
        .collect();
    assert_eq!(dispositions, BTreeSet::from(["retain", "evidence-retain"]));
    for artifact in artifacts {
        assert_eq!(artifact["measurement"]["unit"], "bytes");
        assert_eq!(artifact["measurement"]["presence"], "absent");
        assert_eq!(artifact["measurement"]["footprint"]["logical_bytes"], 0);
        assert!(artifact["measurement"]["window_started_unix_millis"].is_u64());
        assert_eq!(artifact["action"], "see worktree-action");
        assert_eq!(artifact["qualification"], "not-evaluated-by-report");
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn inventory_enumeration_failure_is_preserved_and_target_status_stays_local() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::imported();
    configure_inventory(&h, &[], Some("original inventory failure"));
    let (code, report) = h.invoke("maintain", false);
    assert_ne!(code, 0);
    assert_eq!(
        report["complete_inventory"]["member_enumeration_complete"],
        false
    );
    let rows = original(&h, &report);
    assert!(rows.iter().any(|r| r["kind"] == "enumeration-error"));
    assert!(
        rows.iter()
            .any(|r| r["kind"] == "worktree" && r["state"]["path"] == value!(h.target))
    );
    configure_inventory(&h, &[], None);
    let (_, status) = h.invoke("status", true);
    assert!(status["complete_inventory"].is_null());
    assert_eq!(status["worktree_status"]["scope"], "target");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn missing_owner_marker_is_an_unknown_member_and_does_not_hide_known_addresses() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::imported();
    let unknown = h.record.parent().unwrap().join("unknown-address");
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("ledger.json"), b"preserved unknown work").unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_ne!(code, 0);
    let rows = original(&h, &report);
    assert!(rows.iter().any(|r| r["kind"] == "worktree"
        && r["phase"] == "before"
        && r["state"]["path"] == value!(h.target)));
    assert!(rows.iter().any(|r| r["kind"] == "worktree"
        && r["state"]["path"].is_null()
        && r["state"]["record_path"] == value!(unknown)));
    assert_eq!(
        report["complete_inventory"]["member_enumeration_complete"],
        false
    );
    assert_eq!(
        fs::read(unknown.join("ledger.json")).unwrap(),
        b"preserved unknown work"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn inventory_writer_preserves_partial_io_and_rejects_the_existing_byte_limit() {
    struct Failing {
        bytes: Vec<u8>,
        remaining: usize,
    }
    impl std::io::Write for Failing {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.remaining == 0 {
                return Err(std::io::Error::other("original report IO failure"));
            }
            let n = self.remaining.min(bytes.len());
            self.bytes.extend_from_slice(&bytes[..n]);
            self.remaining -= n;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Failing {
        bytes: vec![],
        remaining: 7,
    };
    let (mut bytes, mut rows) = (0, 0);
    let error = chrono_worktree::write_inventory_record(
        &mut output,
        &value!({"member":"preserve partial bytes"}),
        &mut bytes,
        &mut rows,
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(error.contains("original report IO failure"));
    assert_eq!(output.bytes.len(), 7);
    assert_eq!((bytes, rows), (0, 0));
    let mut output = Vec::new();
    let error = chrono_worktree::write_inventory_record(
        &mut output,
        &value!({"member":"not written"}),
        &mut bytes,
        &mut rows,
        |_| Err("existing publication byte limit".into()),
    )
    .unwrap_err();
    assert_eq!(error, "existing publication byte limit");
    assert!(output.is_empty());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn inventory_batches_small_records_without_losing_members_or_changing_the_limit() {
    use std::io::Write;
    #[derive(Clone, Default)]
    struct Counted(std::rc::Rc<std::cell::RefCell<(usize, Vec<u8>)>>);
    impl Write for Counted {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut state = self.0.borrow_mut();
            state.0 += 1;
            state.1.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let counted = Counted::default();
    let mut output = chrono_worktree::inventory_writer(counted.clone());
    let expected: Vec<_> = (0..10_000).map(|i| value!({"member": i})).collect();
    let (mut bytes, mut rows) = (0, 0);
    for row in &expected {
        chrono_worktree::write_inventory_record(
            &mut output,
            row,
            &mut bytes,
            &mut rows,
            |_| Ok(()),
        )
        .unwrap();
    }
    let before_rejection = (bytes, rows);
    assert_eq!(
        chrono_worktree::write_inventory_record(
            &mut output,
            &value!({"member": "over limit"}),
            &mut bytes,
            &mut rows,
            |_| Err("existing publication byte limit".into()),
        )
        .unwrap_err(),
        "existing publication byte limit"
    );
    assert_eq!((bytes, rows), before_rejection);
    output.flush().unwrap();
    let state = counted.0.borrow();
    let actual: Vec<_> = state
        .1
        .split(|b| *b == b'\n')
        .filter(|r| !r.is_empty())
        .map(|r| json(r).unwrap())
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(bytes as usize, state.1.len());
    assert_eq!(rows, 10_000);
    assert!(
        state.0 < 100,
        "each small record still reaches the underlying writer: {} writes",
        state.0
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn ordinary_completed_parent_expands_all_actions_and_original_receipts() {
    let parent = json(include_bytes!("fixtures/maintenance-repeat-actions.json")).unwrap();
    let drain = parent["drain"].as_array().unwrap();
    let paths: BTreeSet<_> = drain
        .iter()
        .map(|row| row["path"].as_str().unwrap())
        .collect();
    let mut expanded_indices = BTreeSet::new();
    for path in paths {
        let projection =
            chrono_worktree::inventory_worktree_action(Path::new(path), &parent["drain"]);
        let pointers = projection["parent_report_pointers"].as_array().unwrap();
        let expanded: Vec<_> = pointers
            .iter()
            .map(|pointer| {
                let pointer = pointer.as_str().unwrap();
                let index: usize = pointer.strip_prefix("/drain/").unwrap().parse().unwrap();
                assert!(expanded_indices.insert(index));
                let row = parent.pointer(pointer).unwrap();
                assert_eq!(row["path"], path);
                (index, row)
            })
            .collect();
        assert!(expanded.windows(2).all(|pair| pair[0].0 < pair[1].0));
        if let Some(expected) = parent["expected_repeated"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == path)
        {
            assert_eq!(expanded.len(), expected["steps"].as_u64().unwrap() as usize);
            assert_eq!(
                value!(
                    expanded
                        .iter()
                        .map(|(_, row)| &row["status"])
                        .collect::<Vec<_>>()
                ),
                expected["statuses"]
            );
            assert_eq!(
                value!(
                    expanded
                        .iter()
                        .map(|(_, row)| &row["receipt"])
                        .collect::<Vec<_>>()
                ),
                expected["receipts"]
            );
        }
    }
    assert_eq!(expanded_indices, (0..drain.len()).collect());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn repeated_retention_records_expand_without_losing_observations_or_measurements() {
    let base = value!({"id":"output-a","path":"state/report.json",
        "metadata_path":"/owner/objects/output-a/record.json",
        "output":{"identity":"device:inode:file","phase":"released",
            "outcome":{"diagnostic":"x".repeat(4096)}},
        "roles":["consumer:live"],"incoming":["output-parent"]});
    let originals: Vec<_> = (0..32)
        .map(|i| {
            let mut record = base.clone();
            match i {
                7 => record["roles"] = value!([]),
                11 => record["incoming"] = value!([]),
                19 => record["output"]["identity"] = value!("device:new-inode:file"),
                _ => {}
            }
            value!({"kind":"declaration-state","source_round":format!("round-{i}"),
            "address":"output-a","record":record,
            "measurement":{"window_started_unix_millis":i,"allocated_bytes":i*512},
            "error":if i == 23 {value!("original unknown result")} else {Value::Null}})
        })
        .collect();
    let mut index = std::collections::BTreeMap::new();
    let mut encoded = vec![value!({"kind":"scope"})];
    for original in &originals {
        let mut row = original.clone();
        chrono_worktree::compact_inventory_retention_record(
            &mut row,
            &mut index,
            encoded.len() as u64 + 1,
        )
        .unwrap();
        encoded.push(row);
    }
    assert_eq!(index.len(), 4);
    assert_eq!(encoded.len(), originals.len() + 1);
    for (offset, expected) in originals.iter().enumerate() {
        let line = offset + 2;
        let mut expanded = encoded[offset + 1].clone();
        if let Some(reference) = expanded["record_ref"].as_u64() {
            assert!(reference > 0 && reference < line as u64);
            let first = &encoded[reference as usize - 1];
            assert!(first["record"].is_object());
            assert!(first["record_ref"].is_null());
            expanded.as_object_mut().unwrap().remove("record_ref");
            expanded["record"] = first["record"].clone();
        }
        assert_eq!(&expanded, expected, "line {line}");
    }
    assert_eq!(encoded[2]["record_ref"], 2);
    for offset in [7, 11, 19] {
        assert!(encoded[offset + 1]["record"].is_object());
    }
    let before = serde_json::to_vec(&originals).unwrap().len();
    let after = serde_json::to_vec(&encoded).unwrap().len();
    assert!(before > 65536);
    assert!(after < before / 2, "before={before} after={after}");
    let mut unrelated = value!({"kind":"round","error":"original failure"});
    let expected = unrelated.clone();
    chrono_worktree::compact_inventory_retention_record(&mut unrelated, &mut index, 100).unwrap();
    assert_eq!(unrelated, expected);
    let mut unknown = value!({"kind":"declaration-state","record":null,"error":"unknown"});
    let expected = unknown.clone();
    chrono_worktree::compact_inventory_retention_record(&mut unknown, &mut index, 101).unwrap();
    assert_eq!(unknown, expected);
}

#[cfg(not(verus_keep_ghost))]
fn linked_retention_host(name: &str) -> (Host, PathBuf) {
    use chrono_harness::retained_artifacts::retention::POLICY_PATH;
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    let path = h.root.join(POLICY_PATH);
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["max_related_outputs"] = value!(512);
    policy["max_summaries"] = value!(128);
    policy["max_bytes_per_round"] = value!(67108864);
    policy["max_millis_per_round"] = value!(25);
    policy["max_output_bytes"] = value!(1073741824);
    policy["max_manifest_nodes"] = value!(16384);
    policy["max_metadata_bytes_per_round"] = value!(67108864);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join(name);
    let (code, report, error) = h.invoke("feature", name, &target);
    assert_eq!(code, 0, "{report} {error}");
    fs::create_dir_all(target.join(".chrono-harness/state/worktrees")).unwrap();
    (h, target)
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn unified_maintenance_reclaims_linked_outputs_and_measures_their_actual_root() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    let (h, target) = linked_retention_host("linked-retention");
    let inventory = Inventory::adopted(&target).unwrap().unwrap();
    let expired = ".chrono-harness/state/worktrees/linked-expired.bin";
    let protected = ".chrono-harness/state/worktrees/linked-protected.bin";
    let live = ".chrono-harness/state/worktrees/linked-live.bin";
    for path in [expired, protected, live] {
        fs::write(target.join(path), b"linked original").unwrap();
    }
    fs::write(h.root.join(protected), b"a different coordinator file").unwrap();
    for path in [expired, protected] {
        Inventory::adopted(&target)
            .unwrap()
            .unwrap()
            .begin("lifecycle-original", path, value!(null))
            .unwrap()
            .complete(value!({"status":"failed","error":"original failure"}))
            .unwrap();
    }
    inventory
        .reference_paths("needed-failure", &[protected.into()])
        .unwrap();
    let live_publication = Inventory::adopted(&target)
        .unwrap()
        .unwrap()
        .begin("lifecycle-original", live, value!(null))
        .unwrap();
    let source = fs::read(target.join("payload")).unwrap();
    let mut observed = Vec::new();
    for attempt in 0..8 {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        let rows = original_at(&h.root, &report);
        let measured = rows
            .iter()
            .find(|row| {
                row["kind"] == "retention"
                    && row["source_root"] == value!(target)
                    && row["observation"]["kind"] == "declaration-state"
                    && row["observation"]["record"]["path"] == protected
            })
            .expect("the coordinator must enumerate the linked retention owner");
        assert_eq!(
            measured["observation"]["measurement"]["footprint"]["logical_bytes"],
            15
        );
        assert_eq!(
            fs::read(target.join(protected)).unwrap(),
            b"linked original"
        );
        assert_eq!(fs::read(target.join(live)).unwrap(), b"linked original");
        assert_eq!(
            fs::read(h.root.join(protected)).unwrap(),
            b"a different coordinator file"
        );
        assert_eq!(fs::read(target.join("payload")).unwrap(), source);
        observed.push(report);
        if !target.join(expired).exists() {
            break;
        }
        assert!(
            attempt < 7,
            "bounded owner maintenance did not reclaim the expired linked output: {observed:?}"
        );
    }
    assert!(!target.join(expired).exists());
    drop(live_publication);
    inventory.reference_paths("needed-failure", &[]).unwrap();
    drop(inventory);
    // A later coordinator round must reopen the same owner and continue after release.
    for attempt in 0..8 {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        if !target.join(protected).exists() {
            break;
        }
        assert!(
            attempt < 7,
            "released linked original was not reclaimed: {report}"
        );
    }
    assert!(!target.join(protected).exists());
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn broken_linked_retention_preserves_unknown_bytes_and_keeps_coordinator_work() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::POLICY_PATH;
    let (h, target) = linked_retention_host("broken-linked-retention");
    fs::write(target.join(POLICY_PATH), b"unknown retention policy\n").unwrap();
    let source = fs::read(target.join("payload")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_ne!(
        code, 0,
        "a linked policy failure must remain visible: {report} {error}"
    );
    let rows = original_at(&h.root, &report);
    assert!(rows.iter().any(|row| row["kind"] == "retention"
        && row["source_root"] == value!(h.root)
        && row["observation"]["kind"] == "declaration"));
    assert!(rows.iter().any(|row| {
        row["kind"] == "enumeration-error"
            && row["scope"]
                .as_str()
                .is_some_and(|scope| scope.contains(target.to_str().unwrap()))
    }));
    assert_eq!(
        fs::read(target.join(POLICY_PATH)).unwrap(),
        b"unknown retention policy\n"
    );
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
    assert_eq!(
        report["complete_inventory"]["member_enumeration_complete"],
        false
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn explicit_maintenance_writes_local_record_references_and_keeps_live_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, create_adopted};
    let h = EnrollmentOriginalsFixture::imported();
    let owner = Inventory::adopted(&h.root).unwrap().unwrap();
    let leaf_path = h.root.join(".chrono-harness/state/required-leaf.json");
    let leaf = create_adopted(&h.root, "lifecycle-original", &leaf_path, false, value!({}))
        .unwrap()
        .unwrap();
    fs::write(&leaf_path, b"required leaf").unwrap();
    let leaf_id = leaf.id().to_owned();
    leaf.complete_sealed(value!({"status":"passed"})).unwrap();
    // Two independent leaves reach the same rooted parent. After their real
    // handoff below, only that parent owns their continued retention.
    let peer_path = h.root.join(".chrono-harness/state/required-peer.json");
    let peer = create_adopted(&h.root, "lifecycle-original", &peer_path, false, value!({}))
        .unwrap()
        .unwrap();
    fs::write(&peer_path, b"required peer").unwrap();
    let peer_id = peer.id().to_owned();
    peer.complete_sealed(value!({"status":"passed"})).unwrap();
    let parent_path = h.root.join(".chrono-harness/state/required-parent.json");
    let parent = create_adopted(
        &h.root,
        "lifecycle-original",
        &parent_path,
        false,
        value!({}),
    )
    .unwrap()
    .unwrap();
    fs::write(&parent_path, b"required parent").unwrap();
    let parent_id = parent.id().to_owned();
    owner
        .dependencies(&parent_id, &[leaf_id.clone(), peer_id.clone()])
        .unwrap();
    parent.complete_sealed(value!({"status":"passed"})).unwrap();
    owner
        .reference("actual-inventory-consumer", &[parent_id.clone()])
        .unwrap();
    // create_adopted keeps a recovery root for each lifecycle original. Release
    // the leaves' temporary roots only after the parent and its consumer have
    // taken custody, just as the actual lifecycle report publisher does.
    for path in [&leaf_path, &peer_path] {
        let relative = path.strip_prefix(&h.root).unwrap().to_str().unwrap();
        owner
            .reference_paths(&format!("recovery:{relative}"), &[])
            .unwrap();
    }
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert_eq!(
        report["complete_inventory"]["schema"],
        "chrono-maintenance-inventory/v2"
    );
    let rows = original(&h, &report);
    let mut references = 0;
    let mut members = BTreeSet::new();
    for (offset, row) in rows.iter().enumerate() {
        let observation = &row["observation"];
        if observation["kind"] != "declaration-state" {
            continue;
        }
        let record = if let Some(line) = observation["record_ref"].as_u64() {
            assert!(line > 0 && line < (offset + 1) as u64);
            let first = &rows[line as usize - 1]["observation"];
            assert!(first["record_ref"].is_null());
            assert!(first["record"].is_object());
            assert!(observation["record"].is_null());
            assert!(observation["measurement"].is_object());
            references += 1;
            &first["record"]
        } else {
            &observation["record"]
        };
        members.insert(record["id"].as_str().unwrap().to_owned());
    }
    assert!(references > 0);
    assert!(members.contains(&leaf_id));
    assert!(members.contains(&peer_id));
    assert!(members.contains(&parent_id));
    assert_eq!(fs::read(&leaf_path).unwrap(), b"required leaf");
    assert_eq!(fs::read(&peer_path).unwrap(), b"required peer");
    assert_eq!(fs::read(&parent_path).unwrap(), b"required parent");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn replaced_linked_attachment_cannot_authorize_retention_cleanup() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    let (h, target) = linked_retention_host("replaced-linked-retention");
    let expired = ".chrono-harness/state/worktrees/linked-expired.bin";
    fs::write(target.join(expired), b"foreign original").unwrap();
    Inventory::adopted(&target)
        .unwrap()
        .unwrap()
        .begin("lifecycle-original", expired, value!(null))
        .unwrap()
        .complete(value!({"status":"failed","error":"original failure"}))
        .unwrap();
    let source = fs::read(target.join("payload")).unwrap();
    let attachment = fs::read(target.join(".git")).unwrap();
    let foreign = b"gitdir: /unknown/foreign-owner\n";
    fs::write(target.join(".git"), foreign).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    // Restore the fixture's actual attachment after capturing the foreign bytes.
    let observed_attachment = fs::read(target.join(".git")).unwrap();
    fs::write(target.join(".git"), attachment).unwrap();
    assert_ne!(code, 0, "{report} {error}");
    assert_eq!(observed_attachment, foreign);
    assert_eq!(fs::read(target.join(expired)).unwrap(), b"foreign original");
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
    let rows = original_at(&h.root, &report);
    assert!(!rows.iter().any(|row| {
        row["kind"] == "retention"
            && row["source_root"] == value!(target)
            && row["observation"]["kind"] == "declaration-state"
    }));
}
