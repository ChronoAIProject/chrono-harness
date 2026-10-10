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
    let (mut file, partial) = tempfile::Builder::new()
        .prefix("unpublished-marker-")
        .tempfile_in(STATE)
        .unwrap()
        .keep()
        .unwrap();
    file.write_all(bytes.as_ref()).unwrap();
    drop(file);
    fs::rename(partial, Path::new(STATE).join(name)).unwrap();
}

fn wait(name: &str) {
    while !Path::new(STATE).join(name).exists() {
        thread::sleep(Duration::from_millis(20));
    }
}

fn exec(mut command: Command) -> ! {
    panic!("fixture exec failed: {}", command.exec());
}

fn python_holder() {
    mark("python-holder", std::process::id().to_string());
    wait("python-release");
    mark("python-done", "joined");
}

fn main() -> ExitCode {
    let invocation = std::env::args_os().next().unwrap();
    match Path::new(&invocation).file_name().and_then(|s| s.to_str()) {
        Some("rustup") => return ExitCode::SUCCESS,
        Some(name @ ("cargo" | "rustc")) => {
            println!("{name} 1.95.0 (fixture)");
            return ExitCode::SUCCESS;
        }
        Some("git") => {
            python_holder();
            return ExitCode::SUCCESS;
        }
        _ => (),
    }
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
        "python-consumer" => {
            let config: serde_json::Value = serde_json::from_slice(
                &fs::read(Path::new(STATE).join("python-consumer.json")).unwrap(),
            )
            .unwrap();
            let tools = std::path::PathBuf::from(config["tools"].as_str().unwrap());
            let path = std::env::join_paths(
                std::iter::once(tools)
                    .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
            )
            .unwrap();
            mark("python-wrapper", std::process::id().to_string());
            let mut python = Command::new("/usr/bin/python3");
            python.env("PATH", path).args([
                "-B",
                ".chrono-harness/state/python-consumer.py",
                config["script"].as_str().unwrap(),
                config["route"].as_str().unwrap(),
            ]);
            exec(python);
        }
        "python-holder" => python_holder(),
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
        "bootstrap-error" => {
            io::stdout().write_all(b"original build stdout\n").unwrap();
            io::stderr().write_all(b"original build stderr\n").unwrap();
            fs::write(std::env::args_os().nth(2).unwrap(), "drift").unwrap();
            io::stdout().flush().unwrap();
            return ExitCode::from(7);
        }
        "cargo-prepare" | "cargo-run" => {
            let root = std::env::current_dir().unwrap();
            let fixture = Path::new(STATE).join("cargo-case");
            let mut cargo = Command::new("cargo");
            // This fixture owns one native library test. Compile/list/run that
            // target without starting unrelated library or documentation builds.
            // Keep compiler and linker stages in the original bounded stderr.
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
        "temporary-producer" | "temporary-cargo" | "temporary-capture" => {
            let root = std::env::current_dir().unwrap();
            let (store, outputs) = chrono_worktree::TemporaryHost::registered_store(
                &root,
                ".chrono-harness/worktree.json",
                "test-allocation",
            )
            .unwrap();
            let allocation = chrono_worktree::TemporaryHost::allocate(
                &store,
                "test-allocation",
                &outputs,
                "native 空白 λ ",
            )
            .unwrap();
            let allocated = allocation.path();
            fs::write(allocated.join("source.txt"), b"original source\n").unwrap();
            fs::write(allocated.join("partial.bin"), [255, 0, 10]).unwrap();
            fs::create_dir_all(allocated.join("cache 空白")).unwrap();
            fs::write(allocated.join("cache 空白/output"), "rebuildable").unwrap();
            mark("temporary-parent", std::process::id().to_string());
            mark("temporary-path", allocated.as_os_str().as_encoded_bytes());
            if mode == "temporary-capture" {
                let mut child = Command::new(std::env::current_exe().unwrap());
                child
                    .arg("capture-child")
                    .current_dir(allocated)
                    .process_group(0);
                allocation.capture_output(&mut child).unwrap();
                return ExitCode::SUCCESS;
            }
            if mode == "temporary-cargo" {
                let project = allocated.join("cargo 空白");
                fs::create_dir_all(project.join("src")).unwrap();
                fs::write(
                    project.join("Cargo.toml"),
                    include_bytes!("native_cargo/Cargo.toml"),
                )
                .unwrap();
                fs::write(
                    project.join("src/lib.rs"),
                    include_bytes!("native_cargo/src/lib.rs"),
                )
                .unwrap();
                let cwd = if Path::new(STATE).join("vary-cargo-cwd").exists() {
                    let cwd = allocated.join("different cwd λ");
                    fs::create_dir(&cwd).unwrap();
                    cwd
                } else {
                    allocated.to_owned()
                };
                let mut cargo = Command::new("cargo");
                cargo
                    .current_dir(cwd)
                    .env_remove("CARGO_TARGET_DIR")
                    .env("CHRONO_TEMPORARY_CARGO_ROOT", allocated)
                    .env("CHRONO_TEMPORARY_HELPER", std::env::current_exe().unwrap())
                    .args(["test", "--lib", "--offline", "--manifest-path"])
                    .arg(project.join("Cargo.toml"));
                allocation.bind_command(&mut cargo).unwrap();
                assert!(cargo.status().unwrap().success());
                return ExitCode::SUCCESS;
            }
            let mut child = Command::new(std::env::current_exe().unwrap());
            child
                .arg("temporary-descendant")
                .arg(allocated)
                .process_group(0)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            allocation.bind_command(&mut child).unwrap();
            let mut child = child.spawn().unwrap();
            mark("temporary-child", child.id().to_string());
            wait("temporary-parent-release");
            assert!(child.wait().unwrap().success());
        }
        "capture-child" | "capture-once" => {
            io::stdout().write_all(b"\xffpartial stdout\0\n").unwrap();
            io::stderr().write_all(b"\xfepartial stderr\0\n").unwrap();
            io::stdout().flush().unwrap();
            io::stderr().flush().unwrap();
            if mode == "capture-child" {
                fs::write("capture-active", std::process::id().to_string()).unwrap();
                while !Path::new("native-release").exists() {
                    thread::sleep(Duration::from_millis(10));
                }
                fs::write("capture-done", "actual final write complete").unwrap();
            }
            return ExitCode::from(7);
        }
        "temporary-descendant" => {
            let allocation = std::path::PathBuf::from(std::env::args_os().nth(2).unwrap());
            assert_eq!(std::env::temp_dir(), allocation);
            let nested = tempfile::Builder::new()
                .prefix("nested λ ")
                .tempdir()
                .unwrap()
                .keep();
            assert!(nested.starts_with(&allocation));
            fs::write(
                allocation.join("native-active"),
                std::process::id().to_string(),
            )
            .unwrap();
            while !allocation.join("native-release").exists() {
                thread::sleep(Duration::from_millis(10));
            }
            let output = std::env::args_os()
                .nth(3)
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| "cache 空白/output".into());
            assert!(!fs::read(allocation.join(output)).unwrap().is_empty());
            fs::write(allocation.join("native-done"), "read and released").unwrap();
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
                    let mut cargo = Command::new("cargo");
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
                    assert!(
                        output.status.success(),
                        "actual independent Cargo project failed"
                    );
                }
            }
            mark(
                "cargo-cost.json",
                serde_json::to_vec(&observations).unwrap(),
            );
            if let Some(stage) = plan["hold_stage"].as_str() {
                if plan["detached_consumer"] == true {
                    let mut command = Command::new(std::env::current_exe().unwrap());
                    command
                        .args(["bounded-consumer", stage])
                        .process_group(0)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    command.spawn().unwrap();
                    // The actual consumer holds inherited descriptors and publishes
                    // its own read handshake. This bounded build need not pretend
                    // to stay live while an independent installation consumes it.
                    return ExitCode::SUCCESS;
                }
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
        "bounded-consumer" => {
            let stage = std::env::args().nth(2).unwrap();
            let target = Path::new("independent/tests/target/debug/deps");
            let original: Vec<_> = fs::read_dir(target)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            assert!(!original.is_empty());
            mark(&format!("{stage}-active"), std::process::id().to_string());
            wait(&format!("{stage}-release"));
            assert!(
                original.iter().all(|p| p.exists()),
                "active native consumer lost generated output"
            );
            if stage == "installation" {
                fs::create_dir_all(".chrono-harness/bin").unwrap();
                fs::copy(
                    original.iter().find(|p| p.is_file()).unwrap(),
                    ".chrono-harness/bin/installed-cargo-output",
                )
                .unwrap();
            }
            mark(&format!("{stage}-done"), "actual final read complete");
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
