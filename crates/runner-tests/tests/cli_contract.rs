use chrono_harness::{CommandSpec, Status, dispatch, run_process};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

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

fn fixture(script: &str) -> (TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".chrono-harness/ci")).unwrap();
    let path = dir.path().join(".chrono-harness/ci/check.json");
    fs::write(&path,serde_json::to_vec(&json!({"schema":"chrono-ci-check/v1","judge":{"program":"python3","args":["-c",script],"timeout_seconds":15,"output_limit_bytes":4096},"policy":{},"report_path":".chrono-harness/state/check.json"})).unwrap()).unwrap();
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
const RESPONSE: &str = "import json,sys; r=json.load(sys.stdin); print(json.dumps({'protocol':'chrono-ci-judge/v1','request_id':r['request_id'],'status':'passed','results':[{'id':'external','status':'passed','cause':'real execution','exit_code':0}],'evidence':{'executed':True}}))";
#[test]
fn real_child_and_report_publish() {
    let (dir, p) = fixture(RESPONSE);
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
    let script = RESPONSE
        .replace("chrono-ci-judge/v1", "chrono-ci-judge/v2")
        .replace("{'executed':True}", "{'bytes':list(range(256))*32}");
    let (dir, p) = fixture(&script);
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
fn actual_failure_is_preserved() {
    let script = RESPONSE
        .replace("'passed'", "'failed'")
        .replace("'exit_code':0", "'exit_code':7")
        + ";sys.exit(7)";
    let (_dir, p) = fixture(&script);
    let r = run(&p);
    assert_eq!(r.exit_code, 1);
    assert!(r.stdout.contains("registered") == false);
    assert!(r.stdout.contains("\"exit_code\": 7"));
}
#[test]
fn malformed_empty_and_identity_mismatch_fail() {
    for s in [
        "pass".to_string(),
        "print('{}')".into(),
        "print('not-json')".into(),
        RESPONSE.replace("r['request_id']", "'other'"),
        RESPONSE.replace("'protocol':'chrono-ci-judge/v1'", "'protocol':'wrong'"),
        RESPONSE.replace(
            "[{'id':'external','status':'passed','cause':'real execution','exit_code':0}]",
            "[]",
        ),
        RESPONSE.to_owned() + ";print('{}')",
    ] {
        let (_dir, p) = fixture(&s);
        let r = run(&p);
        assert_eq!(r.exit_code, 2, "{s}: {r:?}");
        assert!(r.stdout.contains("transport_failure"));
    }
}
#[test]
fn exit_status_mismatch_fails() {
    for script in [
        RESPONSE.to_owned() + ";sys.exit(9)",
        RESPONSE
            .replace("'passed'", "'failed'")
            .replace("'exit_code':0", "'exit_code':9"),
    ] {
        let (_dir, p) = fixture(&script);
        assert_eq!(run(&p).exit_code, 2);
    }
}
#[test]
fn bounds_cover_timeout_and_output() {
    for script in ["import time;time.sleep(8)", "print('x'*8192)"] {
        let (_dir, p) = fixture(script);
        let mut cfg: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        cfg["judge"]["timeout_seconds"] = 1.into();
        fs::write(&p, serde_json::to_vec(&cfg).unwrap()).unwrap();
        let start = std::time::Instant::now();
        let r = run(&p);
        assert_eq!(r.exit_code, 2);
        assert!(start.elapsed().as_secs() < 5);
    }
}
#[test]
fn duplicate_json_and_arguments_fail() {
    let (_dir, p) = fixture(RESPONSE);
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
    let (_dir, p) = fixture(RESPONSE);
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
            program: "python3".into(),
            env: Default::default(),
            args: vec![
                "-c".into(),
                "import sys,os;print(sys.argv[1]);print(os.getcwd())".into(),
                text.into(),
            ],
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
fn not_required_is_a_typed_status() {
    assert_eq!(
        serde_json::to_string(&Status::NotRequired).unwrap(),
        "\"not-required\""
    );
}
#[test]
fn external_judge_result_exit_invariants() {
    for (status, child_exit, reported_exit, judge_exit, expected) in [
        ("not-required", None, "None", 0, 0),
        ("passed", None, "None", 0, 0),
        ("failed", None, "None", 1, 1),
        ("passed", Some(0), "code", 0, 0),
        ("failed", Some(7), "code", 7, 1),
        ("not-required", Some(7), "code", 0, 2),
        ("not-required", Some(0), "code", 0, 2),
        ("passed", Some(7), "code", 0, 2),
        ("failed", Some(0), "code", 1, 2),
    ] {
        let execution = child_exit.map_or(String::new(), |code| {
            format!("code=subprocess.run(['/bin/sh','-c','exit {code}']).returncode;")
        });
        let script = format!(
            "import json,sys,subprocess; r=json.load(sys.stdin); {execution} print(json.dumps({{'protocol':r['protocol'],'request_id':r['request_id'],'status':'{status}','results':[{{'id':'external','status':'{status}','cause':'fixture result','exit_code':{reported_exit}}}],'evidence':{{'fixture':True}}}}));sys.exit({judge_exit})"
        );
        let (_dir, path) = fixture(&script);
        let result = run(&path);
        assert_eq!(
            result.exit_code, expected,
            "{status}/{reported_exit}: {result:?}"
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
    for script in [
        RESPONSE.replacen("'status':'passed'", "'status':'not-required'", 1),
        RESPONSE
            .replace(
                "'id':'external','status':'passed'",
                "'id':'external','status':'not-required'",
            )
            .replace("'exit_code':0", "'exit_code':None"),
    ] {
        let (_dir, path) = fixture(&script);
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
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::Builder::new()
        .prefix("command cwd ")
        .tempdir()
        .unwrap();
    let tools = dir.path().join("tools");
    fs::create_dir(&tools).unwrap();
    let executable = tools.join("sh");
    let bytes = b"#!/bin/sh\nexit 9\n";
    fs::write(&executable, bytes).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let mut spec = CommandSpec {
        program: "sh".into(),
        args: vec!["-c".into(), "exit 0".into()],
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
        assert_eq!(result.sha256, chrono_harness::sha256(bytes));
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
    let alias = dir.path().join("registered-shell");
    std::os::unix::fs::symlink("/bin/sh", &alias).unwrap();
    let result = run_process(
        dir.path(),
        &CommandSpec {
            program: alias.to_str().unwrap().into(),
            args: vec!["-c".into(), "printf '%s' \"$0\"".into()],
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
    for (bytes, expected_exit) in [("bytes([239,191,189])", 0), ("bytes([255])", 2)] {
        let script = format!(
            "import json,sys\nr=json.load(sys.stdin)\ns={{'protocol':r['protocol'],'request_id':r['request_id'],'status':'passed','results':[{{'id':'encoding','status':'passed','cause':'MARKER','exit_code':0}}],'evidence':{{'executed':True}}}}\nsys.stdout.buffer.write(json.dumps(s).encode().replace(b'MARKER',{bytes}))"
        );
        let (_dir, p) = fixture(&script);
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
