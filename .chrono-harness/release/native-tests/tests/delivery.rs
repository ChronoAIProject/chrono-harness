#[path = "support/recipe.rs"]
mod fixture;
use chrono_host_release::{
    delivery::{self, Package},
    identity::{Channel, Identity, VerifiedCi},
    receipt::Selection,
    recipe::Loaded,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

fn identity() -> Identity {
    Identity {
        channel: Channel::Preview,
        version: "v1.2.3-dev.10.1.gaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        source_commit: "a".repeat(40),
        source_tree: "b".repeat(40),
        verified_ci: Some(VerifiedCi {
            run_id: 10,
            attempt: 1,
        }),
    }
}
fn selected() -> Selection {
    Selection {
        job: "native_collect".into(),
        artifact_id: "30".into(),
        run_id: "20".into(),
        attempt: "1".into(),
    }
}

#[test]
fn dependency_selection_keeps_the_original_attempt_and_rejects_current_failures() {
    let valid = json!({"native_collect":{"result":"success","outputs":{"run_id":"20","attempt":"1","artifact_id":"30"}}});
    assert_eq!(
        delivery::select("native_collect", &valid, "20", "2").unwrap(),
        selected()
    );
    for (path, value) in [
        ("result", json!("failure")),
        ("result", json!("cancelled")),
        ("result", json!("skipped")),
        ("run_id", json!("21")),
        ("attempt", json!("3")),
        ("artifact_id", json!("0")),
    ] {
        let mut invalid = valid.clone();
        if path == "result" {
            invalid["native_collect"][path] = value;
        } else {
            invalid["native_collect"]["outputs"][path] = value;
        }
        assert!(
            delivery::select("native_collect", &invalid, "20", "2").is_err(),
            "{path}"
        );
    }
}

fn package_fixture() -> (fixture::Fixture, Loaded, Package, Value) {
    let f = fixture::Fixture::new();
    let loaded = Loaded::load(f.root()).unwrap();
    let package = Package {
        platform: "linux-x86_64".into(),
        directory: "package".into(),
    };
    let root = f.root().join("package");
    fs::create_dir(&root).unwrap();
    let mut members = BTreeMap::new();
    let mut observed = BTreeMap::new();
    for asset in ["a", "b"] {
        let name = format!("{asset}-linux-x86_64");
        let bytes = format!("fixture executable {asset}").into_bytes();
        fs::write(root.join(&name), &bytes).unwrap();
        members.insert(
            asset,
            json!({"file":name,"sha256":chrono_harness::sha256(&bytes),"size":bytes.len()}),
        );
        observed.insert(asset,json!({"path":name,"sha256":chrono_harness::sha256(&bytes),"size":bytes.len(),"mode":493}));
    }
    fixture::write(
        &root,
        "release.json",
        &json!({"schema":"chrono-release/v1","version":identity().version,"source_commit":identity().source_commit,"source_tree":identity().source_tree,"platforms":{"linux-x86_64":members}}),
    );
    fs::write(root.join("empty"), b"").unwrap();
    let empty = json!({"path":"empty","size":0,"sha256":chrono_harness::sha256(b"")});
    let mut originals = BTreeMap::new();
    for id in ["build_a", "build_b", "test_none", "test_a"] {
        let dir = root.join("originals").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("receipt.json"), b"original receipt").unwrap();
        originals.insert(id,json!({"receipt.json":{"path":"receipt.json","size":16,"sha256":chrono_harness::sha256(b"original receipt")}}));
    }
    let report = json!({"schema":"chrono-release-unit/v1","status":"passed","failure":null,"started_ns":1,"finished_ns":2,"unit":serde_json::to_value(&loaded.recipe.collector).unwrap(),"lineage":{"kind":"github","run_id":"20","attempt":"1","job":"native_collect"},"context":{"source_commit":identity().source_commit,"source_tree":identity().source_tree,"release_identity":identity()},"package":{"release_sha256":chrono_harness::file_identity(&root.join("release.json")).unwrap().0,"assets":observed},"originals":originals,"processes":[{"exit_code":0,"stdout":empty,"stderr":empty}]});
    fixture::write(&root, "receipt.json", &report);
    (f, loaded, package, report)
}

#[test]
fn consumes_exact_collector_source_lineage_originals_and_asset_bytes() {
    let (f, loaded, package, _) = package_fixture();
    delivery::verify_package(f.root(), &loaded, &package, &selected(), &identity()).unwrap();
    fs::write(f.root().join("package/a-linux-x86_64"), b"changed").unwrap();
    assert!(
        delivery::verify_package(f.root(), &loaded, &package, &selected(), &identity()).is_err()
    );
}

#[test]
fn passing_label_cannot_hide_bad_identity_originals_or_failed_processes() {
    for case in 0..7 {
        let (f, loaded, package, mut report) = package_fixture();
        match case {
            0 => report["context"]["release_identity"]["verified_ci"]["attempt"] = json!(2),
            1 => report["lineage"]["attempt"] = json!("2"),
            2 => report["processes"][0]["exit_code"] = json!(17),
            3 => {
                fs::write(
                    f.root().join("package/originals/build_a/receipt.json"),
                    b"changed",
                )
                .unwrap();
            }
            4 => report["package"]["release_sha256"] = json!("c".repeat(64)),
            5 => report["originals"]
                .as_object_mut()
                .unwrap()
                .remove("test_a")
                .map(|_| ())
                .unwrap(),
            6 => report["finished_ns"] = Value::Null,
            _ => unreachable!(),
        }
        fixture::write(&f.root().join("package"), "receipt.json", &report);
        assert!(
            delivery::verify_package(f.root(), &loaded, &package, &selected(), &identity())
                .is_err(),
            "{case}"
        );
    }
}
