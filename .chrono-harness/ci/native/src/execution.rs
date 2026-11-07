//! 在现有生命周期所有者的限时／限量内执行登记操作，保留并实时转交原字节。
use crate::{
    bindings::{self, Environment},
    verified_bindings as policy,
};
use chrono_harness::{copy_hashed, file_identity};
pub use chrono_judge_registration::execution::{Method, methods};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    os::unix::process::ExitStatusExt,
    path::Path,
    process::Stdio,
    thread,
    time::Instant,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stream {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub executable_sha256: String,
    pub pid: u32,
    pub exit_code: i32,
    pub stdout: Stream,
    pub stderr: Stream,
    pub duration_seconds: f64,
}

struct Tee<F: Write> {
    file: std::fs::File,
    forward: F,
}
impl<F: Write> Write for Tee<F> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.file.write_all(bytes)?;
        self.forward.write_all(bytes)?;
        self.forward.flush()?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.forward.flush()
    }
}

fn stream(
    root: &Path,
    path: &Path,
    input: impl Read,
    forward: impl Write,
) -> Result<Stream, String> {
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let (sha256, size) = copy_hashed(input, Tee { file, forward })?;
    Ok(Stream {
        path: path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("non UTF-8 stream path")?
            .into(),
        sha256,
        size,
    })
}

/// 调用方供应当前宿主输出；测试使用标准 Write 接口观察同一条转交链。
/// 返回前等待原 Child 和两条流结束；这里不另造调度、限额或租约协议。
pub fn run_operation(
    root: &Path,
    directory: &Path,
    index: usize,
    method: &Method,
    executable: &Path,
    environment: &Environment,
    stdout: impl Write + Send,
    stderr: impl Write + Send,
) -> Result<Operation, String> {
    let executable_sha256 = file_identity(executable)?.0;
    let program = executable
        .to_str()
        .ok_or("non UTF-8 operation executable")?
        .to_owned();
    let cwd = root.to_str().ok_or("non UTF-8 operation root")?.to_owned();
    let mut argv = vec![program];
    argv.extend(method.argv.iter().cloned());
    let arguments: Vec<_> = method.argv.iter().map(String::as_str).collect();
    let mut command = bindings::command(root, executable, &arguments, environment)?;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let started = Instant::now();
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id();
    let output = child.stdout.take().ok_or("missing operation stdout")?;
    let errors = child.stderr.take().ok_or("missing operation stderr")?;
    let output_path = directory.join(format!("{index}.stdout"));
    let error_path = directory.join(format!("{index}.stderr"));
    let (status, output, errors) = thread::scope(|scope| {
        let output = scope.spawn(|| stream(root, &output_path, output, stdout));
        let errors = scope.spawn(|| stream(root, &error_path, errors, stderr));
        let status = child.wait();
        // 两个 worker 都必须加入，再传播任一流或等待失败。
        (status, output.join(), errors.join())
    });
    let status = status.map_err(|error| error.to_string())?;
    let stdout = output.map_err(|_| "operation stdout worker panicked")??;
    let stderr = errors.map_err(|_| "operation stderr worker panicked")??;
    let exit_code = policy::process_exit(status.code(), status.signal())
        .ok_or("missing terminal operation status")?;
    Ok(Operation {
        operation: method.operation.clone(),
        tool: method.tool.clone(),
        argv,
        cwd,
        executable_sha256,
        pid,
        exit_code,
        stdout,
        stderr,
        duration_seconds: started.elapsed().as_secs_f64(),
    })
}
