//! 下载器外部接口 fixture；不访问网络，原退出和响应头由生产 CLI 消费。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn valid_tar_command(argc: usize, has_destination: bool, strip_single_root: bool) -> (valid: bool)
    ensures valid == (argc == 7 && has_destination && strip_single_root),
{
    argc == 7 && has_destination && strip_single_root
}
}

#[cfg(not(verus_keep_ghost))]
use std::{env, fs, process::ExitCode};

#[cfg(not(verus_keep_ghost))]
fn run() -> Result<u8, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let selected = |flag| -> Result<&str, String> {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .ok_or("missing curl flag")?;
        args.get(index + 1)
            .map(String::as_str)
            .ok_or("missing curl value".into())
    };
    if args.first().is_some_and(|arg| arg == "-xf") {
        let destination = std::path::Path::new(selected("-C")?);
        if !valid_tar_command(
            args.len(),
            args.get(2).is_some_and(|arg| arg == "-C"),
            args.get(4..)
                .is_some_and(|tail| tail == ["--strip-components", "1", "--no-same-owner"]),
        ) {
            return Err("unexpected tar arguments".into());
        }
        for member in [
            "verus",
            "cargo-verus",
            "rust_verify",
            "z3",
            "libvstd.rlib",
            "vstd.vir",
        ] {
            fs::copy(&args[1], destination.join(member)).map_err(|e| e.to_string())?;
        }
        if std::path::Path::new("fail-extraction").exists() {
            return Ok(42);
        }
        return Ok(0);
    }
    fs::write("download-calls.json", serde_json::to_vec(&args).unwrap())
        .map_err(|e| e.to_string())?;
    fs::write(selected("--dump-header")?, b"fixture response headers\n")
        .map_err(|e| e.to_string())?;
    fs::copy("download-body", selected("--output")?).map_err(|e| e.to_string())?;
    let code = fs::read_to_string("download-exit")
        .unwrap_or_else(|_| "0".into())
        .parse::<u8>()
        .map_err(|e| e.to_string())?;
    if code != 0 {
        eprintln!("fixture download failed");
    }
    Ok(code)
}
#[cfg(not(verus_keep_ghost))]
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
