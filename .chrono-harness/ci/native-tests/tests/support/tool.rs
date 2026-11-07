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

fn run() -> Result<u8, String> {
    let invocation = env::args().next().ok_or("missing invocation")?;
    let executable = Path::new(&invocation);
    let root = executable.parent().ok_or("missing fixture root")?;
    let name = executable
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid tool name")?;
    let args: Vec<String> = env::args().skip(1).collect();
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
