mod provider;

use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;

fn provider_observation(phase: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".chrono-harness/state/mock-launches.jsonl")
        .unwrap();
    let row = serde_json::json!({
        "phase": phase,
        "pid": std::process::id(),
        "unix_ns": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string(),
        "argv": std::env::args().skip(1).collect::<Vec<_>>()
    });
    writeln!(file, "{row}").unwrap();
}

fn diagnostic(mode: &str) {
    use serde_json::{Value, json};

    let mut calls = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".chrono-harness/state/judge-calls")
        .unwrap();
    calls.write_all(b"judge").unwrap();
    let request: Value = serde_json::from_reader(io::stdin().lock()).unwrap();
    let mut stdout = io::stdout().lock();
    if mode == "deep" {
        let mut evidence = json!({});
        for _ in 0..125 {
            evidence = json!({"nested": evidence});
        }
        let response = json!({
            "protocol": request["protocol"], "request_id": request["request_id"],
            "status": "passed", "results": [{"id": "deep", "status": "passed",
                "cause": "actual result", "exit_code": 0}], "evidence": evidence,
        });
        serde_json::to_writer(&mut stdout, &response).unwrap();
        writeln!(stdout).unwrap();
        return;
    }
    let case = std::env::args().nth(2).expect("diagnostic case");
    if case == "transport" {
        writeln!(stdout, "{}", "original-malformed-transport".repeat(10000)).unwrap();
    } else {
        assert!(matches!(case.as_str(), "failed" | "blocked"));
        let results: Vec<_> = (0..30)
            .map(|i| {
                json!({
                    "id": format!("identified-operation-{i}{}", "x".repeat(1000)),
                    "status": case,
                    "cause": format!("actionable-cause-{i}{}", "z".repeat(10000)),
                    "exit_code": if case == "failed" { Some(9) } else { None },
                })
            })
            .collect();
        serde_json::to_writer(
            &mut stdout,
            &json!({
                "protocol": request["protocol"], "request_id": request["request_id"],
                "status": case, "results": results,
                "evidence": {"original": "complete-original-evidence"},
            }),
        )
        .unwrap();
        writeln!(stdout).unwrap();
    }
    stdout.flush().unwrap();
    std::process::exit(9);
}

fn main() {
    if let Ok(provider) = std::env::var("CHRONO_CI_TEST_PROVIDER") {
        match provider.as_str() {
            "units" => {
                provider_observation("entered");
                provider::units(&std::env::args().skip(1).collect::<Vec<_>>());
                provider_observation("returned");
                return;
            }
            _ => panic!("unknown provider fixture: {provider}"),
        }
    }
    let executable = std::env::args_os().next().unwrap();
    match Path::new(&executable).file_name().unwrap().as_bytes() {
        b"parent-gh" => return provider::parent(&std::env::args().skip(1).collect::<Vec<_>>()),
        b"$CHRONO_BASE" => {
            let mut stdout = io::stdout().lock();
            for arg in std::env::args_os().skip(1) {
                stdout.write_all(arg.as_bytes()).unwrap();
                stdout.write_all(b"\n").unwrap();
            }
            stdout.flush().unwrap();
            let failure: &[u8] = match std::env::var("CHRONO_TEST_RECORDING").as_deref() {
                Ok("full") => b"original-failure\n",
                Ok("scope") => b"\xffscope-consumer-failure",
                _ => panic!("missing recording fixture mode"),
            };
            io::stderr().write_all(failure).unwrap();
            std::process::exit(23);
        }
        _ => {}
    }
    let mode = std::env::args().nth(1).expect("transport mode");
    if mode == "--version" {
        println!("chrono-ci-test-transport 1");
        return;
    }
    if matches!(mode.as_str(), "diagnostic" | "deep") {
        diagnostic(&mode);
        return;
    }
    io::copy(&mut io::stdin().lock(), &mut io::sink()).unwrap();
    let mut stdout = io::stdout().lock();
    match mode.as_str() {
        "timeout-partial" => {
            stdout.write_all(b"{\n").unwrap();
            stdout.flush().unwrap();
            std::thread::sleep(Duration::from_secs(8));
        }
        "output-limit" => {
            stdout.write_all(&[b'x'; 8192]).unwrap();
            stdout.write_all(b"\n").unwrap();
            stdout.flush().unwrap();
        }
        _ => panic!("unknown transport mode: {mode}"),
    }
}
