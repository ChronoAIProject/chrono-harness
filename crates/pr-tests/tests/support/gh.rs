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
    // This is checked in the real child, before its simulated remote effect.
    let operations = fs::read_dir(".chrono-harness/state/pr-operations")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir() && !p.join("result.json").exists())
        .filter_map(|operation| {
            let mut call = 1;
            while operation.join(format!("call-{call}.result.json")).exists() {
                call += 1;
            }
            let intent: Value = serde_json::from_slice(
                &fs::read(operation.join(format!("call-{call}.intent.json"))).ok()?,
            )
            .ok()?;
            let launch: Value = serde_json::from_slice(
                &fs::read(intent["process_originals"]["launch"].as_str()?).ok()?,
            )
            .ok()?;
            (launch["child_pid"] == std::process::id()).then_some((operation, call, intent))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        operations.len(),
        1,
        "one original launch binds this actual fixture child"
    );
    let (operation, call, intent) = &operations[0];
    assert_eq!(intent["state"], "intent");
    assert_eq!(intent["handle"], Value::Null);
    assert_eq!(intent["result"], Value::Null);
    assert_eq!(
        fs::read(intent["stdin_original"]["path"].as_str().unwrap()).unwrap(),
        input.as_bytes()
    );
    if step["block_receipt"] == true {
        fs::create_dir(operation.join(format!("call-{call}.result.json"))).unwrap();
    }
    if step["block_result"] == true {
        fs::create_dir(operation.join("result.json")).unwrap();
    }
    let mut log = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("calls.jsonl")
        .unwrap();
    use std::io::Write;
    let mut record = serde_json::to_vec(&json!({"args":args,"stdin":input})).unwrap();
    record.push(b'\n');
    log.write_all(&record).unwrap();
    assert_eq!(
        args[0..2],
        serde_json::from_value::<Vec<String>>(step["prefix"].clone()).unwrap()
    );
    if let Some(expected) = step["stdin"].as_str() {
        assert_eq!(input, expected);
    }
    fs::write("index", (index + 1).to_string()).unwrap();
    #[cfg(unix)]
    if step["pause_before_response"] == true {
        let mut ready = std::os::unix::net::UnixStream::connect("response.sock").unwrap();
        let bytes =
            serde_json::to_vec(&json!({"operation":operation.to_str().unwrap(),"call":call}))
                .unwrap();
        ready
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        ready.write_all(&bytes).unwrap();
        let mut release = [0];
        ready.read_exact(&mut release).unwrap();
    }
    print!("{}", step["stdout"].as_str().unwrap_or(""));
    eprint!("{}", step["stderr"].as_str().unwrap_or(""));
    std::io::stdout().flush().unwrap();
    std::io::stderr().flush().unwrap();
    #[cfg(unix)]
    if step["wait_for_interruption"] == true {
        // A blocking native handshake proves dispatch; the enclosing owner joins
        // the interrupted PR and this child. No state/file polling in the fixture.
        let mut ready = std::os::unix::net::UnixStream::connect("ready.sock").unwrap();
        let bytes =
            serde_json::to_vec(&json!({"operation":operation.to_str().unwrap(),"call":call}))
                .unwrap();
        ready
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        ready.write_all(&bytes).unwrap();
        let mut release = [0];
        ready.read_exact(&mut release).unwrap();
    }
    if let Some(ms) = step["sleep_ms"].as_u64() {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    std::process::exit(step["exit"].as_i64().unwrap_or(0) as i32);
}
