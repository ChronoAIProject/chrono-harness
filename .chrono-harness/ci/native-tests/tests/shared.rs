#[path = "support/construction.rs"]
mod support;
use chrono_host_bootstrap::{
    bindings::{self, Environment},
    construction::Construction,
    profile::Loaded,
};
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};
use support::Fixture;

const CONFIG: &str = ".chrono-harness/ci/shared.json";
const TRANSFER: &str = ".chrono-harness/cache/startup";

fn cli(root: &Path, environment: &Environment, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
    command.current_dir(root).env_clear().envs(
        environment
            .iter()
            .filter(|(key, _)| key.as_os_str() != OsStr::new(chrono_harness::process_fds::ENV)),
    );
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    command.arg(root).arg(CONFIG).args(args).output().unwrap()
}

fn passed(output: Output) {
    assert!(
        output.status.success(),
        "status={}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn original() -> Fixture {
    configured(false)
}

fn configured(build_at_export: bool) -> Fixture {
    let mut fixture = Fixture::new();
    fs::write(
        fixture.root.join(".chrono-harness/ci/entry.sh"),
        include_bytes!("support/producer.sh"),
    )
    .unwrap();
    let producer = fixture.tools.join("producer");
    fs::copy(env!("CARGO_BIN_EXE_chrono-test-bootstrap-tool"), &producer).unwrap();
    fixture.environment.insert(
        "CHRONO_TEST_BOOTSTRAP_PRODUCER".into(),
        producer.into_os_string(),
    );
    fixture.environment.insert(
        "CHRONO_TEST_BOOTSTRAP_NATIVE".into(),
        env!("CARGO_BIN_EXE_chrono-test-bootstrap-native").into(),
    );
    fixture.config["tools"]["sh"] = json!("/bin/sh");
    fixture.config["participation"]["git"] = json!("/usr/bin/git");
    let path = fixture.root.join(".chrono-harness/ci/registry.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let native = bindings::native_platform().unwrap();
    registry["platforms"][format!("{}/{}", native.system, native.machine)] =
        registry["platforms"]["fixture/native"].clone();
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
    let mut extra = fixture.config.clone();
    extra["prerequisites"] = json!([]);
    extra["report_path"] = json!(".chrono-harness/state/bootstrap-extra.json");
    extra["install"][0]["destination"] = json!(".chrono-harness/bin/extra");
    fs::write(
        fixture.root.join(".chrono-harness/ci/extra.json"),
        serde_json::to_vec(&extra).unwrap(),
    )
    .unwrap();
    fs::write(
        fixture.root.join(CONFIG),
        serde_json::to_vec(&json!({
            "schema":"chrono-host-startup/v1", "directory":format!("{TRANSFER}/"),
            "profiles":{"core":{"config":".chrono-harness/ci/build.json","build":build_at_export},
                "extra":{"config":".chrono-harness/ci/extra.json","build":false}},
            "consumers":{"unit":["core"]},
            "installer":{"profile":"core","destination":".chrono-harness/bin/cargo"}
        }))
        .unwrap(),
    )
    .unwrap();
    let loaded = fixture.commit();
    if !build_at_export {
        let mut build = Construction::begin(&fixture.root, loaded, &fixture.environment).unwrap();
        build.prerequisites(&mut vec![], &mut vec![]).unwrap();
        build.select().unwrap();
        build.operations(&mut vec![], &mut vec![]).unwrap();
        build
            .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
            .unwrap();
        build.finish(Ok(())).unwrap().outcome.unwrap();
    }
    let loaded = Loaded::read(
        &fixture.root,
        ".chrono-harness/ci/extra.json",
        &native,
        &fixture.environment,
    )
    .unwrap();
    let mut build = Construction::begin(&fixture.root, loaded, &fixture.environment).unwrap();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    build.select().unwrap();
    build.operations(&mut vec![], &mut vec![]).unwrap();
    build
        .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/extra")
        .unwrap();
    build.finish(Ok(())).unwrap().outcome.unwrap();
    fixture
}

fn export(fixture: &Fixture) -> String {
    let mut environment = fixture.environment.clone();
    let output = fixture.root.join(".chrono-harness/state/github-output");
    environment.insert("GITHUB_OUTPUT".into(), output.as_os_str().to_owned());
    passed(cli(&fixture.root, &environment, &["produce"]));
    let output = fs::read_to_string(output).unwrap();
    let outputs: std::collections::BTreeMap<_, _> = output
        .lines()
        .map(|line| line.split_once('=').unwrap())
        .collect();
    assert_eq!(
        outputs["installer_sha256"],
        chrono_harness::file_identity(&fixture.root.join(".chrono-harness/bin/cargo"))
            .unwrap()
            .0
    );
    outputs["binding"].to_owned()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for item in fs::read_dir(source).unwrap() {
        let item = item.unwrap();
        let target = destination.join(item.file_name());
        if item.file_type().unwrap().is_dir() {
            copy_tree(&item.path(), &target);
        } else {
            fs::copy(item.path(), target).unwrap();
        }
    }
}

fn consumer(fixture: &Fixture) -> (tempfile::TempDir, Environment) {
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new("git");
    command
        .args(["clone", "--quiet", "--no-hardlinks"])
        .arg(&fixture.root)
        .arg(directory.path());
    for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        command.env_remove(key);
    }
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    passed(command.output().unwrap());
    copy_tree(
        &fixture.root.join(TRANSFER),
        &directory.path().join(TRANSFER),
    );
    copy_tree(&fixture.tools, &directory.path().join("native tools"));
    let mut environment = fixture.environment.clone();
    let mut search = directory.path().join("native tools").into_os_string();
    search.push(":");
    search.push(environment.get(OsStr::new("PATH")).unwrap());
    environment.insert("PATH".into(), search);
    (directory, environment)
}

#[test]
fn exports_installs_and_verifies_original_bytes_with_relocated_local_sdk() {
    let fixture = original();
    let binding = export(&fixture);
    let manifest = fs::read(fixture.root.join(TRANSFER).join("manifest.json")).unwrap();
    assert_eq!(chrono_harness::sha256(&manifest), binding);
    let descriptor: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(
        descriptor["profiles"]["core"]["files"][0]["path"], "tools/core/cargo",
        "the startup launcher needs an explicit stable payload path"
    );
    assert_eq!(
        fs::read(fixture.root.join(format!(
            ".chrono-harness/state/startup-originals/manifest-{binding}.json"
        )))
        .unwrap(),
        manifest
    );
    let (consumer, mut environment) = consumer(&fixture);
    environment.insert("CHRONO_STARTUP_BINDING".into(), binding.clone().into());
    passed(cli(consumer.path(), &environment, &["install", "unit"]));
    passed(cli(consumer.path(), &environment, &["verify", "core"]));
    assert_eq!(
        fs::read(consumer.path().join(".chrono-harness/bin/cargo")).unwrap(),
        fs::read(fixture.root.join(".chrono-harness/bin/cargo")).unwrap()
    );
    let imported: Value = serde_json::from_slice(
        &fs::read(consumer.path().join(".chrono-harness/state/bootstrap.json")).unwrap(),
    )
    .unwrap();
    let original = fs::read(fixture.root.join(".chrono-harness/state/bootstrap.json")).unwrap();
    assert_eq!(
        imported["provenance"],
        json!({"kind":"shared-startup", "binding":binding, "report_sha256":chrono_harness::sha256(&original)})
    );
    let result: Value = serde_json::from_slice(
        &fs::read(
            consumer
                .path()
                .join(".chrono-harness/state/startup-install.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        result["local_executables"]["core"][0]["path"],
        consumer.path().join("native tools/cargo").to_str().unwrap()
    );
    assert!(
        !cli(consumer.path(), &environment, &["install", "unit"])
            .status
            .success()
    );
    fs::set_permissions(
        consumer.path().join(".chrono-harness/bin/cargo"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(
        !cli(consumer.path(), &environment, &["verify", "core"])
            .status
            .success()
    );
}

#[test]
fn rejects_corrupt_artifact_before_any_installation_write() {
    let fixture = original();
    let binding = export(&fixture);
    for fault in [
        "tool",
        "unselected-tool",
        "stream",
        "report",
        "binding",
        "source",
        "destination",
        "sdk",
    ] {
        let (consumer, mut environment) = consumer(&fixture);
        environment.insert("CHRONO_STARTUP_BINDING".into(), binding.clone().into());
        let manifest: Value = serde_json::from_slice(
            &fs::read(consumer.path().join(TRANSFER).join("manifest.json")).unwrap(),
        )
        .unwrap();
        match fault {
            "tool" => fs::write(
                consumer.path().join(TRANSFER).join(
                    manifest["profiles"]["core"]["files"][0]["path"]
                        .as_str()
                        .unwrap(),
                ),
                b"corrupt",
            )
            .unwrap(),
            "unselected-tool" => fs::write(
                consumer.path().join(TRANSFER).join(
                    manifest["profiles"]["extra"]["files"][0]["path"]
                        .as_str()
                        .unwrap(),
                ),
                b"corrupt",
            )
            .unwrap(),
            "stream" => {
                let report: Value = serde_json::from_slice(
                    &fs::read(consumer.path().join(TRANSFER).join("reports/core.json")).unwrap(),
                )
                .unwrap();
                fs::write(
                    consumer
                        .path()
                        .join(TRANSFER)
                        .join(report["operations"][0]["stdout"]["path"].as_str().unwrap()),
                    b"corrupt",
                )
                .unwrap();
            }
            "report" => fs::write(
                consumer.path().join(TRANSFER).join("reports/core.json"),
                b"{}",
            )
            .unwrap(),
            "binding" => {
                environment.insert("CHRONO_STARTUP_BINDING".into(), "0".repeat(64).into());
            }
            "source" => fs::write(consumer.path().join(CONFIG), b"{}").unwrap(),
            "destination" => {
                fs::create_dir_all(consumer.path().join(".chrono-harness/bin/cargo")).unwrap()
            }
            "sdk" => fs::write(consumer.path().join("native tools/cargo"), b"corrupt").unwrap(),
            _ => unreachable!(),
        }
        let output = cli(consumer.path(), &environment, &["install", "unit"]);
        assert!(!output.status.success(), "accepted {fault}");
        assert!(
            !consumer
                .path()
                .join(".chrono-harness/state/startup-install.json")
                .exists(),
            "published {fault}"
        );
        assert!(
            !consumer
                .path()
                .join(".chrono-harness/state/bootstrap.json")
                .exists(),
            "wrote report for {fault}"
        );
        if fault != "destination" {
            assert!(
                !consumer.path().join(".chrono-harness/bin/cargo").exists(),
                "wrote tool for {fault}"
            );
        }
    }
}

#[test]
fn rebound_metadata_cannot_replace_original_execution_or_declared_payloads() {
    let fixture = original();
    export(&fixture);
    for fault in [
        "platform",
        "destination",
        "payload-path",
        "mode",
        "failed-report",
        "missing-operation",
    ] {
        let (consumer, mut environment) = consumer(&fixture);
        let manifest_path = consumer.path().join(TRANSFER).join("manifest.json");
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        match fault {
            "platform" => manifest["platform"]["machine"] = json!("other"),
            "destination" => {
                manifest["profiles"]["core"]["files"][0]["destination"] =
                    json!(".chrono-harness/bin/other")
            }
            "mode" => manifest["profiles"]["core"]["files"][0]["mode"] = json!(0o644),
            "payload-path" => {
                let file = &mut manifest["profiles"]["core"]["files"][0];
                fs::rename(
                    consumer
                        .path()
                        .join(TRANSFER)
                        .join(file["path"].as_str().unwrap()),
                    consumer.path().join(TRANSFER).join("tools/other"),
                )
                .unwrap();
                file["path"] = json!("tools/other");
            }
            "failed-report" | "missing-operation" => {
                let reference = &mut manifest["profiles"]["core"]["report"];
                let report_path = consumer
                    .path()
                    .join(TRANSFER)
                    .join(reference["path"].as_str().unwrap());
                let mut report: Value =
                    serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
                if fault == "failed-report" {
                    report["status"] = json!("failed");
                } else {
                    report["operations"].as_array_mut().unwrap().pop();
                }
                let bytes = serde_json::to_vec(&report).unwrap();
                reference["sha256"] = json!(chrono_harness::sha256(&bytes));
                fs::write(report_path, bytes).unwrap();
            }
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&manifest).unwrap();
        environment.insert(
            "CHRONO_STARTUP_BINDING".into(),
            chrono_harness::sha256(&bytes).into(),
        );
        fs::write(&manifest_path, bytes).unwrap();
        let output = cli(consumer.path(), &environment, &["install", "unit"]);
        assert!(!output.status.success(), "accepted rebound {fault}");
        assert!(
            !consumer.path().join(".chrono-harness/bin/cargo").exists(),
            "installed rebound {fault}"
        );
        assert!(
            !consumer
                .path()
                .join(".chrono-harness/state/startup-install.json")
                .exists(),
            "published rebound {fault}"
        );
    }
}

#[test]
fn verifier_rejects_installed_and_local_sdk_drift_without_rewriting_originals() {
    let fixture = original();
    let binding = export(&fixture);
    for fault in [
        "bytes",
        "report",
        "stream",
        "sdk-bytes",
        "sdk-path",
        "compiler",
        "wrapper",
        "workspace-wrapper",
    ] {
        let (consumer, mut environment) = consumer(&fixture);
        environment.insert("CHRONO_STARTUP_BINDING".into(), binding.clone().into());
        passed(cli(consumer.path(), &environment, &["install", "unit"]));
        let receipt = consumer
            .path()
            .join(".chrono-harness/state/startup-install.json");
        let before = fs::read(&receipt).unwrap();
        let original = consumer
            .path()
            .join(".chrono-harness/state/startup-originals/core.json");
        let original_bytes = fs::read(&original).unwrap();
        let alternate = consumer.path().join("alternate tools");
        match fault {
            "bytes" => {
                fs::write(consumer.path().join(".chrono-harness/bin/cargo"), b"drift").unwrap()
            }
            "report" => fs::write(
                consumer.path().join(".chrono-harness/state/bootstrap.json"),
                b"{}",
            )
            .unwrap(),
            "stream" => {
                let report: Value = serde_json::from_slice(&original_bytes).unwrap();
                fs::write(
                    consumer
                        .path()
                        .join(report["operations"][0]["stdout"]["path"].as_str().unwrap()),
                    b"drift",
                )
                .unwrap();
            }
            "sdk-bytes" => fs::write(consumer.path().join("native tools/cargo"), b"drift").unwrap(),
            "sdk-path" => {
                copy_tree(&consumer.path().join("native tools"), &alternate);
                let mut search = alternate.as_os_str().to_owned();
                search.push(":");
                search.push(environment.get(OsStr::new("PATH")).unwrap());
                environment.insert("PATH".into(), search);
            }
            "compiler" | "wrapper" | "workspace-wrapper" => {
                copy_tree(&consumer.path().join("native tools"), &alternate);
                let key = match fault {
                    "compiler" => "RUSTC",
                    "wrapper" => "RUSTC_WRAPPER",
                    _ => "RUSTC_WORKSPACE_WRAPPER",
                };
                environment.insert(key.into(), alternate.join("rustc").into_os_string());
            }
            _ => unreachable!(),
        }
        let output = cli(consumer.path(), &environment, &["verify", "core"]);
        assert!(!output.status.success(), "accepted drift {fault}");
        assert_eq!(
            fs::read(&receipt).unwrap(),
            before,
            "rewrote receipt for {fault}"
        );
        assert_eq!(
            fs::read(&original).unwrap(),
            original_bytes,
            "rewrote original for {fault}"
        );
    }
}

#[test]
fn produce_builds_only_declared_profiles_and_keeps_originals_on_output_failure() {
    let fixture = configured(true);
    assert!(
        !fixture
            .root
            .join(".chrono-harness/state/bootstrap.json")
            .exists()
    );
    let extra = fs::read(
        fixture
            .root
            .join(".chrono-harness/state/bootstrap-extra.json"),
    )
    .unwrap();
    let binding = export(&fixture);
    assert!(fixture.root.join("prepared").exists());
    assert!(
        fixture
            .root
            .join(".chrono-harness/state/bootstrap.json")
            .is_file()
    );
    assert_eq!(
        fs::read(
            fixture
                .root
                .join(".chrono-harness/state/bootstrap-extra.json")
        )
        .unwrap(),
        extra
    );
    assert_eq!(binding.len(), 64);
    let calls: Vec<Value> =
        serde_json::from_slice(&fs::read(fixture.tools.join("calls.json")).unwrap()).unwrap();
    assert_eq!(
        calls
            .iter()
            .filter(|row| row[0] == "producer")
            .collect::<Vec<_>>(),
        vec![&json!([
            "producer",
            fixture.root,
            ".chrono-harness/ci/build.json"
        ])]
    );

    let fixture = original();
    let mut environment = fixture.environment.clone();
    environment.insert("GITHUB_OUTPUT".into(), fixture.root.as_os_str().to_owned());
    assert!(
        !cli(&fixture.root, &environment, &["produce"])
            .status
            .success()
    );
    let manifest = fs::read(fixture.root.join(TRANSFER).join("manifest.json")).unwrap();
    let binding = chrono_harness::sha256(&manifest);
    assert_eq!(
        fs::read(fixture.root.join(format!(
            ".chrono-harness/state/startup-originals/manifest-{binding}.json"
        )))
        .unwrap(),
        manifest
    );
}

#[test]
fn registered_producer_failure_keeps_raw_streams_exit_and_unpublished_transfer() {
    for code in [37, 0] {
        let fixture = configured(true);
        let mut environment = fixture.environment.clone();
        environment.insert("CHRONO_TEST_PRODUCER_EXIT".into(), code.to_string().into());
        let output = cli(&fixture.root, &environment, &["produce"]);
        assert_eq!(output.status.code(), Some(if code == 0 { 1 } else { code }));
        assert_eq!(output.stdout, b"producer stdout\x00\xff\n");
        assert!(output.stderr.starts_with(b"producer stderr\xfe\n"));
        assert!(!fixture.root.join(TRANSFER).exists());
        assert!(
            !fixture
                .root
                .join(".chrono-harness/state/bootstrap.json")
                .exists()
        );
        assert!(
            fixture
                .root
                .join(".chrono-harness/state/bootstrap-extra.json")
                .is_file()
        );
    }
}

#[test]
fn invalid_registration_and_dirty_source_fail_before_transfer_publication() {
    let fixture = original();
    let raw = fs::read(fixture.root.join(CONFIG)).unwrap();
    let original: Value = serde_json::from_slice(&raw).unwrap();
    let extra_path = fixture.root.join(".chrono-harness/ci/extra.json");
    let extra_raw = fs::read(&extra_path).unwrap();
    for (change, expected) in [
        ("directory", "overlaps"),
        ("duplicate-destination", "separate tools"),
        ("duplicate-report", "separate reports"),
        ("unknown-profile", "explicit unique"),
        ("duplicate-profile", "explicit unique"),
        ("profile-name", "profile name"),
        ("unknown-field", "unknown field"),
        ("installer", "explicitly installed"),
        ("installer-profile", "explicitly installed"),
        ("missing-installer-profile", "explicit unique"),
    ] {
        let mut config = original.clone();
        match change {
            "directory" => config["directory"] = json!(".chrono-harness/state/transfer/"),
            "duplicate-destination" => {
                let mut extra: Value = serde_json::from_slice(&extra_raw).unwrap();
                extra["install"][0]["destination"] = json!(".chrono-harness/bin/cargo");
                fs::write(&extra_path, serde_json::to_vec(&extra).unwrap()).unwrap();
            }
            "duplicate-report" => {
                config["profiles"]["extra"]["config"] = json!(".chrono-harness/ci/build.json")
            }
            "unknown-profile" => config["consumers"]["unit"] = json!(["absent"]),
            "duplicate-profile" => config["consumers"]["unit"] = json!(["core", "core"]),
            "profile-name" => {
                config["profiles"]["bad/name"] = config["profiles"]["core"].clone();
            }
            "unknown-field" => config["unknown"] = json!(true),
            "installer" => config["installer"]["destination"] = json!(".chrono-harness/bin/absent"),
            "installer-profile" => config["installer"]["profile"] = json!("absent"),
            "missing-installer-profile" => config["consumers"]["unit"] = json!(["extra"]),
            _ => unreachable!(),
        }
        fs::write(
            fixture.root.join(CONFIG),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let error = chrono_host_bootstrap::shared::Shared::read(
            &fixture.root,
            CONFIG,
            &bindings::native_platform().unwrap(),
        )
        .err()
        .unwrap_or_else(|| panic!("accepted {change}"));
        assert!(error.contains(expected), "{change}: {error}");
        assert!(!fixture.root.join(TRANSFER).exists(), "published {change}");
        fs::write(&extra_path, &extra_raw).unwrap();
    }
    fs::write(fixture.root.join(CONFIG), raw).unwrap();
    fs::write(
        fixture.root.join(".chrono-harness/ci/entry.sh"),
        b"dirty source",
    )
    .unwrap();
    let mut environment = fixture.environment.clone();
    environment.insert(
        "GITHUB_OUTPUT".into(),
        fixture
            .root
            .join(".chrono-harness/state/output")
            .into_os_string(),
    );
    let output = cli(&fixture.root, &environment, &["produce"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires clean source"));
    assert!(!fixture.root.join(TRANSFER).exists());
}
