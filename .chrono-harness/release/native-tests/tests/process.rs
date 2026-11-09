use chrono_host_release::process::execute;
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::Duration,
};

fn command(_root: &std::path::Path, steps: Value) -> chrono_harness::CommandSpec {
    chrono_harness::CommandSpec {
        program: env!("CARGO_BIN_EXE_chrono-test-release-operation").into(),
        args: vec!["run".into(), steps.to_string()],
        env: std::env::vars().collect(),
        timeout_seconds: 10,
        output_limit_bytes: chrono_harness::verified_limits::MAX_OUTPUT_BYTES,
    }
}

struct Forward(mpsc::Sender<Vec<u8>>);
impl Write for Forward {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .send(bytes.to_vec())
            .map_err(|_| io::Error::other("consumer closed"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BrokenForward;
impl Write for BrokenForward {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "closed live output",
        ))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn original_parent_exit_does_not_leave_a_descendant_running_into_the_next_phase() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let mut command = command(source.path(), json!([]));
    command.program = env!("CARGO_BIN_EXE_chrono-test-release-tool").into();
    command.args = vec!["--process-parent".into()];
    let captured = execute(
        source.path(),
        &command,
        output.path(),
        0,
        Duration::from_secs(10),
        io::sink(),
        io::sink(),
    )
    .unwrap();
    assert_eq!(captured.child_exit, Some(0));
    assert!(!source.path().join("late-descendant").exists());
    assert!(!String::from_utf8_lossy(&captured.stdout.bytes).contains("late descendant tail"));
    assert!(captured.passed(), "{:?}", captured.failures);
}

#[test]
fn deadline_keeps_bytes_actually_observed_and_actual_termination_signal() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let command = command(
        source.path(),
        json!([
            {"Stdout":[0,255,10]}, {"Stderr":[254,0,10]}, {"SleepMillis":2000},
            {"Create":{"path":"too-late","bytes":[1]}}
        ]),
    );
    let mut live_stdout = Vec::new();
    let mut live_stderr = Vec::new();
    let captured = execute(
        source.path(),
        &command,
        output.path(),
        0,
        Duration::from_millis(250),
        &mut live_stdout,
        &mut live_stderr,
    )
    .unwrap();
    assert!(!captured.passed());
    assert_eq!(captured.child_exit, Some(-9));
    assert!(captured.failures.iter().any(|s| s.contains("timed out")));
    // The deadline includes real startup; bytes not reached before cancellation stay absent.
    assert!([0, 255, 10].starts_with(&captured.stdout.bytes));
    assert!([254, 0, 10].starts_with(&captured.stderr.bytes));
    assert_eq!(live_stdout, captured.stdout.bytes);
    assert_eq!(live_stderr, captured.stderr.bytes);
    for stream in [&captured.stdout, &captured.stderr] {
        assert_eq!(
            fs::read(output.path().join(stream.path.as_ref().unwrap())).unwrap(),
            stream.bytes
        );
        assert_eq!(
            stream.receipt()["sha256"],
            chrono_harness::sha256(&stream.bytes)
        );
    }
    assert!(!source.path().join("too-late").exists());
    assert_eq!(captured.observation["exit_signal"], 9);
    assert_eq!(captured.observation["timeout_ns"], 250_000_000);
}

#[test]
fn output_bound_keeps_only_original_prefix_and_reports_incomplete_stream() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let mut command = command(source.path(), json!([]));
    // Produce the original 100,000 bytes without exceeding Linux's per-argument bound.
    command.program = env!("CARGO_BIN_EXE_chrono-test-release-tool").into();
    command.args = vec!["--process-output-bound".into()];
    command.output_limit_bytes = 1024;
    let captured = execute(
        source.path(),
        &command,
        output.path(),
        0,
        Duration::from_secs(10),
        io::sink(),
        io::sink(),
    )
    .unwrap();
    assert!(!captured.passed());
    assert!(
        captured
            .failures
            .iter()
            .any(|s| s.contains("output limit exceeded"))
    );
    assert_eq!(captured.stdout.bytes, vec![7u8; 1024]);
    assert!(!captured.stdout.complete);
    assert_eq!(captured.stdout.receipt()["status"], "incomplete");
    assert_eq!(
        fs::read(output.path().join("processes/0.stdout")).unwrap(),
        captured.stdout.bytes
    );
}

#[test]
fn both_original_streams_are_live_before_child_exit_and_output_can_be_outside_source() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stdout = vec![0u8, 255, 10];
    let stderr = vec![254u8, 0, 10];
    let command = command(
        source.path(),
        json!([
            {"Stdout":stdout}, {"Stderr":stderr},
            {"Rendezvous":{"address":address.to_string(),"id":"release"}}, {"Exit":41}
        ]),
    );
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    let captured = thread::scope(|scope| {
        let running = scope.spawn(|| {
            let result = execute(
                source.path(),
                &command,
                output.path(),
                0,
                Duration::from_secs(10),
                Forward(out_tx),
                Forward(err_tx),
            );
            let _ = TcpStream::connect(address);
            result
        });
        let (mut child, _) = listener.accept().unwrap();
        let out = out_rx.recv_timeout(Duration::from_secs(10));
        let err = err_rx.recv_timeout(Duration::from_secs(10));
        let release = child.write_all(b"!");
        let captured = running.join().unwrap().unwrap();
        release.unwrap();
        assert_eq!(out.unwrap(), stdout);
        assert_eq!(err.unwrap(), stderr);
        captured
    });
    assert_eq!(captured.child_exit, Some(41));
    assert_eq!(captured.exit_code(), 41);
    assert!(!captured.passed());
    assert!(!captured.auxiliary_failed());
    assert_eq!(captured.stdout.bytes, stdout);
    assert_eq!(captured.stderr.bytes, stderr);
    let receipt = captured.receipt("build", Some("build.fixture"));
    for (name, expected) in [("stdout", stdout), ("stderr", stderr)] {
        let stream = &receipt[name];
        assert_eq!(stream["sha256"], chrono_harness::sha256(&expected));
        assert_eq!(stream["size"], expected.len());
        assert_eq!(
            fs::read(output.path().join(stream["path"].as_str().unwrap())).unwrap(),
            expected
        );
    }
    assert!(receipt["finished_ns"].as_u64().unwrap() >= receipt["started_ns"].as_u64().unwrap());
}

#[test]
fn forwarding_failure_drains_the_original_tail_and_keeps_actual_child_failure() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let first = vec![1u8; 20000];
    let last = vec![2u8; 28000];
    let command = command(
        source.path(),
        json!([
            {"Stdout":first}, {"Stderr":[0,255,10]}, {"Stdout":last}, {"Exit":41}
        ]),
    );
    let captured = execute(
        source.path(),
        &command,
        output.path(),
        0,
        Duration::from_secs(10),
        BrokenForward,
        io::sink(),
    )
    .unwrap();
    assert_eq!(captured.child_exit, Some(41));
    assert_eq!(captured.exit_code(), 41);
    assert!(captured.auxiliary_failed());
    assert_eq!(captured.stdout.bytes, [first, last].concat());
    assert_eq!(captured.stderr.bytes, [0, 255, 10]);
    assert_eq!(
        fs::read(output.path().join(captured.stdout.path.as_ref().unwrap())).unwrap(),
        captured.stdout.bytes
    );
    assert!(
        captured
            .stdout
            .failures
            .iter()
            .any(|f| f.contains("closed live output"))
    );
}

#[test]
fn child_success_is_failure_when_live_forwarding_failed() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let command = command(source.path(), json!([{"Stdout":[1,2,3]},{"Exit":0}]));
    let captured = execute(
        source.path(),
        &command,
        output.path(),
        0,
        Duration::from_secs(10),
        BrokenForward,
        io::sink(),
    )
    .unwrap();
    assert_eq!(captured.child_exit, Some(0));
    assert_eq!(captured.exit_code(), 2);
    assert!(!captured.passed());
    assert_eq!(captured.stdout.bytes, [1, 2, 3]);
}

#[test]
fn retention_failure_keeps_original_bytes_without_inventing_files_or_overwriting_child_failure() {
    for (child, expected_exit) in [(41, 41), (0, 2)] {
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::write(
            output.path().join("processes"),
            b"independent existing file",
        )
        .unwrap();
        let command = command(
            source.path(),
            json!([
                {"Stdout":[0,255,10]}, {"Stderr":[254,0,10]}, {"Exit":child}
            ]),
        );
        let captured = execute(
            source.path(),
            &command,
            output.path(),
            0,
            Duration::from_secs(10),
            io::sink(),
            io::sink(),
        )
        .unwrap();
        assert_eq!(captured.child_exit, Some(child));
        assert_eq!(captured.exit_code(), expected_exit);
        assert!(!captured.passed());
        assert!(captured.stdout.path.is_none() && captured.stderr.path.is_none());
        assert_eq!(captured.stdout.bytes, [0, 255, 10]);
        assert_eq!(captured.stderr.bytes, [254, 0, 10]);
        let receipt = captured.receipt("build", None);
        assert_eq!(receipt["stdout"]["status"], "unavailable");
        assert_eq!(receipt["stdout"]["bytes"], json!([0, 255, 10]));
        assert_eq!(receipt["stderr"]["bytes"], json!([254, 0, 10]));
        assert_eq!(
            fs::read(output.path().join("processes")).unwrap(),
            b"independent existing file"
        );
    }
}
