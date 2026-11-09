//! 发布单元的实时原流。沿用现有描述符转交；辅助 IO 失败后仍排空两流、加入原进程。
use crate::verified_recipe as policy;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct CapturedStream {
    pub bytes: Vec<u8>,
    pub path: Option<String>,
    pub failures: Vec<String>,
    pub complete: bool,
}

impl CapturedStream {
    /// 路径只引用实际写成的文件；保留失败时继续提供已观察的原字节。
    pub fn receipt(&self) -> Value {
        let mut value = json!({
            "size": self.bytes.len(),
            "sha256": chrono_harness::sha256(&self.bytes),
        });
        if let Some(path) = &self.path {
            value["path"] = json!(path);
        } else {
            value["status"] = json!("unavailable");
            value["error"] = json!(self.failures.join("; "));
            value["bytes"] = json!(self.bytes);
        }
        if !self.complete {
            value["status"] = json!("incomplete");
        }
        value
    }
}

#[derive(Debug)]
pub struct CapturedProcess {
    pub pid: u32,
    pub argv: Vec<String>,
    pub cwd: String,
    pub started_ns: u128,
    pub finished_ns: Option<u128>,
    pub duration_seconds: f64,
    /// 与原配方一致：正常退出为 0..255，信号为负数；未知终态不填写成功。
    pub child_exit: Option<i32>,
    pub stdout: CapturedStream,
    pub stderr: CapturedStream,
    pub failures: Vec<String>,
    pub observation: Value,
}

impl CapturedProcess {
    pub fn auxiliary_failed(&self) -> bool {
        !self.failures.is_empty()
            || !self.stdout.failures.is_empty()
            || !self.stderr.failures.is_empty()
    }

    pub fn exit_code(&self) -> i32 {
        policy::result_exit(self.child_exit, self.auxiliary_failed())
    }

    pub fn passed(&self) -> bool {
        policy::passed(self.child_exit, self.auxiliary_failed())
    }

    pub fn receipt(&self, phase: &str, operation: Option<&str>) -> Value {
        json!({
            "phase": phase, "operation": operation,
            "argv": self.argv, "cwd": self.cwd, "pid": self.pid,
            "started_ns": self.started_ns, "finished_ns": self.finished_ns,
            "exit_code": self.child_exit,
            "stdout": self.stdout.receipt(), "stderr": self.stderr.receipt(),
            "observation": self.observation,
        })
    }
}

fn timestamp() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| error.to_string())
}

struct StreamWriter<W> {
    file: Option<std::fs::File>,
    relative: String,
    forward: Option<W>,
    failures: Vec<String>,
}

impl<W: Write> StreamWriter<W> {
    fn new(output: &Path, relative: String, forward: W) -> Self {
        let mut failures = Vec::new();
        let file = match fs::create_dir_all(output.join("processes")).and_then(|()| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output.join(&relative))
        }) {
            Ok(file) => Some(file),
            Err(error) => {
                failures.push(format!("retention: {error}"));
                None
            }
        };
        Self {
            file,
            relative,
            forward: Some(forward),
            failures,
        }
    }

    fn finish(mut self, bytes: Vec<u8>, complete: bool) -> CapturedStream {
        if let Some(file) = self.file.as_mut() {
            if let Err(error) = file.flush() {
                self.failures.push(format!("retention flush: {error}"));
                self.file = None;
            }
        }
        CapturedStream {
            bytes,
            path: (self.file.is_some() && complete).then_some(self.relative),
            failures: self.failures,
            complete,
        }
    }
}

impl<W: Write> Write for StreamWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(file) = self.file.as_mut() {
            if let Err(error) = file.write_all(bytes) {
                self.failures.push(format!("retention: {error}"));
                self.file = None;
            }
        }
        if let Some(forward) = self.forward.as_mut() {
            if let Err(error) = forward.write_all(bytes).and_then(|()| forward.flush()) {
                self.failures.push(format!("forwarding: {error}"));
                self.forward = None;
            }
        }
        // Auxiliary errors must not interrupt either original reader or replace child failure.
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Reuse the runner's bounded process owner, deadline, inherited capabilities and stream readers.
/// Output paths remain relative to the release output, which may lie outside the source clone.
pub fn execute(
    root: &Path,
    spec: &chrono_harness::CommandSpec,
    output: &Path,
    index: usize,
    timeout: std::time::Duration,
    stdout: impl Write + Send,
    stderr: impl Write + Send,
) -> Result<CapturedProcess, String> {
    let digest = chrono_harness::file_identity(Path::new(&spec.program))?.0;
    let mut stdout = StreamWriter::new(output, format!("processes/{index}.stdout"), stdout);
    let mut stderr = StreamWriter::new(output, format!("processes/{index}.stderr"), stderr);
    let started_ns = timestamp()?;
    let started = Instant::now();
    let original = chrono_harness::run_process_observed_forwarded_for(
        root,
        spec,
        b"",
        &digest,
        timeout,
        chrono_harness::ProcessForward {
            stdout: &mut stdout,
            stderr: &mut stderr,
        },
    )?;
    let child_exit = if let Some(signal) = original.exit_signal {
        Some(-signal)
    } else if original.exit_code >= 0 {
        Some(original.exit_code)
    } else {
        None
    };
    let observation = json!({"launcher_pid":original.launcher_pid,"executable":original.executable,"sha256":original.sha256,"environment_digest":original.environment_digest,"exit_signal":original.exit_signal,"engine_exit_code":original.exit_code,"failure":original.failure,"timeout_ns":timeout.as_nanos(),"output_limit_bytes":spec.output_limit_bytes});
    let mut failures: Vec<String> = original.failure.into_iter().collect();
    if child_exit.is_none() {
        failures.push("unavailable terminal child status".into());
    }
    let finished_ns = match timestamp() {
        Ok(time) => Some(time),
        Err(error) => {
            failures.push(format!("finish timestamp: {error}"));
            None
        }
    };
    Ok(CapturedProcess {
        pid: original
            .child_pid
            .ok_or("missing original child identity")?,
        argv: original.argv,
        cwd: original
            .cwd
            .to_str()
            .ok_or("non UTF-8 release cwd")?
            .to_owned(),
        started_ns,
        finished_ns,
        duration_seconds: started.elapsed().as_secs_f64(),
        child_exit,
        stdout: stdout.finish(original.stdout_bytes, original.stdout_complete),
        stderr: stderr.finish(original.stderr_bytes, original.stderr_complete),
        failures,
        observation,
    })
}
