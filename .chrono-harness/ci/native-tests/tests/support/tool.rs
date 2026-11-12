//! Git／工作树所有者／Rustup／Cargo／rustc 的命令边界 mock；不执行外部工具。
use serde_json::{Value, json};
use std::{
    env, fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, ExitCode},
};

fn command_mock(root: &Path, name: &str, args: &[String]) -> Result<u8, String> {
    let read = |path: &Path| -> Result<Value, String> {
        serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    };
    let calls = root.join(format!("{name}-calls.json"));
    let mut observed = if calls.exists() {
        read(&calls)?
    } else {
        json!([])
    };
    observed
        .as_array_mut()
        .ok_or("invalid mock calls")?
        .push(json!({
            "argv":args,"cwd":env::current_dir().map_err(|error| error.to_string())?
        }));
    fs::write(
        calls,
        serde_json::to_vec(&observed).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let specification = read(&root.join(format!("{name}.json")))?;
    let key = serde_json::to_string(args).map_err(|error| error.to_string())?;
    let response = specification
        .get(&key)
        .ok_or_else(|| format!("unexpected mock {name} argv: {args:?}"))?;
    if let Some(expected) = response.get("cwd") {
        if *expected != json!(env::current_dir().map_err(|error| error.to_string())?) {
            return Err(format!("mock {name} cwd differs"));
        }
    }
    if let Some(values) = response.get("environment").and_then(Value::as_object) {
        for (name, expected) in values {
            if json!(env::var(name).ok()) != *expected {
                return Err(format!("mock environment differs: {name}"));
            }
        }
    }
    if let Some(expected) = response.get("inherited_fds") {
        if json!(chrono_harness::process_fds::inherited_available()?) != *expected {
            return Err("mock inherited capabilities differ".into());
        }
    }
    if let Some(expected) = response.get("stdin") {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if json!(bytes) != *expected {
            return Err(format!("mock {name} stdin differs"));
        }
    }
    for (name, mut output) in [
        ("stdout", Box::new(std::io::stdout()) as Box<dyn Write>),
        ("stderr", Box::new(std::io::stderr()) as Box<dyn Write>),
    ] {
        let bytes: Vec<u8> =
            serde_json::from_value(response.get(name).cloned().unwrap_or(json!([])))
                .map_err(|error| error.to_string())?;
        output
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
    }
    u8::try_from(
        response["exit_code"]
            .as_u64()
            .ok_or("missing mock exit code")?,
    )
    .map_err(|error| error.to_string())
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
    if name == "sh" {
        let (script, forwarded) = args
            .split_first()
            .ok_or("missing launcher fixture script")?;
        if fs::read(script).map_err(|e| e.to_string())? != include_bytes!("producer.sh") {
            return Err("unknown bootstrap Shell fixture bytes".into());
        }
        let mut command = Command::new(
            env::var_os("CHRONO_TEST_BOOTSTRAP_PRODUCER").ok_or("missing Rust producer binding")?,
        );
        command.args(forwarded);
        chrono_harness::process_fds::forward_command(&mut command, &[])?;
        use std::os::unix::process::CommandExt;
        return Err(format!(
            "bootstrap launcher fixture exec failed: {}",
            command.exec()
        ));
    }
    if matches!(name, "git" | "chrono-worktree") {
        return command_mock(root, name, &args);
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
                "minimal",
                "--component",
                "rustfmt",
            ] => {
                if root.join("fail-install").exists() {
                    return Ok(17);
                }
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
