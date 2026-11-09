#[path = "context.rs"]
mod context;
#[path = "recipe.rs"]
mod recipe;
use chrono_host_release::recipe::Loaded;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub struct Fixture {
    pub host: recipe::Fixture,
    _case: tempfile::TempDir,
    _tools: tempfile::TempDir,
    pub outputs: tempfile::TempDir,
    _nested: tempfile::TempDir,
    pub git: PathBuf,
    pub outcomes: serde_json::Map<String, Value>,
}

fn capture(command: &mut Command) -> Output {
    chrono_harness::process_fds::forward_command(command, &[]).unwrap();
    command.output().unwrap()
}

impl Fixture {
    pub fn new(behavior: &str, tracked: bool) -> Self {
        let mut host = recipe::Fixture::new();
        let state = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.chrono-harness/state/release-test-failures");
        fs::create_dir_all(&state).unwrap();
        let case = tempfile::Builder::new()
            .prefix("rust-release-")
            .tempdir_in(state)
            .unwrap();
        host.temp = tempfile::Builder::new()
            .prefix("source-")
            .tempdir_in(case.path())
            .unwrap();
        let tools = tempfile::Builder::new()
            .prefix("tools-")
            .tempdir_in(case.path())
            .unwrap();
        let outputs = tempfile::Builder::new()
            .prefix("results-")
            .tempdir_in(case.path())
            .unwrap();
        let nested = tempfile::Builder::new()
            .prefix("chrono-native-publication-failure-")
            .tempdir()
            .unwrap();
        for (name, raw) in [
            ("stdout", b"nested stdout".as_slice()),
            ("stderr", b"nested stderr".as_slice()),
            ("process.json", b"{\"exit_code\":37}".as_slice()),
        ] {
            fs::write(nested.path().join(name), raw).unwrap();
        }
        let preparation = nested.path().join("state/collection/preparation");
        fs::create_dir_all(&preparation).unwrap();
        fs::write(preparation.join("original.bytes"), [0u8, 253]).unwrap();
        let git = chrono_harness::resolve_program(host.root(), "git", None).unwrap();
        host.config["tools"]["git"] = json!(git);
        for name in ["cargo", "rustc", "rustup", "probe"] {
            let file = tools.path().join(name);
            fs::copy(env!("CARGO_BIN_EXE_chrono-test-release-tool"), &file).unwrap();
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
            host.config["tools"][name] = json!(file);
        }
        host.config["limits"] = json!({"unit_seconds":120,"platform_seconds":600});
        host.config["failure_evidence"]["test.none"] = json!(["failures/test-a/"]);
        // 真实打包器要求发布安装器；它仍由 build_a 产生并暂存到 consumer/a。
        host.config["units"][0]["asset"] = json!("chrono-distribution");
        host.config["consumer_staging"]["bindings"][0]["asset"] = json!("chrono-distribution");
        host.config["collector"]["package_asset"] = json!("chrono-distribution");
        host.config["verification_consumers"]["test_a"]["release_assets"] =
            json!(["chrono-distribution"]);
        host.plan["assets"].as_object_mut().unwrap().remove("a");
        host.plan["assets"]["chrono-distribution"] = json!("out/a");
        host.save();
        fs::create_dir_all(host.root().join("a")).unwrap();
        fs::create_dir_all(host.root().join("b")).unwrap();
        fs::write(host.root().join("a/Cargo.toml"), "external manifest a").unwrap();
        fs::write(host.root().join("b/Cargo.toml"), "external manifest b").unwrap();
        fs::write(
            host.root().join(".gitignore"),
            "/out/\n/consumer/\n/handoffs/\n/failures/\n/dependencies.json\n",
        )
        .unwrap();
        let fixture = json!({"behavior":behavior,"distribution":env!("CARGO_BIN_EXE_chrono-test-release-distribution"),"nested":nested.path()});
        fs::write(
            host.root()
                .join(".chrono-harness/release/tool-fixture.json"),
            serde_json::to_vec(&fixture).unwrap(),
        )
        .unwrap();
        if tracked {
            fs::create_dir(host.root().join("consumer")).unwrap();
            fs::write(host.root().join("consumer/a"), b"tracked original").unwrap();
        }
        let f = Self {
            host,
            _case: case,
            _tools: tools,
            outputs,
            _nested: nested,
            git,
            outcomes: serde_json::Map::new(),
        };
        f.git(&["init", "--quiet"]);
        f.git(&["add", "--all"]);
        if tracked {
            f.git(&["add", "-f", "--", "consumer/a"]);
        }
        f.git(&[
            "-c",
            "user.name=Release test",
            "-c",
            "user.email=release-test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "self-contained release inputs",
        ]);
        f
    }

    pub fn root(&self) -> &Path {
        self.host.root()
    }
    pub fn git(&self, args: &[&str]) -> Output {
        let out = capture(Command::new(&self.git).current_dir(self.root()).args(args));
        assert!(
            out.status.success(),
            "git {args:?}: exit {:?}, stdout={}, stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
    pub fn handoff(&self, id: &str) -> PathBuf {
        self.root().join(
            Loaded::load(self.root())
                .unwrap()
                .unit(id)
                .unwrap()
                .handoff(),
        )
    }
    pub fn report(&self, id: &str) -> Value {
        serde_json::from_slice(&fs::read(self.handoff(id).join("receipt.json")).unwrap()).unwrap()
    }
    pub fn invoke_at(
        &self,
        id: &str,
        output: &Path,
        dependencies: &Value,
        collect: bool,
    ) -> Output {
        let mut command = context::command(env!("CARGO_BIN_EXE_chrono-test-release-cli"));
        command.arg(self.root()).arg(output);
        if collect {
            command.arg("--collect");
        } else {
            command.args(["--unit", id]);
        }
        command
            .env("CHRONO_RELEASE_LOCAL", "1")
            .env("CHRONO_RELEASE_RUN", "self-contained-run")
            .env("CHRONO_RELEASE_ATTEMPT", "1")
            .env("CHRONO_RELEASE_JOB", id)
            .env("CHRONO_RELEASE_DEPENDENCIES", dependencies.to_string());
        capture(&mut command)
    }
    pub fn invoke(&mut self, id: &str) -> Output {
        let loaded = Loaded::load(self.root()).unwrap();
        let unit = loaded.unit(id).unwrap();
        let dependencies: serde_json::Map<_, _> = unit
            .needs()
            .iter()
            .map(|id| (id.clone(), self.outcomes[id].clone()))
            .collect();
        let out = self.invoke_at(id, &self.handoff(id), &json!(dependencies), false);
        if out.status.success() {
            let bytes = fs::read(self.handoff(id).join("receipt.json")).unwrap();
            self.outcomes.insert(id.into(),json!({"result":"success","outputs":{"artifact_id":chrono_harness::sha256(&bytes),"run_id":"self-contained-run","attempt":"1"}}));
        }
        out
    }
    pub fn success(&mut self, id: &str) {
        let out = self.invoke(id);
        assert!(
            out.status.success(),
            "{id}: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    pub fn local(&self, output: &Path) -> Output {
        capture(
            context::command(env!("CARGO_BIN_EXE_chrono-test-release-cli"))
                .arg(self.root())
                .arg(output),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.host.temp.disable_cleanup(true);
            self._case.disable_cleanup(true);
            self._tools.disable_cleanup(true);
            self.outputs.disable_cleanup(true);
            self._nested.disable_cleanup(true);
            eprintln!(
                "original Rust release fixture retained for failure diagnosis: {}",
                self._case.path().display()
            );
            eprintln!(
                "original native nested fixture: {}",
                self._nested.path().display()
            );
        }
    }
}
