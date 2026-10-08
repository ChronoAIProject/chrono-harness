use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use std::time::Duration;

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::from_value(value.clone()).unwrap()
}

fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .get(name)
        .expect("required protocol observation field")
}

fn main() {
    let case: Value = serde_json::from_slice(&std::fs::read("judge-case.json").unwrap()).unwrap();
    let kind = case["kind"].as_str().unwrap_or("response");
    if kind == "bound" {
        println!("{}", case["response"]);
        io::stdout().flush().unwrap();
        eprintln!("diagnostic-before-bound");
        io::stderr().flush().unwrap();
        if case["timeout"] == true {
            std::thread::sleep(Duration::from_secs(30));
        } else {
            io::stderr().write_all(&vec![b'x'; 4096]).unwrap();
            io::stderr().flush().unwrap();
        }
        return;
    }
    if kind == "sleep" {
        std::thread::sleep(Duration::from_secs(case["seconds"].as_u64().unwrap()));
        return;
    }
    let request: Value = if kind == "raw" && case["consume_request"] != true {
        Value::Null
    } else {
        serde_json::from_reader(io::stdin().lock()).unwrap()
    };
    if case["initial_only"] == true {
        assert!(request.get("base").is_none() && request.get("delta").is_none());
    }
    if let Some(marker) = case["marker"].as_str() {
        std::fs::write(marker, "yes").unwrap();
    }
    let mut response = json!({
        "protocol": request["protocol"], "request_id": request["request_id"],
        "judge_id": request["judge_id"], "status": "pass", "findings": [],
        "evidence": [], "outputs": {}
    });
    if let Some(fields) = case["fields"].as_object() {
        for (key, value) in fields {
            response[key] = value.clone();
        }
    }
    let mut exit = case["exit"].as_i64().unwrap_or(0) as i32;
    match kind {
        "response" | "raw" => {}
        "environment" => {
            assert!(std::env::var_os("HOME").is_none());
            assert_eq!(std::env::var("EXPLICIT").unwrap(), "yes");
            assert_eq!(std::env::args().nth(1).unwrap(), case["argument"]);
            assert_eq!(
                std::env::current_dir().unwrap(),
                std::fs::canonicalize(request["candidate"]["root"].as_str().unwrap()).unwrap()
            );
        }
        "argv" => response["outputs"]["argv"] = json!(std::env::args().skip(1).collect::<Vec<_>>()),
        "dag" => {
            let expected = json!({"a": [], "b": ["a"], "c": ["b"], "independent": []});
            let id = request["judge_id"].as_str().unwrap();
            let prior: Vec<_> = request["prior_results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["judge_id"].clone())
                .collect();
            assert_eq!(json!(prior), expected[id]);
            if id == "a" {
                response["outputs"] = json!({"impact": {
                    "seeds": ["file:example"], "edges": [], "tests": [], "retired_tests": []
                }, "named": 42});
            }
            if id == "b" {
                assert_eq!(request["impact"]["seeds"], json!(["file:example"]));
                assert_eq!(request["prior_results"][0]["outputs"]["named"], 42);
            }
            if id == "c" {
                assert!(field(&request, "impact").is_null());
            }
        }
        "blocked-dag" => {
            assert_ne!(request["judge_id"], "blocked");
            if request["judge_id"] == "bad" {
                response["status"] = "error".into();
                response["findings"] = json!([{"code":"E_FIXTURE", "level":"error",
                    "message":"known error", "delta_refs":["/fixture"], "causes":[]}]);
                exit = 2;
            }
        }
        "distinct-impacts" => {
            response["outputs"]["impact"] = json!({"producer": request["judge_id"]});
        }
        "prior-process" => {
            if request["judge_id"] == "consumer" {
                let observations = request["observations"]["judges"].as_array().unwrap();
                assert_eq!(observations.len(), 1);
                let original = &observations[0];
                assert_eq!(original["id"], "producer");
                let process = chrono_harness::full::expand_process(&original["process"]).unwrap();
                assert_eq!(
                    field(original, "request_digest"),
                    field(&process, "stdin_sha256")
                );
                let stdout = bytes(&process["stdout_bytes"]);
                assert_eq!(digest(&stdout), process["stdout_sha256"]);
                assert_eq!(
                    serde_json::from_slice::<Value>(&stdout).unwrap(),
                    request["prior_results"][0]
                );
                assert_eq!(
                    field(original, "request_id"),
                    field(&request["prior_results"][0], "request_id")
                );
                assert_eq!(process["exit_code"], 0);
                assert!(field(&process, "failure").is_null());
            }
        }
        "retained-process" => {
            for row in request["observations"]["judges"].as_array().unwrap() {
                let process = &row["process"];
                assert!(matches!(
                    process["encoding"].as_str(),
                    Some("chrono-retained-process/v1" | "chrono-retained-process/v2")
                ));
                assert!(
                    process.get("stdout_bytes").is_none() && process.get("stderr_bytes").is_none()
                );
                let expanded = chrono_harness::full::expand_process(process).unwrap();
                let stdout = bytes(&expanded["stdout_bytes"]);
                let stderr = bytes(&expanded["stderr_bytes"]);
                assert_eq!(digest(&stdout), process["stdout_sha256"]);
                assert_eq!(digest(&stderr), process["stderr_sha256"]);
                assert_eq!(stderr, (0..=255).collect::<Vec<u8>>());
                assert_eq!(
                    serde_json::from_slice::<Value>(&stdout).unwrap(),
                    row["response"]
                );
            }
            io::stderr()
                .write_all(&(0..=255).collect::<Vec<u8>>())
                .unwrap();
            io::stderr().flush().unwrap();
        }
        other => panic!("unknown native protocol case: {other}"),
    }
    let mut output = if kind == "raw" {
        bytes(&case["stdout"])
    } else {
        let mut output = serde_json::to_vec(&response).unwrap();
        if let Some(replacement) = case.get("replace_marker") {
            let at = output
                .windows(6)
                .position(|window| window == b"MARKER")
                .unwrap();
            output.splice(at..at + 6, bytes(replacement));
        } else {
            output.push(b'\n');
        }
        output
    };
    if let Some(suffix) = case.get("suffix") {
        output.extend(bytes(suffix));
    }
    if let Some(stderr) = case.get("stderr") {
        io::stderr().write_all(&bytes(stderr)).unwrap();
        io::stderr().flush().unwrap();
    }
    io::stdout().write_all(&output).unwrap();
    io::stdout().flush().unwrap();
    std::process::exit(exit);
}
