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
        assert!(result.passed());
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
        p.operations[0].method.argv[1].push_str(";open('a-done','w').write('done')");
        p.operations[5].method.argv[1].push_str(";open('z-done','w').write('done')");
        for index in [1, 2] {
            p.operations[index].method.argv[1] = format!(
                "assert open('z-done').read()=='done';{}",
                p.operations[index].method.argv[1]
            );
        }
        p.operations[2].method.argv[1] = format!(
            "assert open('a-done').read()=='done';{}",
            p.operations[2].method.argv[1]
        );
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
            ("a", "raise SystemExit(9)"),
            ("b", "open('forbidden','x').write('ran')"),
            ("z", "open('independent','x').write('ran')"),
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
    let port = listener.local_addr().unwrap().port();
    let scripts: Vec<_> = ids.iter().map(|id| format!("import socket;s=socket.create_connection(('127.0.0.1',{port}));s.sendall(b'{id}\\n');assert s.recv(1)==b'!';print('{id}')")).collect();
    let commands: Vec<_> = ids
        .iter()
        .zip(&scripts)
        .map(|(id, s)| (*id, s.as_str()))
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
    p.operations[1].method.argv[1].push_str(";open('b-completed','w').write('done')");
    p.operations[2].method.argv[1] = format!(
        "assert open('b-completed').read()=='done';{}",
        p.operations[2].method.argv[1]
    );
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
        assert!(r.passed());
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
        p.operations[0].method.argv[1].push_str(";open('a-completed','w').write('done')");
        p.operations[1].method.argv[1] = format!(
            "assert open('a-completed').read()=='done';{}",
            p.operations[1].method.argv[1]
        );
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
            ("a", "open('a','x').write('a')"),
            ("b", "assert open('a').read()=='a';open('b','x').write('b')"),
            ("c", "assert open('a').read()=='a';open('c','x').write('c')"),
            (
                "d",
                "assert open('b').read()=='b';assert open('c').read()=='c';open('d','x').write('d')",
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
    assert!(r.passed());
    assert_eq!(r.executed.len(), 4);
    assert_eq!(std::fs::read_to_string(d.path().join("d")).unwrap(), "d");
}
#[test]
fn timeout_and_failed_exit_keep_original_bytes_and_only_block_descendants() {
    for script in [
        "import os,time;os.write(1,b'original');time.sleep(5)",
        "import os;os.write(1,b'original');raise SystemExit(9)",
    ] {
        let (_d, mut p) = plan(
            &[
                ("a", script),
                ("b", "raise Exception('blocked')"),
                ("z", "print('sibling')"),
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
        if script.contains("sleep") {
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
        &[("a", "print('must not run')"), ("z", "print('sibling')")],
        &[("test:a", vec!["a"]), ("test:z", vec!["z"])],
    );
    let mut bad = p.tools["python"].clone();
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
            ("a", "open('first','x').write('done')"),
            ("b", "assert open('first').read()=='done'"),
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
    assert!(r.passed());
    assert!(d.path().join("first").exists());
}

#[test]
fn spawn_failure_preserves_cause_and_continues_independent_operation() {
    use std::os::unix::fs::PermissionsExt;
    let (d, mut p) = plan(
        &[("a", "print('unreachable')"), ("z", "print('sibling')")],
        &[("test:a", vec!["a"]), ("test:z", vec!["z"])],
    );
    let path = d.path().join("not-executable");
    std::fs::copy(&p.tools["python"].path, &path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let mut bad = p.tools["python"].clone();
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
            ("a", "import os;os.write(1,b'x'*8000)"),
            ("b", "raise Exception('blocked')"),
            ("z", "print('sibling')"),
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
            ("a", "print('original')"),
            ("b", "raise Exception('blocked')"),
            (
                "z",
                "open('sibling','x').write('executed');print('sibling')",
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
    p.operations[1].method.argv[1].push_str(";open('b-completed','w').write('done')");
    p.operations[2].method.argv[1] = format!(
        "try:\n assert open('b-completed').read()=='done'\nexcept FileNotFoundError:\n open('excess-launch','w').write('observed')\n raise\n{}",
        p.operations[2].method.argv[1]
    );
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
        // Wait for c's actual failed process observation to become terminal,
        // while held a/b cannot produce the required marker. This is bounded
        // by their existing five-second process deadline.
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
                .contains("FileNotFoundError")
        );
    });
}
