use chrono_harness::{initial, sha256, wire};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    process::{Command, ExitCode},
};

fn run(mode: &str, bytes: &[u8]) -> Result<u8, String> {
    let value = chrono_harness::json(bytes)?;
    if mode == "inventory" {
        // These are independent consumer expectations, not defaults for missing input.
        assert!(value.get("base").is_none() && value.get("delta").is_none());
        let request: initial::Request = serde_json::from_value(value).map_err(|e| e.to_string())?;
        request.validate()?;
        match request.judge_id.as_str() {
            "dependent" => {
                assert_eq!(request.prior_results.len(), 1);
                let prior = serde_json::to_value(&request.prior_results[0]).unwrap();
                assert_eq!(prior["judge_id"], "initial-registration");
                assert_eq!(prior["status"], "pass");
                assert_eq!(
                    prior["outputs"]["inventory"]["scope"],
                    "schema-references-checkout"
                );
                let original = request.observations["judges"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|record| record["id"] == "initial-registration")
                    .unwrap();
                assert_eq!(prior, original["response"]);
            }
            "independent" => assert!(request.prior_results.is_empty()),
            _ => return Err("unsupported inventory consumer".into()),
        }
        fs::create_dir_all(".chrono-harness/state").map_err(|e| e.to_string())?;
        fs::write(
            format!(".chrono-harness/state/{}.request.json", request.judge_id),
            bytes,
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            format!(".chrono-harness/state/{}-ran", request.judge_id),
            b"ran",
        )
        .map_err(|e| e.to_string())?;
        let mut response = request.response(wire::Status::Pass);
        response.outputs.insert("plugin".into(), json!(true));
        serde_json::to_writer(io::stdout().lock(), &response).map_err(|e| e.to_string())?;
        return Ok(0);
    }
    if !["named", "broken", "conflict"].contains(&mode) {
        return Err("unsupported regression mode".into());
    }
    let request: wire::Request = serde_json::from_value(value).map_err(|e| e.to_string())?;
    request.validate()?;
    fs::create_dir_all(".chrono-harness/state").map_err(|e| e.to_string())?;
    // Separate original requests are retained even when producers disagree.
    let name = match (mode, request.judge_id.as_str()) {
        ("named", "observations") => "observations",
        ("broken", "broken" | "dependent") => "broken",
        ("conflict", "first") => "first",
        ("conflict", "second") => "second",
        _ => return Err("unsupported report consumer".into()),
    };
    fs::write(format!(".chrono-harness/state/{name}.request.json"), bytes)
        .map_err(|e| e.to_string())?;
    if mode == "broken" {
        io::stdout()
            .write_all(b"invalid JSON\n")
            .map_err(|e| e.to_string())?;
        return Ok(9);
    }
    let mut response = request.response(wire::Status::Pass);
    let outputs = if mode == "conflict" {
        json!({"impact":{"seeds":[request.judge_id]},"tests":null,"costs":{"unknown":["fixture"]}})
    } else {
        // This is an actual external tool observation, with no Python test logic.
        let tool = "/usr/bin/python3";
        let child = Command::new(tool)
            .arg("--version")
            .output()
            .map_err(|e| e.to_string())?;
        let exit = child
            .status
            .code()
            .ok_or("version child terminated without exit code")?;
        let path = ".chrono-harness/state/tool-version.txt";
        fs::write(path, &child.stdout).map_err(|e| e.to_string())?;
        fs::write(
            ".chrono-harness/state/tool-version.stderr.bin",
            &child.stderr,
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            ".chrono-harness/state/tool-version.status.json",
            serde_json::to_vec(&json!({"exit_code":exit})).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        let evidence =
            json!({"path":path,"sha256":sha256(&child.stdout),"kind":"fixture-tool-version"});
        let version = String::from_utf8(child.stdout).map_err(|e| e.to_string())?;
        response.status = wire::Status::Warn;
        response.findings = serde_json::from_value(json!([{"code":"W_FIXTURE_COST","level":"warning","message":"fixture costs unmeasured","delta_refs":["/costs"],"causes":[]}])).unwrap();
        response.evidence = serde_json::from_value(json!([evidence.clone()])).unwrap();
        json!({
            "tools":[{"path":tool,"version":version.trim(),"evidence":[evidence.clone()]}],
            "effective_inputs":{"scope":"fixture-context-only","context":request.context},
            "impact":{"seeds":request.delta.iter().map(|d|format!("file:{}",d.path)).collect::<Vec<_>>(),"edges":[],"tests":[],"retired_tests":[]},
            "tests":[{"id":"fixture-version-command","state":"executed","exit_code":exit,"evidence":[evidence]}],
            "costs":{"declared_before":null,"declared_after":null,"affected_tests":["fixture-version-command"],"retired_tests":[],"unknown":["fixture-version-command"],"measured":null},
            "fixture_extra":{"retained":true}
        })
    };
    response.outputs =
        serde_json::from_value::<std::collections::BTreeMap<String, Value>>(outputs).unwrap();
    serde_json::to_writer(io::stdout().lock(), &response).map_err(|e| e.to_string())?;
    Ok(0)
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("expected one regression mode");
        return ExitCode::from(2);
    }
    let mut bytes = Vec::new();
    let result = io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())
        .and_then(|_| run(&args[0], &bytes));
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
