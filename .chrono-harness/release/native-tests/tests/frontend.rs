//! 发布工具的真实 CLI 与外部 bootstrap 报告样本；样本不冒充真实原生构建。
#[path = "support/recipe.rs"]
mod recipe;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

struct Fixture {
    host: recipe::Fixture,
    output: tempfile::TempDir,
    report: Value,
}

impl Fixture {
    fn new() -> Self {
        let host = recipe::Fixture::new();
        let root = host.root();
        fs::write(
            root.join(".gitignore"),
            "/.chrono-harness/bin/\n/.chrono-harness/state/\n/frontend/\n",
        )
        .unwrap();
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        let binary = root.join(".chrono-harness/bin/chrono-host-release");
        fs::copy(env!("CARGO_BIN_EXE_chrono-test-release-cli"), &binary).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        let system = match std::env::consts::OS {
            "macos" => "Darwin",
            "linux" => "Linux",
            other => panic!("unsupported {other}"),
        };
        let machine = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => "arm64",
            (_, machine) => machine,
        };
        let profile = json!({"schema":"chrono-bootstrap/v1", "projects":".chrono-harness/bootstrap-projects.json", "rust_toolchain":"1.95.0", "tools":{},
            "operations":["build.frontend"], "install":[{"source":"out/frontend", "destination":".chrono-harness/bin/chrono-host-release"}],
            "participation":{"coordinator_root":"git-main-worktree", "program":".chrono-harness/bin/chrono-worktree", "git":"/usr/bin/git", "config":".chrono-harness/worktree.json"},
            "entrypoint":{"tool":"bash", "script":".chrono-harness/ci/entry.sh"}, "require_tool_bindings":true,
            "tool_bindings_registry":".chrono-harness/ci/registry.json", "report_path":".chrono-harness/state/frontend-bootstrap.json"});
        recipe::write(root, ".chrono-harness/ci/bootstrap-release.json", &profile);
        recipe::write(
            root,
            ".chrono-harness/bootstrap-projects.json",
            &json!({"projects":[{"id":"frontend", "actions":{"build":{"operation":"build.frontend", "tool":"cargo", "argv":["build", "--locked"]}}}]}),
        );
        recipe::write(
            root,
            ".chrono-harness/ci/registry.json",
            &json!({"schema":"chrono-bootstrap-toolchains/v1", "rust_toolchain":"1.95.0", "platforms":{
                format!("{system}/{machine}"):{"tools":{"cargo":{"sha256":"a".repeat(64),"version":"cargo fixture"},"rustc":{"sha256":"b".repeat(64),"version":"rustc fixture"}}}
            }}),
        );
        fs::write(
            root.join(".chrono-harness/ci/entry.sh"),
            "registered bootstrap fixture\n",
        )
        .unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "--all"],
            vec![
                "-c",
                "user.name=Release fixture",
                "-c",
                "user.email=release@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "frontend inputs",
            ],
        ] {
            assert!(
                Command::new("/usr/bin/git")
                    .args(args)
                    .current_dir(root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let git = |revision| {
            String::from_utf8(
                Command::new("/usr/bin/git")
                    .args(["rev-parse", revision])
                    .current_dir(root)
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_owned()
        };
        let source =
            json!({"commit":git("HEAD"), "tree":git("HEAD^{tree}"), "dirty":false, "error":null});
        let mut registrations = serde_json::Map::new();
        for path in [
            ".chrono-harness/ci/bootstrap-release.json",
            ".chrono-harness/bootstrap-projects.json",
            ".chrono-harness/ci/registry.json",
            ".chrono-harness/ci/entry.sh",
        ] {
            registrations.insert(
                path.into(),
                json!(chrono_harness::sha256(&fs::read(root.join(path)).unwrap())),
            );
        }
        let stream = |path: &str, raw: &[u8]| {
            fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
            fs::write(root.join(path), raw).unwrap();
            json!({"path":path,"sha256":chrono_harness::sha256(raw),"size":raw.len()})
        };
        let report = json!({"schema":"chrono-bootstrap-result/v1", "config":".chrono-harness/ci/bootstrap-release.json", "host_root":root,
            "status":"passed", "registrations":registrations, "versions":{"cargo":"cargo fixture","rustc":"rustc fixture"},
            "environment":{"RUSTUP_TOOLCHAIN":"1.95.0","RUSTC":"/fixture/rustc","RUSTC_WRAPPER":"","RUSTC_WORKSPACE_WRAPPER":""},
            "selection":{"toolchain":"1.95.0","cargo":"/fixture/cargo","rustc":"/fixture/rustc","compiler":"/fixture/rustc"},
            "executables":[{"path":"/fixture/cargo","sha256":"a".repeat(64),"version":"cargo fixture"},{"path":"/fixture/rustc","sha256":"b".repeat(64),"version":"rustc fixture"}],
            "source":{"state":"clean","commit":source["commit"],"tree":source["tree"],"before":source,"after":source},
            "operations":[{"operation":"build.frontend","tool":"cargo","argv":["/fixture/cargo","build","--locked"],"cwd":root,"executable_sha256":"a".repeat(64),"pid":123,"exit_code":0,"duration_seconds":0.1,
                "stdout":stream(".chrono-harness/state/frontend.stdout", b"original build\xff\n"), "stderr":stream(".chrono-harness/state/frontend.stderr", b"diagnostic\xfe\n")}],
            "installed":[{"path":".chrono-harness/bin/chrono-host-release","sha256":chrono_harness::file_identity(&binary).unwrap().0}]});
        let fixture = Self {
            host,
            output: tempfile::tempdir().unwrap(),
            report,
        };
        fixture.save();
        fixture
    }

    fn save(&self) {
        recipe::write(
            self.host.root(),
            ".chrono-harness/state/frontend-bootstrap.json",
            &self.report,
        );
    }

    fn export(&self, destination: &Path) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-release-cli"));
        command
            .args([
                self.host.root().as_os_str(),
                destination.as_os_str(),
                "--export-frontend".as_ref(),
            ])
            .env("CHRONO_RELEASE_RUN", "1234")
            .env("CHRONO_RELEASE_ATTEMPT", "1")
            .env("CHRONO_RELEASE_JOB", "native_frontend");
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        command.output().unwrap()
    }

    fn exported(&self) -> std::path::PathBuf {
        let directory = self.output.path().join("export");
        let out = self.export(&directory);
        assert!(
            out.status.success(),
            "stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut tar = tar::Archive::new(fs::File::open(directory.join("executable.tar")).unwrap());
        let mut entries = tar.entries().unwrap();
        entries
            .next()
            .unwrap()
            .unwrap()
            .unpack(directory.join("chrono-host-release"))
            .unwrap();
        directory
    }

    fn selected(&self) -> chrono_host_release::receipt::Selection {
        serde_json::from_value(
            json!({"job":"native_frontend","artifact_id":"98765","run_id":"1234","attempt":"1"}),
        )
        .unwrap()
    }

    fn validate(
        &self,
        directory: &Path,
        selected: &chrono_host_release::receipt::Selection,
        executing: Option<&Path>,
    ) -> Result<Value, String> {
        let loaded = chrono_host_release::recipe::Loaded::load(self.host.root()).unwrap();
        let source = serde_json::from_value(self.report["source"]["before"].clone()).unwrap();
        chrono_host_release::frontend::validate(
            self.host.root(),
            directory,
            &loaded.recipe.native_frontends["fixture"],
            &chrono_host_bootstrap::bindings::native_platform().unwrap(),
            &source,
            selected,
            executing,
        )
    }
}

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
