#[path = "support/native_executable.rs"]
mod native_executable;

use chrono_harness::{CommandSpec, Status, dispatch, run_process};
use serde_json::{Value, json};

use std::fs;
use tempfile::TempDir;

const CHILD: &str = env!("CARGO_BIN_EXE_chrono-test-cli-child");

#[test]
fn report_reference_publication_is_an_explicit_scoped_v3_contract() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("check.json");
    let mut config = serde_json::json!({"schema":"chrono-ci-check/v3","judge":{"program":CHILD,"args":[],"timeout_seconds":15,"output_limit_bytes":4096},"policy":{},"report_path":".chrono-harness/state/check.json"});
    fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    assert!(
        !chrono_harness::units::reference_publication(&chrono_harness::load_config(&path).unwrap())
            .unwrap()
    );
    for schema in [
        "chrono-ci-check/v1",
        "chrono-ci-check/v2",
        "chrono-ci-check/v3",
    ] {
        for value in [
            Value::Null,
            Value::Bool(true),
            serde_json::json!("unknown"),
            serde_json::json!("retained-reference/v1"),
            serde_json::json!("retained-reference/v2"),
        ] {
            config["schema"] = schema.into();
            config["policy"]["report_publication"] = value.clone();
            fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
            let loaded = chrono_harness::load_config(&path);
            assert_eq!(
                loaded.is_ok(),
                schema == "chrono-ci-check/v3"
                    && matches!(
                        value.as_str(),
                        Some("retained-reference/v1" | "retained-reference/v2")
                    )
            );
            if let Ok(config) = loaded {
                assert!(chrono_harness::units::reference_publication(&config).unwrap());
            }
        }
    }
}

#[test]
fn scoped_stream_storage_preserves_raw_process_identity_and_detects_bad_originals() {
    use chrono_harness::{prepared, scoped_report};
    let dir = tempfile::tempdir().unwrap();
    let process = run_process(
        dir.path(),
        &CommandSpec {
            program: CHILD.into(),
            args: vec!["pipe".into()],
            env: Default::default(),
            timeout_seconds: 15,
            output_limit_bytes: 4096,
        },
        b"\x00\xff\xf0\x80",
    )
    .unwrap();
    assert_eq!(process.stdout_bytes, b"EOF:\x00\xff\xf0\x80");
    assert_eq!(process.stderr_bytes, b"joined");
    let record =
        scoped_report::retain_process(dir.path(), ".chrono-harness/state/preparation/", &process)
            .unwrap();
    for key in ["stdout", "stderr", "stdout_bytes", "stderr_bytes"] {
        assert!(record.get(key).is_none());
    }
    let restored =
        scoped_report::restore_process(&record, |o| prepared::read_original(dir.path(), o, None))
            .unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&process).unwrap()
    );
    let repeated =
        scoped_report::retain_process(dir.path(), ".chrono-harness/state/preparation/", &process)
            .unwrap();
    assert_eq!(record, repeated);
    for stream in ["stdout", "stderr"] {
        let key = format!("{stream}_original");
        let path = dir.path().join(record[&key]["path"].as_str().unwrap());
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"damaged bytes").unwrap();
        assert!(
            scoped_report::restore_process(&record, |o| prepared::read_original(
                dir.path(),
                o,
                None
            ))
            .is_err()
        );
        let error = scoped_report::retain_process(
            dir.path(),
            ".chrono-harness/state/preparation/",
            &process,
        )
        .unwrap_err();
        assert!(
            error.contains(&format!("cannot retain judge {stream}")) && error.contains("exit 0"),
            "{error}"
        );
        fs::write(&path, &original).unwrap();
        let mut changed = record.clone();
        changed[format!("{stream}_sha256")] = json!("0".repeat(64));
        assert!(
            scoped_report::restore_process(&changed, |o| prepared::read_original(
                dir.path(),
                o,
                None
            ))
            .unwrap_err()
            .contains("digest mismatch")
        );
        let mut changed = record.clone();
        changed[format!("{stream}_bytes")] = json!([]);
        assert!(
            scoped_report::restore_process(&changed, |o| prepared::read_original(
                dir.path(),
                o,
                None
            ))
            .unwrap_err()
            .contains("cannot contain inline streams")
        );
    }
}

#[test]
#[cfg(unix)]
fn invalid_utf8_cli_argument_emits_structured_domain_error_on_stderr() {
    use std::os::unix::ffi::OsStringExt;
    let binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../runner/target/debug/chrono-harness");
    let output = std::process::Command::new(binary)
        .arg(std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let text = std::str::from_utf8(&output.stderr).unwrap();
    assert_eq!(text.lines().count(), 1);
    let record: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(record["schema"], "chrono-log/v1");
    assert_eq!(record["event"], "cli.invalid_argument");
    assert_eq!(record["error"]["schema"], "chrono-error/v1");
    assert_eq!(record["error"]["error"]["code"], "E_USAGE");
    assert_eq!(record["error"]["error"]["source"], serde_json::Value::Null);
    assert_eq!(record["error"]["error"]["context"]["argument_index"], 0);
    assert_eq!(
        record["error"]["error"]["context"]["argument_bytes"],
        json!([255])
    );
}

#[test]
fn standard_sha256_vectors_match_memory_stream_and_file() {
    use std::io::{Cursor, Read};
    struct Chunks(Cursor<Vec<u8>>);
    impl Read for Chunks {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let length = buffer.len().min(63);
            self.0.read(&mut buffer[..length])
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input");
    for (bytes, expected) in [
        (
            vec![],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            b"abc".to_vec(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".to_vec(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
        (
            vec![b'a'; 1_000_000],
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
        ),
    ] {
        assert_eq!(chrono_harness::sha256(&bytes), expected);
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            chrono_harness::file_identity(&path).unwrap(),
            (expected.into(), bytes.len() as u64)
        );
        let mut copied = Vec::new();
        let identity =
            chrono_harness::copy_hashed(Chunks(Cursor::new(bytes.clone())), &mut copied).unwrap();
        assert_eq!(identity, (expected.into(), bytes.len() as u64));
        assert_eq!(copied, bytes);
    }
}

fn fixture(case: Value) -> (TempDir, String) {
    let dir = tempfile::tempdir_in(std::path::Path::new(CHILD).parent().unwrap()).unwrap();
    fs::create_dir_all(dir.path().join(".chrono-harness/ci")).unwrap();
    fs::write(
        dir.path().join("cli-case.json"),
        serde_json::to_vec(&case).unwrap(),
    )
    .unwrap();
    let path = dir.path().join(".chrono-harness/ci/check.json");
    fs::write(&path,serde_json::to_vec(&json!({"schema":"chrono-ci-check/v1","judge":{"program":CHILD,"args":["judge"],"timeout_seconds":15,"output_limit_bytes":4096},"policy":{},"report_path":".chrono-harness/state/check.json"})).unwrap()).unwrap();
    let name = path.to_str().unwrap().to_string();
    (dir, name)
}
fn run(path: &str) -> chrono_harness::CliOutput {
    dispatch(&[
        "check",
        "--config",
        path,
        "--base",
        &"a".repeat(40),
        "--candidate",
        &"b".repeat(40),
    ])
}

#[test]
fn real_child_and_report_publish() {
    let (dir, p) = fixture(json!({}));
    let r = run(&p);
    assert_eq!(r.exit_code, 0, "{}", r.stderr);
    let v: serde_json::Value = serde_json::from_str(&r.stdout).unwrap();
    assert_eq!(v["judge"]["exit_code"], 0);
    assert_eq!(v["response"]["results"][0]["id"], "external");
    assert!(
        dir.path()
            .join(".chrono-harness/state/check.json")
            .is_file()
    );
}

#[test]
fn unit_report_compacts_formatting_and_preserves_original_process_bytes() {
    let (dir, p) = fixture(
        json!({"fields":{"protocol":"chrono-ci-judge/v2", "evidence":{
        "bytes":(0..=255).cycle().take(8192).collect::<Vec<u8>>()}}}),
    );
    let mut config: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    config["schema"] = json!("chrono-ci-check/v3");
    config["judge"]["output_limit_bytes"] = json!(1024 * 1024);
    config["policy"]["units"] =
        json!({"one":{"tests":["test:one"],"report_path":".chrono-harness/state/one/check.json"}});
    fs::write(&p, serde_json::to_vec(&config).unwrap()).unwrap();
    let r = dispatch(&[
        "check",
        "--config",
        &p,
        "--base",
        &"a".repeat(40),
        "--candidate",
        &"b".repeat(40),
        "--unit",
        "one",
    ]);
    assert_eq!(r.exit_code, 0, "{}", r.stderr);
    assert_eq!(
        r.stdout.lines().count(),
        1,
        "byte arrays must not acquire per-byte indentation"
    );
    assert_eq!(
        fs::read(dir.path().join(".chrono-harness/state/one/check.json")).unwrap(),
        r.stdout.as_bytes()
    );
    let report: serde_json::Value = serde_json::from_str(&r.stdout).unwrap();
    let original: Vec<u8> =
        serde_json::from_value(report["judge"]["stdout_bytes"].clone()).unwrap();
    assert_eq!(
        report["judge"]["stdout_sha256"],
        chrono_harness::sha256(&original)
    );
    assert_eq!(
        report["judge"]["stdout"],
        std::str::from_utf8(&original).unwrap()
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&original).unwrap(),
        report["response"]
    );
    let preserved: Vec<u8> =
        serde_json::from_value(report["response"]["evidence"]["bytes"].clone()).unwrap();
    assert_eq!(preserved, (0..=255).cycle().take(8192).collect::<Vec<u8>>());
}

#[test]
fn full_unit_selector_rejects_legacy_profile_before_launch() {
    let (_dir, p) = fixture(json!({}));
    let mut config: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    config["schema"] = serde_json::Value::Null;
    config["schema_version"] = json!(2);
    fs::write(&p, serde_json::to_vec(&config).unwrap()).unwrap();
    let r = dispatch(&[
        "check",
        "--config",
        &p,
        "--base",
        &"a".repeat(40),
        "--candidate",
        &"b".repeat(40),
        "--unit",
        "one",
    ]);
    assert_eq!(r.exit_code, 2);
    assert!(r.stderr.contains("full-v3"));
}

#[test]
fn full_execution_units_reject_duplicate_report_paths() {
    let profile = json!({
        "schema_version": 3,
        "execution_units": {
            "units": {
                "one": {"tests": ["test:one"], "report_path": ".chrono-harness/state/unit.json"},
                "two": {"tests": ["test:two"], "report_path": ".chrono-harness/state/unit.json"}
            },
            "shared_operations": {},
            "collection_limits": {"manifest_bytes": 1024, "report_bytes": 1024},
            "report_path": ".chrono-harness/state/collection.json"
        }
    });
    let error = chrono_harness::units::full_execution_units(&profile).unwrap_err();
    assert!(error.contains("report paths overlap"), "{error}");
}

#[test]
fn publication_preflight_rejects_symlink_and_parent_type_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".chrono-harness/state")).unwrap();
    fs::write(dir.path().join(".chrono-harness/state/file"), b"blocked").unwrap();
    let error =
        chrono_harness::prepare_publication(dir.path(), ".chrono-harness/state/file/report.json")
            .unwrap_err();
    assert!(
        error.contains("publication parent is not a directory"),
        "{error}"
    );
    std::os::unix::fs::symlink(
        "report.json",
        dir.path().join(".chrono-harness/state/link.json"),
    )
    .unwrap();
    let error = chrono_harness::prepare_publication(dir.path(), ".chrono-harness/state/link.json")
        .unwrap_err();
    assert!(error.contains("symlink path is not allowed"), "{error}");
}

#[test]
fn actual_failure_is_preserved() {
    let (_dir, p) = fixture(json!({"exit":7,"fields":{"status":"failed","results":[{
        "id":"external","status":"failed","cause":"real execution","exit_code":7}]}}));
    let r = run(&p);
    assert_eq!(r.exit_code, 1);
    assert!(r.stdout.contains("registered") == false);
    assert!(r.stdout.contains("\"exit_code\": 7"));
}
#[test]
fn malformed_empty_and_identity_mismatch_fail() {
    for case in [
        json!({"kind":"raw","text":""}),
        json!({"kind":"raw","text":"{}\n"}),
        json!({"kind":"raw","text":"not-json\n"}),
        json!({"fields":{"request_id":"other"}}),
        json!({"fields":{"protocol":"wrong"}}),
        json!({"fields":{"results":[]}}),
        json!({"extra_json":true}),
    ] {
        let (_dir, p) = fixture(case.clone());
        let r = run(&p);
        assert_eq!(r.exit_code, 2, "{case}: {r:?}");
        assert!(r.stdout.contains("transport_failure"));
    }
}
#[test]
fn exit_status_mismatch_fails() {
    for case in [
        json!({"exit":9}),
        json!({"fields":{"status":"failed","results":[{
            "id":"external","status":"failed","cause":"real execution","exit_code":9}]}}),
    ] {
        let (_dir, p) = fixture(case);
        assert_eq!(run(&p).exit_code, 2);
    }
}
#[test]
fn bounds_cover_timeout_and_output() {
    for (case, cause, timeout) in [
        (json!({"kind":"sleep"}), "process timed out", 1),
        // Output bounds allow ordinary child startup so this case exercises
        // the output limit independently of the dedicated timeout case.
        (json!({"kind":"flood"}), "process output limit exceeded", 30),
    ] {
        let (_dir, p) = fixture(case);
        let mut cfg: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        cfg["judge"]["timeout_seconds"] = timeout.into();
        fs::write(&p, serde_json::to_vec(&cfg).unwrap()).unwrap();
        let start = std::time::Instant::now();
        let r = run(&p);
        assert_eq!(r.exit_code, 2);
        assert!(start.elapsed().as_secs() < timeout + 4);
        let report: serde_json::Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(
            report["transport_failure"],
            cause,
            "elapsed={:?}, exit={}, stdout_bytes={}, stderr_bytes={}",
            start.elapsed(),
            report["judge"]["exit_code"],
            report["judge"]["stdout_bytes"].as_array().unwrap().len(),
            report["judge"]["stderr_bytes"].as_array().unwrap().len()
        );
        assert_eq!(report["judge"]["failure"], cause);
        let bytes: Vec<u8> =
            serde_json::from_value(report["judge"]["stdout_bytes"].clone()).unwrap();
        assert!(bytes.len() <= 4096);
        assert_eq!(
            report["judge"]["stdout_sha256"],
            chrono_harness::sha256(&bytes)
        );
    }
}
#[test]
fn duplicate_json_and_arguments_fail() {
    let (_dir, p) = fixture(json!({}));
    let bytes = fs::read_to_string(&p)
        .unwrap()
        .replace("\"schema\":", "\"schema\":\"bad\",\"schema\":");
    fs::write(&p, bytes).unwrap();
    assert_eq!(run(&p).exit_code, 2);
    assert_eq!(
        dispatch(&[
            "check",
            "--initial",
            "--initial",
            "--candidate",
            "x",
            "--config",
            &p
        ])
        .exit_code,
        2
    );
}
#[test]
fn proposed_full_profile_remains_unsupported() {
    let (_dir, p) = fixture(json!({}));
    fs::write(&p, r#"{"schema_version":1,"status":"proposed"}"#).unwrap();
    assert_ne!(run(&p).exit_code, 0);
}
#[test]
fn unknown_and_malformed_commands_are_errors() {
    for args in [
        vec!["checks"],
        vec!["spec"],
        vec!["spec", "status", "x"],
        vec!["--version", "check"],
        vec!["check"],
        vec!["check", "--help"],
    ] {
        assert_ne!(dispatch(&args).exit_code, 0);
    }
}
#[test]
fn status_help_and_version_are_information_only() {
    let s = dispatch(&["spec", "status"]);
    assert_eq!(s.exit_code, 0);
    assert!(s.stdout.contains("ENFORCEMENT=not-implemented"));
    assert!(s.stdout.contains("CI_CHECK=chrono-ci-check/v1"));
    assert!(dispatch(&[]).stdout.contains("NOT IMPLEMENTED"));
    assert!(
        dispatch(&["--version"])
            .stdout
            .starts_with("chrono-harness ")
    );
}
#[test]
fn exact_argv_and_spaced_cwd_reach_child() {
    let dir = tempfile::Builder::new()
        .prefix("host with spaces ")
        .tempdir()
        .unwrap();
    let text = "literal $HOME `whoami` ' \"";
    let r = run_process(
        dir.path(),
        &CommandSpec {
            program: CHILD.into(),
            env: Default::default(),
            args: vec!["echo".into(), text.into()],
            timeout_seconds: 15,
            output_limit_bytes: 4096,
        },
        &[],
    )
    .unwrap();
    assert_eq!(r.stdout.lines().next(), Some(text));
    assert!(r.stdout.contains(dir.path().to_str().unwrap()));
}

#[test]
fn input_pipe_delivers_exact_bytes_and_eof_for_empty_and_nonempty_requests() {
    let dir = tempfile::tempdir().unwrap();
    for input in [b"".as_slice(), b"\0\xffliteral input\n".as_slice()] {
        let result = run_process(
            dir.path(),
            &CommandSpec {
                program: CHILD.into(),
                env: Default::default(),
                args: vec!["pipe".into()],
                timeout_seconds: 15,
                output_limit_bytes: 4096,
            },
            input,
        )
        .unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout_bytes, [b"EOF:".as_slice(), input].concat());
        assert_eq!(result.stderr_bytes, b"joined");
        assert_eq!(result.stdin_sha256, chrono_harness::sha256(input));
        assert!(result.failure.is_none());
    }
}
#[test]
fn not_required_is_a_typed_status() {
    assert_eq!(
        serde_json::to_string(&Status::NotRequired).unwrap(),
        "\"not-required\""
    );
}
#[test]
fn external_judge_result_exit_invariants() {
    for (status, child_exit, judge_exit, expected) in [
        ("not-required", None, 0, 0),
        ("passed", None, 0, 0),
        ("failed", None, 1, 1),
        ("passed", Some(0), 0, 0),
        ("failed", Some(7), 7, 1),
        ("not-required", Some(7), 0, 2),
        ("not-required", Some(0), 0, 2),
        ("passed", Some(7), 0, 2),
        ("failed", Some(0), 1, 2),
    ] {
        let (_dir, path) = fixture(json!({"echo_protocol":true, "child_exit":child_exit,
            "exit":judge_exit,"fields":{"status":status,"results":[{"id":"external",
            "status":status,"cause":"fixture result","exit_code":null}],"evidence":{"fixture":true}}}));
        let result = run(&path);
        assert_eq!(
            result.exit_code, expected,
            "{status}/{child_exit:?}: {result:?}"
        );
        let report: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
        assert_eq!(report["judge"]["exit_code"], judge_exit);
        assert_eq!(report.get("transport_failure").is_some(), expected == 2);
        if status == "not-required" && child_exit.is_some() {
            assert!(
                report["transport_failure"]
                    .as_str()
                    .unwrap()
                    .contains("not-required")
            );
        }
    }
}

#[test]
fn aggregate_not_required_exactly_matches_unused_results() {
    for case in [
        json!({"fields":{"status":"not-required"}}),
        json!({"fields":{"results":[{"id":"external","status":"not-required",
            "cause":"real execution","exit_code":null}]}}),
    ] {
        let (_dir, path) = fixture(case);
        let result = run(&path);
        assert_eq!(result.exit_code, 2);
        let report: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
        assert_eq!(
            report["transport_failure"],
            "judge exit/status/results disagreement"
        );
    }
}

#[test]
fn command_path_override_selects_and_hashes_the_invoked_executable() {
    let dir = tempfile::Builder::new()
        .prefix("command cwd ")
        .tempdir_in(std::path::Path::new(CHILD).parent().unwrap())
        .unwrap();
    let tools = dir.path().join("tools");
    fs::create_dir(&tools).unwrap();
    let executable = tools.join("true");
    native_executable::install(CHILD, &executable).unwrap();
    let bytes = fs::read(CHILD).unwrap();
    let mut spec = CommandSpec {
        program: "true".into(),
        args: vec![],
        env: Default::default(),
        timeout_seconds: 15,
        output_limit_bytes: 4096,
    };
    let inherited = run_process(dir.path(), &spec, &[]).unwrap();
    assert_eq!(inherited.exit_code, 0);
    assert_ne!(inherited.executable, executable);
    assert_eq!(
        inherited.sha256,
        chrono_harness::sha256(&fs::read(&inherited.executable).unwrap())
    );
    for path in [tools.to_str().unwrap(), "tools", "missing:tools"] {
        spec.env.insert("PATH".into(), path.into());
        let result = run_process(dir.path(), &spec, &[]).unwrap();
        assert_eq!(result.exit_code, 9, "PATH={path}");
        assert_eq!(result.executable, executable);
        assert_eq!(result.sha256, chrono_harness::sha256(&bytes));
    }
    spec.env.insert("PATH".into(), "missing".into());
    assert!(
        run_process(dir.path(), &spec, &[])
            .unwrap_err()
            .contains("executable not found")
    );
}
#[test]
fn symlinked_tool_preserves_invocation_identity() {
    let dir = tempfile::tempdir().unwrap();
    let alias = dir.path().join("registered-tool");
    std::os::unix::fs::symlink(CHILD, &alias).unwrap();
    let result = run_process(
        dir.path(),
        &CommandSpec {
            program: alias.to_str().unwrap().into(),
            args: vec!["argv0".into()],
            env: Default::default(),
            timeout_seconds: 15,
            output_limit_bytes: 4096,
        },
        &[],
    )
    .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, alias.to_str().unwrap());
    assert_eq!(result.executable, alias);
}

#[test]
fn scoped_cli_embedded_invalid_utf8_is_protocol_error_and_valid_replacement_passes() {
    for (bytes, expected_exit) in [(vec![239u8, 191, 189], 0), (vec![255], 2)] {
        let (_dir, p) = fixture(json!({"echo_protocol":true,"replacement_bytes":bytes,
            "fields":{"results":[{"id":"encoding","status":"passed","cause":"MARKER","exit_code":0}]}}));
        let output = std::process::Command::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../runner/target/debug/chrono-harness"),
        )
        .args([
            "check",
            "--config",
            &p,
            "--base",
            &"a".repeat(40),
            "--candidate",
            &"b".repeat(40),
        ])
        .output()
        .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "scoped CLI must reject original invalid bytes"
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let raw: Vec<u8> = serde_json::from_value(report["judge"]["stdout_bytes"].clone()).unwrap();
        assert_eq!(std::str::from_utf8(&raw).is_ok(), expected_exit == 0);
        assert_eq!(report["judge"]["exit_code"], 0);
        if expected_exit == 0 {
            assert_eq!(report["response"]["results"][0]["cause"], "�");
        } else {
            assert!(report.get("transport_failure").is_some());
        }
    }
}

#[test]
fn retained_entry_validation_reaches_raw_paths_and_missing_members_offline() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    fs::write(root.join("runner"), "runner").unwrap();
    fs::write(root.join("config"), "config").unwrap();
    let template = vec![
        "runner".into(),
        "check".into(),
        "--config".into(),
        "config".into(),
    ];
    let mut entry = json!({"argv":template,"cwd":root});
    entry["resolved_paths"] =
        chrono_harness::resolve_invocation(&root, &entry, &template, &json!({})).unwrap();
    dir.close().unwrap();
    chrono_harness::validate_invocation_observation(&entry, &template, &json!({})).unwrap();
    let mut wrong = entry.clone();
    wrong["argv"][3] = "other".into();
    assert!(
        chrono_harness::validate_invocation_observation(&wrong, &template, &json!({}))
            .unwrap_err()
            .contains("raw entry identity")
    );
    let mut wrong = entry.clone();
    wrong["resolved_paths"]["paths"][3]["actual"] = "other".into();
    assert!(
        chrono_harness::validate_invocation_observation(&wrong, &template, &json!({}))
            .unwrap_err()
            .contains("canonical path")
    );
    for field in ["raw_digest", "paths"] {
        let mut missing = entry.clone();
        missing["resolved_paths"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            chrono_harness::validate_invocation_observation(&missing, &template, &json!({}))
                .unwrap_err()
                .contains("E_ROUTE_MISSING")
        );
    }
    for field in ["actual", "expected"] {
        let mut missing = entry.clone();
        missing["resolved_paths"]["paths"][3]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            chrono_harness::validate_invocation_observation(&missing, &template, &json!({}))
                .unwrap_err()
                .contains("E_ROUTE_MISSING")
        );
    }
}

#[test]
fn collection_manifest_repeated_separators_are_filesystem_aliases() {
    use chrono_harness::units::{overlap, validate_collection_paths};
    for (a, b) in [
        (
            ".chrono-harness/state//report.json",
            ".chrono-harness/state/report.json",
        ),
        (
            ".chrono-harness//state/unit",
            ".chrono-harness/state/unit/report.json",
        ),
    ] {
        assert!(overlap(a, b));
        assert!(overlap(b, a));
    }
    assert!(
        validate_collection_paths(
            ".chrono-harness/state//collected.json",
            ".chrono-harness/state/collected.json",
            &[]
        )
        .is_err()
    );
    assert!(
        validate_collection_paths(
            ".chrono-harness/state//one.json",
            ".chrono-harness/state/collected.json",
            &[".chrono-harness/state/one.json".into()]
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn selector_alias_and_parent_traversal_preserve_below_root_guards() {
    use std::os::unix::fs::symlink;
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for parent in [
        source.join(".chrono-harness/state/implementation-repair/selector-fixtures"),
        source.join("crates/runner-tests/target/selector-fixtures"),
    ] {
        fs::create_dir_all(&parent).unwrap();
        let dir = tempfile::tempdir_in(parent).unwrap();
        let root = dir.path().join("host");
        fs::create_dir_all(root.join(".chrono-harness")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let alias = dir.path().join("alias");
        symlink(&root, &alias).unwrap();
        let marker = root.join(".chrono-harness/facts-launches");
        let tool = root.join(".chrono-harness/git-fixture");
        native_executable::install(CHILD, &tool).unwrap();
        // Alias guards are checked before launch; legal controls must allow the
        // native fixture to start, independently of the dedicated timeout test.
        let target = json!({"schema_version":3,"facts_git":{"tool":"git","input":"git-bytes"},"tools":[{"id":"git","program":tool,"resolution":"PATH-once","version_argv":["--version"],"expected_version":"expected"}],"environment":{"inherit":[],"values":{},"inputs":[{"id":"git-bytes","location":tool,"presence":"present","sha256":chrono_harness::sha256(&fs::read(&tool).unwrap())}]},"protocol":{"timeout_seconds":30,"stdout_limit_bytes":4096}});
        fs::write(
            root.join(".chrono-harness/direct.json"),
            serde_json::to_vec(&target).unwrap(),
        )
        .unwrap();
        let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        fs::write(root.join(".chrono-harness/config.json"),serde_json::to_vec(&json!({"schema":"chrono-git-configs/v1","platforms":{platform:".chrono-harness/direct.json"}})).unwrap()).unwrap();
        symlink("config.json", root.join(".chrono-harness/linked.json")).unwrap();
        fs::create_dir(root.join(".chrono-harness/real-dir")).unwrap();
        symlink("real-dir", root.join(".chrono-harness/linked-dir")).unwrap();
        let runner = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../runner/target/debug/chrono-harness");
        let invoke = |cwd: &std::path::Path, path: &std::path::Path| {
            std::process::Command::new(&runner)
                .current_dir(cwd)
                .args([
                    "check",
                    "--config",
                    path.to_str().unwrap(),
                    "--base",
                    &"a".repeat(40),
                    "--candidate",
                    &"b".repeat(40),
                    "--context",
                    ".chrono-harness/state/context.json",
                ])
                .output()
                .unwrap()
        };
        for path in [
            root.join(".chrono-harness/linked.json"),
            alias.join(".chrono-harness/linked.json"),
            alias.join(".chrono-harness/linked-dir/../config.json"),
        ] {
            let out = invoke(dir.path(), &path);
            assert_eq!(out.status.code(), Some(2));
            assert!(
                String::from_utf8_lossy(&out.stderr).contains("symlink path"),
                "{}: {}",
                path.display(),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(!marker.exists(), "facts launched for {}", path.display());
        }
        let nested = root.join("nested");
        fs::create_dir(&nested).unwrap();
        for (cwd, path) in [
            (
                dir.path().to_path_buf(),
                alias.join(".chrono-harness/config.json"),
            ),
            (
                nested.clone(),
                std::path::PathBuf::from("../.chrono-harness/config.json"),
            ),
            (
                nested,
                std::path::PathBuf::from("../../host/.chrono-harness/config.json"),
            ),
        ] {
            let out = invoke(&cwd, &path);
            assert_eq!(out.status.code(), Some(2));
            assert!(
                marker.exists(),
                "legal control did not reach selected producer: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            fs::remove_file(&marker).unwrap();
        }
    }
}

#[test]
fn scoped_signal_termination_reports_the_process_failure_before_empty_json() {
    let (dir, path) = fixture(json!({}));
    let mut config: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config["schema"] = json!("chrono-ci-check/v3");
    config["judge"]["program"] = json!(env!("CARGO_BIN_EXE_chrono-test-process-lifecycle"));
    config["judge"]["args"] = json!(["terminated", "empty"]);
    config["policy"]["units"] =
        json!({"one":{"tests":["test:one"],"report_path":".chrono-harness/state/one/check.json"}});
    fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    let result = dispatch(&[
        "check",
        "--config",
        &path,
        "--base",
        &"a".repeat(40),
        "--candidate",
        &"b".repeat(40),
        "--unit",
        "one",
    ]);
    assert_eq!(result.exit_code, 2, "{result:?}");
    let report: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    let pid = fs::read_to_string(dir.path().join("terminated.pid")).unwrap();
    let expected = format!("process terminated by signal 9 (pid {pid})");
    assert_eq!(report["transport_failure"], expected);
    assert_eq!(report["judge"]["failure"], expected);
    assert_eq!(report["judge"]["exit_code"], -1);
    assert_eq!(report["judge"]["stdout_bytes"], json!([]));
}
