use std::{
    fs,
    io::{self, Write},
    os::unix::process::CommandExt,
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

fn exec(mut command: Command) -> ! {
    panic!("fixture exec failed: {}", command.exec());
}

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).expect("consumer mode");
    match mode.as_str() {
        "--version" => println!("fixture"),
        "noop" => (),
        "ordinary" => print!("ordinary-work"),
        "finished" => print!("finished"),
        "check" => {
            let response: serde_json::Value = serde_json::from_slice(
                &fs::read(Path::new(STATE).join("inner-check-response.json")).unwrap(),
            )
            .unwrap();
            io::stdout()
                .write_all(response["stdout"].as_str().unwrap().as_bytes())
                .unwrap();
            io::stderr()
                .write_all(response["stderr"].as_str().unwrap().as_bytes())
                .unwrap();
            io::stdout().flush().unwrap();
            return ExitCode::from(u8::try_from(response["exit"].as_u64().unwrap()).unwrap());
        }
        "output-branch" => {
            fs::create_dir_all("output λ").unwrap();
            fs::write("output λ/use", "managed-output").unwrap();
            let mut git = Command::new("git");
            git.args(["symbolic-ref", "--short", "HEAD"]);
            exec(git);
        }
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
        "unknown-use" => {
            mark("ready.tmp", std::process::id().to_string());
            fs::rename(
                Path::new(STATE).join("ready.tmp"),
                Path::new(STATE).join("ready"),
            )
            .unwrap();
            wait("release");
        }
        "waiting-completion" => {
            fs::create_dir_all(STATE).unwrap();
            wait("release");
            print!("completed");
        }
        "cargo-prepare" | "cargo-run" => {
            let root = std::env::current_dir().unwrap();
            let fixture = Path::new(STATE).join("cargo-case");
            let mut cargo = Command::new("cargo");
            cargo.args(["test", "--offline", "--manifest-path"]);
            cargo.arg(fixture.join("Cargo.toml"));
            if mode == "cargo-prepare" {
                fs::create_dir_all(fixture.join("src")).unwrap();
                fs::write(
                    fixture.join("Cargo.toml"),
                    include_bytes!("native_cargo/Cargo.toml"),
                )
                .unwrap();
                fs::write(
                    fixture.join("src/lib.rs"),
                    include_bytes!("native_cargo/src/lib.rs"),
                )
                .unwrap();
                cargo.arg("--no-run");
            } else {
                mark("cargo-wrapper", std::process::id().to_string());
                cargo.env("CHRONO_NATIVE_FIXTURE_ROOT", root);
                cargo.args(["--", "--nocapture"]);
            }
            exec(cargo);
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
