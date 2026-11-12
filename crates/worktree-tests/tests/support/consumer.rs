mod cargo_model;
#[path = "../../../runner-tests/tests/support/git_model.rs"]
mod git_model;
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
    let pending = Path::new(STATE).join(format!(".{name}-{}.tmp", std::process::id()));
    fs::write(&pending, bytes).unwrap();
    fs::rename(pending, Path::new(STATE).join(name)).unwrap();
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
    let invocation = std::env::args_os().next().unwrap();
    match Path::new(&invocation).file_name().and_then(|s| s.to_str()) {
        Some("rustup") => {
            if std::env::args().nth(1).as_deref() == Some("which") {
                let name = std::env::args().last().unwrap();
                println!(
                    "{}",
                    std::env::current_exe()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join(name)
                        .display()
                );
            }
            return ExitCode::SUCCESS;
        }
        Some(name @ ("cargo" | "rustc")) => {
            println!("{name} 1.95.0 (fixture)");
            return ExitCode::SUCCESS;
        }
        _ => (),
    }
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let mode = arguments.first().expect("consumer mode");
    if mode == "--cargo-model" {
        return cargo_model::dispatch(&arguments[1..]);
    }
    if mode == "--native-holder" {
        return cargo_model::dispatch(&arguments);
    }
    match mode.as_str() {
        "--version" => println!("fixture"),
        "noop" => (),
        "ordinary" => print!("ordinary-work"),
        "finished" => print!("finished"),
        "artifact-success" | "artifact-failure" => {
            let directory = std::env::var_os("CHRONO_ARTIFACT_DIRECTORY")
                .expect("registered operation artifact directory");
            let directory = Path::new(&directory);
            assert!(directory.is_dir());
            assert_eq!(
                std::env::var_os("TMPDIR").as_deref(),
                Some(directory.as_os_str())
            );
            fs::create_dir(directory.join("arbitrary products")).unwrap();
            fs::write(
                directory.join("arbitrary products/original.bin"),
                b"original artifact\0\xff",
            )
            .unwrap();
            print!("{}", directory.display());
            eprint!("original artifact diagnostic");
            return ExitCode::from(if mode == "artifact-failure" { 23 } else { 0 });
        }
        "check" => {
            let response: serde_json::Value = serde_json::from_slice(
                &fs::read(Path::new(STATE).join("inner-check-response.json")).unwrap(),
            )
            .unwrap();
            if let Some(expected) = response["authentication"].as_object() {
                for (key, value) in expected {
                    assert_eq!(std::env::var(key).ok(), value.as_str().map(str::to_owned));
                }
            }
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
            let bytes = git_model::run(
                std::env::current_dir().unwrap(),
                &["symbolic-ref".into(), "--short".into(), "HEAD".into()],
                &[],
            )
            .unwrap();
            io::stdout().write_all(&bytes).unwrap();
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
            let ready = format!("ready-{}", std::os::unix::process::parent_id());
            mark(&ready, &pid);
            wait("release");
            print!("completed");
        }
        "unknown-use" => {
            mark("ready", std::process::id().to_string());
            wait("release");
        }
        "waiting-completion" => {
            fs::create_dir_all(STATE).unwrap();
            wait("release");
            print!("completed");
        }
        "detached-parent" => {
            let mut child = Command::new(std::env::current_exe().unwrap());
            child
                .arg("detached-holder")
                .process_group(0)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            chrono_harness::process_fds::forward_command(&mut child, &[]).unwrap();
            let _detached = child.spawn().unwrap();
        }
        "detached-holder" => {
            mark("detached", std::process::id().to_string());
            wait("release");
            mark("detached-done", "done");
        }
        "bootstrap-holder" => {
            fs::create_dir_all("output λ").unwrap();
            fs::write("output λ/cache", "rebuilt").unwrap();
            mark("bootstrap-holder", "ready");
            wait("bootstrap-release");
        }
        "cargo-prepare" | "cargo-run" => {
            let root = std::env::current_dir().unwrap();
            let fixture = Path::new(STATE).join("cargo-case");
            let mut cargo = Command::new(std::env::current_exe().unwrap());
            cargo.arg("--cargo-model");
            // Keep the original Cargo argv at an explicit mock boundary; the
            // holder is a real Rust child and still owns its original leases.
            cargo.args(["test", "--lib", "--offline", "--verbose", "--manifest-path"]);
            cargo.env("RUSTC_LOG", "rustc_codegen_ssa::back::link=info");
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
                cargo.args(["--", "--list"]);
            } else {
                mark("cargo-wrapper", std::process::id().to_string());
                cargo.env("CHRONO_NATIVE_FIXTURE_ROOT", root);
                cargo.args(["--", "--nocapture"]);
            }
            exec(cargo);
        }
        "custody-holder" => {
            use std::io::Read;
            let root =
                std::path::PathBuf::from(std::env::var_os("CHRONO_TEMPORARY_CARGO_ROOT").unwrap());
            assert!(root.is_absolute());
            assert_eq!(std::env::temp_dir(), root);
            let mut stream =
                std::net::TcpStream::connect(std::env::var("CHRONO_TEMPORARY_SYNC").unwrap())
                    .unwrap();
            stream.write_all(b"ready").unwrap();
            stream.read_exact(&mut [0]).unwrap();
            if let Some(member) = std::env::var_os("CHRONO_TEMPORARY_MEMBER") {
                assert!(!fs::read(root.join(member)).unwrap().is_empty());
            } else {
                assert_eq!(
                    fs::read(root.join("cache 空白/output")).unwrap(),
                    b"rebuildable"
                );
            }
        }
        "allocation-lease" => {
            use std::os::fd::AsRawFd;
            unsafe extern "C" {
                fn flock(fd: i32, operation: i32) -> i32;
            }
            let path = std::env::args_os()
                .nth(2)
                .expect("actual allocation lease path");
            let lease = fs::File::open(path).unwrap();
            // LOCK_EX | LOCK_NB on both explicitly supported Unix platforms.
            assert_eq!(unsafe { flock(lease.as_raw_fd(), 2 | 4) }, -1);
            assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
            io::stdout().write_all(b"allocation-live\xff\0").unwrap();
            io::stderr()
                .write_all(b"original-native-stderr\xfe")
                .unwrap();
        }
        "capture-once" => {
            io::stdout().write_all(b"\xffpartial stdout\0\n").unwrap();
            io::stderr().write_all(b"\xfepartial stderr\0\n").unwrap();
            io::stdout().flush().unwrap();
            io::stderr().flush().unwrap();
            if std::env::var_os("CHRONO_CAPTURE_INTERRUPT").is_some() {
                unsafe extern "C" {
                    fn getppid() -> i32;
                    fn kill(pid: i32, signal: i32) -> i32;
                }
                assert_eq!(unsafe { kill(getppid(), 9) }, 0);
            }
            return ExitCode::from(7);
        }
        "capture-input" => {
            use std::io::Read;
            let mut bytes = vec![];
            io::stdin().read_to_end(&mut bytes).unwrap();
            io::stdout().write_all(&bytes).unwrap();
            io::stderr().write_all(b"\xfeinput\0\n").unwrap();
            return ExitCode::from(9);
        }
        "bounded-cargo" => {
            let plan: serde_json::Value = serde_json::from_slice(
                &fs::read(Path::new(STATE).join("cargo-plan.json")).unwrap(),
            )
            .unwrap();
            let mut observations = Vec::new();
            for pass in 0..plan["passes"].as_u64().unwrap_or(1) {
                for (manifest, operation) in plan["manifests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(plan["operations"].as_array().unwrap())
                {
                    let began = std::time::Instant::now();
                    let mut cargo = Command::new(std::env::current_exe().unwrap());
                    cargo.arg("--cargo-model");
                    cargo.env_remove("CARGO_TARGET_DIR").args([
                        operation.as_str().unwrap(),
                        "--locked",
                        "--offline",
                        "--manifest-path",
                        manifest.as_str().unwrap(),
                    ]);
                    let output = cargo.output().unwrap();
                    observations.push(serde_json::json!({"pass":pass,"manifest":manifest,"elapsed_seconds":began.elapsed().as_secs_f64(),"exit":output.status.code()}));
                    io::stdout().write_all(&output.stdout).unwrap();
                    io::stderr().write_all(&output.stderr).unwrap();
                    assert!(output.status.success(), "registered Cargo mock failed");
                }
            }
            mark(
                "cargo-cost.json",
                serde_json::to_vec(&observations).unwrap(),
            );
            if let Some(stage) = plan["hold_stage"].as_str() {
                // All these consumers read the real generated output under the same lease.
                let target = Path::new(plan["manifests"][1].as_str().unwrap())
                    .parent()
                    .unwrap()
                    .join("target/debug/deps");
                let original: Vec<_> = fs::read_dir(&target)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                assert!(!original.is_empty());
                mark(&format!("{stage}-active"), "ready");
                wait(&format!("{stage}-release"));
                assert!(
                    original.iter().all(|p| p.exists()),
                    "active consumer lost generated output"
                );
                if stage == "installation" {
                    fs::create_dir_all(".chrono-harness/bin").unwrap();
                    let input = original.iter().find(|p| p.is_file()).unwrap();
                    fs::copy(input, ".chrono-harness/bin/installed-cargo-output").unwrap();
                }
            }
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
