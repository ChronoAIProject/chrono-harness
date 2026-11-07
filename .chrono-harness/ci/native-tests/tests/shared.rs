#[path = "support/construction.rs"]
mod support;
use chrono_host_bootstrap::bindings::{self, Environment};
use serde_json::{Value, json};
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt, path::Path, process::{Command, Output}};
use support::Fixture;

const CONFIG: &str = ".chrono-harness/ci/shared.json";
const TRANSFER: &str = ".chrono-harness/cache/startup";

fn cli(root: &Path, environment: &Environment, args: &[&str]) -> Output {
    let mut command = bindings::command(root,
        Path::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native")), &[], environment).unwrap();
    command.arg(root).arg(CONFIG).args(args).output().unwrap()
}

fn passed(output: Output) {
    assert!(output.status.success(), "status={}\nstdout={}\nstderr={}", output.status,
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

fn original() -> Fixture {
    let fixture = Fixture::new();
    let path = fixture.root.join(".chrono-harness/ci/registry.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let native = bindings::native_platform().unwrap();
    registry["platforms"][format!("{}/{}", native.system, native.machine)] = registry["platforms"]["fixture/native"].clone();
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
    fs::write(fixture.root.join(CONFIG), serde_json::to_vec(&json!({
        "schema":"chrono-host-startup/v1", "directory":format!("{TRANSFER}/"),
        "profiles":{"core":{"config":".chrono-harness/ci/build.json","build":false}},
        "consumers":{"unit":["core"]}
    })).unwrap()).unwrap();
    let mut build = fixture.committed();
    build.prerequisites(&mut vec![], &mut vec![]).unwrap();
    build.select().unwrap();
    build.operations(&mut vec![], &mut vec![]).unwrap();
    build.install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo").unwrap();
    build.finish(Ok(())).unwrap().outcome.unwrap();
    fixture
}

fn export(fixture: &Fixture) -> String {
    let mut environment = fixture.environment.clone();
    let output = fixture.root.join(".chrono-harness/state/github-output");
    environment.insert("GITHUB_OUTPUT".into(), output.as_os_str().to_owned());
    passed(cli(&fixture.root, &environment, &["produce"]));
    let output = fs::read_to_string(output).unwrap();
    output.trim().strip_prefix("binding=").unwrap().to_owned()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for item in fs::read_dir(source).unwrap() {
        let item = item.unwrap();
        let target = destination.join(item.file_name());
        if item.file_type().unwrap().is_dir() { copy_tree(&item.path(), &target); }
        else { fs::copy(item.path(), target).unwrap(); }
    }
}

fn consumer(fixture: &Fixture) -> (tempfile::TempDir, Environment) {
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new("git");
    command.args(["clone", "--quiet", "--no-hardlinks"]).arg(&fixture.root).arg(directory.path());
    for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] { command.env_remove(key); }
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    passed(command.output().unwrap());
    copy_tree(&fixture.root.join(TRANSFER), &directory.path().join(TRANSFER));
    copy_tree(&fixture.tools, &directory.path().join("native tools"));
    let mut environment = fixture.environment.clone();
    let mut search = directory.path().join("native tools").into_os_string();
    search.push(":"); search.push(environment.get(OsStr::new("PATH")).unwrap());
    environment.insert("PATH".into(), search);
    (directory, environment)
}

#[test]
fn exports_installs_and_verifies_original_bytes_with_relocated_local_sdk() {
    let fixture = original();
    let binding = export(&fixture);
    let manifest = fs::read(fixture.root.join(TRANSFER).join("manifest.json")).unwrap();
    assert_eq!(chrono_harness::sha256(&manifest), binding);
    assert_eq!(fs::read(fixture.root.join(format!(".chrono-harness/state/startup-originals/manifest-{binding}.json"))).unwrap(), manifest);
    let (consumer, mut environment) = consumer(&fixture);
    environment.insert("CHRONO_STARTUP_BINDING".into(), binding.clone().into());
    passed(cli(consumer.path(), &environment, &["install", "unit"]));
    passed(cli(consumer.path(), &environment, &["verify", "core"]));
    assert_eq!(fs::read(consumer.path().join(".chrono-harness/bin/cargo")).unwrap(), fs::read(fixture.root.join(".chrono-harness/bin/cargo")).unwrap());
    let imported: Value = serde_json::from_slice(&fs::read(consumer.path().join(".chrono-harness/state/bootstrap.json")).unwrap()).unwrap();
    let original = fs::read(fixture.root.join(".chrono-harness/state/bootstrap.json")).unwrap();
    assert_eq!(imported["provenance"], json!({"kind":"shared-startup", "binding":binding, "report_sha256":chrono_harness::sha256(&original)}));
    let result: Value = serde_json::from_slice(&fs::read(consumer.path().join(".chrono-harness/state/startup-install.json")).unwrap()).unwrap();
    assert_eq!(result["local_executables"]["core"][0]["path"], consumer.path().join("native tools/cargo").to_str().unwrap());
    assert!(!cli(consumer.path(), &environment, &["install", "unit"]).status.success());
    fs::set_permissions(consumer.path().join(".chrono-harness/bin/cargo"), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!cli(consumer.path(), &environment, &["verify", "core"]).status.success());
}

#[test]
fn rejects_corrupt_artifact_before_any_installation_write() {
    let fixture = original();
    let binding = export(&fixture);
    for fault in ["tool", "stream", "report", "binding", "source", "destination", "sdk"] {
        let (consumer, mut environment) = consumer(&fixture);
        environment.insert("CHRONO_STARTUP_BINDING".into(), binding.clone().into());
        let manifest: Value = serde_json::from_slice(&fs::read(consumer.path().join(TRANSFER).join("manifest.json")).unwrap()).unwrap();
        match fault {
            "tool" => fs::write(consumer.path().join(TRANSFER).join(manifest["profiles"]["core"]["files"][0]["path"].as_str().unwrap()), b"corrupt").unwrap(),
            "stream" => {
                let report: Value = serde_json::from_slice(&fs::read(consumer.path().join(TRANSFER).join("reports/core.json")).unwrap()).unwrap();
                fs::write(consumer.path().join(TRANSFER).join(report["operations"][0]["stdout"]["path"].as_str().unwrap()), b"corrupt").unwrap();
            }
            "report" => fs::write(consumer.path().join(TRANSFER).join("reports/core.json"), b"{}").unwrap(),
            "binding" => { environment.insert("CHRONO_STARTUP_BINDING".into(), "0".repeat(64).into()); }
            "source" => fs::write(consumer.path().join(CONFIG), b"{}").unwrap(),
            "destination" => fs::create_dir_all(consumer.path().join(".chrono-harness/bin/cargo")).unwrap(),
            "sdk" => fs::write(consumer.path().join("native tools/cargo"), b"corrupt").unwrap(),
            _ => unreachable!(),
        }
        let output = cli(consumer.path(), &environment, &["install", "unit"]);
        assert!(!output.status.success(), "accepted {fault}");
        assert!(!consumer.path().join(".chrono-harness/state/startup-install.json").exists(), "published {fault}");
        assert!(!consumer.path().join(".chrono-harness/state/bootstrap.json").exists(), "wrote report for {fault}");
        if fault != "destination" { assert!(!consumer.path().join(".chrono-harness/bin/cargo").exists(), "wrote tool for {fault}"); }
    }
}
