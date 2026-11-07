#[path = "support/construction.rs"]
mod support;
use chrono_host_bootstrap::{
    bindings::NativePlatform, original::Producer, profile, source::SourceObservation,
};
use serde_json::{Value, json};
use std::fs;
use support::Fixture;

fn original(fixture: &Fixture) -> Vec<u8> {
    let mut build = fixture.committed();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    build.select().unwrap();
    build.operations(&mut vec![], &mut vec![]).unwrap();
    build
        .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
        .unwrap();
    let published = build.finish(Ok(())).unwrap();
    assert!(published.outcome.is_ok());
    fs::read(published.original).unwrap()
}

fn input(fixture: &Fixture, path: &str) -> Result<Vec<u8>, String> {
    profile::read(&fixture.root, path, b"")
}

fn producer(fixture: &Fixture) -> Producer {
    Producer::read(
        ".chrono-harness/ci/build.json",
        &NativePlatform {
            system: "fixture".into(),
            machine: "native".into(),
        },
        &|path| input(fixture, path),
    )
    .unwrap()
}

fn observed(raw: &[u8]) -> SourceObservation {
    let report: Value = serde_json::from_slice(raw).unwrap();
    serde_json::from_value(report["source"]["before"].clone()).unwrap()
}

#[test]
fn qualifies_actual_ordered_execution_and_binary_streams_without_rerunning_tools() {
    let mut fixture = Fixture::new();
    fixture.projects["scripts"][0]["actions"]["execute"]["argv"] = json!([
        "run",
        serde_json::to_string(&json!([{"Stdout":[0,255,10]}, {"Stderr":[254,0,10]}])).unwrap()
    ]);
    let raw = original(&fixture);
    let calls = fs::read(fixture.tools.join("calls.json")).unwrap();
    let qualified = producer(&fixture)
        .validate(&raw, &observed(&raw), &|path| input(&fixture, path))
        .unwrap();
    assert_eq!(qualified.operations.len(), 2);
    assert_eq!(
        fs::read(fixture.root.join(&qualified.operations[0].stdout.path)).unwrap(),
        [0, 255, 10]
    );
    assert_eq!(
        fs::read(fixture.root.join(&qualified.operations[0].stderr.path)).unwrap(),
        [254, 0, 10]
    );
    assert_eq!(fs::read(fixture.tools.join("calls.json")).unwrap(), calls);
}

#[test]
fn rejects_missing_or_forged_execution_source_and_selection_facts() {
    let fixture = Fixture::new();
    let raw = original(&fixture);
    let original: Value = serde_json::from_slice(&raw).unwrap();
    let producer = producer(&fixture);
    let observed = observed(&raw);
    producer
        .validate(&raw, &observed, &|path| input(&fixture, path))
        .unwrap();
    for (pointer, replacement) in [
        ("/schema", json!("other")),
        ("/status", json!("failed")),
        ("/config", json!(".chrono-harness/ci/other.json")),
        ("/host_root", json!("relative")),
        ("/source/state", json!("dirty")),
        ("/source/commit", json!("0".repeat(40))),
        ("/source/before/dirty", json!(true)),
        ("/source/after/error", json!("failed")),
        ("/executables/0/path", json!("relative")),
        ("/executables/0/sha256", json!("0".repeat(64))),
        ("/executables/1/version", json!("rustc unbound")),
        ("/selection/compiler", json!("/other/compiler")),
        ("/versions/cargo", json!("other")),
        ("/environment/RUSTC_WRAPPER", json!("unregistered-wrapper")),
        ("/environment/CARGO_INCREMENTAL", json!("1")),
        (
            "/registrations/.chrono-harness~1ci~1build.json",
            json!("0".repeat(64)),
        ),
        ("/operations/0/tool", json!("other")),
        ("/operations/0/argv/1", json!("other")),
        ("/operations/0/cwd", json!("/other")),
        ("/operations/0/exit_code", json!(17)),
        ("/operations/0/exit_code", json!(false)),
        ("/operations/0/pid", json!(0)),
        ("/operations/1/executable_sha256", json!("0".repeat(64))),
    ] {
        let mut changed = original.clone();
        *changed
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing fixture field {pointer}")) = replacement;
        assert!(
            producer
                .validate(&serde_json::to_vec(&changed).unwrap(), &observed, &|path| {
                    input(&fixture, path)
                })
                .is_err(),
            "accepted {pointer}"
        );
    }
    for field in [
        "selection",
        "executables",
        "source",
        "registrations",
        "operations",
    ] {
        let mut changed = original.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert!(
            producer
                .validate(&serde_json::to_vec(&changed).unwrap(), &observed, &|path| {
                    input(&fixture, path)
                })
                .is_err(),
            "accepted missing {field}"
        );
    }
    for change in ["reorder", "missing", "registration", "imported"] {
        let mut changed = original.clone();
        match change {
            "reorder" => changed["operations"].as_array_mut().unwrap().reverse(),
            "missing" => {
                changed["operations"].as_array_mut().unwrap().pop();
            }
            "registration" => {
                changed["registrations"]["unregistered"] = json!("0".repeat(64));
            }
            "imported" => changed["provenance"] = Value::Null,
            _ => unreachable!(),
        }
        assert!(
            producer
                .validate(&serde_json::to_vec(&changed).unwrap(), &observed, &|path| {
                    input(&fixture, path)
                })
                .is_err(),
            "accepted {change}"
        );
    }
}

#[test]
fn rejects_changed_missing_shared_or_escaping_original_streams() {
    let fixture = Fixture::new();
    let raw = original(&fixture);
    let original: Value = serde_json::from_slice(&raw).unwrap();
    let producer = producer(&fixture);
    let observed = observed(&raw);
    producer
        .validate(&raw, &observed, &|path| input(&fixture, path))
        .unwrap();
    for replacement in [
        json!({"path": original["operations"][0]["stdout"]["path"], "sha256":"0".repeat(64), "size":4}),
        json!({"path": original["operations"][0]["stdout"]["path"], "sha256": original["operations"][0]["stdout"]["sha256"], "size":999}),
        original["operations"][0]["stderr"].clone(),
        json!({"path":".chrono-harness/state/../ci/build.json", "sha256":"0".repeat(64), "size":0}),
        json!({"path":".chrono-harness/state/absent", "sha256":"0".repeat(64), "size":0}),
    ] {
        let mut changed = original.clone();
        changed["operations"][0]["stdout"] = replacement;
        assert!(
            producer
                .validate(&serde_json::to_vec(&changed).unwrap(), &observed, &|path| {
                    input(&fixture, path)
                })
                .is_err()
        );
    }
    let path = fixture.root.join(
        original["operations"][0]["stdout"]["path"]
            .as_str()
            .unwrap(),
    );
    fs::write(path, b"forged original bytes").unwrap();
    assert!(
        producer
            .validate(&raw, &observed, &|path| input(&fixture, path))
            .is_err()
    );
}

#[test]
fn fixed_python_producer_data_survives_current_source_drift_without_old_execution_or_home_effects()
{
    use chrono_host_bootstrap::source::Committed;
    let mut fixture = Fixture::new();
    fixture.config["entrypoint"] =
        json!({"tool":"python3", "script":".chrono-harness/ci/bootstrap.py"});
    fixture.config["owner_upgrade"] = json!(".chrono-harness/ci/upgrade.json");
    fixture.config["host_environment"] = json!({"HOME":".chrono-harness/cache/producer-home"});
    let fixed = [
        ".chrono-harness/ci/bootstrap.py",
        ".chrono-harness/ci/bootstrap_owner.py",
        ".chrono-harness/ci/bootstrap_shared.py",
    ];
    for path in fixed {
        fs::write(
            fixture.root.join(path),
            b"this is fixed source data, not executable Python\0\xff",
        )
        .unwrap();
    }
    fs::write(fixture.root.join(".chrono-harness/ci/upgrade.json"), b"{}").unwrap();
    std::os::unix::fs::symlink("entry.sh", fixture.root.join(".chrono-harness/ci/linked")).unwrap();
    let raw = original(&fixture);
    let mut report: Value = serde_json::from_slice(&raw).unwrap();
    // 外部 Python producer 报告形状夹具：操作和流来自本测试的真实 Rust 子进程，
    // 这里只补入该固定数据格式额外登记的两份模块摘要，不声称执行过历史 Python。
    for path in &fixed[1..] {
        report["registrations"][*path] = json!(chrono_harness::sha256(
            &fs::read(fixture.root.join(path)).unwrap()
        ));
    }
    let source = observed(&raw);
    let reader = Committed::new(
        &fixture.root,
        &fixture.environment,
        source.commit.as_ref().unwrap(),
        source.tree.as_ref().unwrap(),
    )
    .unwrap();
    let calls = fs::read(fixture.tools.join("calls.json")).unwrap();
    for path in fixed {
        fs::remove_file(fixture.root.join(path)).unwrap();
    }
    fs::write(
        fixture.root.join(".chrono-harness/projects.json"),
        b"not current JSON",
    )
    .unwrap();
    fs::remove_file(fixture.root.join(".chrono-harness/ci/registry.json")).unwrap();
    std::os::unix::fs::symlink(
        "absent",
        fixture.root.join(".chrono-harness/ci/registry.json"),
    )
    .unwrap();
    fs::remove_dir(fixture.root.join(".chrono-harness/cache/producer-home")).unwrap();
    let producer = Producer::read(
        ".chrono-harness/ci/build.json",
        &NativePlatform {
            system: "fixture".into(),
            machine: "native".into(),
        },
        &|path| reader.read(path),
    )
    .unwrap();
    let raw = serde_json::to_vec(&report).unwrap();
    producer
        .validate(&raw, &reader.observation(), &|path| input(&fixture, path))
        .unwrap();
    assert_eq!(fs::read(fixture.tools.join("calls.json")).unwrap(), calls);
    assert!(
        !fixture
            .root
            .join(".chrono-harness/cache/producer-home")
            .exists()
    );
    assert!(
        reader
            .read(".chrono-harness/ci/linked")
            .unwrap_err()
            .contains("not a regular file")
    );
    assert!(reader.read(".chrono-harness/ci/absent").is_err());
    assert!(reader.read(".chrono-harness/ci/../projects.json").is_err());
    for path in &fixed[1..] {
        let mut changed = report.clone();
        changed["registrations"]
            .as_object_mut()
            .unwrap()
            .remove(*path);
        assert!(
            producer
                .validate(
                    &serde_json::to_vec(&changed).unwrap(),
                    &reader.observation(),
                    &|path| input(&fixture, path)
                )
                .is_err()
        );
    }
    let mut changed = report;
    changed["environment"]["HOME"] = json!("/consumer/current-home");
    assert!(
        producer
            .validate(
                &serde_json::to_vec(&changed).unwrap(),
                &reader.observation(),
                &|path| input(&fixture, path)
            )
            .is_err()
    );
}

#[test]
fn immutable_reader_rejects_symbolic_missing_and_mismatched_producer_identities() {
    use chrono_host_bootstrap::source::Committed;
    let fixture = Fixture::new();
    for (commit, tree) in [
        ("HEAD".to_owned(), "0".repeat(40)),
        ("0".repeat(40), "HEAD".to_owned()),
        ("A".repeat(40), "0".repeat(40)),
        ("0".repeat(39), "0".repeat(40)),
    ] {
        assert!(
            Committed::new(&fixture.root, &fixture.environment, &commit, &tree)
                .err()
                .unwrap()
                .contains("immutable producer commit and tree")
        );
    }
    let raw = original(&fixture);
    let source = observed(&raw);
    let mismatch = Committed::new(
        &fixture.root,
        &fixture.environment,
        source.commit.as_ref().unwrap(),
        &"1".repeat(40),
    )
    .err()
    .unwrap();
    assert!(mismatch.contains("tree differs"), "{mismatch}");
    assert!(
        Committed::new(
            &fixture.root,
            &fixture.environment,
            &"1".repeat(40),
            source.tree.as_ref().unwrap()
        )
        .is_err()
    );
}

#[test]
fn native_entrypoint_qualifies_the_fixed_original_and_rejects_other_hosts_and_escaping_references()
{
    use chrono_host_bootstrap::bindings::native_platform;
    let fixture = Fixture::new();
    let registry_path = fixture.root.join(".chrono-harness/ci/registry.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    let native = native_platform().unwrap();
    registry["platforms"][format!("{}/{}", native.system, native.machine)] =
        registry["platforms"]["fixture/native"].clone();
    fs::write(&registry_path, serde_json::to_vec(&registry).unwrap()).unwrap();
    let raw = original(&fixture);
    let calls = fs::read(fixture.tools.join("calls.json")).unwrap();
    let invoke = |arguments: &[&str]| {
        let mut command =
            std::process::Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
        command
            .current_dir(&fixture.root)
            .args(arguments)
            .env_clear()
            .envs(fixture.environment.iter().filter(|(key, _)| {
                key.as_os_str() != std::ffi::OsStr::new(chrono_harness::process_fds::ENV)
            }));
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        command.output().unwrap()
    };
    let output = invoke(&[
        "validate-original-execution",
        ".",
        ".chrono-harness/state/bootstrap.json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        receipt["schema"],
        "chrono-bootstrap-original-execution-validation/v1"
    );
    assert_eq!(receipt["sha256"], chrono_harness::sha256(&raw));
    assert_eq!(receipt["operations"], 2);
    assert_eq!(
        receipt["source"],
        serde_json::from_slice::<Value>(&raw).unwrap()["source"]
    );
    for arguments in [
        vec![
            "validate-original-execution",
            ".",
            ".chrono-harness/state/../ci/build.json",
        ],
        vec![
            "validate-original-execution",
            ".",
            ".chrono-harness/ci/build.json",
        ],
        vec!["unknown", ".", ".chrono-harness/state/bootstrap.json"],
        vec![],
    ] {
        let output = invoke(&arguments);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let mut changed: Value = serde_json::from_slice(&raw).unwrap();
    changed["host_root"] = json!(fixture.root.join("another-host"));
    fs::write(
        fixture.root.join(".chrono-harness/state/bootstrap.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let output = invoke(&[
        "validate-original-execution",
        ".",
        ".chrono-harness/state/bootstrap.json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("another host"));
    assert_eq!(fs::read(fixture.tools.join("calls.json")).unwrap(), calls);
}
