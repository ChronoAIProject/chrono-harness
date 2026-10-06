use serde_json::Value;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let request: Value = serde_json::from_reader(io::stdin().lock()).unwrap();
    let root = request["candidate"]["root"].as_str().unwrap();
    let mut child = Command::new(Path::new(root).join(".chrono-harness/bin/chrono-judge-cost"))
        .args(["--protocol", "chrono-judge/v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        serde_json::to_writer(&mut stdin, &request).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    let mut response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let binding = response
        .get_mut("outputs")
        .and_then(|v| v.get_mut("costs"))
        .and_then(|v| v.get_mut("binding"))
        .expect("actual cost judge must publish its binding");
    assert!(binding.is_object());
    binding["candidate_tree"] = "0".repeat(40).into();
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &response).unwrap();
    stdout.write_all(b"\n").unwrap();
}
