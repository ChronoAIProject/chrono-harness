use chrono_ci::units;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

#[allow(dead_code)]
#[path = "support/tools.rs"]
mod tools;

fn provider() -> Value {
    let mut value: Value =
        serde_json::from_slice(include_bytes!("fixtures/native-provider.json")).unwrap();
    value["native_adoption"]["schema"] = json!("chrono-native-adoption/v2");
    let adoption = value["native_adoption"].as_object_mut().unwrap();
    adoption.remove("adapter_path");
    adoption.remove("interpreter");
    value
}

#[test]
fn native_observation_preserves_nanosecond_ordering_and_requires_utc() {
    use chrono_ci::native::validate_observation;
    for fraction in ["0", "1", "12345", "123456", "123456789"] {
        let time = format!("2026-01-01T00:00:00.{fraction}Z");
        assert!(
            validate_observation(&json!({"branch_started_at":time,"observed_at":time}), &time)
                .is_ok()
        );
    }
    let context = json!({"branch_started_at":"2026-01-01T00:00:00.123456788Z","observed_at":"2026-01-01T00:00:00.123456789Z"});
    assert!(validate_observation(&context, "2026-01-01T00:00:00.123456789+00:00").is_ok());
    assert!(validate_observation(&context, "2026-01-01T00:00:00.123456788Z").is_err());
    for (start, observed) in [
        (
            "2026-01-01T00:00:00.123456789Z",
            "2026-01-01T00:00:00.123456788Z",
        ),
        ("2026-01-01T00:00:00+08:00", "2026-01-01T00:00:00Z"),
        ("2026-01-01T00:00:00.1234567890Z", "2026-01-01T00:00:00Z"),
    ] {
        assert!(
            validate_observation(
                &json!({"branch_started_at":start,"observed_at":observed}),
                "2026-01-02T00:00:00Z"
            )
            .is_err()
        );
    }
}

fn read(root: &Path, value: &Value) -> Value {
    serde_json::from_slice(&fs::read(root.join(value["path"].as_str().unwrap())).unwrap()).unwrap()
}

#[test]
fn native_transport_keeps_actual_bytes_credentials_and_bounded_failures() {
    use chrono_ci::native::{Process, process};
    let host = tools::temporary_host("native transport λ ");
    let root = host.path();
    let bin = root.join("tools");
    fs::create_dir(&bin).unwrap();
    let tool = bin.join("declared-inputs");
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_chrono-ci-test-transport"), &tool).unwrap();
    let mut value = provider();
    value["native_adoption"]["inputs_program"] = json!("declared-inputs");
    let config: units::Config = serde_json::from_value(value).unwrap();
    fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    fs::write(
        root.join(".chrono-harness/config.json"),
        serde_json::to_vec(&json!({"environment":{"credential_environment":["SECRET"]}})).unwrap(),
    )
    .unwrap();
    let env = BTreeMap::from([
        ("PATH".into(), bin.to_str().unwrap().into()),
        ("BUSINESS_VALUE".into(), "actual value".into()),
        ("SECRET".into(), "actual credential".into()),
    ]);
    let request =
        |mode: &str, prefix: &str, timeout_seconds: f64, output_limit_bytes: usize| Process {
            argv: vec!["declared-inputs".into(), "native-child".into(), mode.into()],
            stdin: b"original input".to_vec(),
            directory: ".chrono-harness/state/collection/preparation/".into(),
            prefix: prefix.into(),
            timeout_seconds,
            output_limit_bytes,
        };
    let (raw, refs) = process(
        root,
        &config,
        &request("echo", "good", 5., 1024),
        env.clone(),
    )
    .unwrap();
    assert_eq!(raw, b"\0\xfforiginal input");
    let receipt = read(root, refs.last().unwrap());
    assert_eq!(receipt["argv"][0], json!(tool));
    assert_eq!(receipt["configured_argv"][0], "declared-inputs");
    assert_eq!(
        receipt["executable"]["sha256"],
        chrono_harness::sha256(&fs::read(&tool).unwrap())
    );
    assert_eq!(
        receipt["environment"]["BUSINESS_VALUE"],
        chrono_harness::sha256(b"actual value")
    );
    assert!(receipt["environment"].get("SECRET").is_none());
    assert_eq!(receipt["omitted_credentials"], json!(["SECRET"]));
    for (key, bytes) in [
        ("stdin", b"original input".as_slice()),
        ("stdout", raw.as_slice()),
        ("stderr", b"original stderr".as_slice()),
    ] {
        assert_eq!(
            fs::read(root.join(receipt[key]["path"].as_str().unwrap())).unwrap(),
            bytes
        );
    }
    let mut undeclared = request("echo", "undeclared", 5., 1024);
    undeclared.argv[0] = tool.to_str().unwrap().into();
    assert!(
        process(root, &config, &undeclared, env.clone())
            .unwrap_err()
            .contains("registered acquisition")
    );
    for (mode, timeout, failure) in [
        ("exit", 5., Value::Null),
        ("overflow", 5., json!("output bound exceeded")),
        ("timeout", 0.1, json!("timeout")),
    ] {
        let error = process(
            root,
            &config,
            &request(mode, mode, timeout, 32),
            env.clone(),
        )
        .unwrap_err();
        let path = error.split("retained at ").nth(1).expect(&error);
        let receipt: Value = serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap();
        assert_eq!(receipt["failure"], failure);
        assert!(
            fs::read(root.join(receipt["stdout"]["path"].as_str().unwrap()))
                .unwrap()
                .len()
                <= 32
        );
        if mode == "exit" {
            assert_eq!(receipt["exit_code"], 7);
            assert_eq!(
                fs::read(root.join(receipt["stderr"]["path"].as_str().unwrap())).unwrap(),
                b"real failure"
            );
        }
    }
}

#[test]
fn native_adoption_generates_rust_commands_without_an_interpreter_or_adapter() {
    let value = provider();
    let config: units::Config = serde_json::from_value(value.clone())
        .expect("native adoption v2 must require only the installed Rust tools");
    let outputs = units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    assert_eq!(outputs.len(), 1);
    let workflow = &outputs[".github/workflows/collection.yml"];
    for operation in ["publish", "acquire"] {
        assert!(
            workflow.contains(&format!("native-adoption {operation}")),
            "{workflow}"
        );
    }
    assert!(!workflow.contains("native.py"));
    assert!(!workflow.contains("python"));
    let mut obsolete = value;
    obsolete["native_adoption"]["interpreter"] = json!("/usr/bin/python3");
    assert!(serde_json::from_value::<units::Config>(obsolete).is_err());
}
