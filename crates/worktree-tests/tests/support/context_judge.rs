use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
    time::Duration,
};

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).expect("judge mode");
    if mode == "--version" {
        println!("fixture");
        return ExitCode::SUCCESS;
    }
    if mode == "hold" {
        fs::create_dir_all(".chrono-harness/state").unwrap();
        fs::write(
            ".chrono-harness/state/check-holder.tmp",
            std::process::id().to_string(),
        )
        .unwrap();
        fs::rename(
            ".chrono-harness/state/check-holder.tmp",
            ".chrono-harness/state/check-holder",
        )
        .unwrap();
        while !Path::new(".chrono-harness/state/check-release").exists() {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    if matches!(mode.as_str(), "warn" | "fail" | "transport" | "blocked") {
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(".chrono-harness/state/judge-calls")
            .unwrap()
            .write_all(b"actual")
            .unwrap();
    }
    let request: Value = serde_json::from_reader(io::stdin().lock()).unwrap();
    if mode == "transport" {
        println!("{}", "complete-malformed-original".repeat(20000));
        return ExitCode::from(4);
    }
    let (status, findings, outputs, exit) = match mode.as_str() {
        "context" | "hold" => {
            let context: Value = serde_json::from_slice(
                &fs::read(request["context"]["path"].as_str().unwrap()).unwrap(),
            )
            .unwrap();
            assert_eq!(context["schema_version"], 2);
            assert_eq!(context["candidate"], request["candidate"]["commit"]);
            ("pass", vec![], json!({"context":context}), 0)
        }
        "warn" | "fail" | "blocked" => {
            let warn = mode == "warn";
            let status = if warn { "warn" } else { "fail" };
            let level = if warn { "warning" } else { "error" };
            let count = if mode == "blocked" { 2 } else { 30 };
            let findings = (0..count)
                .map(|i| {
                    json!({
                        "code":format!("IDENTIFIED_{i}{}", "x".repeat(1000)),
                        "level":level,
                        "message":format!("actionable-{i}{}", "z".repeat(10000)),
                        "delta_refs":["/delta/0/path","/delta/0"],"causes":[]
                    })
                })
                .collect();
            (
                status,
                findings,
                json!({"original":"FULL_ORIGINAL".repeat(50000)}),
                u8::from(!warn),
            )
        }
        _ => panic!("unknown judge mode: {mode}"),
    };
    let response = json!({"protocol":request["protocol"],"request_id":request["request_id"],
        "judge_id":request["judge_id"],"status":status,"findings":findings,"evidence":[],"outputs":outputs});
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &response).unwrap();
    stdout.write_all(b"\n").unwrap();
    stdout.flush().unwrap();
    ExitCode::from(exit)
}
