use chrono_harness::{CommandSpec, Status, dispatch, run_process};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

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
