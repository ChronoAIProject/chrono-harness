#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
mod chrono_git_boundary;
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    incomplete_execution(false);
}

#[test]
fn execution_diagnostics_delayed_admission_does_not_consume_interruption_window() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    incomplete_execution(true);
}

struct ReadinessGate {
    log: fs::File,
    listener: Option<std::net::TcpListener>,
    socket: Option<std::net::TcpStream>,
    observed: Arc<Mutex<Value>>,
    startup_millis: i32,
}
impl Write for ReadinessGate {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.log.write_all(bytes)?;
        let row: Value = serde_json::from_slice(bytes).map_err(std::io::Error::other)?;
        if row["event"] == "process.readers_ready_wait" {
            use std::os::fd::AsRawFd;
            let listener = self
                .listener
                .take()
                .ok_or_else(|| std::io::Error::other("duplicate outer reader barrier"))?;
            let began = std::time::Instant::now();
            let mut event = libc::pollfd {
                fd: listener.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // One bounded native wait, never a status-poll loop. A nonblocking
            // accept also keeps a stale readiness notification finite.
            let polled = unsafe { libc::poll(&mut event, 1, self.startup_millis) };
            let mut observed = if polled == 1 && event.revents & libc::POLLIN != 0 {
                match (|| -> std::io::Result<Value> {
                    let (socket, peer) = listener.accept()?;
                    socket.set_nonblocking(false)?;
                    socket.set_read_timeout(Some(std::time::Duration::from_secs(1)))?;
                    let mut bytes = Vec::new();
                    let read = std::io::BufReader::new(&socket).read_until(b'\n', &mut bytes);
                    self.socket = Some(socket);
                    Ok(json!({"peer":peer.to_string(),"bytes":bytes,
                        "read_error":read.err().map(|e|e.to_string())}))
                })() {
                    Ok(value) => value,
                    Err(error) => json!({"startup_error":error.to_string()}),
                }
            } else if polled == 0 {
                json!({"startup_error":"business readiness timed out"})
            } else if polled < 0 {
                json!({"startup_error":std::io::Error::last_os_error().to_string(),
                    "poll_result":polled,"poll_events":event.revents})
            } else {
                json!({"startup_error":"readiness listener was not readable",
                    "poll_result":polled,"poll_events":event.revents})
            };
            observed["wait_micros"] = json!(began.elapsed().as_micros());
            *self.observed.lock().unwrap() = observed;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.log.flush()
    }
}

fn diagnostic_execution(
    h: &Host,
    req: &Request,
    listener: std::net::TcpListener,
    admission: Option<std::net::TcpListener>,
) -> chrono_harness::ProcessResult {
    let directory = h.root().join(".chrono-harness/state");
    fs::create_dir_all(&directory).unwrap();
    let input = serde_json::to_vec(req).unwrap();
    fs::write(directory.join("request.json"), &input).unwrap();
    let config: Value = serde_json::from_slice(&fs::read(h.root().join(CONFIG)).unwrap()).unwrap();
    listener.set_nonblocking(true).unwrap();
    let observed = Arc::new(Mutex::new(Value::Null));
    let logger = chrono_diagnostics::logging::Logger::new(
        "ci-monitor-fixture",
        chrono_diagnostics::logging::Level::DEBUG,
        ReadinessGate {
            log: fs::File::create(directory.join("monitor.jsonl")).unwrap(),
            listener: Some(listener),
            socket: None,
            observed: observed.clone(),
            // Reuse this fixture's existing judge admission bound. The actual
            // engine monitor remains one second and the operation three seconds.
            startup_millis: i32::try_from(
                config["judge"]["timeout_seconds"].as_u64().unwrap() * 1000,
            )
            .unwrap(),
        },
    );
    let spec = chrono_harness::CommandSpec {
        program: child().into(),
        args: vec!["judge-debug".into()],
        // The ordinary judge performs Git admission in this child too. Bind the
        // same private Git fixture that created this host's synthetic commits.
        env: BTreeMap::from([("PATH".into(), std::env::var("PATH").unwrap())]),
        timeout_seconds: 1,
        output_limit_bytes: 65536,
    };
    let digest = chrono_harness::file_identity(Path::new(child())).unwrap().0;
    let admitted = std::sync::atomic::AtomicBool::new(false);
    let address = admission
        .as_ref()
        .map(|listener| listener.local_addr().unwrap());
    let result = std::thread::scope(|scope| {
        let task = scope.spawn(|| {
            let result = logger.run(|| {
                chrono_harness::run_process_observed_retained(
                    h.root(),
                    &spec,
                    &input,
                    &digest,
                    &chrono_harness::ProcessEvidence {
                        root: h.root(),
                        stdout: &directory.join("producer.stdout"),
                        stderr: &directory.join("producer.stderr"),
                        launch: &directory.join("producer.launch.json"),
                        publication: None,
                    },
                )
            });
            // Early admission failure must wake the original single accept.
            if let Some(address) = address {
                if !admitted.load(std::sync::atomic::Ordering::Acquire) {
                    if let Ok(mut socket) = std::net::TcpStream::connect(address) {
                        let _ = socket.write_all(b"finished!");
                    }
                }
            }
            result
        });
        if let Some(admission) = admission {
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
        }
        task.join().unwrap().unwrap()
    });
    logger.finish().unwrap();
    fs::write(
        directory.join("readiness.json"),
        serde_json::to_vec(&*observed.lock().unwrap()).unwrap(),
    )
    .unwrap();
    fs::write(
        directory.join("result.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
    // The gate owns the ready socket until the real engine has joined its work.
    drop(logger);
    result
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
        let input = serde_json::to_vec(&req).unwrap();
        let result =
            diagnostic_execution(&h, &req, listener, delayed_admission.then_some(admission));
        let readiness: Value =
            serde_json::from_slice(&fs::read(directory.join("readiness.json")).unwrap()).unwrap();
        let stdout = directory.join("producer.stdout");
        let stderr = directory.join("producer.stderr");
        // Keep the producer originals on failed assertions; successful fixtures are
        // exclusively test-owned and the existing TempDir owner removes them.
        assert_eq!(result.failure.as_deref(), Some("process timed out"));
        assert!(result.owned_work_joined);
        assert!(result.stdout_complete && result.stderr_complete);
        assert_eq!(fs::read(&stdout).unwrap(), result.stdout_bytes);
        assert_eq!(fs::read(&stderr).unwrap(), result.stderr_bytes);
        assert!(readiness["startup_error"].is_null(), "{readiness}");
        assert!(readiness["read_error"].is_null(), "{readiness}");
        let ready: Vec<u8> = serde_json::from_value(readiness["bytes"].clone()).unwrap();
        let (_, ready_ns) = std::str::from_utf8(&ready)
            .unwrap()
            .strip_prefix("ready:")
            .unwrap()
            .trim_end_matches('\n')
            .split_once(':')
            .unwrap();
        let ready_ns: u64 = ready_ns.parse().unwrap();
        let deadline =
            result.failure_record.as_ref().unwrap().error.context["terminal_deadline_ns"]
                .as_u64()
                .unwrap();
        // Acquisition before spawn is outside the monitor's one-second budget.
        // Compare the actual nested readiness to that monitor's original deadline.
        assert!(
            ready_ns < deadline,
            "nested execution was not ready before the one-second interruption: {readiness}; deadline={deadline}"
        );
        assert!(
            ready_ns <= deadline.checked_sub(1_000_000_000).unwrap(),
            "the one-second monitor started before real business readiness: {readiness}; deadline={deadline}"
        );
        let rows = records(&fs::read(directory.join("diagnostics.jsonl")).unwrap());
        let launch = rows
            .iter()
            .find(|r| r["event"] == "process.launched" && r["context"]["operation"] == "test.suite")
            .unwrap();
        assert_eq!(launch["context"]["request_id"], req.request_id);
        assert_eq!(launch["context"]["candidate"], req.candidate);
        assert!(launch["context"]["child_pid"].is_number());
        assert_eq!(launch["context"]["config_sha256"], req.config_sha256);
        let plan = &launch["context"]["plan"];
        assert!(plan.as_str().is_some_and(|identity| !identity.is_empty()));
        assert_eq!(launch["level"], "debug");
        // Ordinary CI can publish a genuine failed Response while cancellation
        // joins its operations. A kill may leave only a prefix; success is never valid.
        match serde_json::from_slice::<Response>(&result.stdout_bytes) {
            Ok(response) => {
                assert_eq!(response.request_id, req.request_id);
                assert_eq!(response.status, Status::Failed);
                let evidence = &response.evidence;
                assert_eq!(evidence["plan"]["identity"], *plan);
                assert_eq!(evidence["selected"], json!(["test:suite"]));
                assert_eq!(evidence["blocked"], json!([]));
                assert_eq!(evidence["removed"], json!({}));
                let failed = response
                    .results
                    .iter()
                    .find(|r| r.id == "test.suite")
                    .unwrap();
                assert_eq!(failed.status, Status::Failed);
                assert_eq!(failed.cause, "process cancelled by enclosing owner");
                assert_eq!(failed.exit_code, Some(-1));
                let executed = evidence["executed"].as_array().unwrap();
                assert_eq!(executed.len(), 1);
                assert_eq!(executed[0]["operation"], "test.suite");
                let process =
                    chrono_harness::full::expand_process(&executed[0]["receipt"]["process"])
                        .unwrap();
                assert_eq!(process["child_pid"], launch["context"]["child_pid"]);
                assert_eq!(process["exit_code"], -1);
                assert_eq!(process["failure"], "process cancelled by enclosing owner");
                assert_eq!(process["stdout_bytes"], json!([255, 0, 10]));
                assert_eq!(process["stderr_bytes"], json!([254, 0, 10]));
            }
            Err(error) => assert!(error.is_eof(), "invalid CI response output: {error}"),
        }
        assert_eq!(
            ready,
            format!("ready:{}:{ready_ns}\n", launch["context"]["child_pid"]).as_bytes()
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
            "diagnostics.jsonl",
            "monitor.jsonl",
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
fn execution_diagnostics_early_admission_rejection_joins_without_readiness() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let h = host(None);
    h.change_registry(CONFIG, |v| {
        v["policy"]
            .as_object_mut()
            .unwrap()
            .remove("execution_diagnostics");
    });
    h.commit();
    let mut req = request(&h);
    req.scope = Some(chrono_harness::units::Scope::Unit {
        unit: "missing".into(),
    });
    let asserted = std::panic::catch_unwind(|| {
        let result = diagnostic_execution(&h, &req, listener, None);
        let directory = h.root().join(".chrono-harness/state");
        let ready: Value =
            serde_json::from_slice(&fs::read(directory.join("readiness.json")).unwrap()).unwrap();
        assert_eq!(ready["startup_error"], "business readiness timed out");
        assert_eq!(result.exit_code, 1);
        assert!(result.failure.is_none(), "{result:?}");
        assert!(result.owned_work_joined);
        assert!(result.stdout_complete && result.stderr_complete);
        assert_eq!(
            fs::read(directory.join("producer.stdout")).unwrap(),
            result.stdout_bytes
        );
        assert_eq!(
            fs::read(directory.join("producer.stderr")).unwrap(),
            result.stderr_bytes
        );
        let response: Response = serde_json::from_slice(&result.stdout_bytes).unwrap();
        assert_eq!(response.request_id, req.request_id);
        assert_eq!(response.status, Status::Failed);
        let rows = records(&fs::read(directory.join("diagnostics.jsonl")).unwrap());
        assert!(
            rows.iter()
                .all(|r| r["event"] != "projects.operation_started")
        );
    });
    if let Err(error) = asserted {
        eprintln!(
            "failed early admission fixture retained at {}",
            h.dir.keep().display()
        );
        std::panic::resume_unwind(error);
    }
}

#[test]
#[cfg(unix)]
fn execution_diagnostics_request_matches_real_host_root_before_effects() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host(Some("success".into()));
    h.change_registry(CONFIG, |v| {
        v["policy"]["tools"] = json!({"fixture":OPERATION_TOOL});
    });
    h.change_registry(".chrono-harness/projects.json", |v| {
        v["projects"][1]["actions"]["execute"]["tool"] = json!("fixture");
        v["projects"][1]["actions"]["execute"]["argv"] = json!(["fixture-operation", "check.case"]);
    });
    h.commit();
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("host-link");
    std::os::unix::fs::symlink(fs::canonicalize(h.root()).unwrap(), &alias).unwrap();
    let mut req = request(&h);
    req.host_root = alias;
    let other = Host::new();
    for field in ["root", "candidate", "config"] {
        let mut changed = req.clone();
        match field {
            "root" => changed.host_root = other.root().to_path_buf(),
            "candidate" => changed.candidate = "0".repeat(40),
            "config" => changed.config_sha256 = "0".repeat(64),
            _ => unreachable!(),
        }
        let response = chrono_judge_ci::judge(&changed);
        assert_eq!(response.status, Status::Failed, "{field}: {response:?}");
        assert_eq!(
            h.calls(),
            0,
            "{field} mismatch executed business operations"
        );
        assert_eq!(other.calls(), 0);
    }
    let response = chrono_judge_ci::judge(&req);
    pass(&response);
    let canonical_root = fs::canonicalize(&req.host_root).unwrap();
    assert_ne!(canonical_root, req.host_root);
    assert_eq!(response.evidence["plan"]["root"], json!(canonical_root));
    assert_eq!(h.calls(), 1);
    let repeated = chrono_judge_ci::judge(&req);
    fail(&repeated, "do not redispatch");
    assert_eq!(h.calls(), 1);
    let mut canonical_req = unit_request(&h, req.base.as_deref().unwrap(), &req.candidate);
    canonical_req.host_root = canonical_root;
    pass(&chrono_judge_ci::judge(&canonical_req));
    assert_eq!(h.calls(), 2);
}

#[test]
fn execution_diagnostics_request_validates_unit_and_uses_registered_execution() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host(Some("success".into()));
    let req = request(&h);
    let mut invalid = req.clone();
    invalid.scope = Some(chrono_harness::units::Scope::Unit {
        unit: "missing".into(),
    });
    let rejected = chrono_judge_ci::judge(&invalid);
    assert_eq!(rejected.status, Status::Failed);
    assert_eq!(rejected.evidence["executed"], json!([]));
    let response = chrono_judge_ci::judge(&req);
    pass(&response);
    let operations = response.evidence["plan"]["operations"].as_array().unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0]["method"]["operation"], "test.suite");
    assert_eq!(operations[0]["timeout_seconds"], 3);
    assert_eq!(operations[0]["output_limit_bytes"], 4096);
    assert_eq!(response.evidence["executed"].as_array().unwrap().len(), 1);
    assert_eq!(response.evidence["executed"][0]["operation"], "test.suite");
}

#[test]
fn execution_diagnostics_debug_process_preserves_context_and_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
                    root: h.root(),
                    stdout: &stdout,
                    stderr: &stderr,
                    launch: &launch,
                    publication: None,
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    let next = unit_request(&h, req.base.as_deref().unwrap(), &req.candidate);
    let failed = chrono_judge_ci::judge_with_diagnostic_sink(&next, Reject);
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
