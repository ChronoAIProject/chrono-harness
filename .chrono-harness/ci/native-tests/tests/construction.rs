use chrono_host_bootstrap::{
    bindings::{Environment, NativePlatform},
    construction::{Construction, Published},
    profile::Loaded,
};
use serde_json::{Value, json};
use std::{ffi::OsStr, fs, path::PathBuf, process::Command};

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    tools: PathBuf,
    environment: Environment,
    config: Value,
    projects: Value,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        let tools = root.join("native tools");
        fs::create_dir(&tools).unwrap();
        fs::create_dir_all(root.join(".chrono-harness/ci")).unwrap();
        fs::create_dir(root.join("out")).unwrap();
        fs::write(
            root.join(".chrono-harness/ci/entry.sh"),
            b"exec registered-native-bootstrap\n",
        )
        .unwrap();
        for name in ["rustup", "cargo", "rustc"] {
            fs::copy(
                env!("CARGO_BIN_EXE_chrono-test-bootstrap-tool"),
                tools.join(name),
            )
            .unwrap();
        }
        let bound = |name| json!({"sha256":chrono_harness::file_identity(&tools.join(name)).unwrap().0,"version":format!("{name} 1.95.0 (fixture)")});
        let registry = json!({"schema":"chrono-bootstrap-toolchains/v1","rust_toolchain":"1.95.0",
            "platforms":{"fixture/native":{"tools":{"cargo":bound("cargo"),"rustc":bound("rustc")}}}});
        fs::write(
            root.join(".chrono-harness/ci/registry.json"),
            serde_json::to_vec(&registry).unwrap(),
        )
        .unwrap();
        let config = json!({"schema":"chrono-bootstrap/v1","projects":".chrono-harness/projects.json","rust_toolchain":"1.95.0",
            "tools":{"native":env!("CARGO_BIN_EXE_chrono-test-bootstrap-operation")},"prerequisites":["prepare"],"operations":["build"],
            "install":[{"source":"out/cargo","destination":".chrono-harness/bin/cargo"}],
            "participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":"/fixture/git","config":".chrono-harness/worktree.json"},
            "entrypoint":{"tool":"sh","script":".chrono-harness/ci/entry.sh"},"require_tool_bindings":true,
            "tool_bindings_registry":".chrono-harness/ci/registry.json","rust_incremental":false});
        let projects = json!({"projects":[{"id":"p","actions":{"build":{"operation":"build","tool":"cargo","argv":["build-fixture",root.join("out/cargo")]}}}],
            "scripts":[{"id":"s","actions":{"execute":{"operation":"prepare","tool":"native","argv":["run",serde_json::to_string(&json!([{"Stdout":[112,114,101,10]},{"Create":{"path":"prepared","bytes":[111,107]}}])).unwrap()]}}}]});
        let mut environment: Environment = std::env::vars_os().collect();
        let mut search = tools.as_os_str().to_owned();
        search.push(":");
        search.push(environment.get(OsStr::new("PATH")).unwrap());
        environment.insert("PATH".into(), search);
        for key in ["RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            environment.remove(OsStr::new(key));
        }
        Self {
            _directory: directory,
            root,
            tools,
            environment,
            config,
            projects,
        }
    }
    fn loaded(&self) -> Loaded {
        fs::write(
            self.root.join(".chrono-harness/ci/build.json"),
            serde_json::to_vec(&self.config).unwrap(),
        )
        .unwrap();
        fs::write(
            self.root.join(".chrono-harness/projects.json"),
            serde_json::to_vec(&self.projects).unwrap(),
        )
        .unwrap();
        Loaded::read(
            &self.root,
            ".chrono-harness/ci/build.json",
            &NativePlatform {
                system: "fixture".into(),
                machine: "native".into(),
            },
            &self.environment,
        )
        .unwrap()
    }
    fn begin(&self) -> Construction<'_> {
        Construction::begin(&self.root, self.loaded(), &self.environment).unwrap()
    }
    fn committed(&self) -> Construction<'_> {
        let loaded = self.loaded();
        fs::write(self.root.join(".gitignore"), b"native tools/\nout/\nprepared\n.chrono-harness/state/\n.chrono-harness/bin/\n.chrono-harness/cache/\n").unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        ] {
            let mut command = Command::new("git");
            command.current_dir(&self.root).args(args);
            command
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE");
            chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Construction::begin(&self.root, loaded, &self.environment).unwrap()
    }
    fn report(&self, published: &Published) -> Value {
        serde_json::from_slice(&fs::read(&published.original).unwrap()).unwrap()
    }
    fn failed_operation(&mut self, operation: &str, exit: i32) {
        let steps = json!([{"Stdout":[0,255,10]},{"Stderr":[254,0,10]},{"Exit":exit}]);
        let action = json!({"operation":operation,"tool":"native","argv":["run",serde_json::to_string(&steps).unwrap()]});
        if operation == "prepare" {
            self.projects["scripts"][0]["actions"]["execute"] = action;
        } else {
            self.projects["projects"][0]["actions"]["build"] = action;
        }
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
