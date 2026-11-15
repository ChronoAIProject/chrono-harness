use chrono_host_bootstrap::{
    bindings::{Environment, NativePlatform},
    environment::{self, Failure, Prepared},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, fs, path::PathBuf};

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    tools: PathBuf,
    inherited: Environment,
    configured: BTreeMap<String, String>,
    config: Value,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        let tools = root.join("native tools");
        fs::create_dir(&tools).unwrap();
        let mut configured = BTreeMap::new();
        let mut bindings = serde_json::Map::new();
        for name in ["rustup", "cargo", "rustc"] {
            let path = tools.join(name);
            fs::copy(env!("CARGO_BIN_EXE_chrono-test-bootstrap-tool"), &path).unwrap();
            if name != "rustup" {
                configured.insert(name.into(), path.to_str().unwrap().into());
                bindings.insert(name.into(), json!({"path":path,"sha256":chrono_harness::file_identity(&path).unwrap().0,"version":format!("{name} 1.95.0 (fixture)")}));
            }
        }
        let mut inherited: Environment = std::env::vars_os().collect();
        inherited.insert("PATH".into(), tools.as_os_str().to_owned());
        for key in ["RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            inherited.remove(OsStr::new(key));
        }
        let config = json!({"rust_toolchain":"1.95.0","require_tool_bindings":true,
            "native_platform":{"system":"fixture","machine":"native"}, "tool_bindings":bindings});
        Self {
            _directory: directory,
            root,
            tools,
            inherited,
            configured,
            config,
        }
    }
    fn native(&self) -> NativePlatform {
        NativePlatform {
            system: "fixture".into(),
            machine: "native".into(),
        }
    }
    fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.config).unwrap()
    }
    fn select(&self) -> Result<environment::Selected, Failure> {
        environment::select(
            &self.root,
            &self.bytes(),
            &self.native(),
            None,
            &self.inherited,
            &self.configured,
        )
    }
    fn calls(&self) -> Vec<Vec<String>> {
        serde_json::from_slice(&fs::read(self.tools.join("calls.json")).unwrap()).unwrap()
    }
    fn call(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).into()).collect()
    }
}

#[test]
fn ordinary_selection_installs_minimal_with_rustfmt_and_preserves_installer_exit() {
    let mut fixture = Fixture::new();
    fixture.config["rust_incremental"] = json!(false);
    let selected = fixture.select().unwrap();
    assert_eq!(
        fixture.calls()[0],
        Fixture::call(&["rustup", "run", "1.95.0", "cargo", "--version"])
    );
    assert!(fixture.calls().contains(&Fixture::call(&[
        "rustup",
        "toolchain",
        "install",
        "1.95.0",
        "--profile",
        "minimal",
        "--component",
        "rustfmt"
    ])));
    assert_eq!(
        selected
            .environment
            .get(OsStr::new("CARGO_INCREMENTAL"))
            .unwrap(),
        "0"
    );
    assert_eq!(
        selected
            .environment
            .get(OsStr::new("CARGO_TERM_COLOR"))
            .unwrap(),
        "never"
    );
    let failure = Fixture::new();
    fs::write(failure.tools.join("fail-install"), b"").unwrap();
    match failure.select().unwrap_err() {
        Failure::Installer(status) => assert_eq!(status.code(), Some(17)),
        other => panic!("unexpected failure: {other:?}"),
    }
    assert_eq!(failure.calls().len(), 2);
}

#[test]
fn preparation_rejects_invalid_or_conflicting_environment_before_process_or_directory_effects() {
    let mut fixture = Fixture::new();
    fixture.config["host_environment"] = json!({"HOME":".chrono-harness/cache/home"});
    let prepared = Prepared::parse(&fixture.root, &fixture.bytes(), &fixture.inherited).unwrap();
    assert_eq!(
        prepared.host_directories,
        vec![fixture.root.join(".chrono-harness/cache/home")]
    );
    assert!(!fixture.root.join(".chrono-harness").exists());
    let original = fixture.config.clone();
    for update in [
        json!({"environment":{"CHRONO_PROCESS_FDS":"7"}}),
        json!({"environment":{"bad=key":"value"}}),
        json!({"environment":{"key":"bad\0value"}}),
        json!({"environment":{"HOME":"duplicate"}}),
        json!({"host_environment":{}}),
        json!({"host_environment":{"PATH":".chrono-harness/cache/path"}}),
        json!({"host_environment":{"HOME":".chrono-harness/cache/../elsewhere"}}),
        json!({"host_environment":{"HOME":".chrono-harness/cache//home"}}),
        json!({"rust_incremental":"false"}),
    ] {
        fixture.config = original.clone();
        for (key, value) in update.as_object().unwrap() {
            fixture.config[key] = value.clone();
        }
        assert!(fixture.select().is_err(), "accepted {}", fixture.config);
        assert!(!fixture.tools.join("calls.json").exists());
        assert!(!fixture.root.join(".chrono-harness").exists());
    }
    fixture.config = original;
    fs::create_dir_all(fixture.root.join(".chrono-harness/cache")).unwrap();
    std::os::unix::fs::symlink(
        &fixture.tools,
        fixture.root.join(".chrono-harness/cache/home"),
    )
    .unwrap();
    assert!(fixture.select().is_err());
    assert!(!fixture.tools.join("calls.json").exists());
}
