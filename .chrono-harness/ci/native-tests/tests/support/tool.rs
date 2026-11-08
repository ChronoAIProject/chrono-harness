//! Rustup／Cargo／rustc 的真实进程接口夹具；不触及已安装 SDK。
use serde_json::{Value, json};
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, ExitCode},
};

// 模拟已有 owner 的外部协议；构建阶段仍执行真正的候选 Rust bootstrap。
fn owner_phase(executable: &Path, args: &[String]) -> Result<u8, String> {
    let argument = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1))
            .cloned()
            .ok_or_else(|| format!("missing owner argument: {name}"))
    };
    let root = std::path::PathBuf::from(argument("--host-root")?);
    let phase = match args[0].as_str() {
        "migrate" => "migration",
        "bootstrap" => "construction",
        "deploy-owner" => "deployment",
        _ => return Err("unknown owner phase".into()),
    };
    let consumer = argument("--report-consumer")?;
    let selected = root.join(".chrono-harness/bin/cargo");
    let mut calls = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(".chrono-harness/state/owner-phase-calls.jsonl"))
        .map_err(|error| error.to_string())?;
    writeln!(
        calls,
        "{}",
        json!({"phase":phase,"consumer":consumer,"argv":args,
        "program":executable,"program_sha256":chrono_harness::file_identity(executable)?.0,
        "selected":chrono_host_bootstrap::owner::file_binding(&selected)?})
    )
    .map_err(|error| error.to_string())?;
    let defect = env::var("CHRONO_TEST_OWNER_DEFECT").unwrap_or_default();
    let defective = env::var("CHRONO_TEST_OWNER_DEFECT_PHASE").as_deref() == Ok(phase);
    let report_path = format!(".chrono-harness/state/worktrees/{phase}.json");
    let mut report = json!({"status":"used","report_path":report_path,
        "report_consumer":{"schema":"chrono-retention-consumer/v1","consumer":consumer,
            "purpose":"test owner phase original consumption",
            "originals":[{"id":"phase-original","path":report_path,"identity":"external-fixture"}]}});
    if defective {
        match defect.as_str() {
            "missing-consumer" => {
                report.as_object_mut().unwrap().remove("report_consumer");
            }
            "wrong-consumer" => report["report_consumer"]["consumer"] = json!("another caller"),
            "wrong-original" => {
                report["report_consumer"]["originals"][0]["path"] =
                    json!(".chrono-harness/state/worktrees/another.json")
            }
            "child-exit" => {}
            _ => return Err("unknown owner phase defect".into()),
        }
    }
    let mut wrapper_exit = 0;
    if phase == "construction" && (!defective || defect == "child-exit") {
        let (program, arguments) = if defective {
            (
                env::var("CHRONO_TEST_OWNER_OPERATION").map_err(|error| error.to_string())?,
                vec![
                    "run".into(),
                    json!([{"Stdout":[0,255,10]},{"Stderr":[254,0,10]},{"Exit":7}]).to_string(),
                ],
            )
        } else {
            (
                env::var("CHRONO_TEST_OWNER_NATIVE").map_err(|error| error.to_string())?,
                vec![
                    "construct-owner-stage".into(),
                    root.to_string_lossy().into_owned(),
                    argument("--bootstrap-config")?,
                ],
            )
        };
        let mut environment: std::collections::BTreeMap<String, String> = env::vars().collect();
        environment.insert(
            "CHRONO_WORKTREE_BOOTSTRAP".into(),
            root.to_string_lossy().into_owned(),
        );
        let process = chrono_harness::run_process(
            &root,
            &chrono_harness::CommandSpec {
                program,
                args: arguments,
                env: environment,
                timeout_seconds: 60,
                output_limit_bytes: 1024 * 1024,
            },
            b"",
        )?;
        wrapper_exit = if process.exit_code == 0 { 0 } else { 2 };
        report["status"] = json!(if wrapper_exit == 0 { "used" } else { "failed" });
        report["managed_process"] = chrono_harness::full::compact_process(
            &serde_json::to_value(process).map_err(|error| error.to_string())?,
        )?;
    }
    if phase == "deployment" {
        let stage: Value = serde_json::from_slice(
            &fs::read(root.join(argument("--stage-result")?)).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let staged = root.join(
            stage["owner_stage"]["staged"]["path"]
                .as_str()
                .ok_or("missing staged owner")?,
        );
        fs::rename(&selected, selected.with_file_name("previous-owner"))
            .map_err(|error| error.to_string())?;
        fs::copy(staged, &selected).map_err(|error| error.to_string())?;
        fs::write(
            root.join(chrono_host_bootstrap::owner::CURRENT),
            serde_json::to_vec(&json!({
                "source_commit":stage["source"]["commit"],
                "installed":chrono_host_bootstrap::owner::file_binding(&selected)?,
            }))
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    let original = root.join(&report_path);
    fs::create_dir_all(original.parent().unwrap()).map_err(|error| error.to_string())?;
    let raw = serde_json::to_vec(&report).map_err(|error| error.to_string())?;
    fs::write(original, &raw).map_err(|error| error.to_string())?;
    std::io::stdout()
        .write_all(&raw)
        .map_err(|error| error.to_string())?;
    Ok(wrapper_exit)
}

fn run() -> Result<u8, String> {
    let invocation = env::args().next().ok_or("missing invocation")?;
    let executable = Path::new(&invocation);
    let root = executable.parent().ok_or("missing fixture root")?;
    let name = executable
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid tool name")?;
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| ["migrate", "bootstrap", "deploy-owner"].contains(&arg.as_str()))
    {
        return owner_phase(executable, &args);
    }
    let calls = root.join("calls.json");
    let mut values: Vec<Value> = if calls.exists() {
        serde_json::from_slice(&fs::read(&calls).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        vec![]
    };
    let mut call = vec![json!(name)];
    call.extend(args.iter().map(|arg| json!(arg)));
    values.push(json!(call));
    fs::write(
        calls,
        serde_json::to_vec(&values).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut selected = serde_json::Map::new();
    for key in [
        "RUSTUP_HOME",
        "CARGO_HOME",
        "HOME",
        "TMPDIR",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTUP_TOOLCHAIN",
    ] {
        selected.insert(key.into(), env::var(key).map_or(Value::Null, Value::String));
    }
    let mut stream = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("environments.jsonl"))
        .map_err(|e| e.to_string())?;
    writeln!(stream, "{}", json!({"tool":name,"environment":selected}))
        .map_err(|e| e.to_string())?;
    if name == "producer" {
        if let Ok(code) = env::var("CHRONO_TEST_PRODUCER_EXIT") {
            std::io::stdout()
                .write_all(b"producer stdout\x00\xff\n")
                .map_err(|e| e.to_string())?;
            std::io::stderr()
                .write_all(b"producer stderr\xfe\n")
                .map_err(|e| e.to_string())?;
            return code.parse::<u8>().map_err(|e| e.to_string());
        }
        let mut command = Command::new(
            env::var_os("CHRONO_TEST_BOOTSTRAP_NATIVE").ok_or("missing native producer")?,
        );
        command.args(&args);
        chrono_harness::process_fds::forward_command(&mut command, &[])?;
        return Ok(command
            .status()
            .map_err(|e| e.to_string())?
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1));
    } else if name == "rustup" {
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        match words.as_slice() {
            ["which", "--toolchain", "1.95.0", tool @ ("cargo" | "rustc")] => {
                println!("{}", root.join(tool).display())
            }
            ["run", "1.95.0", "cargo", "--version"] => {
                return Ok(if root.join("cargo-ready").exists() {
                    0
                } else {
                    1
                });
            }
            ["run", "1.95.0", "rustfmt", "--version"] => {
                return Ok(if root.join("fmt-ready").exists() {
                    0
                } else {
                    1
                });
            }
            [
                "toolchain",
                "install",
                "1.95.0",
                "--profile",
                "default",
                "--no-self-update",
            ] => {
                if root.join("fail-install").exists() {
                    return Ok(17);
                }
                if !root.join("no-default-cargo").exists() {
                    fs::write(root.join("cargo-ready"), b"").map_err(|e| e.to_string())?;
                }
                if !root.join("no-default-fmt").exists() {
                    fs::write(root.join("fmt-ready"), b"").map_err(|e| e.to_string())?;
                }
            }
            [
                "toolchain",
                "install",
                "1.95.0",
                "--profile",
                "minimal",
                "--component",
                "rustfmt",
            ] => {
                fs::write(root.join("cargo-ready"), b"").map_err(|e| e.to_string())?;
                fs::write(root.join("fmt-ready"), b"").map_err(|e| e.to_string())?;
            }
            ["component", "add", "--toolchain", "1.95.0", "rustfmt"] => {
                if root.join("fail-component").exists() {
                    return Ok(8);
                }
                fs::write(root.join("fmt-ready"), b"").map_err(|e| e.to_string())?;
            }
            _ => return Ok(99),
        }
    } else if ["cargo", "rustc"].contains(&name)
        && args == ["--version"]
        && env::var("RUSTUP_TOOLCHAIN").as_deref() == Ok("1.95.0")
    {
        println!("{name} 1.95.0 (fixture)");
    } else if name == "cargo" && env::var("CHRONO_TEST_CARGO_DOMAIN").as_deref() == Ok("1") {
        if !args.iter().any(|arg| arg == "--locked") {
            return Err("Cargo fixture requires the registered locked command".into());
        }
        let manifest = args
            .iter()
            .position(|arg| arg == "--manifest-path")
            .and_then(|index| args.get(index + 1))
            .ok_or("missing Cargo manifest")?;
        let mut locked = fs::read(manifest).map_err(|error| error.to_string())?;
        locked.push(0);
        locked.extend(
            fs::read(Path::new(manifest).with_file_name("Cargo.lock"))
                .map_err(|error| error.to_string())?,
        );
        println!(
            "{}",
            json!({"cargo_home":env::var("CARGO_HOME").map_err(|error| error.to_string())?,
            "offline":env::var("CARGO_NET_OFFLINE").as_deref()==Ok("true"),
            "manifest":manifest,"locked_input":chrono_harness::sha256(&locked),"argv":args})
        );
    } else if name == "cargo"
        && args.first().map(String::as_str) == Some("build-fixture")
        && args.len() == 2
    {
        println!("cargo original stdout");
        eprintln!("cargo original stderr");
        let mut command = Command::new(env::var_os("RUSTC").ok_or("missing compiler")?);
        command.args(["compile-fixture", &args[1]]);
        chrono_harness::process_fds::forward_command(&mut command, &[])?;
        return Ok(command
            .status()
            .map_err(|e| e.to_string())?
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1));
    } else if name == "rustc"
        && args.first().map(String::as_str) == Some("compile-fixture")
        && args.len() == 2
    {
        let compiler = fs::canonicalize(executable).map_err(|e| e.to_string())?;
        fs::write(format!("{}.selection.json",args[1]), serde_json::to_vec(&json!({"compiler":compiler,"wrapper":env::var("RUSTC_WRAPPER").ok(),"workspace_wrapper":env::var("RUSTC_WORKSPACE_WRAPPER").ok()})).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        fs::copy(executable, &args[1]).map_err(|e| e.to_string())?;
        fs::set_permissions(&args[1], fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    } else {
        return Ok(99);
    }
    Ok(0)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
