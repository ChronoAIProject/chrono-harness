use chrono_harness::sha256;
use serde_json::{Value, json};
use std::io::{self, Read, Write};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("version") {
        println!("prepared-producer 0.1.0");
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("malformed") {
        print!("{{}}");
        return;
    }
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let request: Value = serde_json::from_slice(&input).unwrap();
    let report_path = ".chrono-harness/state/preparation/producer-report.json";
    let report = serde_json::to_vec(&json!({
        "current_observation": {"producer": true}
    }))
    .unwrap();
    let root = request["host_root"].as_str().unwrap();
    let report_file = std::path::Path::new(root).join(report_path);
    std::fs::create_dir_all(report_file.parent().unwrap()).unwrap();
    std::fs::write(&report_file, &report).unwrap();
    let report_sha256 = sha256(&report);
    let result = json!({
        "schema": "chrono-check-inputs/v1",
        "request_sha256": sha256(&input),
        "source": request["source"],
        "profile": request["profile"],
        "base": "a".repeat(40),
        "candidate": "b".repeat(40),
        "initial": false,
        "context": Value::Null,
        "scope": Value::Null,
        "evidence": {
            "current_observation": {"producer": true},
            "report_path": report_path,
            "report_sha256": report_sha256,
        },
        "originals": [{"path": report_path, "sha256": report_sha256}],
    });
    serde_json::to_writer(io::stdout(), &result).unwrap();
    io::stdout().write_all(b"\n").unwrap();
}
