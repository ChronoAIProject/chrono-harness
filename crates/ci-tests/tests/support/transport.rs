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

fn declared_git() -> ! {
    use std::os::unix::process::CommandExt;

    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read("declared-git.json").unwrap()).unwrap();
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.iter().any(|arg| arg == "fetch") {
        if std::env::var_os("UNREGISTERED_TRANSPORT_INPUT").is_some() {
            std::process::exit(71);
        }
        let value = std::env::var_os("SSH_AUTH_SOCK").unwrap_or_else(|| "ABSENT".into());
        std::fs::write(
            config["observation"].as_str().unwrap(),
            value.as_os_str().as_bytes(),
        )
        .unwrap();
    }
    let error = std::process::Command::new(config["program"].as_str().unwrap())
        .args(arguments)
        .exec();
    panic!("declared Git execution: {error}");
}

fn preparation_fixture() {
    use serde_json::{Value, json};
    use std::fs::{self, OpenOptions};
    use std::io::Read;

    let fixture: Value =
        serde_json::from_slice(&fs::read(".chrono-harness/state/producer-fixture.json").unwrap())
            .unwrap();
    let case = fixture["case"].as_str().unwrap();
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(".chrono-harness/state/producer-calls")
        .unwrap();
    if std::env::args().nth(1).as_deref() == Some("--version") {
        if case == "version" {
            trace.write_all(b"probe").unwrap();
            println!("{}", "wrong-probe-version".repeat(20000));
        } else {
            println!("chrono-worktree 0.1.0");
        }
        return;
    }
    trace.write_all(b"acquisition").unwrap();
    match case {
        "failed" => {
            println!("{}", "producer-stdout-original".repeat(20000));
            eprintln!(
                "E_FIXTURE_ACQUISITION: fetch failed {}",
                "stderr-original".repeat(20000)
            );
            std::process::exit(9);
        }
        "decode" => println!("{}", "not-json-original".repeat(20000)),
        "validation" => {
            let mut raw = Vec::new();
            io::stdin().read_to_end(&mut raw).unwrap();
            let request: Value = serde_json::from_slice(&raw).unwrap();
            println!(
                "{}",
                json!({
                    "schema": "wrong-input-schema",
                    "request_sha256": chrono_harness::sha256(&raw),
                    "source": request["source"], "profile": request["profile"],
                    "base": fixture["base"], "candidate": "b".repeat(40),
                    "initial": false, "scope": null, "context": null,
                    "originals": [],
                    "evidence": {"note": "original-validation-evidence".repeat(20000)},
                })
            );
        }
        _ => panic!("unknown preparation fixture: {case}"),
    }
}

fn retention_failure_fixture() {
    use serde_json::{Value, json};
    use std::fs;

    fs::rename(
        ".chrono-harness/state/preparation",
        ".chrono-harness/state/saved-preparation",
    )
    .unwrap();
    std::os::unix::fs::symlink("saved-preparation", ".chrono-harness/state/preparation").unwrap();
    let request: Value = serde_json::from_reader(io::stdin().lock()).unwrap();
    println!(
        "{}",
        json!({
            "protocol": request["protocol"], "request_id": request["request_id"],
            "status": "passed", "results": [{"id": "fixture", "status": "passed",
                "cause": "actual", "exit_code": 0}], "evidence": {"actual": true},
        })
    );
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
        b"fixture-producer" => return preparation_fixture(),
        b"retention-failure" => return retention_failure_fixture(),
        b"declared-git" => declared_git(),
        b"parent-gh" => return provider::parent(&std::env::args().skip(1).collect::<Vec<_>>()),
        b"$CHRONO_BASE" => {
            let (failure, exit_code, record_json): (&[u8], i32, bool) =
                match std::env::var("CHRONO_TEST_RECORDING").as_deref() {
                    Ok("full") => (b"original-failure\n", 23, false),
                    Ok("scope") => (b"\xffscope-consumer-failure", 23, false),
                    Ok("release") => (b"original failure\n", 17, true),
                    Ok("release-units") => (b"original failure\xff", 17, true),
                    _ => panic!("missing recording fixture mode"),
                };
            if record_json {
                let args: Vec<_> = std::env::args().skip(1).collect();
                std::fs::write("args.json", serde_json::to_vec(&args).unwrap()).unwrap();
            } else {
                let mut stdout = io::stdout().lock();
                for arg in std::env::args_os().skip(1) {
                    stdout.write_all(arg.as_bytes()).unwrap();
                    stdout.write_all(b"\n").unwrap();
                }
                stdout.flush().unwrap();
            }
            io::stderr().write_all(failure).unwrap();
            std::process::exit(exit_code);
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
        "large-original" => {
            stdout.write_all(b"large-original-evidence\n").unwrap();
            let mut state = 0x12345678u32;
            let mut bytes = vec![0; 1_100_000];
            for byte in &mut bytes {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *byte = (state >> 24) as u8;
            }
            stdout.write_all(&bytes).unwrap();
            stdout.flush().unwrap();
        }
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
