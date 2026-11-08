#[path = "support/receipt.rs"]
mod fixture;
#[path = "support/frontend.rs"]
mod frontend_fixture;
use chrono_host_release::{
    receipt::{self, Kind, Lineage},
    recipe::{Loaded, Scope},
};
use fixture::{Fixture, original};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink};

#[test]
fn native_frontend_is_required_separately_from_business_assets() {
    let mut host = fixture::recipe::Fixture::new();
    host.config["native_frontends"] = json!({"fixture":{
        "job":"native_frontend", "directory":"frontend",
        "executable":"chrono-host-release", "bootstrap_profile":".chrono-harness/ci/bootstrap-release.json",
        "bootstrap_report":"bootstrap.json", "cost":"unmeasured"
    }});
    host.save();
    let loaded = Loaded::load(host.root()).unwrap();
    let parent = Lineage {
        kind: Kind::Github,
        run_id: "1234".into(),
        attempt: "3".into(),
        job: "native_test_none".into(),
    };
    let input = json!({"native_frontend":{"result":"success","outputs":{
        "artifact_id":"98765","run_id":"1234","attempt":"1"
    }}});
    let select = |input: &Value| {
        receipt::select(
            &loaded,
            Scope::Unit("test_none"),
            &parent,
            &serde_json::to_vec(input).unwrap(),
        )
    };
    let value = serde_json::to_value(select(&input).unwrap()).unwrap();
    assert_eq!(value["units"], json!({}));
    assert_eq!(
        value["frontend"],
        json!({"job":"native_frontend","artifact_id":"98765","run_id":"1234","attempt":"1"})
    );
    assert!(select(&json!({})).is_err());
    for outcome in ["failure", "cancelled", "skipped"] {
        let mut bad = input.clone();
        bad["native_frontend"]["result"] = json!(outcome);
        assert!(select(&bad).is_err(), "{outcome}");
    }
    for (key, value) in [
        ("run_id", "foreign"),
        ("attempt", "4"),
        ("artifact_id", "by-name"),
    ] {
        let mut bad = input.clone();
        bad["native_frontend"]["outputs"][key] = json!(value);
        assert!(select(&bad).is_err(), "{key}");
    }
    let mut extra = input.clone();
    extra["unregistered-tool"] = input["native_frontend"].clone();
    assert!(select(&extra).unwrap_err().contains("membership"));
}

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
    let input = json!({"native_build_a":{"result":"success","outputs":{"artifact_id":"98765","run_id":"1234","attempt":"1"}},"native_frontend":{"result":"success","outputs":{"artifact_id":"98766","run_id":"1234","attempt":"1"}}});
    let select = |v: &Value| {
        receipt::select(
            &loaded,
            Scope::Unit("test_a"),
            &parent,
            &serde_json::to_vec(v).unwrap(),
        )
    };
    let selected = select(&input).unwrap();
    assert_eq!(selected.units["build_a"].attempt, "1");
    assert_eq!(selected.frontend.as_ref().unwrap().artifact_id, "98766");
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
fn local_receipt_requires_the_original_receipt_digest() {
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
}

#[test]
fn native_receipt_matches_original_job_run_attempt_and_rejects_unknown_fields() {
    use chrono_host_release::receipt::Selection;
    let f = frontend_fixture::Fixture::new();
    let frontend = f.exported();
    let out = f.unit(&frontend.join("chrono-host-release"));
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let loaded = chrono_host_release::recipe::Loaded::load(f.host.root()).unwrap();
    let unit = loaded.unit("test_none").unwrap();
    let report_path = f.host.root().join("handoffs/none/receipt.json");
    let mut report: Value = serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    let context = report["context"].clone();
    let parent = Lineage {
        kind: Kind::Github,
        run_id: "1234".into(),
        attempt: "3".into(),
        job: "native_collect".into(),
    };
    let selected = Selection {
        job: "native_test_none".into(),
        artifact_id: "98765".into(),
        run_id: "1234".into(),
        attempt: "2".into(),
    };
    let read = |selected: &Selection| {
        receipt::read(&loaded, f.host.root(), &context, &parent, unit, selected)
    };
    assert!(
        read(&selected).is_ok(),
        "{:?}",
        read(&selected).unwrap_err()
    );
    for (field, value) in [("job", "foreign"), ("run_id", "foreign"), ("attempt", "1")] {
        let mut wrong = serde_json::to_value(&selected).unwrap();
        wrong[field] = json!(value);
        assert!(
            read(&serde_json::from_value(wrong).unwrap()).is_err(),
            "{field}"
        );
    }
    report["lineage"]["artifact_id"] = json!("unexpected");
    fs::write(report_path, serde_json::to_vec(&report).unwrap()).unwrap();
    assert!(read(&selected).is_err());
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

#[test]
fn validated_build_receipt_imports_exact_bytes_mode_and_producing_lineage() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new(true);
    let bytes = b"explicit external executable bytes";
    f.archive(bytes, 0o751);
    let selected = f.save();
    let loaded = f.loaded();
    let imported = receipt::import_build(
        &loaded,
        f.host.root(),
        &f.context,
        &f.parent,
        loaded.unit(f.id).unwrap(),
        &selected,
    )
    .unwrap();
    let output = f.host.root().join("out/a");
    assert_eq!(fs::read(&output).unwrap(), bytes);
    assert_eq!(
        fs::metadata(output).unwrap().permissions().mode() & 0o7777,
        0o751
    );
    assert_eq!(imported.asset, "a");
    assert_eq!(imported.lineage["receipt_sha256"], selected.artifact_id);
    assert_eq!(
        imported.lineage["selection"],
        serde_json::to_value(&selected).unwrap()
    );
    assert_eq!(
        imported.original.receipt["assets"]["a"],
        imported.lineage["identity"]
    );
}

#[test]
fn failed_receipt_archive_or_member_identity_preserves_existing_consumer() {
    for mutation in 0..8 {
        let mut f = Fixture::new(true);
        f.archive(b"selected program", 0o755);
        let output = f.host.root().join("out/a");
        fs::create_dir_all(output.parent().unwrap()).unwrap();
        fs::write(&output, b"existing consumer").unwrap();
        match mutation {
            0 => f.report["status"] = json!("failed"),
            1 => f.report["assets"]["a"]["path"] = json!("other/program"),
            2 => f.report["assets"]["extra"] = json!({}),
            3 => f.report["assets"]["a"]["sha256"] = json!("9".repeat(64)),
            4 => f.report["assets"]["a"]["mode"] = json!(0o751),
            5 => f.report["transport"]["path"] = json!("other.tar"),
            6 => fs::write(f.directory().join("executable.tar"), b"changed archive").unwrap(),
            7 => f.report["processes"][8]["exit_code"] = json!(9),
            _ => unreachable!(),
        }
        let selected = f.save();
        let loaded = f.loaded();
        assert!(
            receipt::import_build(
                &loaded,
                f.host.root(),
                &f.context,
                &f.parent,
                loaded.unit(f.id).unwrap(),
                &selected
            )
            .is_err(),
            "mutation {mutation}"
        );
        assert_eq!(fs::read(output).unwrap(), b"existing consumer");
    }
}

#[test]
fn verifier_consumption_checks_full_identity_and_selected_producer_before_after() {
    let mut producer = Fixture::new(true);
    producer.archive(b"selected program", 0o755);
    let selected = producer.save();
    let loaded = producer.loaded();
    let imported = receipt::import_build(
        &loaded,
        producer.host.root(),
        &producer.context,
        &producer.parent,
        loaded.unit(producer.id).unwrap(),
        &selected,
    )
    .unwrap();
    let assets = json!({"a":imported.lineage});
    let mut f = Fixture::for_unit("test_a");
    let input = json!({"build_a":{"result":"success","outputs":{"artifact_id":selected.artifact_id,"run_id":"same-run","attempt":"1"}}});
    f.report["dependency_input"] = original(
        &f.directory(),
        "dependency-context.json",
        &serde_json::to_vec(&input).unwrap(),
    );
    f.report["selected"] = json!({"build_a":selected});
    f.report["assets"] = assets.clone();
    let mut destination = assets["a"]["identity"].clone();
    destination["path"] = json!("consumer/a");
    let files = json!([assets["a"]["identity"], destination]);
    f.report["consumer_staging"] = json!({"observations":[
        {"phase":"staged","operation":null,"matches":true,"files":files},
        {"phase":"before-verification","operation":"test.a","matches":true,"files":files},
        {"phase":"after-verification","operation":"test.a","matches":true,"files":files}
    ]});
    let original = f.report.clone();
    let verified = f.read().unwrap();
    receipt::verify_consumption(&f.loaded(), &verified, &assets).unwrap();
    for mutation in 0..8 {
        f.report = original.clone();
        match mutation {
            0 => f.report["assets"]["a"]["receipt_sha256"] = json!("9".repeat(64)),
            1 => {
                f.report["consumer_staging"]["observations"][2]["files"][1]["sha256"] =
                    json!("9".repeat(64))
            }
            2 => f.report["consumer_staging"]["observations"][1]["files"][0]["mode"] = json!(0o751),
            3 => f.report["consumer_staging"]["observations"][2]["files"] = json!([]),
            4 => f.report["consumer_staging"]["observations"][1]["matches"] = json!(false),
            5 => f.report["consumer_staging"]["observations"][1]["operation"] = json!("unrelated"),
            6 => {
                f.report["consumer_staging"]["observations"][2]["phase"] =
                    json!("before-verification")
            }
            7 => {
                f.report["consumer_staging"]["observations"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            _ => unreachable!(),
        }
        let verified = f.read().unwrap();
        assert!(
            receipt::verify_consumption(&f.loaded(), &verified, &assets).is_err(),
            "mutation {mutation}"
        );
    }
}
