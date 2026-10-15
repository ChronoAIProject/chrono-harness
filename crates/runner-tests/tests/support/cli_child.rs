use serde_json::{Value, json};
use std::io::{self, Read, Write};

fn main() {
    let executable = std::env::current_exe().unwrap();
    match executable.file_name().unwrap().to_str().unwrap() {
        "true" => std::process::exit(9),
        "git-fixture" => {
            let mut marker = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(executable.parent().unwrap().join("facts-launches"))
                .unwrap();
            marker.write_all(b"launch").unwrap();
            print!("wrong-version");
            return;
        }
        _ => {}
    }
    match std::env::args().nth(1).unwrap().as_str() {
        "exit" => std::process::exit(std::env::args().nth(2).unwrap().parse().unwrap()),
        "echo" => {
            println!("{}", std::env::args().nth(2).unwrap());
            println!("{}", std::env::current_dir().unwrap().display());
        }
        "argv0" => print!("{}", std::env::args().next().unwrap()),
        "pipe" => {
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            assert_eq!(unsafe { libc::fstat(0, stat.as_mut_ptr()) }, 0);
            assert_eq!(
                unsafe { stat.assume_init() }.st_mode & libc::S_IFMT,
                libc::S_IFIFO
            );
            let mut bytes = Vec::new();
            io::stdin().read_to_end(&mut bytes).unwrap();
            io::stdout().write_all(b"EOF:").unwrap();
            io::stdout().write_all(&bytes).unwrap();
            io::stderr().write_all(b"joined").unwrap();
        }
        "filter-clean" => println!("canonical"),
        "filter-fail" => {
            eprint!("unexpected filter execution");
            std::process::exit(93);
        }
        "judge" => judge(),
        "registry-snapshot" => {
            let args: Vec<_> = std::env::args().collect();
            match chrono_harness::facts::registry_snapshot(
                &std::env::current_dir().unwrap(),
                &args[2],
                &args[3],
            ) {
                Ok(snapshot) => println!(
                    "{}",
                    json!({
                        "entry_path": snapshot.entry_path,
                        "effective_path": snapshot.effective_path,
                        "selection": snapshot.selection,
                        "identity": chrono_harness::facts::registry_identity(&snapshot.values, &args[3]).unwrap(),
                        "values": snapshot.values,
                    })
                ),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(2);
                }
            }
        }
        mode => panic!("unknown CLI fixture mode {mode}"),
    }
}

fn judge() {
    let case: Value = serde_json::from_slice(&std::fs::read("cli-case.json").unwrap()).unwrap();
    match case["kind"].as_str().unwrap_or("response") {
        "sleep" => {
            std::thread::sleep(std::time::Duration::from_secs(8));
            return;
        }
        "raw" => {
            io::stdout()
                .write_all(case["text"].as_str().unwrap().as_bytes())
                .unwrap();
            return;
        }
        "flood" => {
            println!("{}", "x".repeat(8192));
            return;
        }
        "response" => {}
        mode => panic!("unknown judge fixture mode {mode}"),
    }
    let request: Value = serde_json::from_reader(io::stdin().lock()).unwrap();
    let mut response = json!({"protocol":"chrono-ci-judge/v1", "request_id":request["request_id"],
        "status":"passed", "results":[{"id":"external","status":"passed","cause":"real execution","exit_code":0}],
        "evidence":{"executed":true}});
    if case["echo_protocol"] == true {
        response["protocol"] = request["protocol"].clone();
    }
    if let Some(fields) = case["fields"].as_object() {
        for (key, value) in fields {
            response[key] = value.clone();
        }
    }
    if let Some(code) = case["child_exit"].as_i64() {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["exit", &code.to_string()])
            .status()
            .unwrap();
        assert_eq!(child.code(), Some(code as i32));
        response["results"][0]["exit_code"] = json!(child.code().unwrap());
    }
    let mut raw = serde_json::to_vec(&response).unwrap();
    if let Some(bytes) = case.get("replacement_bytes") {
        let position = raw.windows(6).position(|s| s == b"MARKER").unwrap();
        let bytes: Vec<u8> = serde_json::from_value(bytes.clone()).unwrap();
        raw.splice(position..position + 6, bytes);
    }
    raw.push(b'\n');
    if case["extra_json"] == true {
        raw.extend_from_slice(b"{}\n");
    }
    io::stdout().write_all(&raw).unwrap();
    io::stdout().flush().unwrap();
    std::process::exit(case["exit"].as_i64().unwrap_or(0) as i32);
}
