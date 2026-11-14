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
    let descriptor = &report["complete_inventory"];
    let path = descriptor["path"].as_str().unwrap();
    let bytes = fs::read(h.root.join(path)).unwrap();
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
