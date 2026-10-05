use chrono_judge_projects::execute;
use chrono_judge_registration::execution::{Method, Plan};
use chrono_judge_routes::prepare;
use serde_json::json;
use std::collections::BTreeMap;
#[path = "support/execution_fixture.rs"]
mod execution_fixture;
use execution_fixture::Step;

fn assert_passed(result: &chrono_judge_projects::Results) {
    assert!(
        result.passed(),
        "operation results: {}",
        json!({
            "tests": result.tests,
            "operations": result.executed.iter().chain(&result.blocked).map(|row| {
                json!({
                    "operation": row.operation,
                    "status": row.status,
                    "error": row.error,
                    "process": row.receipt.as_ref().map(|receipt| {
                        let process = &receipt.process;
                        json!({
                            "exit_code": process.exit_code,
                            "failure": process.failure,
                            "stdout": process.stdout,
                            "stderr": process.stderr,
                        })
                    }),
                })
            }).collect::<Vec<_>>(),
        })
    );
}

fn stdout(bytes: impl AsRef<[u8]>) -> Step {
    Step::Stdout(bytes.as_ref().to_vec())
}
fn append(path: &str, bytes: &str) -> Step {
    Step::Append {
        path: path.into(),
        bytes: bytes.as_bytes().to_vec(),
    }
}
fn create(path: &str, bytes: &str) -> Step {
    Step::Create {
        path: path.into(),
        bytes: bytes.as_bytes().to_vec(),
    }
}
fn require(path: &str, bytes: &str) -> Step {
    Step::Require {
        path: path.into(),
        bytes: bytes.as_bytes().to_vec(),
    }
}
fn edit_steps(
    p: &mut chrono_judge_routes::Execution,
    index: usize,
    edit: impl FnOnce(&mut Vec<Step>),
) {
    let mut steps = serde_json::from_str(&p.operations[index].method.argv[1]).unwrap();
    edit(&mut steps);
    p.operations[index].method.argv[1] = serde_json::to_string(&steps).unwrap();
}
fn plan(
    commands: &[(&str, Vec<Step>)],
    sequences: &[(&str, Vec<&str>)],
) -> (tempfile::TempDir, chrono_judge_routes::Execution) {
    let dir = tempfile::tempdir().unwrap();
    let child = std::path::PathBuf::from(env!("CARGO_BIN_EXE_chrono-test-project-execution"));
    let version = std::process::Command::new(&child)
        .arg("--version")
        .output()
        .unwrap();
    assert!(version.status.success());
    let tools = json!([{"id":"fixture","program":child,"version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}]);
    let methods = commands
        .iter()
        .map(|(id, steps)| {
            (
                id.to_string(),
                vec![Method {
                    owner: "script:consumer".into(),
                    operation: id.to_string(),
                    tool: "fixture".into(),
                    argv: vec!["run".into(), serde_json::to_string(steps).unwrap()],
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
            ("a", vec![stdout("PASS\n"), Step::Exit(7)]),
            ("b", vec![Step::Exit(97)]),
            ("z", vec![stdout("independent\n")]),
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
        &[("flood", vec![stdout(vec![b'x'; 8000])])],
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
            ("a", vec![append("order", "a")]),
            ("b", vec![require("order", "a"), append("order", "b")]),
            ("c", vec![require("order", "ab"), append("order", "c")]),
        ],
        &[("test:b", vec!["a", "b"]), ("test:c", vec!["a", "b", "c"])],
    );
    let result = execute(&plan).unwrap();
    assert_passed(&result);
    assert_eq!(
        std::fs::read_to_string(d.path().join("order")).unwrap(),
        "abc"
    );
    assert_eq!(result.executed.len(), 3);
}

#[test]
fn arbitrary_operation_bytes_remain_evidence_without_protocol_decoding() {
    let (_d, plan) = plan(
        &[("raw", vec![stdout([255]), Step::Stderr(vec![254])])],
        &[("test:t", vec!["raw"])],
    );
    let result = execute(&plan).unwrap();
    assert_passed(&result);
    let process = &result.executed[0].receipt.as_ref().unwrap().process;
    assert_eq!(process.stdout_bytes, vec![255]);
    assert_eq!(process.stderr_bytes, vec![254]);
    assert_eq!(process.stdout_sha256, chrono_harness::sha256(&[255]));
    assert_eq!(process.stderr_sha256, chrono_harness::sha256(&[254]));
    assert_eq!(process.stdout, "�");
}

fn scheduled(
    mut p: chrono_judge_routes::Execution,
    cap: usize,
    claims: serde_json::Value,
) -> chrono_judge_routes::Execution {
    let policy =
        serde_json::from_value(json!({"max_running":cap,"resources":["shared"],"claims":claims}))
            .unwrap();
    p = p.with_scheduling(Some(policy)).unwrap();
    p
}
fn prioritized(
    p: chrono_judge_routes::Execution,
    priority: serde_json::Value,
) -> chrono_judge_routes::Execution {
    let mut policy = serde_json::to_value(p.scheduling.as_ref().unwrap()).unwrap();
    policy["priority"] = priority;
    p.with_scheduling(Some(serde_json::from_value(policy).unwrap()))
        .unwrap()
}

#[test]
fn priority_controls_actual_launches_and_preserves_canonical_results() {
    let (_d, p, listener) = barrier_plan(&["a", "b", "c", "d"]);
    let p = scheduled(
        p,
        1,
        json!({
            "a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},
            "c":{"resources":[],"outputs":[]},"d":{"resources":[],"outputs":[]},
            "outside-selected-unit":{"resources":[],"outputs":[]}
        }),
    );
    let unchanged = prioritized(p.clone(), json!([]));
    assert_eq!(unchanged.identity, p.identity);
    assert_eq!(
        serde_json::to_value(&unchanged).unwrap(),
        serde_json::to_value(&p).unwrap()
    );
    let ordered = prioritized(p.clone(), json!(["c", "b", "outside-selected-unit"]));
    assert_ne!(ordered.identity, p.identity);
    assert_ne!(
        ordered.identity,
        prioritized(p, json!(["b", "c", "outside-selected-unit"])).identity
    );
    let mut tampered = serde_json::to_value(&ordered).unwrap();
    tampered["scheduling"]["priority"] = json!(["b", "c", "outside-selected-unit"]);
    assert!(
        serde_json::from_value::<chrono_judge_routes::Execution>(tampered)
            .unwrap()
            .validate()
            .is_err()
    );
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| execute(&ordered).unwrap());
        for expected in ["c", "b", "a", "d"] {
            let (id, stream) = arrival(&listener);
            assert_eq!(id, expected);
            release(stream);
        }
        let result = worker.join().unwrap();
        assert_passed(&result);
        assert_eq!(
            result
                .executed
                .iter()
                .map(|r| r.operation.as_str())
                .collect::<Vec<_>>(),
            ["a", "b", "c", "d"]
        );
    });
}

#[test]
fn priority_waits_for_dependencies_and_conflicts_without_holding_disjoint_work() {
    for outputs in [false, true] {
        let (_d, mut p, listener) = barrier_plan(&["a", "b", "c", "x", "y", "z"]);
        p.selected.get_mut("test:c").unwrap().operations = vec!["a".into(), "c".into()];
        p.operations[2].predecessors = std::collections::BTreeSet::from(["a".into()]);
        edit_steps(&mut p, 0, |s| s.push(create("a-done", "done")));
        edit_steps(&mut p, 5, |s| s.push(create("z-done", "done")));
        for index in [1, 2] {
            edit_steps(&mut p, index, |s| s.insert(0, require("z-done", "done")));
        }
        edit_steps(&mut p, 2, |s| s.insert(0, require("a-done", "done")));
        let shared = if outputs {
            json!({"resources":[],"outputs":["out/nested/"]})
        } else {
            json!({"resources":["shared"],"outputs":[]})
        };
        let holder = if outputs {
            json!({"resources":[],"outputs":["out/"]})
        } else {
            shared.clone()
        };
        let p = prioritized(
            scheduled(
                p,
                2,
                json!({
                    "a":{"resources":[],"outputs":[]},"b":shared,"c":shared,
                    "x":{"resources":[],"outputs":[]},"y":{"resources":[],"outputs":[]},"z":holder
                }),
            ),
            json!(["c", "z", "b", "x"]),
        );
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| execute(&p).unwrap());
            let mut first = BTreeMap::from([arrival(&listener), arrival(&listener)]);
            assert_eq!(
                first.keys().map(String::as_str).collect::<Vec<_>>(),
                ["x", "z"]
            );
            release(first.remove("x").unwrap());
            let (id, a) = arrival(&listener);
            assert_eq!(id, "a");
            release(a);
            let (id, y) = arrival(&listener);
            assert_eq!(id, "y");
            release(y);
            release(first.remove("z").unwrap());
            for expected in ["c", "b"] {
                let (id, stream) = arrival(&listener);
                assert_eq!(id, expected);
                release(stream);
            }
            assert!(worker.join().unwrap().passed());
        });
    }
}

#[test]
fn priority_does_not_run_failed_descendants_or_drop_unlisted_work() {
    let (d, p) = plan(
        &[
            ("a", vec![Step::Exit(9)]),
            ("b", vec![create("forbidden", "ran")]),
            ("z", vec![create("independent", "ran")]),
        ],
        &[("test:b", vec!["a", "b"]), ("test:z", vec!["z"])],
    );
    let p = prioritized(
        scheduled(
            p,
            2,
            json!({
                "a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"z":{"resources":[],"outputs":[]}
            }),
        ),
        json!(["b"]),
    );
    let result = execute(&p).unwrap();
    assert!(!result.passed());
    assert_eq!(result.blocked[0].operation, "b");
    assert_eq!(result.tests["test:z"], "passed");
    assert_eq!(
        result.executed[0]
            .receipt
            .as_ref()
            .unwrap()
            .process
            .exit_code,
        9
    );
    assert!(!d.path().join("forbidden").exists());
    assert_eq!(
        std::fs::read_to_string(d.path().join("independent")).unwrap(),
        "ran"
    );
}
fn barrier_plan(
    ids: &[&str],
) -> (
    tempfile::TempDir,
    chrono_judge_routes::Execution,
    std::net::TcpListener,
) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let commands: Vec<_> = ids
        .iter()
        .map(|id| {
            (
                *id,
                vec![
                    Step::Rendezvous {
                        address: address.clone(),
                        id: (*id).into(),
                    },
                    stdout(format!("{id}\n")),
                ],
            )
        })
        .collect();
    let tests: Vec<_> = ids.iter().map(|id| format!("test:{id}")).collect();
    let sequences: Vec<_> = tests
        .iter()
        .zip(ids)
        .map(|(test, id)| (test.as_str(), vec![*id]))
        .collect();
    let (d, p) = plan(&commands, &sequences);
    (d, p, listener)
}
fn arrival(listener: &std::net::TcpListener) -> (String, std::net::TcpStream) {
    use std::io::Read;
    let start = std::time::Instant::now();
    loop {
        match listener.accept() {
            Ok((mut s, _)) => {
                s.set_nonblocking(false).unwrap();
                s.set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut id = String::new();
                loop {
                    let mut byte = [0];
                    s.read_exact(&mut byte).unwrap();
                    if byte[0] == b'\n' {
                        break;
                    }
                    id.push(byte[0] as char);
                }
                return (id, s);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    start.elapsed() < std::time::Duration::from_secs(6),
                    "no registered process reached rendezvous"
                );
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) => panic!("{e}"),
        }
    }
}
fn release(mut s: std::net::TcpStream) {
    use std::io::Write;
    s.write_all(b"!").unwrap();
}
#[test]
fn rendezvous_proves_overlap_cap_and_canonical_rows() {
    let (_d, mut p, listener) = barrier_plan(&["a", "b", "c"]);
    // A third launch before either held slot finishes fails an independent
    // process-side witness, even if its socket arrival would otherwise queue.
    edit_steps(&mut p, 1, |s| s.push(create("b-completed", "done")));
    edit_steps(&mut p, 2, |s| s.insert(0, require("b-completed", "done")));
    let p = scheduled(
        p,
        2,
        json!({"a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"c":{"resources":[],"outputs":[]}}),
    );
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| execute(&p).unwrap());
        let mut first = BTreeMap::from([arrival(&listener), arrival(&listener)]);
        assert_eq!(
            first.keys().map(String::as_str).collect::<Vec<_>>(),
            ["a", "b"]
        );
        // b completes first. c can reach its rendezvous only after that slot is
        // released, while a is still blocked on the independent oracle.
        release(first.remove("b").unwrap());
        let (id, c) = arrival(&listener);
        assert_eq!(id, "c");
        release(c);
        release(first.remove("a").unwrap());
        let r = worker.join().unwrap();
        assert_passed(&r);
        assert_eq!(
            r.executed
                .iter()
                .map(|r| r.operation.as_str())
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    });
}
#[test]
fn resource_and_nested_output_exclusion_allow_disjoint_ready_work() {
    for outputs in [false, true] {
        let (_d, mut p, listener) = barrier_plan(&["a", "b", "z"]);
        edit_steps(&mut p, 0, |s| s.push(create("a-completed", "done")));
        edit_steps(&mut p, 1, |s| s.insert(0, require("a-completed", "done")));
        let p = scheduled(
            p,
            2,
            if outputs {
                json!({"a":{"resources":[],"outputs":["out/"]},"b":{"resources":[],"outputs":["out/nested/"]},"z":{"resources":[],"outputs":[]}})
            } else {
                json!({"a":{"resources":["shared"],"outputs":[]},"b":{"resources":["shared"],"outputs":[]},"z":{"resources":[],"outputs":[]}})
            },
        );
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| execute(&p).unwrap());
            let mut first = BTreeMap::from([arrival(&listener), arrival(&listener)]);
            assert_eq!(
                first.keys().map(String::as_str).collect::<Vec<_>>(),
                ["a", "z"]
            );
            release(first.remove("z").unwrap());
            release(first.remove("a").unwrap());
            let (id, b) = arrival(&listener);
            assert_eq!(id, "b");
            release(b);
            assert!(worker.join().unwrap().passed());
        });
    }
}
#[test]
fn diamond_runs_shared_work_once_and_waits_for_both_predecessors() {
    let (d, p) = plan(
        &[
            ("a", vec![create("a", "a")]),
            ("b", vec![require("a", "a"), create("b", "b")]),
            ("c", vec![require("a", "a"), create("c", "c")]),
            (
                "d",
                vec![require("b", "b"), require("c", "c"), create("d", "d")],
            ),
        ],
        &[
            ("test:b", vec!["a", "b", "d"]),
            ("test:c", vec!["a", "c", "d"]),
        ],
    );
    let p = scheduled(
        p,
        2,
        json!({"a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"c":{"resources":[],"outputs":[]},"d":{"resources":[],"outputs":[]}}),
    );
    let r = execute(&p).unwrap();
    assert_passed(&r);
    assert_eq!(r.executed.len(), 4);
    assert_eq!(std::fs::read_to_string(d.path().join("d")).unwrap(), "d");
}
#[test]
fn timeout_and_failed_exit_keep_original_bytes_and_only_block_descendants() {
    for timeout in [true, false] {
        let (_d, mut p) = plan(
            &[
                (
                    "a",
                    vec![
                        stdout("original"),
                        if timeout {
                            Step::SleepMillis(5000)
                        } else {
                            Step::Exit(9)
                        },
                    ],
                ),
                ("b", vec![Step::Exit(97)]),
                ("z", vec![stdout("sibling\n")]),
            ],
            &[("test:b", vec!["a", "b"]), ("test:z", vec!["z"])],
        );
        for op in &mut p.operations {
            op.timeout_seconds = 1;
        }
        for plan in p.selected.values_mut() {
            plan.timeout_seconds = 1;
        }
        let p = scheduled(
            p,
            2,
            json!({"a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"z":{"resources":[],"outputs":[]}}),
        );
        let r = execute(&p).unwrap();
        assert_eq!(r.blocked[0].operation, "b");
        assert_eq!(r.tests["test:z"], "passed");
        let a = &r.executed[0];
        let process = &a.receipt.as_ref().unwrap().process;
        assert_eq!(process.stdout_bytes, b"original");
        if timeout {
            assert_eq!(a.status, "error");
            assert_eq!(process.failure.as_deref(), Some("process timed out"));
        } else {
            assert_eq!(a.status, "failed");
            assert_eq!(process.exit_code, 9);
        }
    }
}
#[test]
fn prelaunch_digest_failure_does_not_stop_sibling() {
    let (_d, mut p) = plan(
        &[
            ("a", vec![stdout("must not run\n")]),
            ("z", vec![stdout("sibling\n")]),
        ],
        &[("test:a", vec!["a"]), ("test:z", vec!["z"])],
    );
    let mut bad = p.tools["fixture"].clone();
    bad.sha256 = "0".repeat(64);
    p.tools.insert("bad".into(), bad);
    p.operations[0].method.tool = "bad".into();
    let p = scheduled(
        p,
        2,
        json!({"a":{"resources":[],"outputs":[]},"z":{"resources":[],"outputs":[]}}),
    );
    let r = execute(&p).unwrap();
    assert_eq!(r.executed[0].status, "error");
    assert!(r.executed[0].error.as_ref().unwrap().contains("digest"));
    assert_eq!(r.tests["test:z"], "passed");
}
#[test]
fn missing_policy_keeps_serial_process_order_and_legacy_identity_shape() {
    let (d, p) = plan(
        &[
            ("a", vec![create("first", "done")]),
            ("b", vec![require("first", "done")]),
        ],
        &[("test:a", vec!["a"]), ("test:b", vec!["b"])],
    );
    assert!(
        serde_json::to_value(&p)
            .unwrap()
            .get("scheduling")
            .is_none()
    );
    let r = execute(&p).unwrap();
    assert_passed(&r);
    assert!(d.path().join("first").exists());
}

#[test]
fn spawn_failure_preserves_cause_and_continues_independent_operation() {
    use std::os::unix::fs::PermissionsExt;
    let (d, mut p) = plan(
        &[
            ("a", vec![stdout("unreachable\n")]),
            ("z", vec![stdout("sibling\n")]),
        ],
        &[("test:a", vec!["a"]), ("test:z", vec!["z"])],
    );
    let path = d.path().join("not-executable");
    std::fs::copy(&p.tools["fixture"].path, &path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let mut bad = p.tools["fixture"].clone();
    bad.path = path;
    p.tools.insert("bad".into(), bad);
    p.operations[0].method.tool = "bad".into();
    let p = scheduled(
        p,
        2,
        json!({"a":{"resources":[],"outputs":[]},"z":{"resources":[],"outputs":[]}}),
    );
    let r = execute(&p).unwrap();
    assert_eq!(r.executed[0].status, "error");
    assert!(
        r.executed[0]
            .error
            .as_ref()
            .unwrap()
            .contains("Permission denied")
    );
    assert!(r.executed[0].receipt.is_none());
    assert_eq!(r.tests["test:z"], "passed");
}
#[test]
fn concurrent_output_failure_retains_original_bytes_and_sibling_receipt() {
    let (_d, p) = plan(
        &[
            ("a", vec![stdout(vec![b'x'; 8000])]),
            ("b", vec![Step::Exit(97)]),
            ("z", vec![stdout("sibling\n")]),
        ],
        &[("test:b", vec!["a", "b"]), ("test:z", vec!["z"])],
    );
    let p = scheduled(
        p,
        2,
        json!({"a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"z":{"resources":[],"outputs":[]}}),
    );
    let r = execute(&p).unwrap();
    assert_eq!(r.executed[0].status, "error");
    let process = &r.executed[0].receipt.as_ref().unwrap().process;
    assert_eq!(process.stdout_bytes, vec![b'x'; 4096]);
    assert_eq!(
        process.failure.as_deref(),
        Some("process output limit exceeded")
    );
    assert_eq!(r.blocked[0].operation, "b");
    assert_eq!(
        r.executed[1].receipt.as_ref().unwrap().process.stdout_bytes,
        b"sibling\n"
    );
}

#[test]
fn receipt_mismatch_keeps_real_processes_and_does_not_stop_independent_work() {
    let (d, mut p) = plan(
        &[
            ("a", vec![stdout("original\n")]),
            ("b", vec![Step::Exit(97)]),
            (
                "z",
                vec![create("sibling", "executed"), stdout("sibling\n")],
            ),
        ],
        &[("test:b", vec!["a", "b"]), ("test:z", vec!["z"])],
    );
    // A retained noncanonical cwd resolves to the same real directory, but its
    // actual observation cannot satisfy the literal plan binding. Both real
    // launches must remain visible even though their receipts are rejected.
    std::fs::create_dir(d.path().join("alias")).unwrap();
    p.root = d.path().join("alias/..");
    let p = scheduled(
        p,
        2,
        json!({"a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"z":{"resources":[],"outputs":[]}}),
    );
    let r = execute(&p).unwrap();
    assert_eq!(r.blocked[0].operation, "b");
    assert_eq!(r.executed.len(), 2);
    for (row, bytes) in r
        .executed
        .iter()
        .zip([b"original\n".as_slice(), b"sibling\n"])
    {
        assert_eq!(row.status, "error");
        assert!(row.error.as_ref().unwrap().contains("E_RECEIPT_MISMATCH"));
        let process = &row.receipt.as_ref().unwrap().process;
        assert_eq!(process.exit_code, 0);
        assert_eq!(process.stdout_bytes, bytes);
        assert_eq!(process.cwd, std::fs::canonicalize(d.path()).unwrap());
    }
    assert_eq!(
        std::fs::read_to_string(d.path().join("sibling")).unwrap(),
        "executed"
    );
}

#[test]
fn process_side_slot_witness_detects_an_excess_launch() {
    let (d, mut p, listener) = barrier_plan(&["a", "b", "c"]);
    edit_steps(&mut p, 1, |s| s.push(create("b-completed", "done")));
    edit_steps(&mut p, 2, |s| {
        s.insert(
            0,
            Step::RequireWithWitness {
                path: "b-completed".into(),
                bytes: b"done".to_vec(),
                witness: "excess-launch".into(),
            },
        )
    });
    let p = scheduled(
        p,
        3,
        json!({"a":{"resources":[],"outputs":[]},"b":{"resources":[],"outputs":[]},"c":{"resources":[],"outputs":[]}}),
    );
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| execute(&p).unwrap());
        let first = BTreeMap::from([arrival(&listener), arrival(&listener)]);
        assert_eq!(
            first.keys().map(String::as_str).collect::<Vec<_>>(),
            ["a", "b"]
        );
        // Await c's completely published witness while held a/b cannot produce
        // its prerequisite. Release and join every process before inspecting
        // its terminal receipt, keeping the original five-second bound.
        let start = std::time::Instant::now();
        while !d.path().join("excess-launch").exists() {
            assert!(
                start.elapsed() < std::time::Duration::from_secs(5),
                "excess launch was not observed"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            std::fs::read_to_string(d.path().join("excess-launch")).unwrap(),
            "observed"
        );
        for s in first.into_values() {
            release(s);
        }
        let r = worker.join().unwrap();
        assert_eq!(r.executed[2].status, "failed");
        assert!(
            r.executed[2]
                .receipt
                .as_ref()
                .unwrap()
                .process
                .stderr
                .contains("NotFound")
        );
    });
}
