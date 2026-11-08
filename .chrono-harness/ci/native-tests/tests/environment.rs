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
    fn private(&mut self) {
        self.config["toolchain_provision"] = json!({"profile":"default", "rustup_home":".chrono-harness/state/private/rustup", "cargo_home":".chrono-harness/state/private/cargo"});
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
fn private_default_profile_uses_selected_homes_for_every_child_and_binds_the_compiler() {
    let mut fixture = Fixture::new();
    fixture.private();
    fixture
        .inherited
        .insert("RUSTC".into(), "/retired/compiler".into());
    let before = fixture.inherited.clone();
    let selected = fixture.select().unwrap();
    assert_eq!(fixture.inherited, before);
    assert_eq!(
        fixture.calls()[0],
        Fixture::call(&[
            "rustup",
            "toolchain",
            "install",
            "1.95.0",
            "--profile",
            "default",
            "--no-self-update"
        ])
    );
    assert_eq!(
        selected.environment.get(OsStr::new("RUSTC")).unwrap(),
        fixture.tools.join("rustc").as_os_str()
    );
    assert_eq!(selected.versions.rustc, "rustc 1.95.0 (fixture)");
    let traces = fs::read_to_string(fixture.tools.join("environments.jsonl")).unwrap();
    for line in traces.lines() {
        let trace: Value = serde_json::from_str(line).unwrap();
        assert_eq!(
            trace["environment"]["RUSTUP_HOME"],
            fixture
                .root
                .join(".chrono-harness/state/private/rustup")
                .to_str()
                .unwrap()
        );
        assert_eq!(
            trace["environment"]["CARGO_HOME"],
            fixture
                .root
                .join(".chrono-harness/state/private/cargo")
                .to_str()
                .unwrap()
        );
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
    let mut failure = Fixture::new();
    failure.private();
    fs::write(failure.tools.join("fail-install"), b"").unwrap();
    match failure.select().unwrap_err() {
        Failure::Installer(status) => assert_eq!(status.code(), Some(17)),
        other => panic!("unexpected failure: {other:?}"),
    }
    assert_eq!(failure.calls().len(), 1);
}

#[test]
fn private_probe_failure_never_falls_back_to_another_profile_or_selects_tools() {
    for (marker, count) in [("no-default-cargo", 2), ("no-default-fmt", 3)] {
        let mut fixture = Fixture::new();
        fixture.private();
        fs::write(fixture.tools.join(marker), b"").unwrap();
        assert!(
            fixture
                .select()
                .unwrap_err()
                .to_string()
                .contains("probe failed")
        );
        assert_eq!(fixture.calls().len(), count);
        assert!(
            !fixture
                .calls()
                .iter()
                .any(|call| call.get(1).map(String::as_str) == Some("which"))
        );
    }
}

#[test]
fn preparation_rejects_invalid_or_overlapping_declarations_before_process_or_directory_effects() {
    let mut fixture = Fixture::new();
    fixture.private();
    fixture.config["host_environment"] = json!({"HOME":".chrono-harness/cache/home"});
    let prepared = Prepared::parse(&fixture.root, &fixture.bytes(), &fixture.inherited).unwrap();
    assert_eq!(
        prepared.host_directories,
        vec![fixture.root.join(".chrono-harness/cache/home")]
    );
    assert!(!fixture.root.join(".chrono-harness").exists());
    let original = fixture.config.clone();
    for update in [
        json!({"environment":null}),
        json!({"environment":[]}),
        json!({"environment":{"":"value"}}),
        json!({"environment":{"RUSTC":null}}),
        json!({"environment":{"CHRONO_PROCESS_FDS":"7"}}),
        json!({"environment":{"bad=key":"value"}}),
        json!({"environment":{"key":"bad\0value"}}),
        json!({"environment":{"HOME":"duplicate"}}),
        json!({"host_environment":{}}),
        json!({"host_environment":{"PATH":".chrono-harness/cache/path"}}),
        json!({"host_environment":{"HOME":".chrono-harness/cache/../elsewhere"}}),
        json!({"host_environment":{"HOME":".chrono-harness/cache//home"}}),
        json!({"host_environment":{"TMPDIR":"/tmp"}}),
        json!({"host_environment":{"TMPDIR":"../outside"}}),
        json!({"host_environment":{"TMPDIR":".chrono-harness/bin/temporary"}}),
        json!({"host_environment":{"TMPDIR":".chrono-harness/state/owner-tmp"}}),
        json!({"toolchain_provision":{"profile":"minimal","rustup_home":".chrono-harness/state/r","cargo_home":".chrono-harness/state/c"}}),
        json!({"toolchain_provision":{"profile":{"default":null},"rustup_home":".chrono-harness/state/r","cargo_home":".chrono-harness/state/c"}}),
        json!({"toolchain_provision":{"profile":"default","rustup_home":".chrono-harness/state/r","cargo_home":".chrono-harness/state/r/c"}}),
        json!({"toolchain_provision":{"profile":"default","rustup_home":".chrono-harness/state/r","cargo_home":".chrono-harness/state/r"}}),
        json!({"toolchain_provision":null}),
        json!({"toolchain_provision":{}}),
        json!({"toolchain_provision":[]}),
        json!({"toolchain_provision":{"profile":"default","rustup_home":".chrono-harness/state/r","cargo_home":null}}),
        json!({"toolchain_provision":{"profile":"default","rustup_home":"/tmp/global-rust","cargo_home":".chrono-harness/state/c"}}),
        json!({"toolchain_provision":{"profile":"default","rustup_home":".chrono-harness/state/../r","cargo_home":".chrono-harness/state/c"}}),
        json!({"toolchain_provision":{"profile":"default","rustup_home":".chrono-harness/state/r","cargo_home":".chrono-harness/state/c","extra":true}}),
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

    let mut private = Fixture::new();
    private.private();
    fs::create_dir_all(private.root.join(".chrono-harness/state")).unwrap();
    std::os::unix::fs::symlink(
        &private.tools,
        private.root.join(".chrono-harness/state/private"),
    )
    .unwrap();
    assert!(private.select().is_err());
    assert!(!private.tools.join("calls.json").exists());
}

#[test]
fn explicit_source_environment_is_selected_before_private_homes_without_mutating_inherited_values()
{
    let mut fixture = Fixture::new();
    let configured = json!({"CARGO_HOME":"/source/cargo","RUSTUP_HOME":"/source/rustup",
        "RUSTC":"/source/rustc","RUSTFLAGS":"--sysroot=/source/sysroot"});
    fixture.config["environment"] = configured.clone();
    let before = fixture.inherited.clone();
    let prepared = Prepared::parse(&fixture.root, &fixture.bytes(), &fixture.inherited).unwrap();
    for (key, value) in configured.as_object().unwrap() {
        assert_eq!(
            prepared.environment.get(OsStr::new(key)).unwrap(),
            OsStr::new(value.as_str().unwrap())
        );
    }
    fixture.private();
    let private = Prepared::parse(&fixture.root, &fixture.bytes(), &fixture.inherited).unwrap();
    assert_eq!(
        private.environment.get(OsStr::new("RUSTUP_HOME")).unwrap(),
        fixture
            .root
            .join(".chrono-harness/state/private/rustup")
            .as_os_str()
    );
    assert_eq!(
        private.environment.get(OsStr::new("CARGO_HOME")).unwrap(),
        fixture
            .root
            .join(".chrono-harness/state/private/cargo")
            .as_os_str()
    );
    assert_eq!(private.environment.get(OsStr::new("RUSTC")).unwrap(), "");
    assert_eq!(fixture.inherited, before);
    assert!(!fixture.tools.join("calls.json").exists());
    assert!(!fixture.root.join(".chrono-harness").exists());
}

#[test]
fn ordinary_registered_cargo_commands_keep_the_selected_home_and_exact_locked_inputs() {
    use chrono_host_bootstrap::execution::{methods, run_operation};
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let read = |path: &str| -> Value {
        serde_json::from_slice(&fs::read(repository.join(path)).unwrap()).unwrap()
    };
    let selector = read(".chrono-harness/config.json");
    let selected: Vec<_> = selector["platforms"]["macos-aarch64"]["configurations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["path"] == ".chrono-harness/git/pinned-macos.json")
        .collect();
    assert_eq!(selected.len(), 1);
    let config = read(selected[0]["path"].as_str().unwrap());
    assert!(
        !config["environment"]["inherit"]
            .as_array()
            .unwrap()
            .iter()
            .any(|key| key == "CARGO_NET_OFFLINE")
    );
    let registry = methods(&read(".chrono-harness/projects.json")).unwrap();
    let cache = read(".chrono-harness/ci/bootstrap-cache.json");
    assert_eq!(cache["operations"], json!(["build.cache"]));
    let mut fixture = Fixture::new();
    fixture
        .inherited
        .insert("HOME".into(), "/Users/runner".into());
    fixture
        .inherited
        .insert("CARGO_HOME".into(), "/Users/runner/.cargo".into());
    fixture.inherited.remove(OsStr::new("CARGO_NET_OFFLINE"));
    fixture
        .inherited
        .insert("CHRONO_TEST_CARGO_DOMAIN".into(), "1".into());
    let declared = json!({"rust_toolchain":cache["rust_toolchain"],"environment":config["environment"]["values"]});
    let environment = Prepared::parse(
        &fixture.root,
        &serde_json::to_vec(&declared).unwrap(),
        &fixture.inherited,
    )
    .unwrap()
    .environment;
    let cache_environment = Prepared::parse(
        &fixture.root,
        &serde_json::to_vec(&cache).unwrap(),
        &environment,
    )
    .unwrap()
    .environment;
    let originals = fixture.root.join(".chrono-harness/state/cargo-originals");
    fs::create_dir_all(&originals).unwrap();
    for (index, operation) in [
        "build.runner",
        "build.cache",
        "build.distribution",
        "build.instructions",
        "build.judge-routes",
        "build.cache",
    ]
    .iter()
    .enumerate()
    {
        let methods = &registry[*operation];
        assert_eq!(methods.len(), 1);
        let method = &methods[0];
        assert_eq!(method.tool, "cargo");
        let manifest = &method.argv[method
            .argv
            .iter()
            .position(|arg| arg == "--manifest-path")
            .unwrap()
            + 1];
        let lock = std::path::Path::new(manifest).with_file_name("Cargo.lock");
        let mut locked = fs::read(repository.join(manifest)).unwrap();
        locked.push(0);
        locked.extend(fs::read(repository.join(&lock)).unwrap());
        for path in [std::path::Path::new(manifest), lock.as_path()] {
            let target = fixture.root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(repository.join(path), target).unwrap();
        }
        let selected = if index == 5 {
            &cache_environment
        } else {
            &environment
        };
        let result = run_operation(
            &fixture.root,
            &originals,
            index,
            method,
            &fixture.tools.join("cargo"),
            selected,
            vec![],
            vec![],
        )
        .unwrap();
        assert_eq!(result.exit_code, 0);
        let stdout = fs::read(fixture.root.join(&result.stdout.path)).unwrap();
        let observed: Value = serde_json::from_slice(&stdout).unwrap();
        assert_eq!(observed["cargo_home"], "/Users/runner/.cargo");
        assert_eq!(observed["offline"], false);
        assert_eq!(observed["manifest"], manifest.as_str());
        assert_eq!(observed["locked_input"], chrono_harness::sha256(&locked));
        assert_eq!(observed["argv"], json!(method.argv));
        assert_eq!(result.stdout.sha256, chrono_harness::sha256(&stdout));
    }
}
