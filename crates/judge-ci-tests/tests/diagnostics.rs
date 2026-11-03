use super::*;
use std::collections::BTreeMap;
use std::io::{BufRead, Read, Write};
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
        // INFO reports the domain operation and its outcome. Process details
        // remain in the original receipt and the explicitly selected DEBUG log.
        assert!(
            rows.iter()
                .all(|r| !r["event"].as_str().unwrap().starts_with("process.")),
            "process detail leaked into normal INFO diagnostics"
        );
        let started = event(&rows, "projects.operation_started");
        assert_eq!(started["context"]["request_id"], req.request_id);
        assert_eq!(started["context"]["candidate"], req.candidate);
        assert_eq!(started["context"]["config_sha256"], req.config_sha256);
        assert_eq!(started["context"]["operation"], "test.suite");
        assert_eq!(
            started["context"]["plan"],
            response.evidence["plan"]["identity"]
        );
        assert!(
            rows.iter()
                .position(|r| r["event"] == "projects.operation_started")
                .unwrap()
                < rows
                    .iter()
                    .position(|r| r["event"] == "projects.operation_completed")
                    .unwrap()
        );
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
    incomplete_execution(false);
}

#[test]
fn execution_diagnostics_delayed_admission_does_not_consume_interruption_window() {
    incomplete_execution(true);
}

fn incomplete_execution(delayed_admission: bool) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let h = host(Some(listener.local_addr().unwrap().to_string()));
    // Omit the optional INFO adoption so the child fixture can use the existing
    // caller-scoped DEBUG logger without a new product logging API or policy.
    h.change_registry(CONFIG, |v| {
        v["policy"]
            .as_object_mut()
            .unwrap()
            .remove("execution_diagnostics");
    });
    h.commit();
    let admission = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = admission.local_addr().unwrap();
    if delayed_admission {
        h.change_registry(CONFIG, |v| {
            v["policy"]["environment"] = json!({"CHRONO_TEST_ADMISSION": address.to_string()});
        });
        h.commit();
    }
    let req = request(&h);
    let assert_result = std::panic::catch_unwind(|| {
        let directory = h.root().join(".chrono-harness/state");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("request.json"),
            serde_json::to_vec(&req).unwrap(),
        )
        .unwrap();
        let began = std::time::Instant::now();
        let admitted = std::sync::atomic::AtomicBool::new(false);
        let prepared = if delayed_admission {
            std::thread::scope(|scope| {
                let admitted = &admitted;
                let task = scope.spawn(move || {
                    let result = chrono_judge_ci::prepare_execution(&req);
                    // Wake the one native accept on an admission error too.
                    if !admitted.load(std::sync::atomic::Ordering::Acquire) {
                        std::net::TcpStream::connect(address)
                            .unwrap()
                            .write_all(b"finished!")
                            .unwrap();
                    }
                    result
                });
                let (mut socket, _) = admission.accept().unwrap();
                admitted.store(true, std::sync::atomic::Ordering::Release);
                socket
                    .set_read_timeout(Some(std::time::Duration::from_millis(1100)))
                    .unwrap();
                let mut message = [0; 9];
                socket.read_exact(&mut message).unwrap();
                assert_eq!(&message, b"admission");
                let held = std::time::Instant::now();
                let error = socket.read_exact(&mut [0]).unwrap_err();
                let elapsed = held.elapsed();
                fs::write(directory.join("admission.json"), serde_json::to_vec(&json!({
                    "message":message,"held_millis":elapsed.as_millis(),"observation_error":error.to_string()
                })).unwrap()).unwrap();
                assert!(elapsed >= std::time::Duration::from_secs(1));
                assert!(matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ));
                socket.write_all(&[0]).unwrap();
                task.join().unwrap().unwrap()
            })
        } else {
            chrono_judge_ci::prepare_execution(&req).unwrap()
        };
        fs::write(
            directory.join("preparation.json"),
            serde_json::to_vec(&json!({
                "elapsed_millis":began.elapsed().as_millis(),"plan":prepared.plan.identity
            }))
            .unwrap(),
        )
        .unwrap();
        let req = &prepared.request;
        let stdout = directory.join("producer.stdout");
        let stderr = directory.join("producer.stderr");
        let launch = directory.join("producer.launch.json");
        let spec = chrono_harness::CommandSpec {
            program: child().into(),
            args: vec!["execute-debug".into()],
            env: BTreeMap::new(),
            timeout_seconds: 1,
            output_limit_bytes: 65536,
        };
        let digest = chrono_harness::file_identity(Path::new(child())).unwrap().0;
        let input = serde_json::to_vec(&prepared).unwrap();
        fs::write(directory.join("prepared.json"), &input).unwrap();
        let readiness_observed = std::sync::atomic::AtomicBool::new(false);
        let began = std::time::Instant::now();
        let (result, readiness) = std::thread::scope(|scope| {
            let task = scope.spawn(|| {
                let result = chrono_harness::run_process_observed_retained(
                    h.root(),
                    &spec,
                    &input,
                    &digest,
                    &chrono_harness::ProcessEvidence {
                        stdout: &stdout,
                        stderr: &stderr,
                        launch: &launch,
                    },
                );
                // A missing real readiness is also finite: the joined engine
                // wakes this single blocking receiver, without a status poll.
                if !readiness_observed.load(std::sync::atomic::Ordering::Acquire) {
                    if let Ok(mut socket) =
                        std::net::TcpStream::connect(listener.local_addr().unwrap())
                    {
                        let _ = socket.write_all(b"finished\n");
                    }
                }
                result
            });
            let (readiness, socket_guard) = match listener.accept() {
                Ok((socket, peer)) => {
                    readiness_observed.store(true, std::sync::atomic::Ordering::Release);
                    socket
                        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let read = std::io::BufReader::new(&socket).read_until(b'\n', &mut bytes);
                    (
                        json!({"peer":peer.to_string(),"bytes":bytes,
                        "observed_micros":began.elapsed().as_micros(),"read_error":read.err().map(|e|e.to_string())}),
                        Some(socket),
                    )
                }
                Err(error) => (json!({"accept_error":error.to_string()}), None),
            };
            // Never release the ready business process. Join the original
            // finite timeout and all of its engine-owned descendants/streams.
            let result = task.join().unwrap().unwrap();
            drop(socket_guard);
            (result, readiness)
        });
        fs::write(
            directory.join("result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
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
        assert!(readiness["read_error"].is_null(), "{readiness}");
        assert!(
            readiness["observed_micros"].as_u64().unwrap() < 1_000_000,
            "nested execution was not observed before the one-second interruption: {readiness}"
        );
        let rows = records(&result.stderr_bytes);
        let launch = rows
            .iter()
            .find(|r| r["event"] == "process.launched" && r["context"]["operation"] == "test.suite")
            .unwrap();
        assert_eq!(launch["context"]["request_id"], req.request_id);
        assert_eq!(launch["context"]["candidate"], req.candidate);
        assert!(launch["context"]["child_pid"].is_number());
        assert_eq!(launch["context"]["config_sha256"], req.config_sha256);
        assert_eq!(launch["context"]["plan"], prepared.plan.identity);
        assert_eq!(launch["level"], "debug");
        let ready: Vec<u8> = serde_json::from_value(readiness["bytes"].clone()).unwrap();
        assert_eq!(
            ready,
            format!("ready:{}\n", launch["context"]["child_pid"]).as_bytes()
        );
        let identity = rows
            .iter()
            .find(|r| {
                r["event"] == "process.ownership_identity"
                    && r["context"]["child_pid"] == launch["context"]["child_pid"]
            })
            .unwrap();
        // The monitor observes live execution before cancellation. Cleanup
        // can still join the nested engine and publish its genuine error
        // outcome before the enclosing process is killed. Such an event is
        // distinct from a final check response or a successful operation.
        for completed in rows
            .iter()
            .filter(|r| r["event"] == "projects.operation_completed")
        {
            for key in [
                "request_id",
                "candidate",
                "config_sha256",
                "plan",
                "operation",
            ] {
                assert_eq!(completed["context"][key], launch["context"][key]);
            }
            assert_eq!(completed["fields"]["status"], "error");
            assert_eq!(completed["fields"]["has_receipt"], true);
        }
        for completed in rows.iter().filter(|r| {
            r["event"] == "process.completed" && r["context"]["operation"] == "test.suite"
        }) {
            assert_eq!(
                completed["context"]["child_pid"],
                launch["context"]["child_pid"]
            );
            assert_eq!(completed["fields"]["exit_code"], -1);
            assert_eq!(
                completed["fields"]["failure"],
                "process cancelled by enclosing owner"
            );
            assert_eq!(
                completed["fields"]["stdout_sha256"],
                chrono_harness::sha256(&[255, 0, 10])
            );
            assert_eq!(
                completed["fields"]["stderr_sha256"],
                chrono_harness::sha256(&[254, 0, 10])
            );
        }
        let original: Value =
            serde_json::from_slice(&fs::read(&directory.join("producer.launch.json")).unwrap())
                .unwrap();
        assert_eq!(original["child_pid"], result.child_pid.unwrap());
        assert_eq!(original["timeout_seconds"], 1);
        assert_eq!(original["output_limit_bytes"], 65536);
        assert_eq!(original["stdin_sha256"], chrono_harness::sha256(&input));
        assert_eq!(launch["fields"]["timeout_seconds"], 3);
        assert_eq!(launch["fields"]["output_limit_bytes"], 4096);
        assert_eq!(identity["level"], "debug");
        assert_eq!(launch["context"]["launcher_pid"], original["child_pid"]);
        let snapshot =
            &result.failure_record.as_ref().unwrap().error.context["ownership_at_monitor_end"];
        let owned = snapshot["launches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["child_pid"] == launch["context"]["child_pid"])
            .unwrap();
        assert_eq!(owned["terminal"], false);
        assert_eq!(owned["launcher_terminal"], false);
        assert_eq!(owned["terminal_observed_ns"], 0);
        for key in ["slot", "generation", "parent_slot"] {
            assert_eq!(
                owned[key], identity["fields"][key],
                "ownership identity drift: {key}"
            );
        }
    });
    if let Err(error) = assert_result {
        eprintln!(
            "failed diagnostic fixture retained at {}",
            h.dir.keep().display()
        );
        std::panic::resume_unwind(error);
    }
    if let Some(destination) = std::env::var_os("CHRONO_CI_TEST_RECEIPTS") {
        // The registered evidence consumer needs the actual timeout originals
        // even when the assertions pass. This is neither a copied report nor
        // a completion claim; its caller owns acceptance and subsequent release.
        let original = h.dir.keep();
        let identity = chrono_harness::sha256(original.to_str().unwrap().as_bytes());
        let mut files = serde_json::Map::new();
        for name in [
            "request.json",
            "prepared.json",
            "preparation.json",
            "producer.stdout",
            "producer.stderr",
            "producer.launch.json",
            "result.json",
            "readiness.json",
        ] {
            let path = original.join(".chrono-harness/state").join(name);
            let (digest, size) = chrono_harness::file_identity(&path).unwrap();
            files.insert(
                name.into(),
                json!({"path":path,"sha256":digest,"size":size}),
            );
        }
        if delayed_admission {
            let path = original.join(".chrono-harness/state/admission.json");
            let (digest, size) = chrono_harness::file_identity(&path).unwrap();
            files.insert(
                "admission.json".into(),
                json!({"path":path,"sha256":digest,"size":size}),
            );
        }
        let mut receipt = fs::File::create_new(
            Path::new(&destination).join(format!("diagnostic-{identity}.json")),
        )
        .unwrap();
        serde_json::to_writer(
            &mut receipt,
            &json!({"consumer":"finite engineering acceptance",
            "retention":"until caller debugging/acceptance obligations finish; caller owns release",
            "originals":files,"delayed_admission":delayed_admission}),
        )
        .unwrap();
        eprintln!("diagnostic execution originals: {}", original.display());
    }
}

#[test]
fn execution_diagnostics_prepared_consumer_validates_request_and_uses_registered_execution() {
    let h = host(Some("success".into()));
    let req = request(&h);
    let prepared = chrono_judge_ci::prepare_execution(&req).unwrap();
    assert_eq!(prepared.plan.operations.len(), 1);
    assert_eq!(prepared.plan.operations[0].method.operation, "test.suite");
    assert_eq!(prepared.plan.operations[0].timeout_seconds, 3);
    assert_eq!(prepared.plan.operations[0].output_limit_bytes, 4096);
    let mut changed = prepared.clone();
    changed.request.request_id.push_str("-changed");
    assert!(changed.execute().unwrap_err().contains("binding differs"));
    assert!(prepared.execute().unwrap().passed());
    let mut unscoped = req;
    unscoped.scope = None;
    assert!(
        chrono_judge_ci::prepare_execution(&unscoped)
            .unwrap_err()
            .contains("scoped unit")
    );
}

#[test]
fn execution_diagnostics_debug_process_preserves_context_and_originals() {
    use chrono_diagnostics::logging::{Level, Logger};
    for (argument, exit) in [("success", 0), ("failure", 7)] {
        let h = Host::new();
        let directory = h.root().join(".chrono-harness/state");
        fs::create_dir_all(&directory).unwrap();
        let stdout = directory.join("stdout.bytes");
        let stderr = directory.join("stderr.bytes");
        let launch = directory.join("launch.json");
        let input = [0, 255, 10];
        let spec = chrono_harness::CommandSpec {
            program: child().into(),
            args: if exit == 0 {
                vec!["operation".into(), argument.into()]
            } else {
                vec!["operation".into()]
            },
            env: BTreeMap::new(),
            timeout_seconds: 30,
            output_limit_bytes: 4096,
        };
        let digest = chrono_harness::file_identity(Path::new(child())).unwrap().0;
        let sink = Sink::default();
        let logger = Logger::new("real-process-test", Level::DEBUG, sink.clone());
        let process = logger.run(|| {
            let span = tracing::info_span!(
                "test.operation",
                request_id = "real-process-request",
                operation = argument
            );
            let _entered = span.enter();
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
        logger.finish().unwrap();
        let bytes = sink.0.lock().unwrap().clone();
        // Retain producer originals before assertions; failed fixtures survive.
        fs::write(directory.join("debug.jsonl"), &bytes).unwrap();
        fs::write(
            directory.join("process.json"),
            serde_json::to_vec(&process).unwrap(),
        )
        .unwrap();
        let asserted = std::panic::catch_unwind(|| {
            assert_eq!(process.exit_code, exit);
            assert!(process.failure.is_none());
            assert_eq!(process.stdout_bytes, [255, 0, 10]);
            assert_eq!(process.stderr_bytes, [254, 0, 10]);
            assert_eq!(fs::read(&stdout).unwrap(), process.stdout_bytes);
            assert_eq!(fs::read(&stderr).unwrap(), process.stderr_bytes);
            let original: Value = serde_json::from_slice(&fs::read(&launch).unwrap()).unwrap();
            let rows = records(&bytes);
            let started = event(&rows, "process.launched");
            assert_eq!(started["level"], "debug");
            assert_eq!(started["context"]["request_id"], "real-process-request");
            assert_eq!(started["context"]["operation"], argument);
            for key in ["child_pid", "launcher_pid"] {
                assert_eq!(started["context"][key], original[key]);
            }
            let argv: Value =
                serde_json::from_str(started["fields"]["argv"].as_str().unwrap()).unwrap();
            assert_eq!(argv, original["argv"]);
            for key in [
                "cwd",
                "stdin_sha256",
                "timeout_seconds",
                "output_limit_bytes",
            ] {
                assert_eq!(started["fields"][key], original[key]);
            }
            assert_eq!(
                started["fields"]["stdin_sha256"],
                chrono_harness::sha256(&input)
            );
            assert_eq!(started["fields"]["executable_sha256"], original["sha256"]);
            assert_eq!(
                started["fields"]["environment_sha256"],
                original["environment_digest"]
            );
            let completed = event(&rows, "process.completed");
            assert_eq!(completed["level"], "debug");
            assert_eq!(completed["context"]["child_pid"], original["child_pid"]);
            assert_eq!(completed["fields"]["exit_code"], exit);
            assert_eq!(completed["fields"]["stdout_sha256"], process.stdout_sha256);
            assert_eq!(completed["fields"]["stderr_sha256"], process.stderr_sha256);
            assert_eq!(completed["fields"]["stdout_bytes"], 3);
            assert_eq!(completed["fields"]["stderr_bytes"], 3);
            let phases = [
                "process.preparing",
                "process.launched",
                "process.ownership_identity",
                "process.monitor_wait",
                "process.streams_join",
                "process.scope_exit",
                "process.completed",
            ];
            for pair in phases.windows(2) {
                assert!(
                    rows.iter().position(|r| r["event"] == pair[0]).unwrap()
                        < rows.iter().position(|r| r["event"] == pair[1]).unwrap()
                );
            }
        });
        if let Err(error) = asserted {
            eprintln!(
                "failed real process fixture retained at {}",
                h.dir.keep().display()
            );
            std::panic::resume_unwind(error);
        }
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
