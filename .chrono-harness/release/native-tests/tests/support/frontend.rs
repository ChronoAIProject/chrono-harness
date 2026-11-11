//! 发布工具的真实 CLI 与外部 bootstrap 报告样本；样本不冒充真实原生构建。
#[path = "context.rs"]
mod context;
#[path = "recipe.rs"]
pub mod recipe;
use serde_json::{Value, json};
use std::{
    cell::Cell,
    fs,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::Path,
    process::Command,
};

pub struct Fixture {
    case: tempfile::TempDir,
    pub host: recipe::Fixture,
    pub output: tempfile::TempDir,
    pub report: Value,
    calls: Cell<usize>,
}

impl Fixture {
    pub fn new() -> Self {
        let state = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.chrono-harness/state/release-test-failures");
        fs::create_dir_all(&state).unwrap();
        let case = tempfile::Builder::new()
            .prefix("rust-release-frontend-")
            .tempdir_in(&state)
            .unwrap();
        let mut host = recipe::Fixture::new();
        host.temp = tempfile::Builder::new()
            .prefix("source-")
            .tempdir_in(case.path())
            .unwrap();
        let tools = case.path().join("tools");
        fs::create_dir(&tools).unwrap();
        let git_tool = tools.join("git");
        fs::copy(env!("CARGO_BIN_EXE_chrono-test-release-git"), &git_tool).unwrap();
        host.config["tools"]["git"] = json!(git_tool);
        let probe = tools.join("probe");
        fs::copy(env!("CARGO_BIN_EXE_chrono-test-release-tool"), &probe).unwrap();
        host.config["tools"]["probe"] = json!(probe);
        host.save();
        let root = host.root();
        fs::write(
            root.join(".gitignore"),
            "/.chrono-harness/bin/\n/.chrono-harness/state/\n/frontend/\n/handoffs/\n/consumer/\n",
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
            "participation":{"coordinator_root":"git-main-worktree", "program":".chrono-harness/bin/chrono-worktree", "git":git_tool, "config":".chrono-harness/worktree.json"},
            "entrypoint":{"tool":"bash", "script":".chrono-harness/ci/entry.sh"}, "require_tool_bindings":true,
            "tool_bindings_registry":".chrono-harness/ci/registry.json", "report_path":".chrono-harness/state/frontend-bootstrap.json"});
        recipe::write(root, ".chrono-harness/ci/bootstrap-release.json", &profile);
        recipe::write(
            root,
            ".chrono-harness/bootstrap-projects.json",
            &json!({"projects":[{"id":"frontend", "actions":{"build":{"operation":"build.frontend", "tool":"cargo", "argv":["build", "--locked"]}}}],"scripts":[]}),
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
                Command::new(&git_tool)
                    .args(args)
                    .current_dir(root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let git = |revision| {
            String::from_utf8(
                Command::new(&git_tool)
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
            output: tempfile::Builder::new()
                .prefix("results-")
                .tempdir_in(case.path())
                .unwrap(),
            case,
            report,
            calls: Cell::new(0),
        };
        fixture.save();
        fixture
    }

    pub fn save(&self) {
        recipe::write(
            self.host.root(),
            ".chrono-harness/state/frontend-bootstrap.json",
            &self.report,
        );
    }

    pub fn export(&self, destination: &Path) -> std::process::Output {
        let mut command = context::command(env!("CARGO_BIN_EXE_chrono-test-release-cli"));
        command
            .args([
                self.host.root().as_os_str(),
                destination.as_os_str(),
                "--export-frontend".as_ref(),
            ])
            .env("CHRONO_RELEASE_RUN", "1234")
            .env("CHRONO_RELEASE_ATTEMPT", "1")
            .env("CHRONO_RELEASE_JOB", "native_frontend");
        self.capture(command)
    }

    pub fn capture(&self, mut command: Command) -> std::process::Output {
        let mut paths = vec![self.case.path().join("tools")];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
        command.env("PATH", std::env::join_paths(paths).unwrap());
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        let output = command.output().unwrap();
        let index = self.calls.get();
        self.calls.set(index + 1);
        fs::write(
            self.output.path().join(format!("process-{index}.stdout")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            self.output.path().join(format!("process-{index}.stderr")),
            &output.stderr,
        )
        .unwrap();
        recipe::write(
            self.output.path(),
            &format!("process-{index}.json"),
            &json!({
                "argv":std::iter::once(command.get_program()).chain(command.get_args()).map(|s|s.to_string_lossy().into_owned()).collect::<Vec<_>>(),
                "cwd":std::env::current_dir().unwrap(),"exit_code":output.status.code(),"signal":output.status.signal(),
                "stdout":{"size":output.stdout.len(),"sha256":chrono_harness::sha256(&output.stdout)},
                "stderr":{"size":output.stderr.len(),"sha256":chrono_harness::sha256(&output.stderr)}
            }),
        );
        output
    }

    pub fn exported(&self) -> std::path::PathBuf {
        let directory = self.host.root().join("frontend");
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

    pub fn unit(&self, executable: &Path) -> std::process::Output {
        let mut command = context::command(executable);
        command
            .arg(self.host.root())
            .arg(self.host.root().join("handoffs/none"))
            .args(["--unit", "test_none"]);
        self.native_unit(command)
    }

    pub fn unit_through_launcher(&self, directory: &Path) -> std::process::Output {
        let mut command = context::command("/bin/bash");
        command
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../consume.sh"))
            .arg(self.host.root())
            .arg(directory)
            .arg(self.host.root().join("handoffs/none"))
            .args(["--unit", "test_none"]);
        self.native_unit(command)
    }

    fn native_unit(&self, mut command: Command) -> std::process::Output {
        command.env_remove("CHRONO_RELEASE_LOCAL")
            .env("CHRONO_RELEASE_RUN","1234").env("CHRONO_RELEASE_ATTEMPT","2").env("CHRONO_RELEASE_JOB","native_test_none")
            .env("CHRONO_RELEASE_DEPENDENCIES",serde_json::to_string(&json!({"native_frontend":{"result":"success","outputs":{"artifact_id":"98765","run_id":"1234","attempt":"1"}}})).unwrap());
        self.capture(command)
    }

    pub fn selected(&self) -> chrono_host_release::receipt::Selection {
        serde_json::from_value(
            json!({"job":"native_frontend","artifact_id":"98765","run_id":"1234","attempt":"1"}),
        )
        .unwrap()
    }

    pub fn validate(
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

impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.host.temp.disable_cleanup(true);
            self.output.disable_cleanup(true);
            self.case.disable_cleanup(true);
            eprintln!(
                "original release frontend fixture retained: {}",
                self.case.path().display()
            );
        }
    }
}
