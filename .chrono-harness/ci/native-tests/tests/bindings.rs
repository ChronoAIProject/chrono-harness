use chrono_host_bootstrap::bindings::{Declarations, Environment, NativePlatform};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

fn native() -> NativePlatform {
    NativePlatform {
        system: "fixture-system".into(),
        machine: "fixture-machine".into(),
    }
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn direct() -> Value {
    let binding = json!({"path":"old/tool", "sha256":"0".repeat(64), "version":"fixture", "allow_relocation":true});
    json!({"require_tool_bindings":true,
        "native_platform":{"system":"fixture-system", "machine":"fixture-machine"},
        "tool_bindings":{"cargo":binding, "rustc":binding}})
}

#[test]
fn current_json_boundary_preserves_required_bindings_and_rejects_ambiguity() {
    assert!(
        Declarations::parse(b"{}", &native(), None)
            .unwrap()
            .is_none()
    );
    assert!(
        Declarations::parse(&bytes(&direct()), &native(), None)
            .unwrap()
            .is_some()
    );
    for config in [
        json!({"require_tool_bindings":true}),
        json!({"require_tool_bindings":1}),
        json!({"tool_bindings":null}),
        json!({"tool_bindings_registry":"registry", "tool_bindings":{}}),
        json!({"tool_bindings_registry":"registry", "native_platform":{}}),
    ] {
        assert!(
            Declarations::parse(&bytes(&config), &native(), None).is_err(),
            "accepted {config}"
        );
    }
    for (pointer, value) in [
        ("/native_platform/machine", json!("another-machine")),
        ("/tool_bindings/cargo/sha256", json!("A".repeat(64))),
        ("/tool_bindings/rustc/sha256", json!("0".repeat(63))),
        ("/tool_bindings/cargo/allow_relocation", json!("true")),
        ("/tool_bindings/rustc/path", Value::Null),
    ] {
        let mut config = direct();
        *config.pointer_mut(pointer).unwrap() = value;
        assert!(
            Declarations::parse(&bytes(&config), &native(), None).is_err(),
            "accepted {config}"
        );
    }
    assert!(
        Declarations::parse(
            b"{\"require_tool_bindings\":false,\"require_tool_bindings\":true}",
            &native(),
            None
        )
        .is_err()
    );
}

#[test]
fn registry_reads_only_the_explicit_platform_and_keeps_exact_tool_keys() {
    let config = bytes(&json!({"rust_toolchain":"1.95.0", "tool_bindings_registry":"registered"}));
    let tool = json!({"sha256":"0".repeat(64), "version":"fixture", "component":{"url":"unused-source-data"}});
    let original = json!({"schema":"chrono-bootstrap-toolchains/v1", "rust_toolchain":"1.95.0",
        "platforms":{"unselected":null, "fixture-system/fixture-machine":{"tools":{"cargo":tool, "rustc":tool}}}});
    let declarations = Declarations::parse(&config, &native(), Some(&bytes(&original)))
        .unwrap()
        .unwrap();
    assert!(declarations.tools.cargo.allow_relocation);
    assert!(declarations.tools.cargo.path.is_none());
    for (pointer, value) in [
        ("/schema", json!("another-schema")),
        ("/rust_toolchain", json!("another-toolchain")),
        (
            "/platforms/fixture-system~1fixture-machine/tools",
            json!({"cargo":tool}),
        ),
        (
            "/platforms/fixture-system~1fixture-machine/tools",
            json!({"cargo":tool, "rustc":tool, "extra":tool}),
        ),
    ] {
        let mut registry = original.clone();
        *registry.pointer_mut(pointer).unwrap() = value;
        assert!(Declarations::parse(&config, &native(), Some(&bytes(&registry))).is_err());
    }
    assert!(Declarations::parse(&config, &native(), None).is_err());
}

fn selected(rustup: &PathBuf, name: &str) -> PathBuf {
    let result = Command::new(rustup)
        .args(["which", "--toolchain", "1.95.0", name])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    PathBuf::from(String::from_utf8(result.stdout).unwrap().trim())
}

fn real_binding(path: &PathBuf) -> Value {
    let version = Command::new(path)
        .arg("--version")
        .env("RUSTUP_TOOLCHAIN", "1.95.0")
        .output()
        .unwrap();
    assert!(version.status.success());
    json!({"path":path, "sha256":chrono_harness::file_identity(path).unwrap().0,
        "version":String::from_utf8(version.stdout).unwrap().trim()})
}

#[test]
fn actual_rustup_tools_keep_identity_relocation_and_inherited_compiler_constraints() {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    let rustup =
        chrono_harness::resolve_program(&root, "rustup", std::env::var("PATH").ok().as_deref())
            .unwrap();
    let cargo = selected(&rustup, "cargo");
    let rustc = selected(&rustup, "rustc");
    let mut config = direct();
    config["tool_bindings"]["cargo"] = real_binding(&cargo);
    config["tool_bindings"]["rustc"] = real_binding(&rustc);
    let configured = BTreeMap::from([
        ("cargo".into(), cargo.to_str().unwrap().into()),
        ("rustc".into(), rustc.to_str().unwrap().into()),
    ]);
    let mut environment: Environment = std::env::vars_os().collect();
    environment.insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    environment.insert("RUSTC".into(), rustc.as_os_str().to_owned());
    environment.insert("RUSTC_WRAPPER".into(), "".into());
    environment.insert("RUSTC_WORKSPACE_WRAPPER".into(), "".into());
    let declarations = Declarations::parse(&bytes(&config), &native(), None)
        .unwrap()
        .unwrap();
    let result = declarations
        .select(&root, &rustup, "1.95.0", &configured, &environment)
        .unwrap();
    assert_eq!(result.cargo.path, cargo.to_str().unwrap());
    assert_eq!(
        result.rustc.sha256,
        config["tool_bindings"]["rustc"]["sha256"]
    );

    for key in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
        let mut wrapped = environment.clone();
        wrapped.insert(key.into(), "must-not-run".into());
        assert!(
            declarations
                .select(&root, &rustup, "1.95.0", &configured, &wrapped)
                .unwrap_err()
                .contains("wrapper")
        );
    }
    let mut wrong_compiler = environment.clone();
    wrong_compiler.insert("RUSTC".into(), cargo.as_os_str().to_owned());
    assert!(
        declarations
            .select(&root, &rustup, "1.95.0", &configured, &wrong_compiler)
            .unwrap_err()
            .contains("compiler")
    );

    let mut mismatch = config.clone();
    mismatch["tool_bindings"]["cargo"]["sha256"] = json!("0".repeat(64));
    let bad = Declarations::parse(&bytes(&mismatch), &native(), None)
        .unwrap()
        .unwrap();
    assert!(
        bad.select(&root, &rustup, "1.95.0", &configured, &environment)
            .unwrap_err()
            .contains("identity mismatch")
    );

    config["tool_bindings"]["cargo"]["path"] = json!("retired/cargo");
    config["tool_bindings"]["rustc"]["path"] = json!("retired/rustc");
    for name in ["cargo", "rustc"] {
        config["tool_bindings"][name]["allow_relocation"] = json!(true);
    }
    let retired = BTreeMap::from([
        ("cargo".into(), "retired/cargo".into()),
        ("rustc".into(), "retired/rustc".into()),
    ]);
    let relocatable = Declarations::parse(&bytes(&config), &native(), None)
        .unwrap()
        .unwrap();
    assert!(
        relocatable
            .select(&root, &rustup, "1.95.0", &retired, &environment)
            .is_ok()
    );
    config["tool_bindings"]["cargo"]["allow_relocation"] = json!(false);
    let fixed = Declarations::parse(&bytes(&config), &native(), None)
        .unwrap()
        .unwrap();
    assert!(
        fixed
            .select(&root, &rustup, "1.95.0", &retired, &environment)
            .unwrap_err()
            .contains("relocation")
    );
}
