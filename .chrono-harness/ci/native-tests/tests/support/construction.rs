use chrono_host_bootstrap::{
    bindings::{Environment, NativePlatform},
    construction::{Construction, Published},
    profile::Loaded,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

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
        for name in ["rustup", "cargo", "rustc", "git"] {
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
            "participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":tools.join("git"),"config":".chrono-harness/worktree.json"},
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
        let fixture = Self {
            _directory: directory,
            root,
            tools,
            environment,
            config,
            projects,
        };
        fixture.mock_git(
            &[
                "--no-optional-locks",
                "--no-replace-objects",
                "rev-parse",
                "--show-toplevel",
                "--verify",
                "HEAD",
            ],
            json!({"exit_code":128,"stderr":b"mock source unavailable\n".to_vec()}),
        );
        fixture
    }

    pub fn mock_git(&self, args: &[&str], mut response: Value) {
        let path = self.tools.join("git.json");
        let mut specification: Value = if path.exists() {
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap()
        } else {
            json!({})
        };
        if response.get("cwd").is_none() {
            response["cwd"] = json!(self.root);
        }
        specification[serde_json::to_string(args).unwrap()] = response;
        fs::write(path, serde_json::to_vec(&specification).unwrap()).unwrap();
    }

    pub fn git_query(&self, args: &[&str], stdout: impl AsRef<[u8]>) {
        let mut command = vec!["--no-optional-locks", "--no-replace-objects"];
        command.extend_from_slice(args);
        self.mock_git(&command, json!({"exit_code":0,"stdout":stdout.as_ref()}));
    }

    pub fn source(&self, commit: &str, tree: &str, dirty: bool) {
        fs::create_dir_all(self.root.join(".git")).unwrap();
        self.git_query(
            &["rev-parse", "--show-toplevel", "--verify", "HEAD"],
            format!("{}\n{commit}\n", self.root.display()),
        );
        self.git_query(
            &["rev-parse", "--verify", &format!("{commit}^{{tree}}")],
            format!("{tree}\n"),
        );
        self.git_query(&["rev-parse", "--verify", "HEAD"], format!("{commit}\n"));
        self.dirty(dirty);
    }

    pub fn dirty(&self, dirty: bool) {
        self.git_query(
            &[
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "--ignore-submodules=none",
            ],
            if dirty {
                b" M fixture\n".as_slice()
            } else {
                b"".as_slice()
            },
        );
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
        // 仅为本夹具声明固定输入和返回值，不实现或调用 Git。
        let mut inputs = BTreeMap::new();
        for entry in fs::read_dir(self.root.join(".chrono-harness/ci")).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                continue;
            }
            let name = path
                .strip_prefix(&self.root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let link = fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink();
            let bytes = if link {
                fs::read_link(&path)
                    .unwrap()
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec()
            } else {
                fs::read(&path).unwrap()
            };
            inputs.insert(name, (if link { "120000" } else { "100644" }, bytes));
        }
        inputs.insert(
            ".chrono-harness/projects.json".into(),
            (
                "100644",
                fs::read(self.root.join(".chrono-harness/projects.json")).unwrap(),
            ),
        );
        let path = self.tools.join("git.json");
        let mut specification: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let generation = specification["generation"].as_u64().unwrap_or(0) + 1;
        specification["generation"] = json!(generation);
        fs::write(&path, serde_json::to_vec(&specification).unwrap()).unwrap();
        let tree =
            chrono_harness::sha256(&serde_json::to_vec(&json!([generation, inputs])).unwrap())
                [..40]
                .to_owned();
        let commit = chrono_harness::sha256(format!("mock commit {generation} {tree}").as_bytes())
            [..40]
            .to_owned();
        self.source(&commit, &tree, false);
        self.inventory(&self.root);
        for (name, (mode, bytes)) in inputs {
            let blob = chrono_harness::sha256(&bytes)[..40].to_owned();
            self.git_query(
                &["ls-tree", "-z", &commit, "--", &name],
                format!("{mode} blob {blob}\t{name}\0"),
            );
            self.git_query(&["cat-file", "blob", &blob], bytes);
        }
        self.git_query(
            &["ls-tree", "-z", &commit, "--", ".chrono-harness/ci/absent"],
            b"",
        );
        loaded
    }

    pub fn inventory(&self, anchor: &Path) {
        self.mock_git(&["--no-optional-locks", "worktree", "list", "--porcelain", "-z"], json!({"exit_code":0,"stdout":format!("worktree {}\0\0", anchor.display()).into_bytes()}));
    }

    pub fn rebind_git(root: &Path) {
        let root = fs::canonicalize(root).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        let path = root.join("native tools/git.json");
        let mut specification: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for response in specification.as_object_mut().unwrap().values_mut() {
            if response.is_object() {
                response["cwd"] = json!(root);
            }
        }
        let key = serde_json::to_string(&[
            "--no-optional-locks",
            "--no-replace-objects",
            "rev-parse",
            "--show-toplevel",
            "--verify",
            "HEAD",
        ])
        .unwrap();
        let before: Vec<u8> =
            serde_json::from_value(specification[&key]["stdout"].clone()).unwrap();
        let before = String::from_utf8(before).unwrap();
        let commit = before.lines().last().unwrap();
        specification[&key]["stdout"] =
            json!(format!("{}\n{commit}\n", root.display()).into_bytes());
        let key = serde_json::to_string(&[
            "--no-optional-locks",
            "worktree",
            "list",
            "--porcelain",
            "-z",
        ])
        .unwrap();
        specification[&key]["stdout"] =
            json!(format!("worktree {}\0\0", root.display()).into_bytes());
        fs::write(path, serde_json::to_vec(&specification).unwrap()).unwrap();
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
