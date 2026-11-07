use chrono_host_bootstrap::{
    bindings::{Environment, NativePlatform},
    construction::{Construction, Published},
    profile::Loaded,
};
use serde_json::{Value, json};
use std::{ffi::OsStr, fs, path::PathBuf, process::Command};

pub struct Fixture {
    pub _directory: tempfile::TempDir,
    pub root: PathBuf,
    pub tools: PathBuf,
    pub environment: Environment,
    pub config: Value,
    pub projects: Value,
}

impl Fixture {
    pub fn new() -> Self {
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
    pub fn loaded(&self) -> Loaded {
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
    pub fn begin(&self) -> Construction<'_> {
        Construction::begin(&self.root, self.loaded(), &self.environment).unwrap()
    }
    pub fn committed(&self) -> Construction<'_> {
        Construction::begin(&self.root, self.commit(), &self.environment).unwrap()
    }
    pub fn commit(&self) -> Loaded {
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
        loaded
    }
    pub fn report(&self, published: &Published) -> Value {
        serde_json::from_slice(&fs::read(&published.original).unwrap()).unwrap()
    }
    pub fn failed_operation(&mut self, operation: &str, exit: i32) {
        let steps = json!([{"Stdout":[0,255,10]},{"Stderr":[254,0,10]},{"Exit":exit}]);
        let action = json!({"operation":operation,"tool":"native","argv":["run",serde_json::to_string(&steps).unwrap()]});
        if operation == "prepare" {
            self.projects["scripts"][0]["actions"]["execute"] = action;
        } else {
            self.projects["projects"][0]["actions"]["build"] = action;
        }
    }
}
