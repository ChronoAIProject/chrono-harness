use chrono_host_bootstrap::{
    bindings::Environment,
    execution::{Method, methods, run_operation},
};
use serde_json::json;
use std::{
    fs,
    io::{self, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

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

fn method(steps: serde_json::Value) -> Method {
    Method {
        output_contract: None,
        owner: "fixture".into(),
        operation: "fixture.operation".into(),
        tool: "fixture.native".into(),
        argv: vec!["run".into(), serde_json::to_string(&steps).unwrap()],
    }
}

#[test]
fn execution_forwards_binary_streams_before_exit_and_retains_the_originals() {
    let temporary = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temporary.path()).unwrap();
    let directory = root.join("original process");
    fs::create_dir(&directory).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let stdout = vec![0u8, 255, 10];
    let stderr = vec![254u8, 0, 10];
    let method = method(
        json!([{"Stdout":stdout}, {"Stderr":stderr}, {"Rendezvous":{"address":address,"id":"streamed"}}, {"Exit":17}]),
    );
    let environment: Environment = std::env::vars_os().collect();
    let executable = Path::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-operation"));
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    let operation = thread::scope(|scope| {
        let running = scope.spawn(|| {
            let result = run_operation(
                &root,
                &directory,
                4,
                &method,
                executable,
                &environment,
                Forward(out_tx),
                Forward(err_tx),
            );
            // 若启动失败或子进程提前结束，唤醒同一个阻塞 accept，再收取原结果。
            let _ = TcpStream::connect(listener.local_addr().unwrap());
            result
        });
        let (mut child, _) = listener.accept().unwrap();
        // 子进程此时阻塞在原生 rendezvous；先收到两条原字节流，才允许它退出。
        let observed_stdout = out_rx.recv_timeout(Duration::from_secs(10));
        let observed_stderr = err_rx.recv_timeout(Duration::from_secs(10));
        // 释放和加入都先于断言；断言失败也不能把子进程留在 rendezvous。
        let released = child.write_all(b"!");
        let operation = running.join().unwrap().unwrap();
        released.unwrap();
        assert_eq!(observed_stdout.unwrap(), stdout);
        assert_eq!(observed_stderr.unwrap(), stderr);
        operation
    });
    assert_eq!(operation.exit_code, 17);
    assert!(operation.pid > 0);
    assert_eq!(operation.operation, method.operation);
    assert_eq!(operation.tool, method.tool);
    assert_eq!(
        operation.executable_sha256,
        chrono_harness::file_identity(executable).unwrap().0
    );
    assert_eq!(fs::read(root.join(&operation.stdout.path)).unwrap(), stdout);
    assert_eq!(fs::read(root.join(&operation.stderr.path)).unwrap(), stderr);
    assert_eq!(operation.stdout.sha256, chrono_harness::sha256(&stdout));
    assert_eq!(operation.stderr.sha256, chrono_harness::sha256(&stderr));
    assert_eq!(operation.stdout.size, stdout.len() as u64);
    assert_eq!(operation.stderr.size, stderr.len() as u64);
}

#[test]
fn native_bootstrap_reuses_the_current_registration_reader_and_keeps_ambiguity_visible() {
    let action = json!({"operation":"registered","tool":"native","argv":["--version"]});
    let registry = json!({"projects":[{"id":"p","actions":{"build":action}}],"scripts":[{"id":"s","actions":{"execute":action}}]});
    let methods = methods(&registry).unwrap();
    assert_eq!(methods["registered"].len(), 2);
    assert_eq!(methods["registered"][0].owner, "project:p");
    assert_eq!(methods["registered"][1].owner, "script:s");
}
