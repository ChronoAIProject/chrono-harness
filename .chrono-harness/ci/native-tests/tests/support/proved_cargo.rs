//! Cargo 命令边界的 Rust mock；不运行 Cargo、Git、Verus 或其它外部工具。
use serde::Deserialize;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Deserialize)]
struct Invocation {
    argv: Vec<String>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    writes: Vec<(PathBuf, Vec<u8>)>,
    exit: u8,
}

fn main() -> ExitCode {
    let plan = PathBuf::from(std::env::var_os("CHRONO_PROOF_FIXTURE").expect("fixture plan"));
    let calls = plan.with_extension("calls");
    let index: usize = fs::read_to_string(&calls)
        .unwrap_or_else(|_| "0".into())
        .parse()
        .unwrap();
    let invocations: Vec<Invocation> = serde_json::from_slice(&fs::read(&plan).unwrap()).unwrap();
    let invocation = &invocations[index];
    let actual: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(actual, invocation.argv, "exact official Cargo argv");
    assert_eq!(std::env::var("RUSTUP_TOOLCHAIN").unwrap(), "1.95.0");
    assert_eq!(std::env::var("RUSTC_BOOTSTRAP").unwrap(), "1");
    let root = plan.parent().unwrap();
    assert_eq!(
        std::env::var_os("CARGO_HOME").unwrap(),
        root.join("cargo-home").into_os_string(),
        "compiler must consume the caller's Cargo domain"
    );
    assert_eq!(
        std::env::var_os("RUSTUP_HOME").unwrap(),
        root.join("rustup-home").into_os_string(),
        "compiler must consume the caller's Rustup domain"
    );
    fs::write(calls, (index + 1).to_string()).unwrap();
    for (path, bytes) in &invocation.writes {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }
    io::stdout().write_all(&invocation.stdout).unwrap();
    io::stderr().write_all(&invocation.stderr).unwrap();
    ExitCode::from(invocation.exit)
}
