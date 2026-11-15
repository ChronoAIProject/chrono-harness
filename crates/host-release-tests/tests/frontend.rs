//! 真实发布工具消费与外部 bootstrap 报告样本；样本不冒充原生构建。
#[path = "support/frontend.rs"]
mod fixture;
use fixture::{Fixture, recipe};
use serde_json::{Value, json};
use std::{fs, os::fd::AsFd, os::unix::fs::PermissionsExt, path::Path};

#[test]
fn actual_export_preserves_original_bootstrap_streams_and_the_executable_archive() {
    let f = Fixture::new();
    let destination = f.output.path().join("export");
    let out = f.export(&destination);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(destination.join("frontend.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema"], "chrono-release-frontend/v1");
    assert_eq!(
        manifest["lineage"],
        json!({"kind":"github","run_id":"1234","attempt":"1","job":"native_frontend"})
    );
    assert_eq!(
        fs::read(destination.join("bootstrap.json")).unwrap(),
        fs::read(
            f.host
                .root()
                .join(".chrono-harness/state/frontend-bootstrap.json")
        )
        .unwrap()
    );
    for name in ["frontend.stdout", "frontend.stderr"] {
        let path = format!(".chrono-harness/state/{name}");
        assert_eq!(
            fs::read(destination.join(&path)).unwrap(),
            fs::read(f.host.root().join(&path)).unwrap()
        );
    }
    let mut tar = tar::Archive::new(fs::File::open(destination.join("executable.tar")).unwrap());
    let mut members = tar.entries().unwrap();
    let mut member = members.next().unwrap().unwrap();
    assert_eq!(member.path().unwrap(), Path::new("executable"));
    assert_eq!(member.header().mode().unwrap(), 0o755);
    let mut bytes = vec![];
    std::io::Read::read_to_end(&mut member, &mut bytes).unwrap();
    assert_eq!(
        bytes,
        fs::read(
            f.host
                .root()
                .join(".chrono-harness/bin/chrono-host-release")
        )
        .unwrap()
    );
    assert!(members.next().is_none());
    assert!(!f.export(&destination).status.success());
}

#[test]
fn export_rejects_original_build_failure_source_drift_and_wrong_installations_before_output() {
    let mut f = Fixture::new();
    let original = f.report.clone();
    for (index, (pointer, value)) in [
        ("/status", json!("failed")),
        ("/source/before/dirty", json!(true)),
        ("/source/commit", json!("0".repeat(40))),
        ("/operations/0/exit_code", json!(41)),
        ("/operations/0/argv/1", json!("unregistered-build")),
        ("/installed/0/sha256", json!("0".repeat(64))),
        ("/installed/0/path", json!(".chrono-harness/bin/other")),
        ("/installed", json!([])),
        (
            "/installed",
            json!([
                original["installed"][0].clone(),
                original["installed"][0].clone()
            ]),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        f.report = original.clone();
        *f.report.pointer_mut(pointer).unwrap() = value;
        f.save();
        let directory = f.output.path().join(format!("rejected-{index}"));
        let out = f.export(&directory);
        assert_eq!(out.status.code(), Some(2), "{pointer}");
        assert!(!directory.exists(), "{pointer}");
    }
}

#[test]
fn consumer_and_collector_validate_original_production_without_rerunning_bootstrap_tools() {
    let f = Fixture::new();
    let directory = f.exported();
    let binary = directory.join("chrono-host-release");
    let selected = f.selected();
    assert!(f.validate(&directory, &selected, Some(&binary)).is_ok());
    assert!(f.validate(&directory, &selected, None).is_ok());
    for (field, value) in [
        ("job", "unregistered"),
        ("run_id", "foreign"),
        ("attempt", "2"),
    ] {
        let mut changed = serde_json::to_value(&selected).unwrap();
        changed[field] = json!(value);
        assert!(
            f.validate(
                &directory,
                &serde_json::from_value(changed).unwrap(),
                Some(&binary)
            )
            .is_err(),
            "{field}"
        );
    }
    // 外部报告中的编译器路径不存在；成功消费不能执行 producer 的工具。
    assert!(!Path::new("/fixture/cargo").exists());
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o744)).unwrap();
    assert!(f.validate(&directory, &selected, Some(&binary)).is_err());
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let source = f
        .host
        .root()
        .join(".chrono-harness/bin/chrono-host-release");
    assert!(f.validate(&directory, &selected, Some(&source)).is_err());
    fs::write(&binary, b"substituted frontend").unwrap();
    assert!(f.validate(&directory, &selected, Some(&binary)).is_err());
}

#[test]
fn consumer_rejects_relabelled_source_platform_role_and_producer() {
    let f = Fixture::new();
    let directory = f.exported();
    let original: Value =
        serde_json::from_slice(&fs::read(directory.join("frontend.json")).unwrap()).unwrap();
    for (pointer, value) in [
        ("/schema", json!("other")),
        ("/source/commit", json!("0".repeat(40))),
        ("/source/tree", json!("0".repeat(40))),
        ("/source/dirty", json!(true)),
        ("/source/error", json!("failed source read")),
        ("/system", json!("foreign")),
        ("/machine", json!("foreign")),
        ("/lineage/kind", json!("local")),
        ("/lineage/job", json!("foreign")),
        ("/lineage/run_id", json!("foreign")),
        ("/lineage/attempt", json!("2")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        recipe::write(&directory, "frontend.json", &changed);
        assert!(
            f.validate(&directory, &f.selected(), None).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn consumer_rejects_changed_bootstrap_streams_and_even_rebound_invalid_original_reports() {
    let f = Fixture::new();
    let directory = f.exported();
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("frontend.json")).unwrap()).unwrap();
    let bootstrap = fs::read(directory.join("bootstrap.json")).unwrap();
    let path = directory.join(".chrono-harness/state/frontend.stdout");
    let stream = fs::read(&path).unwrap();
    fs::write(&path, b"changed original stream").unwrap();
    assert!(f.validate(&directory, &f.selected(), None).is_err());
    fs::write(&path, stream).unwrap();
    for (pointer, value) in [
        ("/operations/0/exit_code", json!(73)),
        ("/installed/0/sha256", json!("0".repeat(64))),
    ] {
        let mut changed: Value = serde_json::from_slice(&bootstrap).unwrap();
        *changed.pointer_mut(pointer).unwrap() = value;
        let raw = serde_json::to_vec(&changed).unwrap();
        fs::write(directory.join("bootstrap.json"), &raw).unwrap();
        let mut rebound = manifest.clone();
        rebound["bootstrap"]["sha256"] = json!(chrono_harness::sha256(&raw));
        rebound["bootstrap"]["size"] = json!(raw.len());
        recipe::write(&directory, "frontend.json", &rebound);
        assert!(
            f.validate(&directory, &f.selected(), None).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn collector_rejects_archive_content_substitution_even_with_recomputed_transport_digest() {
    let f = Fixture::new();
    let directory = f.exported();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("frontend.json")).unwrap()).unwrap();
    let file = fs::File::create(directory.join("executable.tar")).unwrap();
    let mut builder = tar::Builder::new(file);
    let raw = b"substituted executable";
    let mut header = tar::Header::new_gnu();
    header.set_path("executable").unwrap();
    header.set_size(raw.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder.append(&header, raw.as_slice()).unwrap();
    builder.finish().unwrap();
    drop(builder);
    let (sha, size) = chrono_harness::file_identity(&directory.join("executable.tar")).unwrap();
    manifest["archive"]["sha256"] = json!(sha);
    manifest["archive"]["size"] = json!(size);
    recipe::write(&directory, "frontend.json", &manifest);
    assert!(f.validate(&directory, &f.selected(), None).is_err());
}

#[test]
fn actual_native_unit_retains_original_frontend_metadata_for_the_collector() {
    use chrono_host_release::{
        receipt::{self, Kind, Lineage, Selection},
        recipe::Loaded,
    };
    let f = Fixture::new();
    let binary = f.executable();
    use std::os::unix::fs::MetadataExt;
    let compiled = fs::metadata(env!("CARGO_BIN_EXE_chrono-test-release-cli")).unwrap();
    let installed = fs::metadata(&binary).unwrap();
    assert_eq!(
        (installed.dev(), installed.ino()),
        (compiled.dev(), compiled.ino())
    );
    let directory = binary.parent().unwrap();
    let out = f.unit(&binary);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let handoff = f.host.root().join("handoffs/none");
    let report: Value =
        serde_json::from_slice(&fs::read(handoff.join("receipt.json")).unwrap()).unwrap();
    assert_eq!(report["status"], "passed");
    assert_eq!(report["frontend_input"]["path"], "frontend/frontend.json");
    assert_eq!(
        fs::read(handoff.join("frontend/bootstrap.json")).unwrap(),
        fs::read(directory.join("bootstrap.json")).unwrap()
    );
    assert!(!handoff.join("frontend/executable.tar").exists());
    assert!(!handoff.join("frontend/chrono-host-release").exists());
    let loaded = Loaded::load(f.host.root()).unwrap();
    let parent = Lineage {
        kind: Kind::Github,
        run_id: "1234".into(),
        attempt: "2".into(),
        job: "native_collect".into(),
    };
    let selected = Selection {
        job: "native_test_none".into(),
        run_id: "1234".into(),
        attempt: "2".into(),
        artifact_id: "8765".into(),
    };
    let verify = || {
        receipt::read(
            &loaded,
            f.host.root(),
            &report["context"],
            &parent,
            loaded.unit("test_none").unwrap(),
            &selected,
        )
    };
    assert!(verify().is_ok(), "{:?}", verify().unwrap_err());
    fs::write(
        handoff.join("frontend/.chrono-harness/state/frontend.stderr"),
        b"changed original build stream",
    )
    .unwrap();
    assert!(verify().is_err());
}

#[test]
fn native_launcher_restores_executable_mode_and_runs_the_selected_frontend_without_rust() {
    if std::env::var_os("CHRONO_TEST_RELEASE_LAUNCHER").is_none() {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "native_launcher_restores_executable_mode_and_runs_the_selected_frontend_without_rust",
                "--nocapture",
            ])
            .env("CHRONO_TEST_RELEASE_LAUNCHER", "1");
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let f = Fixture::new();
    let directory = f.host.root().join("frontend");
    assert!(f.export(&directory).status.success());
    assert!(!directory.join("chrono-host-release").exists());
    // 即使直接执行此测试，也要经过正式消费路径的继承能力交接。
    let ownership = (0..27)
        .map(|_| tempfile::tempfile().unwrap())
        .collect::<Vec<_>>();
    let descriptors = ownership.iter().map(AsFd::as_fd).collect::<Vec<_>>();
    let _scope = chrono_harness::process_fds::Scope::new(&descriptors).unwrap();
    let out = f.unit_through_launcher(&directory);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::metadata(directory.join("chrono-host-release"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    let report: Value = serde_json::from_slice(
        &fs::read(f.host.root().join("handoffs/none/receipt.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["status"], "passed");
    assert_eq!(report["frontend_selection"]["attempt"], "1");
    assert_eq!(report["frontend_input"]["path"], "frontend/frontend.json");
}

#[test]
fn native_launcher_rejects_a_missing_archive_before_the_business_action() {
    let f = Fixture::new();
    let directory = f.host.root().join("frontend");
    assert!(f.export(&directory).status.success());
    fs::remove_file(directory.join("executable.tar")).unwrap();
    let out = f.unit_through_launcher(&directory);
    assert!(!out.status.success());
    assert!(!f.host.root().join("handoffs/none").exists());
}

#[test]
fn native_unit_rejects_an_identical_frontend_executed_outside_the_selected_artifact() {
    let f = Fixture::new();
    f.exported();
    let out = f.unit(
        &f.host
            .root()
            .join(".chrono-harness/bin/chrono-host-release"),
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("frontend"));
}

#[test]
fn spawn_failure_retains_the_attempted_command_without_inventing_child_output() {
    let f = Fixture::new();
    let missing = f.host.root().join("missing-frontend");
    let error = f
        .capture_result(&mut std::process::Command::new(&missing))
        .unwrap_err();
    let receipt: Value =
        serde_json::from_slice(&fs::read(f.output.path().join("process-0.json")).unwrap()).unwrap();
    assert_eq!(receipt["argv"], json!([missing]));
    assert_eq!(
        receipt["spawn_error"]["raw_os_error"],
        json!(error.raw_os_error())
    );
    assert_eq!(receipt["spawn_error"]["message"], error.to_string());
    assert!(receipt["exit_code"].is_null());
    assert!(receipt["signal"].is_null());
    assert!(!f.output.path().join("process-0.stdout").exists());
    assert!(!f.output.path().join("process-0.stderr").exists());
}
