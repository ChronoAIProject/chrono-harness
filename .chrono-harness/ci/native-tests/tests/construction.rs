#[path = "support/construction.rs"]
mod support;
use serde_json::{Value, json};
use std::{fs, process::Command};
use support::Fixture;

#[test]
fn native_entry_preserves_private_installer_exit_and_failed_original_without_building_or_installing()
 {
    let mut fixture = Fixture::new();
    let native = chrono_host_bootstrap::bindings::native_platform().unwrap();
    let registry = fixture.root.join(".chrono-harness/ci/registry.json");
    let mut bindings: Value = serde_json::from_slice(&fs::read(&registry).unwrap()).unwrap();
    bindings["platforms"][format!("{}/{}", native.system, native.machine)] =
        bindings["platforms"]["fixture/native"].clone();
    fs::write(registry, serde_json::to_vec(&bindings).unwrap()).unwrap();
    fixture.config["prerequisites"] = json!([]);
    fixture.config["toolchain_provision"] = json!({"profile":"default",
        "rustup_home":".chrono-harness/state/private/rustup", "cargo_home":".chrono-harness/state/private/cargo"});
    fixture.loaded();
    fs::write(fixture.tools.join("fail-install"), b"").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
    command
        .current_dir(&fixture.root)
        .env_clear()
        .envs(fixture.environment.iter().filter(|(key, _)| {
            key.as_os_str() != std::ffi::OsStr::new(chrono_harness::process_fds::ENV)
        }))
        .env("CHRONO_WORKTREE_BOOTSTRAP", &fixture.root)
        .args(["bootstrap", ".", ".chrono-harness/ci/build.json"]);
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let result = command.output().unwrap();
    assert_eq!(
        result.status.code(),
        Some(17),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let calls: Value =
        serde_json::from_slice(&fs::read(fixture.tools.join("calls.json")).unwrap()).unwrap();
    assert_eq!(
        calls,
        json!([[
            "rustup",
            "toolchain",
            "install",
            "1.95.0",
            "--profile",
            "default",
            "--no-self-update"
        ]])
    );
    let stderr = String::from_utf8(result.stderr).unwrap();
    let path = stderr
        .lines()
        .find_map(|line| line.strip_prefix("bootstrap original report: "))
        .unwrap();
    let report: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(report["status"], "failed");
    assert_eq!(report["operations"], json!([]));
    assert_eq!(report["installed"], json!([]));
    assert_eq!(report["versions"], json!({}));
    assert!(report["environment"].as_object().unwrap().is_empty());
    assert!(
        !fixture
            .root
            .join(".chrono-harness/state/bootstrap.json")
            .exists()
    );
    assert!(!fixture.root.join("out/cargo").exists());
    assert!(!fixture.root.join(".chrono-harness/bin/cargo").exists());
    let traces = fs::read_to_string(fixture.tools.join("environments.jsonl")).unwrap();
    let values: Vec<Value> = traces
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values.len(), 1);
    for (key, suffix) in [("RUSTUP_HOME", "rustup"), ("CARGO_HOME", "cargo")] {
        assert_eq!(
            values[0]["environment"][key],
            fixture
                .root
                .join(format!(".chrono-harness/state/private/{suffix}"))
                .to_str()
                .unwrap()
        );
    }
}

#[test]
fn construction_preserves_order_selected_tools_installation_and_original_report() {
    let fixture = Fixture::new();
    let mut build = fixture.committed();
    let mut stdout = vec![];
    let mut stderr = vec![];
    build.prerequisites(&mut stdout, &mut stderr).unwrap();
    assert_eq!(fs::read(fixture.root.join("prepared")).unwrap(), b"ok");
    assert!(!fixture.tools.join("calls.json").exists());
    build.select().unwrap();
    build.operations(&mut stdout, &mut stderr).unwrap();
    let compiler: Value =
        serde_json::from_slice(&fs::read(fixture.root.join("out/cargo.selection.json")).unwrap())
            .unwrap();
    assert_eq!(
        compiler["compiler"],
        fixture.tools.join("rustc").to_str().unwrap()
    );
    assert_eq!(compiler["wrapper"], "");
    assert_eq!(compiler["workspace_wrapper"], "");
    build
        .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
        .unwrap();
    let published = build.finish(Ok(())).unwrap();
    assert!(published.outcome.is_ok());
    let report = fixture.report(&published);
    assert_eq!(report["status"], "passed");
    assert_eq!(report["source"]["state"], "clean");
    assert_eq!(report["source"]["before"], report["source"]["after"]);
    assert_eq!(
        report["source"]["commit"],
        report["source"]["before"]["commit"]
    );
    assert_eq!(report["operations"][0]["operation"], "prepare");
    assert_eq!(report["operations"][1]["operation"], "build");
    assert_eq!(report["selection"]["compiler"], compiler["compiler"]);
    assert_eq!(report["environment"]["CARGO_INCREMENTAL"], "0");
    assert_eq!(
        fs::read(fixture.root.join(".chrono-harness/state/bootstrap.json")).unwrap(),
        fs::read(&published.original).unwrap()
    );
    for row in report["operations"].as_array().unwrap() {
        for channel in ["stdout", "stderr"] {
            let original =
                fs::read(fixture.root.join(row[channel]["path"].as_str().unwrap())).unwrap();
            assert_eq!(row[channel]["sha256"], chrono_harness::sha256(&original));
            assert_eq!(row[channel]["size"], original.len());
        }
    }
    assert!(stdout.starts_with(b"pre\n"));
    assert!(stdout.ends_with(b"cargo original stdout\n"));
    assert_eq!(stderr, b"cargo original stderr\n");
    let installed = fixture.root.join(".chrono-harness/bin/cargo");
    assert_eq!(
        report["installed"][0]["sha256"],
        chrono_harness::file_identity(&installed).unwrap().0
    );
    let mut command = Command::new(installed);
    command.arg("--version").env("RUSTUP_TOOLCHAIN", "1.95.0");
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let consumer = command.output().unwrap();
    assert!(consumer.status.success());
    assert_eq!(consumer.stdout, b"cargo 1.95.0 (fixture)\n");
}

#[test]
fn failed_prerequisite_keeps_original_exit_and_streams_without_selecting_sdk_or_creating_build_homes()
 {
    let mut fixture = Fixture::new();
    fixture.failed_operation("prepare", 29);
    fixture.config["host_environment"] = json!({"HOME":".chrono-harness/cache/build-home"});
    let mut build = fixture.begin();
    let mut stdout = vec![];
    let mut stderr = vec![];
    let failure = build.prerequisites(&mut stdout, &mut stderr).unwrap_err();
    assert_eq!(failure.exit_code, Some(29));
    assert!(
        build
            .select()
            .unwrap_err()
            .message
            .contains("have not succeeded")
    );
    let published = build.finish(Err(failure)).unwrap();
    let report = fixture.report(&published);
    assert_eq!(published.outcome.unwrap_err().exit_code, Some(29));
    assert_eq!(report["status"], "failed");
    assert_eq!(report["operations"][0]["exit_code"], 29);
    assert_eq!(stdout, [0, 255, 10]);
    assert_eq!(stderr, [254, 0, 10]);
    assert!(!fixture.tools.join("calls.json").exists());
    assert!(
        !fixture
            .root
            .join(".chrono-harness/cache/build-home")
            .exists()
    );
    assert!(!fixture.root.join(".chrono-harness/bin/cargo").exists());
}

#[test]
fn failed_build_keeps_old_selected_report_and_original_non_utf8_process_before_installation() {
    let mut fixture = Fixture::new();
    fixture.failed_operation("build", 23);
    let mut build = fixture.begin();
    fs::write(
        fixture.root.join(".chrono-harness/state/bootstrap.json"),
        b"previous original\n",
    )
    .unwrap();
    let mut stdout = vec![];
    let mut stderr = vec![];
    build.prerequisites(&mut stdout, &mut stderr).unwrap();
    build.select().unwrap();
    let failure = build.operations(&mut stdout, &mut stderr).unwrap_err();
    assert_eq!(failure.exit_code, Some(23));
    let published = build.finish(Err(failure)).unwrap();
    let report = fixture.report(&published);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["operations"][1]["exit_code"], 23);
    assert_eq!(
        fs::read(
            fixture
                .root
                .join(report["operations"][1]["stdout"]["path"].as_str().unwrap())
        )
        .unwrap(),
        [0, 255, 10]
    );
    assert_eq!(
        fs::read(fixture.root.join(".chrono-harness/state/bootstrap.json")).unwrap(),
        b"previous original\n"
    );
    assert!(!fixture.root.join(".chrono-harness/bin/cargo").exists());
}

#[test]
fn incomplete_construction_cannot_publish_success_even_if_the_caller_returns_ok() {
    let fixture = Fixture::new();
    let published = fixture.begin().finish(Ok(())).unwrap();
    assert!(
        published
            .outcome
            .as_ref()
            .unwrap_err()
            .message
            .contains("incomplete")
    );
    let report = fixture.report(&published);
    assert_eq!(report["status"], "failed");
    assert!(
        !fixture
            .root
            .join(".chrono-harness/state/bootstrap.json")
            .exists()
    );
    assert!(!fixture.tools.join("calls.json").exists());
    let mut build = fixture.begin();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    build.select().unwrap();
    build.operations(&mut vec![], &mut vec![]).unwrap();
    let published = build.finish(Ok(())).unwrap();
    assert!(published.outcome.is_err());
    assert_eq!(fixture.report(&published)["status"], "failed");
    assert!(!fixture.root.join(".chrono-harness/bin/cargo").exists());
}

#[test]
fn installer_failure_and_secondary_report_io_failure_preserve_the_actual_original_exit() {
    let mut fixture = Fixture::new();
    fixture.config["toolchain_provision"] = json!({"profile":"default","rustup_home":".chrono-harness/state/sdk/rustup","cargo_home":".chrono-harness/state/sdk/cargo"});
    fs::write(fixture.tools.join("fail-install"), b"").unwrap();
    let mut build = fixture.begin();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    let failure = build.select().unwrap_err();
    assert_eq!(failure.exit_code, Some(17));
    fs::create_dir(build.directory.join("report.json")).unwrap();
    let error = match build.finish(Err(failure)) {
        Err(error) => error,
        Ok(_) => panic!("report IO failure was accepted"),
    };
    assert_eq!(error.exit_code, Some(17));
    assert!(error.message.contains("Rustup failed"));
    assert!(error.message.contains("cannot publish bootstrap report"));
}
