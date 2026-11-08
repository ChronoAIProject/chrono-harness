#[path = "support/construction.rs"]
mod support;
use chrono_harness::retained_artifacts::retention::{self, Inventory};
use serde_json::{Value, json};
use std::{fs, process::Command};
use support::Fixture;
#[path = "../../../../crates/runner-tests/tests/support/isolated_capability.rs"]
mod isolated_capability;
use isolated_capability::isolated_capability_test;

fn successful_build(fixture: &Fixture) -> chrono_host_bootstrap::construction::Published {
    if fixture.root.join("prepared").exists() {
        fs::remove_file(fixture.root.join("prepared")).unwrap();
    }
    let mut build = fixture.begin();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    build.select().unwrap();
    build.operations(&mut vec![], &mut vec![]).unwrap();
    build
        .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
        .unwrap();
    let published = build.finish(Ok(())).unwrap();
    assert!(published.outcome.is_ok());
    published
}

fn maintain(fixture: &Fixture) {
    let inventory = Inventory::adopted(&fixture.root).unwrap().unwrap();
    for _ in 0..4 {
        let result = inventory.maintain().unwrap();
        assert_eq!(result["accounting_errors"], json!([]), "{result}");
    }
}

#[test]
fn adopted_bootstrap_rotates_whole_directories_after_current_report_and_readers_release() {
    if !isolated_capability_test(
        "adopted_bootstrap_rotates_whole_directories_after_current_report_and_readers_release",
    ) {
        return;
    }
    let fixture = Fixture::new();
    fixture.adopt_retention(0);
    // 独立保留宿主不声明 Git 所有者；模拟源码身份由源码专属用例验证。
    let first = successful_build(&fixture);
    let first_path = first
        .original
        .strip_prefix(&fixture.root)
        .unwrap()
        .to_str()
        .unwrap();
    let reader = retention::read_guard(&fixture.root, first_path).unwrap();
    assert!(
        reader.is_some(),
        "bootstrap original directory was not adopted"
    );
    let second = successful_build(&fixture);
    let original = fs::read(&second.original).unwrap();
    maintain(&fixture);
    assert!(first.original.exists());
    drop(reader);
    maintain(&fixture);
    assert!(!first.original.parent().unwrap().exists());
    assert_eq!(fs::read(&second.original).unwrap(), original);
    assert_eq!(
        fs::read(fixture.root.join(".chrono-harness/state/bootstrap.json")).unwrap(),
        original
    );
    for row in &second.report.operations {
        for stream in [&row.stdout, &row.stderr] {
            assert_eq!(
                chrono_harness::file_identity(&fixture.root.join(&stream.path)).unwrap(),
                (stream.sha256.clone(), stream.size)
            );
        }
    }
}

#[test]
fn adopted_bootstrap_keeps_reused_streams_until_their_last_report_retires() {
    if !isolated_capability_test(
        "adopted_bootstrap_keeps_reused_streams_until_their_last_report_retires",
    ) {
        return;
    }
    let fixture = Fixture::new();
    fixture.adopt_retention(0);
    let first = successful_build(&fixture);
    fs::remove_file(fixture.root.join("prepared")).unwrap();
    let mut build = fixture.begin();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    build.select().unwrap();
    build.operations(&mut vec![], &mut vec![]).unwrap();
    build
        .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
        .unwrap();
    // The actual reusable operation carries the first producer's exact streams.
    build.report.operations[0] = first.report.operations[0].clone();
    let second = build.finish(Ok(())).unwrap();
    assert!(second.outcome.is_ok());
    maintain(&fixture);
    assert!(first.original.exists());
    let third = successful_build(&fixture);
    maintain(&fixture);
    assert!(!first.original.parent().unwrap().exists());
    assert!(!second.original.parent().unwrap().exists());
    assert!(third.original.exists());
}

#[test]
fn adopted_bootstrap_retains_failed_and_interrupted_originals_for_diagnosis() {
    if !isolated_capability_test(
        "adopted_bootstrap_retains_failed_and_interrupted_originals_for_diagnosis",
    ) {
        return;
    }
    let mut fixture = Fixture::new();
    fixture.adopt_retention(3600);
    fixture.failed_operation("prepare", 29);
    let mut build = fixture.begin();
    let error = build.prerequisites(&mut vec![], &mut vec![]).unwrap_err();
    let published = build.finish(Err(error)).unwrap();
    assert_eq!(published.outcome.unwrap_err().exit_code, Some(29));
    let path = published
        .original
        .strip_prefix(&fixture.root)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        retention::read_guard(&fixture.root, path)
            .unwrap()
            .is_some()
    );
    let build = fixture.begin();
    let interrupted = build.directory.clone();
    maintain(&fixture);
    assert!(interrupted.exists());
    drop(build);
    maintain(&fixture);
    assert!(interrupted.exists());
    assert!(published.original.exists());
    assert!(
        !fixture
            .root
            .join(".chrono-harness/state/bootstrap.json")
            .exists()
    );
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
