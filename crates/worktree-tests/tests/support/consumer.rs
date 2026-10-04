use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, ExitCode, Stdio},
    thread,
    time::Duration,
};

const STATE: &str = ".chrono-harness/state";

fn mark(name: &str, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(STATE).unwrap();
    fs::write(Path::new(STATE).join(name), bytes).unwrap();
}

fn wait(name: &str) {
    while !Path::new(STATE).join(name).exists() {
        thread::sleep(Duration::from_millis(20));
    }
}

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).expect("consumer mode");
    match mode.as_str() {
        "--version" => println!("fixture"),
        "noop" => (),
        "ordinary" => print!("ordinary-work"),
        "rebuild" | "rebuild-evidence" | "rebuild-error" => {
            fs::create_dir_all("output λ").unwrap();
            fs::write("output λ/cache", "rebuilt").unwrap();
            if mode == "rebuild-evidence" {
                mark("evidence", "evidence");
            }
            if mode == "rebuild-error" {
                io::stderr().write_all(b"original-error").unwrap();
                return ExitCode::from(23);
            }
        }
        "managed-child" => {
            mark("ready", "ready");
            wait("release");
            print!("joined-child");
        }
        "shared-holder" => {
            let pid = std::process::id().to_string();
            mark(&format!("ready-{pid}"), &pid);
            wait("release");
            print!("completed");
        }
        "parent" => {
            // This ordinary native child inherits the actual open descriptors
            // and process group. Its parent intentionally returns first, as the
            // lifecycle test requires a surviving, independently released child.
            let _grandchild = Command::new(std::env::current_exe().unwrap())
                .arg("grandchild")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            mark("parent", std::process::id().to_string());
            wait("release-parent");
            mark("parent-done", "done");
        }
        "grandchild" => {
            mark("grandchild", std::process::id().to_string());
            wait("release-grandchild");
            mark("grandchild-done", "done");
        }
        _ => panic!("unknown consumer mode: {mode}"),
    }
    io::stdout().flush().unwrap();
    ExitCode::SUCCESS
}
