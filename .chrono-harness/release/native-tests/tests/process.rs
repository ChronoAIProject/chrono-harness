use chrono_host_release::process::execute;
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::mpsc,
    thread,
    time::Duration,
};

fn command(root: &std::path::Path, steps: Value) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-release-operation"));
    command.current_dir(root).arg("run").arg(steps.to_string());
    command
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
fn both_original_streams_are_live_before_child_exit_and_output_can_be_outside_source() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stdout = vec![0u8, 255, 10];
    let stderr = vec![254u8, 0, 10];
    let mut command = command(
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
                &mut command,
                output.path(),
                0,
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
    let mut command = command(
        source.path(),
        json!([
            {"Stdout":first}, {"Stderr":[0,255,10]}, {"Stdout":last}, {"Exit":41}
        ]),
    );
    let captured = execute(&mut command, output.path(), 0, BrokenForward, io::sink()).unwrap();
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
    let mut command = command(source.path(), json!([{"Stdout":[1,2,3]},{"Exit":0}]));
    let captured = execute(&mut command, output.path(), 0, BrokenForward, io::sink()).unwrap();
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
        let mut command = command(
            source.path(),
            json!([
                {"Stdout":[0,255,10]}, {"Stderr":[254,0,10]}, {"Exit":child}
            ]),
        );
        let captured = execute(&mut command, output.path(), 0, io::sink(), io::sink()).unwrap();
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
