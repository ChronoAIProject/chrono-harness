use chrono_judge_projects::execute;
use chrono_judge_registration::execution::{Method, Plan};
use chrono_judge_routes::prepare;
use serde_json::json;
use std::collections::BTreeMap;
fn plan(
    commands: &[(&str, &str)],
    sequences: &[(&str, Vec<&str>)],
) -> (tempfile::TempDir, chrono_judge_routes::Execution) {
    let dir = tempfile::tempdir().unwrap();
    let python = chrono_harness::resolve_program(dir.path(), "python3", None).unwrap();
    let version = std::process::Command::new(&python)
        .arg("--version")
        .output()
        .unwrap();
    let tools = json!([{"id":"python","program":python,"version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}]);
    let methods = commands
        .iter()
        .map(|(id, script)| {
            (
                id.to_string(),
                vec![Method {
                    owner: "script:consumer".into(),
                    operation: id.to_string(),
                    tool: "python".into(),
                    argv: vec!["-c".into(), script.to_string()],
                }],
            )
        })
        .collect();
    let plans: BTreeMap<_, _> = sequences
        .iter()
        .map(|(id, ops)| {
            (
                id.to_string(),
                Plan {
                    operations: ops.iter().map(|s| s.to_string()).collect(),
                    timeout_seconds: 5,
                    output_limit_bytes: 4096,
                },
            )
        })
        .collect();
    let actions = sequences
        .iter()
        .map(|(id, ops)| (id.to_string(), ops.last().unwrap().to_string()))
        .collect();
    let plan = prepare(
        dir.path(),
        json!({"run":"test"}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &actions,
        &tools,
        BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    (dir, plan)
}
#[test]
fn failing_pass_text_blocks_dependents_and_independent_branch_continues() {
    let (_d, plan) = plan(
        &[
            ("a", "print('PASS'); raise SystemExit(7)"),
            ("b", "raise Exception('must not run')"),
            ("z", "print('independent')"),
        ],
        &[
            ("test:dependent", vec!["a", "b"]),
            ("test:independent", vec!["z"]),
        ],
    );
    let result = execute(&plan).unwrap();
    assert!(!result.passed());
    assert_eq!(result.executed.len(), 2);
    assert_eq!(result.blocked.len(), 1);
    assert_eq!(result.blocked[0].operation, "b");
    assert_eq!(result.tests["test:independent"], "passed");
    let actual = &result.executed[0].receipt.as_ref().unwrap().process;
    assert_eq!(actual.exit_code, 7);
    assert_eq!(actual.stdout, "PASS\n");
}
#[test]
fn output_bound_preserves_bytes_and_cannot_pass() {
    let (_d, plan) = plan(
        &[("flood", "print('x'*8000)")],
        &[("test:t", vec!["flood"])],
    );
    let result = execute(&plan).unwrap();
    assert!(!result.passed());
    let actual = &result.executed[0].receipt.as_ref().unwrap().process;
    assert_eq!(actual.stdout_bytes.len(), 4096);
    assert!(actual.failure.as_ref().unwrap().contains("output limit"));
}
#[test]
fn real_effects_shared_prerequisite_runs_once_in_order() {
    let (d, plan) = plan(
        &[
            ("a", "open('order','a').write('a')"),
            (
                "b",
                "assert open('order').read()=='a';open('order','a').write('b')",
            ),
            (
                "c",
                "assert open('order').read()=='ab';open('order','a').write('c')",
            ),
        ],
        &[("test:b", vec!["a", "b"]), ("test:c", vec!["a", "b", "c"])],
    );
    let result = execute(&plan).unwrap();
    assert!(result.passed());
    assert_eq!(
        std::fs::read_to_string(d.path().join("order")).unwrap(),
        "abc"
    );
    assert_eq!(result.executed.len(), 3);
}

#[test]
fn arbitrary_operation_bytes_remain_evidence_without_protocol_decoding() {
    let (_d, plan) = plan(
        &[(
            "raw",
            "import os;os.write(1,bytes([255]));os.write(2,bytes([254]))",
        )],
        &[("test:t", vec!["raw"])],
    );
    let result = execute(&plan).unwrap();
    assert!(result.passed());
    let process = &result.executed[0].receipt.as_ref().unwrap().process;
    assert_eq!(process.stdout_bytes, vec![255]);
    assert_eq!(process.stderr_bytes, vec![254]);
    assert_eq!(process.stdout_sha256, chrono_harness::sha256(&[255]));
    assert_eq!(process.stderr_sha256, chrono_harness::sha256(&[254]));
    assert_eq!(process.stdout, "�");
}
