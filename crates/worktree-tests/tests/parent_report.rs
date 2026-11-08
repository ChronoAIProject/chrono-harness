use super::*;
use chrono_harness::retained_artifacts::retention::{self, Inventory};

fn host() -> Host {
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    let policy = h.root.join(retention::POLICY_PATH);
    let mut value = json(&fs::read(&policy).unwrap()).unwrap();
    value["producers"]["check-original"] = value!([".chrono-harness/state/"]);
    value["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    value["max_related_outputs"] = value!(512);
    value["diagnostic_keep_seconds"] = value!(0);
    fs::write(policy, serde_json::to_vec(&value).unwrap()).unwrap();
    commit(&h.root);
    h
}

fn parent(h: &Host) -> (String, retention::Publication) {
    let path = ".chrono-harness/state/bootstrap-owner-upgrade/attempt-test".to_owned();
    fs::create_dir_all(h.root.join(".chrono-harness/state/bootstrap-owner-upgrade")).unwrap();
    let publication = retention::create_adopted(
        &h.root,
        "check-original",
        &h.root.join(&path),
        true,
        value!({"status":"started","completion":"not-established"}),
    )
    .unwrap()
    .unwrap();
    (path, publication)
}

fn maintain(h: &Host) {
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    for _ in 0..8 {
        let result = inventory.maintain().unwrap();
        assert_eq!(result["accounting_errors"], value!([]), "{result}");
    }
}

#[test]
fn parent_report_keeps_all_phase_originals_until_the_parent_retires() {
    let h = host();
    let (parent, publication) = parent(&h);
    let mut originals = Vec::new();
    for _ in 0..3 {
        let result = h._dir.capture_bound(
            &mut h.auto_command("status", &["--report-parent", &parent]),
        );
        assert!(result.status.success(), "{result:?}");
        let (_, report, _) = h.received(result);
        assert_eq!(report["report_parent"], parent);
        assert!(report["report_consumer"].is_null());
        originals.push(h.root.join(report["report_path"].as_str().unwrap()));
        maintain(&h);
        assert!(originals.iter().all(|path| path.is_file()));
    }
    publication.complete(value!({"status":"joined"})).unwrap();
    maintain(&h);
    assert!(!h.root.join(parent).exists());
    assert!(originals.iter().all(|path| !path.exists()));
}

#[test]
fn parent_report_rejects_an_unregistered_parent_before_activity() {
    let h = host();
    let parent = ".chrono-harness/state/unregistered-parent";
    fs::create_dir(h.root.join(parent)).unwrap();
    let result = h._dir.capture_bound(
        &mut h.auto_command("status", &["--report-parent", parent]),
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("parent"), "{result:?}");
    assert!(result.stdout.is_empty(), "activity must not publish a report: {result:?}");
}

#[test]
fn parent_report_interruption_keeps_the_original_unknown_and_allows_expiry() {
    use std::os::unix::process::ExitStatusExt;
    let h = host();
    super::automatic::native_git(&h, "parent-report-interrupt", value!({}));
    let (parent, publication) = parent(&h);
    fs::write(h.parent.join("interrupt-parent-report"), "interrupt after publication begins").unwrap();
    let result = h._dir.capture_bound(
        &mut h.auto_command("maintain", &["--report-parent", &parent]),
    );
    assert_eq!(result.status.signal(), Some(9), "{result:?}");
    let snapshot = retention_state::snapshot(&h.root);
    let output = snapshot["outputs"].as_array().unwrap().iter()
        .find(|output| output["path"] == parent).unwrap();
    let references = output["references"].as_array().unwrap();
    assert_eq!(references.len(), 1, "{snapshot}");
    let original = snapshot["outputs"].as_array().unwrap().iter()
        .find(|output| output["id"] == references[0]).unwrap();
    assert_eq!(original["outcome"]["outcome"], "not-established");
    let path = h.root.join(original["path"].as_str().unwrap());
    assert!(fs::read(&path).unwrap().is_empty());
    maintain(&h);
    assert!(path.is_file());
    drop(publication);
    maintain(&h);
    assert!(!h.root.join(parent).exists());
    assert!(!path.exists());
}
