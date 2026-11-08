#[path = "support/receipt.rs"]
mod fixture;
use chrono_host_release::{
    receipt::{self, Kind, Lineage},
    recipe::{Loaded, Scope},
};
use fixture::{Fixture, original};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink};

#[test]
fn literal_non_rust_receipt_needs_no_sdk_and_accepts_exact_time_boundary() {
    let mut f = Fixture::new(false);
    f.report["finished_ns"] = json!(30_000_000_001u64);
    let verified = f.read().unwrap();
    assert_eq!(verified.directory, f.directory());
    assert!(verified.compiler_fingerprint().is_none());
    assert_eq!(verified.receipt["unit"]["operation"], "test.none");
    assert_eq!(verified.sha256, f.save().artifact_id);
    f.report["finished_ns"] = json!(30_000_000_002u64);
    assert!(f.read().unwrap_err().contains("timing"));
}

#[test]
fn native_current_needs_select_original_earlier_attempt_without_accepting_old_failure() {
    let host = fixture::recipe::Fixture::new();
    let loaded = Loaded::load(host.root()).unwrap();
    let parent = Lineage {
        kind: Kind::Github,
        run_id: "1234".into(),
        attempt: "3".into(),
        job: "native_test_a".into(),
    };
    let input = json!({"native_build_a":{"result":"success","outputs":{"artifact_id":"98765","run_id":"1234","attempt":"1"}}});
    let select = |v: &Value| {
        receipt::select(
            &loaded,
            Scope::Unit("test_a"),
            &parent,
            &serde_json::to_vec(v).unwrap(),
        )
    };
    let selected = select(&input).unwrap();
    assert_eq!(selected["build_a"].attempt, "1");
    for outcome in ["failure", "cancelled", "skipped", ""] {
        let mut bad = input.clone();
        bad["native_build_a"]["result"] = json!(outcome);
        assert!(select(&bad).is_err(), "{outcome}");
    }
    for (key, value) in [
        ("attempt", "0"),
        ("attempt", "4"),
        ("attempt", "١"),
        ("run_id", "other"),
        ("artifact_id", "by-name"),
        ("artifact_id", ""),
    ] {
        let mut bad = input.clone();
        bad["native_build_a"]["outputs"][key] = json!(value);
        assert!(select(&bad).is_err(), "{key}={value}");
    }
    let mut extra = input.clone();
    extra["tool-job"] = json!({"result":"success"});
    assert!(select(&extra).unwrap_err().contains("membership"));
    assert!(select(&json!({})).is_err());
    let repeated = br#"{"native_build_a":{},"native_build_a":{}}"#;
    assert!(receipt::select(&loaded, Scope::Unit("test_a"), &parent, repeated).is_err());
}

#[test]
fn native_receipt_matches_original_job_run_attempt_and_local_digest_is_not_optional() {
    let mut f = Fixture::new(false);
    let selection = f.save();
    let loaded = f.loaded();
    let unit = loaded.unit(f.id).unwrap();
    let mut wrong = selection.clone();
    wrong.artifact_id = "0".repeat(64);
    assert!(
        receipt::read(&loaded, f.host.root(), &f.context, &f.parent, unit, &wrong)
            .unwrap_err()
            .contains("digest")
    );
    f.parent.kind = Kind::Github;
    f.parent.job = "native_collect".into();
    f.report["lineage"] =
        json!({"kind":"github","run_id":"same-run","attempt":"1","job":"native_test_none"});
    let mut selected = f.save();
    selected.job = "native_test_none".into();
    selected.artifact_id = "98765".into();
    assert!(
        receipt::read(
            &loaded,
            f.host.root(),
            &f.context,
            &f.parent,
            unit,
            &selected
        )
        .is_ok()
    );
    for (field, value) in [("job", "foreign"), ("run_id", "foreign"), ("attempt", "2")] {
        let mut wrong = selected.clone();
        match field {
            "job" => wrong.job = value.into(),
            "run_id" => wrong.run_id = value.into(),
            _ => wrong.attempt = value.into(),
        }
        assert!(
            receipt::read(&loaded, f.host.root(), &f.context, &f.parent, unit, &wrong).is_err(),
            "{field}"
        );
    }
    f.report["lineage"]["artifact_id"] = json!("unexpected");
    f.save();
    assert!(
        receipt::read(
            &loaded,
            f.host.root(),
            &f.context,
            &f.parent,
            unit,
            &selected
        )
        .is_err()
    );
}

#[test]
fn incompatible_receipt_status_source_unit_timing_and_original_exit_are_rejected() {
    for mutation in 0..13 {
        let mut f = Fixture::new(false);
        match mutation {
            0 => f.report["status"] = json!("failed"),
            1 => f.report["failure"] = json!({"exit_code":7}),
            2 => f.report["context"]["source_commit"] = json!("3".repeat(40)),
            3 => f.report["unit"]["operation"] = json!("other"),
            4 => f.report["started_ns"] = json!(0),
            5 => f.report["finished_ns"] = Value::Null,
            6 => {
                f.report["started_ns"] = json!(100);
                f.report["finished_ns"] = json!(99)
            }
            7 => f.report["processes"][0]["exit_code"] = json!(7),
            8 => f.report["processes"][0]["exit_code"] = json!(false),
            9 => f.report["processes"] = json!([]),
            10 => f.report["schema"] = json!("old"),
            11 => {
                f.report.as_object_mut().unwrap().remove("failure");
            }
            12 => f.report["selected"] = json!({"foreign":{}}),
            _ => unreachable!(),
        }
        assert!(f.read().is_err(), "mutation {mutation}");
    }
}

#[test]
fn missing_corrupt_rehashed_or_linked_originals_cannot_become_passing_evidence() {
    for mutation in 0..7 {
        let mut f = Fixture::new(false);
        let dir = f.directory();
        let path = dir.join(f.report["processes"][0]["stdout"]["path"].as_str().unwrap());
        match mutation {
            0 => fs::remove_file(path).unwrap(),
            1 => fs::write(path, b"changed").unwrap(),
            2 => {
                let outside = tempfile::NamedTempFile::new().unwrap();
                fs::remove_file(&path).unwrap();
                symlink(outside.path(), &path).unwrap();
                assert!(f.read().is_err());
                continue;
            }
            3 => {
                f.report["processes"][0]["stdout"] =
                    original(&dir, "forged.stdout", b"wrong source but rehashed\n")
            }
            4 => fs::write(dir.join("dependency-context.json"), b"[]").unwrap(),
            5 => {
                f.report["dependency_input"] = original(
                    &dir,
                    "forged-input.json",
                    br#"{"foreign":{"result":"success"}}"#,
                )
            }
            6 => {
                f.report["processes"][2]["stdout"] = original(&dir, "dirty.stdout", b" M source\n")
            }
            _ => unreachable!(),
        }
        assert!(f.read().is_err(), "mutation {mutation}");
    }
}

#[test]
fn original_unfiltered_action_and_no_rust_choice_are_checked_at_consumption() {
    for mutation in 0..6 {
        let mut f = Fixture::new(false);
        match mutation {
            0 => f.report["processes"][3]["argv"] = json!(["/tools/probe", "changed"]),
            1 => f.report["processes"][3]["operation"] = json!("wrong"),
            2 => f.report["action_tool"] = json!("git"),
            3 => f.report["toolchain"] = json!({}),
            4 => f.report["processes"][3]["phase"] = json!("toolchain-install"),
            5 => f.report["execution_tools"]["unused"] = fixture::identity("/unused", 'f'),
            _ => unreachable!(),
        }
        assert!(f.read().is_err(), "mutation {mutation}");
    }
}

#[test]
fn rust_receipt_requires_original_install_versions_compiler_bytes_and_build_action() {
    let f = Fixture::new(true);
    assert!(f.read().unwrap().compiler_fingerprint().is_some());
    for mutation in 0..10 {
        let mut f = Fixture::new(true);
        match mutation {
            0 => f.report["toolchain"]["versions"]["rustc"] = json!("rustc other"),
            1 => f.report["processes"][3]["argv"][3] = json!("wrong-toolchain"),
            2 => f.report["processes"][4]["argv"] = json!(["/tools/cargo", "--help"]),
            3 => {
                f.report["toolchain"]["active_compilers"]["rustc"]["path"] = json!("relative/rustc")
            }
            4 => {
                f.report["toolchain"]["active_compilers"]["rustc"]["path"] =
                    json!("/different/rustc")
            }
            5 => {
                f.report["toolchain"]["active_compilers"]["rustc"]["sha256"] = json!("z".repeat(64))
            }
            6 => f.report["environment"]["CARGO_INCREMENTAL"] = json!("0"),
            7 => f.report["processes"][8]["argv"] = json!(["/tools/cargo", "build", "--tests"]),
            8 => f.report["execution_tools"]["cargo"]["size"] = json!(101),
            9 => f.report["toolchain"]["tools"]["cargo"]["mode"] = json!(0),
            _ => unreachable!(),
        }
        assert!(f.read().is_err(), "mutation {mutation}");
    }
}

#[test]
fn compiler_fingerprint_ignores_git_bytes_but_tracks_active_compilers() {
    let mut f = Fixture::new(true);
    let first = f.read().unwrap().compiler_fingerprint().unwrap();
    f.report["toolchain"]["tools"]["git"]["sha256"] = json!("9".repeat(64));
    f.report["execution_tools"]["git"]["sha256"] = json!("9".repeat(64));
    assert_eq!(f.read().unwrap().compiler_fingerprint().unwrap(), first);
    f.report["toolchain"]["active_compilers"]["rustc"]["sha256"] = json!("8".repeat(64));
    assert_ne!(f.read().unwrap().compiler_fingerprint().unwrap(), first);
}
