use serde_json::{Value, json};
use std::{fs, process::Command};

fn execute(root: &std::path::Path, program: &Value) -> std::process::Output {
    fs::write(root.join("check.case"), serde_json::to_vec(program).unwrap()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_chrono-test-project-execution"))
        .current_dir(root)
        .env_clear()
        .env("EMPTY", "")
        .env("DECLARED_EMPTY", "")
        .env("FACTS_GIT", env!("CARGO_BIN_EXE_chrono-test-project-execution"))
        .args(["file", "check.case", "$HOME `literal` <tag>", "", "line\nnext"])
        .output()
        .unwrap()
}

#[test]
fn file_program_consumes_product_order_arguments_and_environment() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("product.case"), br#""DoubleAdd""#).unwrap();
    fs::write(root.path().join("order"), b"ptp").unwrap();
    let output = execute(
        root.path(),
        &json!([
            {"CheckProduct":{"path":"product.case","input":21,"expected":42}},
            {"RequireSuffix":{"path":"order","bytes":[112]}},
            "ObserveProcess",
            {"Append":{"path":"order","bytes":[116]}}
        ]),
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let observation: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        observation,
        json!([
            ["$HOME `literal` <tag>", "", "line\nnext"],
            fs::canonicalize(root.path()).unwrap(),
            "",
            null
        ])
    );
    assert_eq!(fs::read(root.path().join("order")).unwrap(), b"ptpt");
}

#[test]
fn product_and_preparation_failures_stop_before_later_effects() {
    for (product, order) in [("Answer", b"p".as_slice()), ("DoubleMultiply", b"pt")] {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("product.case"),
            serde_json::to_vec(product).unwrap(),
        )
        .unwrap();
        fs::write(root.path().join("order"), order).unwrap();
        let output = execute(
            root.path(),
            &json!([
                {"CheckProduct":{"path":"product.case","input":20,"expected":40}},
                {"RequireSuffix":{"path":"order","bytes":[112]}},
                {"Create":{"path":"ran","bytes":[]}}
            ]),
        );
        assert!(!output.status.success(), "{product}: {output:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!root.path().join("ran").exists());
    }
}

#[test]
fn environment_observation_and_write_keep_actual_exit_and_bytes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("input"), b"before").unwrap();
    let output = execute(
        root.path(),
        &json!([
            {"Write":{"path":"input","bytes":[97,102,116,101,114]}},
            {"ObserveEnvironment":{"names":["ABSENT","EMPTY"]}},
            {"Exit":9},
            {"Create":{"path":"not-reached","bytes":[]}}
        ]),
    );
    assert_eq!(output.status.code(), Some(9), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout).unwrap(), json!([null, ""]));
    assert_eq!(fs::read(root.path().join("input")).unwrap(), b"after");
    assert!(!root.path().join("not-reached").exists());
}

#[test]
fn shared_preparation_count_is_checked_in_the_child() {
    for (order, success) in [("pt", true), ("ppt", false), ("t", false)] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("order"), order).unwrap();
        let output = execute(
            root.path(),
            &json!([{"RequireCount":{"path":"order","byte":112,"count":1}}]),
        );
        assert_eq!(output.status.success(), success, "{order}: {output:?}");
    }
}

#[test]
fn invalid_program_and_product_are_rejected_without_effects() {
    for (program, product) in [
        (json!({"unknown":"program"}), "DoubleAdd"),
        (json!([{"CheckProduct":{"path":"product.case","input":21,"expected":42}}]), "UnknownProduct"),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("product.case"), serde_json::to_vec(product).unwrap()).unwrap();
        let output = execute(root.path(), &program);
        assert!(!output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn native_input_probe_uses_the_declared_tool_and_observes_actual_absence() {
    let root = tempfile::tempdir().unwrap();
    let program = json!([{"CheckNativeInputs":{"path":"external"}}]);
    let absent = execute(root.path(), &program);
    assert!(absent.status.success(), "{absent:?}");
    assert_eq!(absent.stdout, b"data-present False\n");
    fs::write(root.path().join("external"), b"input").unwrap();
    let present = execute(root.path(), &program);
    assert!(present.status.success(), "{present:?}");
    assert_eq!(present.stdout, b"data-present True\n");
    let rejected = Command::new(env!("CARGO_BIN_EXE_chrono-test-project-execution"))
        .current_dir(root.path())
        .env_clear()
        .env("FACTS_GIT", env!("CARGO_BIN_EXE_chrono-test-project-execution"))
        .args(["file", "check.case"])
        .output()
        .unwrap();
    assert!(!rejected.status.success(), "{rejected:?}");
    assert!(rejected.stdout.is_empty());
}
