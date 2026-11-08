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
        fs::write(root.join(".gitignore"), "/.chrono-harness/bin/\n/.chrono-harness/state/\n/frontend/\n").unwrap();
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        let binary = root.join(".chrono-harness/bin/chrono-host-release");
        fs::copy(env!("CARGO_BIN_EXE_chrono-test-release-cli"), &binary).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        let system = match std::env::consts::OS { "macos" => "Darwin", "linux" => "Linux", other => panic!("unsupported {other}") };
        let machine = match (std::env::consts::OS, std::env::consts::ARCH) { ("macos", "aarch64") => "arm64", (_, machine) => machine };
        let profile = json!({"schema":"chrono-bootstrap/v1", "projects":".chrono-harness/bootstrap-projects.json", "rust_toolchain":"1.95.0", "tools":{},
            "operations":["build.frontend"], "install":[{"source":"out/frontend", "destination":".chrono-harness/bin/chrono-host-release"}],
            "participation":{"coordinator_root":"git-main-worktree", "program":".chrono-harness/bin/chrono-worktree", "git":"/usr/bin/git", "config":".chrono-harness/worktree.json"},
            "entrypoint":{"tool":"bash", "script":".chrono-harness/ci/entry.sh"}, "require_tool_bindings":true,
            "tool_bindings_registry":".chrono-harness/ci/registry.json", "report_path":".chrono-harness/state/frontend-bootstrap.json"});
        recipe::write(root, ".chrono-harness/ci/bootstrap-release.json", &profile);
        recipe::write(root, ".chrono-harness/bootstrap-projects.json", &json!({"projects":[{"id":"frontend", "actions":{"build":{"operation":"build.frontend", "tool":"cargo", "argv":["build", "--locked"]}}}]}));
        recipe::write(root, ".chrono-harness/ci/registry.json", &json!({"schema":"chrono-bootstrap-toolchains/v1", "rust_toolchain":"1.95.0", "platforms":{
            format!("{system}/{machine}"):{"tools":{"cargo":{"sha256":"a".repeat(64),"version":"cargo fixture"},"rustc":{"sha256":"b".repeat(64),"version":"rustc fixture"}}}
        }}));
        fs::write(root.join(".chrono-harness/ci/entry.sh"), "registered bootstrap fixture\n").unwrap();
        for args in [vec!["init", "--quiet"], vec!["add", "--all"], vec!["-c", "user.name=Release fixture", "-c", "user.email=release@example.invalid", "commit", "--quiet", "-m", "frontend inputs"]] {
            assert!(Command::new("/usr/bin/git").args(args).current_dir(root).status().unwrap().success());
        }
        let git = |revision| String::from_utf8(Command::new("/usr/bin/git").args(["rev-parse", revision]).current_dir(root).output().unwrap().stdout).unwrap().trim().to_owned();
        let source = json!({"commit":git("HEAD"), "tree":git("HEAD^{tree}"), "dirty":false, "error":null});
        let mut registrations = serde_json::Map::new();
        for path in [".chrono-harness/ci/bootstrap-release.json", ".chrono-harness/bootstrap-projects.json", ".chrono-harness/ci/registry.json", ".chrono-harness/ci/entry.sh"] {
            registrations.insert(path.into(), json!(chrono_harness::sha256(&fs::read(root.join(path)).unwrap())));
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
        let fixture = Self {host, output:tempfile::tempdir().unwrap(), report};
        fixture.save();
        fixture
    }

    fn save(&self) {
        recipe::write(self.host.root(), ".chrono-harness/state/frontend-bootstrap.json", &self.report);
    }

    fn export(&self, destination: &Path) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-release-cli"));
        command.args([self.host.root().as_os_str(), destination.as_os_str(), "--export-frontend".as_ref()])
            .env("CHRONO_RELEASE_RUN", "1234").env("CHRONO_RELEASE_ATTEMPT", "1").env("CHRONO_RELEASE_JOB", "native_frontend");
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        command.output().unwrap()
    }
}

#[test]
fn actual_export_preserves_original_bootstrap_streams_and_the_executable_archive() {
    let f = Fixture::new();
    let destination = f.output.path().join("export");
    let out = f.export(&destination);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let manifest: Value = serde_json::from_slice(&fs::read(destination.join("frontend.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema"], "chrono-release-frontend/v1");
    assert_eq!(manifest["lineage"], json!({"kind":"github","run_id":"1234","attempt":"1","job":"native_frontend"}));
    assert_eq!(fs::read(destination.join("bootstrap.json")).unwrap(), fs::read(f.host.root().join(".chrono-harness/state/frontend-bootstrap.json")).unwrap());
    for name in ["frontend.stdout", "frontend.stderr"] {
        let path = format!(".chrono-harness/state/{name}");
        assert_eq!(fs::read(destination.join(&path)).unwrap(), fs::read(f.host.root().join(&path)).unwrap());
    }
    let mut tar = tar::Archive::new(fs::File::open(destination.join("executable.tar")).unwrap());
    let mut members = tar.entries().unwrap();
    let mut member = members.next().unwrap().unwrap();
    assert_eq!(member.path().unwrap(), Path::new("executable"));
    assert_eq!(member.header().mode().unwrap(), 0o755);
    let mut bytes = vec![];
    std::io::Read::read_to_end(&mut member, &mut bytes).unwrap();
    assert_eq!(bytes, fs::read(f.host.root().join(".chrono-harness/bin/chrono-host-release")).unwrap());
    assert!(members.next().is_none());
    assert!(!f.export(&destination).status.success());
}
