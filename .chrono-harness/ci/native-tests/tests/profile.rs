use chrono_host_bootstrap::{
    bindings::{Environment, NativePlatform},
    profile::{Loaded, Profile},
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    config: Value,
    projects: Value,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir_all(root.join(".chrono-harness/ci")).unwrap();
        fs::write(root.join(".chrono-harness/ci/entry.sh"), b"exec fixture\n").unwrap();
        let tool =
            |name| json!({"sha256":"0".repeat(64),"version":format!("{name} 1.95.0 (fixture)")});
        let registry = json!({"schema":"chrono-bootstrap-toolchains/v1", "rust_toolchain":"1.95.0",
            "platforms":{"fixture/native":{"tools":{"cargo":tool("cargo"),"rustc":tool("rustc")}}}});
        fs::write(
            root.join(".chrono-harness/ci/registry.json"),
            serde_json::to_vec(&registry).unwrap(),
        )
        .unwrap();
        let config = json!({"schema":"chrono-bootstrap/v1","projects":".chrono-harness/projects.json",
            "rust_toolchain":"1.95.0","tools":{"prepare":"/fixture/prepare"},
            "prerequisites":["prepare"],"operations":["build"],
            "install":[{"source":"out/product","destination":".chrono-harness/bin/product"}],
            "participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":"/fixture/git","config":".chrono-harness/worktree.json"},
            "entrypoint":{"tool":"sh","script":".chrono-harness/ci/entry.sh"},
            "require_tool_bindings":true,"tool_bindings_registry":".chrono-harness/ci/registry.json"});
        let projects = json!({"projects":[{"id":"p","actions":{"build":{"operation":"build","tool":"cargo","argv":["build"]}}}],
            "scripts":[{"id":"s","actions":{"execute":{"operation":"prepare","tool":"prepare","argv":["--verify"]}}}]});
        Self {
            _directory: directory,
            root,
            config,
            projects,
        }
    }
    fn load(&self) -> Result<Loaded, String> {
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
        let environment: Environment = std::env::vars_os().collect();
        Loaded::read(
            &self.root,
            ".chrono-harness/ci/build.json",
            &NativePlatform {
                system: "fixture".into(),
                machine: "native".into(),
            },
            &environment,
        )
    }
}

#[test]
fn current_profile_resolves_only_explicit_operations_and_binds_original_inputs_without_effects() {
    let fixture = Fixture::new();
    let loaded = fixture.load().unwrap();
    assert_eq!(loaded.operations["prepare"].argv, ["--verify"]);
    assert_eq!(loaded.operations["build"].owner, "project:p");
    assert_eq!(loaded.registrations.len(), 4);
    for (path, hash) in &loaded.registrations {
        assert_eq!(
            hash,
            &chrono_harness::sha256(&fs::read(fixture.root.join(path)).unwrap())
        );
    }
    assert!(!fixture.root.join(".chrono-harness/state").exists());
    assert!(!fixture.root.join(".chrono-harness/bin").exists());
    assert!(!fixture.root.join(".chrono-harness/cache").exists());
}

#[test]
fn invalid_profile_sequences_modes_paths_and_environment_fail_before_process_effects() {
    let mut fixture = Fixture::new();
    let original = fixture.config.clone();
    for (key, value) in [
        ("schema", json!("unknown")),
        ("require_tool_bindings", json!(false)),
        ("operations", json!(["build", "build"])),
        ("prerequisites", json!(["build"])),
        ("prerequisites", json!([""])),
        ("reuse_owner_operations", json!(["build"])),
        ("reuse_owner_operations", json!(["missing"])),
        ("report_path", json!(".chrono-harness/cache/report.json")),
        ("report_path", json!(".chrono-harness/state/../report.json")),
        ("tool_bindings", json!({})),
        ("native_platform", json!({})),
        (
            "toolchain_provision",
            json!({"profile":"default","rustup_home":".chrono-harness/state/rustup","cargo_home":".chrono-harness/state/cargo"}),
        ),
        (
            "install",
            json!([{"source":"out/product","destination":".chrono-harness/bin/p"},{"source":"out/second","destination":".chrono-harness/bin/p"}]),
        ),
        ("environment", json!({"CHRONO_PROCESS_FDS":"4"})),
    ] {
        fixture.config = original.clone();
        fixture.config[key] = value;
        assert!(fixture.load().is_err(), "accepted {}", fixture.config);
    }
    fixture.config = original;
    fixture.config["owner_upgrade"] = json!(".chrono-harness/ci/upgrade.json");
    fixture.config["owner_stage"] =
        json!({"source":"out/owner","selected":".chrono-harness/bin/chrono-worktree"});
    assert!(fixture.load().is_err());
    assert!(!fixture.root.join(".chrono-harness/state").exists());
    assert!(!fixture.root.join(".chrono-harness/cache").exists());
}

#[test]
fn unknown_ambiguous_and_unconfigured_selected_methods_remain_concrete_errors() {
    let mut fixture = Fixture::new();
    fixture.config["operations"] = json!(["missing"]);
    assert!(
        fixture
            .load()
            .err()
            .unwrap()
            .contains("unknown bootstrap operation: missing")
    );
    fixture.config["operations"] = json!(["build"]);
    fixture.projects["scripts"][0]["actions"]["duplicate"] =
        fixture.projects["projects"][0]["actions"]["build"].clone();
    assert!(
        fixture
            .load()
            .err()
            .unwrap()
            .contains("ambiguous bootstrap operation: build")
    );
    fixture.projects["scripts"][0]["actions"]
        .as_object_mut()
        .unwrap()
        .remove("duplicate");
    fixture.config["tools"] = json!({});
    assert!(
        fixture
            .load()
            .err()
            .unwrap()
            .contains("requires a configured tool: prepare")
    );
    let bytes = serde_json::to_vec(&fixture.config).unwrap();
    assert!(Profile::decode(&fixture.root, &bytes).is_ok());
}
