//! 发布单元的实时原流。沿用现有描述符转交；辅助 IO 失败后仍排空两流、加入原进程。
use crate::verified_recipe as policy;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{Command, Stdio},
    thread,
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
        })
    }
}

fn timestamp() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| error.to_string())
}

fn capture(
    output: &Path,
    relative: String,
    mut input: impl Read,
    mut forward: impl Write,
) -> CapturedStream {
    let mut failures = Vec::new();
    let path = output.join(&relative);
    let mut file = match fs::create_dir_all(output.join("processes")).and_then(|()| {
        OpenOptions::new().create_new(true).write(true).open(&path)
    }) {
        Ok(file) => Some(file),
        Err(error) => {
            failures.push(format!("retention: {error}"));
            None
        }
    };
    let mut forwarding = true;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 65536];
    let complete = loop {
        let size = match input.read(&mut buffer) {
            Ok(0) => break true,
            Ok(size) => size,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => {
                failures.push(format!("read original pipe: {error}"));
                break false;
            }
        };
        let raw = &buffer[..size];
        bytes.extend_from_slice(raw);
        if let Some(destination) = file.as_mut() {
            if let Err(error) = destination.write_all(raw) {
                failures.push(format!("retention: {error}"));
                file = None;
            }
        }
        if forwarding {
            if let Err(error) = forward.write_all(raw).and_then(|()| forward.flush()) {
                failures.push(format!("forwarding: {error}"));
                forwarding = false;
            }
        }
    };
    if let Some(destination) = file.as_mut() {
        if let Err(error) = destination.flush() {
            failures.push(format!("retention flush: {error}"));
            file = None;
        }
    }
    CapturedStream {
        bytes,
        path: if file.is_some() && complete { Some(relative) } else { None },
        failures,
        complete,
    }
}

fn joined_stream(
    result: thread::Result<CapturedStream>,
    name: &str,
) -> CapturedStream {
    result.unwrap_or_else(|_| CapturedStream {
        bytes: Vec::new(), path: None, complete: false,
        failures: vec![format!("{name} worker panicked; original stream is incomplete")],
    })
}

/// Command 由发布单元按明确登记构造。本函数不安装、探测或选择任何 SDK。
/// output 可以在源码／clone 根外；所有流路径相对该输出目录。
/// 沿用旧配方的内存原流，用于小命令返回值及文件保留失败时的原字节。
/// executable.tar 的大执行物不经过这里或 JSON，由独立的流式归档边界处理。
pub fn execute(
    command: &mut Command,
    output: &Path,
    index: usize,
    stdout: impl Write + Send,
    stderr: impl Write + Send,
) -> Result<CapturedProcess, String> {
    let mut argv = Vec::new();
    for argument in std::iter::once(command.get_program()).chain(command.get_args()) {
        argv.push(argument.to_str().ok_or("non UTF-8 release argument")?.to_owned());
    }
    let cwd = match command.get_current_dir() {
        Some(path) => path.to_owned(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    let cwd = fs::canonicalize(cwd).map_err(|error| error.to_string())?;
    command.current_dir(&cwd);
    let cwd = cwd.to_str().ok_or("non UTF-8 release cwd")?.to_owned();
    chrono_harness::process_fds::forward_command(command, &[])?;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let started_ns = timestamp()?;
    let started = Instant::now();
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id();
    let original_stdout = child.stdout.take().expect("piped child stdout");
    let original_stderr = child.stderr.take().expect("piped child stderr");
    let (status, captured_stdout, captured_stderr) = thread::scope(|scope| {
        let stdout = scope.spawn(|| capture(output, format!("processes/{index}.stdout"), original_stdout, stdout));
        let stderr = scope.spawn(|| capture(output, format!("processes/{index}.stderr"), original_stderr, stderr));
        let status = loop {
            match child.wait() {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => break result,
            }
        };
        // 两个 worker 全部加入，才处理任何等待、转交或保留错误。
        (status, stdout.join(), stderr.join())
    });
    let mut failures = Vec::new();
    let child_exit = match status {
        Ok(status) => {
            let code = status.code().or_else(|| status.signal().map(|signal| -signal));
            match code {
                Some(code) if (-127..=255).contains(&code) => Some(code),
                _ => { failures.push("unavailable terminal child status".into()); None }
            }
        }
        Err(error) => { failures.push(format!("wait original child: {error}")); None }
    };
    let finished_ns = match timestamp() {
        Ok(time) => Some(time),
        Err(error) => { failures.push(format!("finish timestamp: {error}")); None }
    };
    Ok(CapturedProcess {
        pid, argv, cwd, started_ns, finished_ns,
        duration_seconds: started.elapsed().as_secs_f64(), child_exit,
        stdout: joined_stream(captured_stdout, "stdout"),
        stderr: joined_stream(captured_stderr, "stderr"), failures,
    })
}
