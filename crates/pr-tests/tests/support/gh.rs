use serde_json::{Value, json};
use std::{fs, io::Read};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let steps: Vec<Value> = serde_json::from_slice(&fs::read("steps.json").unwrap()).unwrap();
    let index: usize = fs::read_to_string("index")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let step = steps
        .get(index)
        .unwrap_or_else(|| panic!("unexpected invocation {args:?}"));
    let mut log = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("calls.jsonl")
        .unwrap();
    use std::io::Write;
    writeln!(log, "{}", json!({"args":args,"stdin":input})).unwrap();
    assert_eq!(
        args[0..2],
        serde_json::from_value::<Vec<String>>(step["prefix"].clone()).unwrap()
    );
    if let Some(expected) = step["stdin"].as_str() {
        assert_eq!(input, expected);
    }
    fs::write("index", (index + 1).to_string()).unwrap();
    print!("{}", step["stdout"].as_str().unwrap_or(""));
    eprint!("{}", step["stderr"].as_str().unwrap_or(""));
    std::io::stdout().flush().unwrap();
    std::io::stderr().flush().unwrap();
    if let Some(ms) = step["sleep_ms"].as_u64() {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    std::process::exit(step["exit"].as_i64().unwrap_or(0) as i32);
}
