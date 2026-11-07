//! 本宿主原生 bootstrap 入口；当前先接通固定 producer 原始执行资格的真实消费。
use chrono_host_bootstrap::{bindings, original, profile};
use serde_json::json;
use std::{path::Path, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let [command, host, reference] = arguments.as_slice() else {
        return Err("usage: chrono-host-bootstrap validate-original-execution HOST REPORT".into());
    };
    if command != "validate-original-execution" {
        return Err("unknown native bootstrap command".into());
    }
    let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
    let reference = reference.to_str().ok_or("non UTF-8 original report path")?;
    let raw = profile::read(&root, reference, b".chrono-harness/state/")?;
    let environment = std::env::vars_os().collect();
    let report = original::validate_fixed_execution(
        &root,
        &environment,
        &bindings::native_platform()?,
        &raw,
    )?;
    println!(
        "{}",
        json!({"schema":"chrono-bootstrap-original-execution-validation/v1",
        "path":reference, "sha256":chrono_harness::sha256(&raw),
        "source": report.source, "operations":report.operations.len()})
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("E_BOOTSTRAP_ORIGINAL: {error}");
            ExitCode::FAILURE
        }
    }
}
