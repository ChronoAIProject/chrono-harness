use super::*;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn child() -> &'static str {
    env!("CARGO_BIN_EXE_chrono-test-ci-diagnostics")
}
fn host(address: Option<String>) -> Host {
    let h = Host::new();
    h.change_registry(CONFIG, |v| {
        v["schema"] = json!("chrono-ci-check/v3");
        v["policy"]["execution_diagnostics"] = json!("chrono-log-stderr/v1");
        v["policy"]["report_publication"] = json!("retained-reference/v2");
        v["policy"]["units"] = json!({"diagnostic":{
            "tests":["test:suite"],"report_path":".chrono-harness/state/diagnostic/check.json"
        }});
        v["policy"]["shared_operations"] = json!({});
        v["policy"]["tools"] = json!({"fixture":child()});
    });
    h.change_registry(".chrono-harness/projects.json", |v| {
        v["projects"][1]["actions"]["execute"]["tool"] = json!("fixture");
        let mut argv = vec!["operation".to_string()];
        argv.extend(address);
        v["projects"][1]["actions"]["execute"]["argv"] = json!(argv);
    });
    explicit_plans(&h);
    h.commit();
    h
}
fn request(h: &Host) -> Request {
    let base = h.head();
    h.write("src.txt", "diagnostic consumer\n");
    let candidate = h.commit();
    unit_request(h, &base, &candidate)
}
fn unit_request(h: &Host, base: &str, candidate: &str) -> Request {
    let mut req = h.request(Some(base), candidate);
    req.protocol = chrono_harness::units::PROTOCOL.into();
    req.scope = Some(chrono_harness::units::Scope::Unit {
        unit: "diagnostic".into(),
    });
    req
}
fn records(bytes: &[u8]) -> Vec<Value> {
    bytes
        .split(|b| *b == b'\n')
        .filter(|s| !s.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}
fn event<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["event"] == name)
        .unwrap_or_else(|| panic!("missing diagnostic {name}"))
}

#[test]
fn execution_diagnostics_failed_check_preserves_originals_and_identity() {
    let h = host(None);
    let req = request(&h);
    let assert_result = std::panic::catch_unwind(|| {
        let directory = h.root().join(".chrono-harness/state");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("request.json"),
            serde_json::to_vec(&req).unwrap(),
        )
        .unwrap();
        let sink = fs::File::create(directory.join("diagnostics.jsonl")).unwrap();
        let response = chrono_judge_ci::judge_with_diagnostic_sink(&req, sink);
        fs::write(
            directory.join("response.json"),
            serde_json::to_vec(&response).unwrap(),
        )
        .unwrap();
        fail(&response, "operation exit 7");
        let process = chrono_harness::full::expand_process(
            &response.evidence["executed"][0]["receipt"]["process"],
        )
        .unwrap();
        assert_eq!(process["stdout_bytes"], json!([255, 0, 10]));
        assert_eq!(process["stderr_bytes"], json!([254, 0, 10]));
        assert_eq!(process["exit_code"], 7);
        let rows = records(&fs::read(directory.join("diagnostics.jsonl")).unwrap());
        let launch = rows
            .iter()
            .find(|r| r["event"] == "process.launched" && r["context"]["operation"] == "test.suite")
            .unwrap();
        assert_eq!(launch["schema"], "chrono-log/v1");
        assert_eq!(launch["context"]["request_id"], req.request_id);
        assert_eq!(launch["context"]["candidate"], req.candidate);
        assert_eq!(launch["context"]["config_sha256"], req.config_sha256);
        assert_eq!(launch["context"]["operation"], "test.suite");
        assert_eq!(
            launch["context"]["plan"],
            response.evidence["plan"]["identity"]
        );
        assert_eq!(launch["context"]["child_pid"], process["child_pid"]);
        assert_eq!(launch["context"]["launcher_pid"], process["launcher_pid"]);
        let argv: Value = serde_json::from_str(launch["fields"]["argv"].as_str().unwrap()).unwrap();
        assert_eq!(argv, process["argv"]);
        assert_eq!(launch["fields"]["executable_sha256"], process["sha256"]);
        let phases: Vec<_> = rows
            .iter()
            .filter(|r| r["context"]["operation"] == "test.suite")
            .map(|r| r["event"].as_str().unwrap())
            .collect();
        for pair in [
            ["process.launched", "process.readers_ready_wait"],
            ["process.readers_ready_wait", "process.monitor_wait"],
            ["process.launched", "process.monitor_wait"],
            ["process.monitor_wait", "process.ownership_drain"],
            ["process.ownership_drain", "process.streams_join"],
            ["process.streams_join", "process.completed"],
            ["process.completed", "projects.operation_completed"],
        ] {
            assert!(
                phases.iter().position(|s| *s == pair[0]).unwrap()
                    < phases.iter().position(|s| *s == pair[1]).unwrap()
            );
        }
        let joins = [
            "projects.workers_join",
            "projects.workers_joined",
            "ci.evaluation_returned",
        ];
        for pair in joins.windows(2) {
            assert!(
                rows.iter().position(|r| r["event"] == pair[0]).unwrap()
                    < rows.iter().position(|r| r["event"] == pair[1]).unwrap()
            );
        }
        assert_eq!(
            event(&rows, "projects.operation_completed")["fields"]["status"],
            "failed"
        );
        assert_eq!(
            rows.iter()
                .find(|r| r["event"] == "process.completed"
                    && r["context"]["operation"] == "test.suite")
                .unwrap()["fields"]["exit_code"],
            7
        );
    });
    if let Err(error) = assert_result {
        eprintln!(
            "failed diagnostic fixture retained at {}",
            h.dir.keep().display()
        );
        std::panic::resume_unwind(error);
    }
}

#[test]
fn execution_diagnostics_incomplete_check_retains_live_producer_events() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let h = host(Some(listener.local_addr().unwrap().to_string()));
    let req = request(&h);
    let assert_result = std::panic::catch_unwind(|| {
        let directory = h.root().join(".chrono-harness/state");
        fs::create_dir_all(&directory).unwrap();
        let stdout = directory.join("producer.stdout");
        let stderr = directory.join("producer.stderr");
        let launch = directory.join("producer.launch.json");
        let spec = chrono_harness::CommandSpec {
            program: child().into(),
            args: vec!["judge".into()],
            env: BTreeMap::new(),
            timeout_seconds: 1,
            output_limit_bytes: 65536,
        };
        let digest = chrono_harness::file_identity(Path::new(child())).unwrap().0;
        let input = serde_json::to_vec(&req).unwrap();
        fs::write(directory.join("request.json"), &input).unwrap();
        let result = std::thread::scope(|scope| {
            let task = scope.spawn(|| {
                chrono_harness::run_process_observed_retained(
                    h.root(),
                    &spec,
                    &input,
                    &digest,
                    &chrono_harness::ProcessEvidence {
                        stdout: &stdout,
                        stderr: &stderr,
                        launch: &launch,
                    },
                )
                .unwrap()
            });
            // The original engine has its own finite deadline. This joins that task;
            // a ready child is never released, so no final business report exists.
            let result = task.join().unwrap();
            result
        });
        fs::write(
            directory.join("result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        // Observe only the already joined producer's actual handshake, without
        // adding a readiness wait or changing the enclosing engine deadline.
        listener.set_nonblocking(true).unwrap();
        let readiness = match listener.accept() {
            Ok((mut socket, peer)) => {
                socket.set_nonblocking(true).unwrap();
                let mut bytes = Vec::new();
                let read = socket.read_to_end(&mut bytes);
                json!({"peer":peer.to_string(),"bytes":bytes,"read_error":read.err().map(|e|e.to_string())})
            }
            Err(error) => json!({"accept_error":error.to_string()}),
        };
        fs::write(
            directory.join("readiness.json"),
            serde_json::to_vec(&readiness).unwrap(),
        )
        .unwrap();
        // Keep the producer originals on failed assertions; successful fixtures are
        // exclusively test-owned and the existing TempDir owner removes them.
        assert_eq!(result.failure.as_deref(), Some("process timed out"));
        assert_eq!(fs::read(&stdout).unwrap(), result.stdout_bytes);
        assert!(
            result.stdout_bytes.is_empty(),
            "incomplete judge must not invent a final response"
        );
        assert_eq!(fs::read(&stderr).unwrap(), result.stderr_bytes);
        let rows = records(&result.stderr_bytes);
        let launch = rows
            .iter()
            .find(|r| r["event"] == "process.launched" && r["context"]["operation"] == "test.suite")
            .unwrap();
        let waiting = rows
            .iter()
            .find(|r| {
                r["event"] == "process.monitor_wait" && r["context"]["operation"] == "test.suite"
            })
            .unwrap();
        assert_eq!(launch["context"]["request_id"], req.request_id);
        assert_eq!(waiting["context"]["candidate"], req.candidate);
        assert_eq!(
            waiting["context"]["child_pid"],
            launch["context"]["child_pid"]
        );
        assert!(
            !rows
                .iter()
                .any(|r| r["event"] == "projects.operation_completed")
        );
        let original: Value =
            serde_json::from_slice(&fs::read(&directory.join("producer.launch.json")).unwrap())
                .unwrap();
        assert_eq!(original["child_pid"], result.child_pid.unwrap());
        assert_eq!(launch["context"]["launcher_pid"], original["child_pid"]);
        let identity = rows
            .iter()
            .find(|r| {
                r["event"] == "process.ownership_identity"
                    && r["context"]["operation"] == "test.suite"
            })
            .unwrap();
        let snapshot =
            &result.failure_record.as_ref().unwrap().error.context["ownership_at_monitor_end"];
        let owned = snapshot["launches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["child_pid"] == launch["context"]["child_pid"])
            .unwrap();
        assert_eq!(identity["fields"]["slot"], owned["slot"]);
        assert_eq!(identity["fields"]["generation"], owned["generation"]);
        assert_eq!(identity["fields"]["parent_slot"], owned["parent_slot"]);
    });
    if let Err(error) = assert_result {
        eprintln!(
            "failed diagnostic fixture retained at {}",
            h.dir.keep().display()
        );
        std::panic::resume_unwind(error);
    }
}

#[test]
fn execution_diagnostics_success_and_unselected_sink_preserve_verdicts() {
    struct Reject;
    impl Write for Reject {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from_raw_os_error(28))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let h = host(Some("success".into()));
    let req = request(&h);
    let sink = Sink::default();
    let response = chrono_judge_ci::judge_with_diagnostic_sink(&req, sink.clone());
    pass(&response);
    let rows = records(&sink.0.lock().unwrap());
    assert_eq!(
        event(&rows, "projects.operation_completed")["fields"]["status"],
        "passed"
    );
    // A configured failed sink cannot turn a passing business result into success.
    let failed = chrono_judge_ci::judge_with_diagnostic_sink(&req, Reject);
    fail(&failed, "ci.execution-diagnostics");
    assert_eq!(
        failed.evidence["executed"][0]["receipt"]["process"]["exit_code"],
        0
    );
    h.change_registry(CONFIG, |v| {
        v["policy"]
            .as_object_mut()
            .unwrap()
            .remove("execution_diagnostics");
    });
    let candidate = h.commit();
    let unselected = unit_request(&h, &req.candidate, &candidate);
    pass(&chrono_judge_ci::judge_with_diagnostic_sink(
        &unselected,
        Reject,
    ));
    h.change_registry(CONFIG, |v| {
        v["policy"]["execution_diagnostics"] = json!("unsupported");
    });
    let candidate = h.commit();
    let unsupported = unit_request(&h, &unselected.candidate, &candidate);
    fail(
        &chrono_judge_ci::judge_with_diagnostic_sink(&unsupported, Sink::default()),
        "unsupported",
    );
}

#[test]
fn execution_diagnostics_sink_failure_retains_business_failure() {
    struct Reject;
    impl Write for Reject {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from_raw_os_error(28))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let h = host(None);
    let req = request(&h);
    let r = chrono_judge_ci::judge_with_diagnostic_sink(&req, Reject);
    fail(&r, "operation exit 7");
    assert!(r.results.iter().any(|v| v.id == "ci.execution-diagnostics"));
    assert_eq!(
        r.evidence["executed"][0]["receipt"]["process"]["exit_code"],
        7
    );
    let errors = &r.evidence["diagnostic_errors"];
    assert_eq!(errors[0]["error"]["code"], "E_LOG_IO");
    assert_eq!(errors[0]["error"]["source"]["native_code"], 28);
}
