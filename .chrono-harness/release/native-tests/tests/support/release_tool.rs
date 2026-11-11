//! 发布测试的真实外部工具接口；业务判定在独立 Rust 测试中。
#[path = "context.rs"]
mod context;
use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn run() -> Result<u8, String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--fixture-parent-context"] || args == ["--fixture-parent-context", "explicit"] {
        let mut child = context::command(std::env::current_exe().map_err(|e| e.to_string())?);
        child.arg("--fixture-observe-context");
        if args.len() == 2 {
            child.env("CHRONO_RELEASE_RUN", "fixture-run");
            child.env("GITHUB_REPOSITORY", "fixture/repository");
        }
        chrono_harness::process_fds::forward_command(&mut child, &[])?;
        let output = child.output().map_err(|e| e.to_string())?;
        io::stdout()
            .write_all(&output.stdout)
            .map_err(|e| e.to_string())?;
        io::stderr()
            .write_all(&output.stderr)
            .map_err(|e| e.to_string())?;
        return Ok(output.status.code().unwrap_or(91) as u8);
    }
    if args == ["--fixture-observe-context"] {
        let release: std::collections::BTreeMap<_, _> = std::env::vars()
            .filter(|(name, _)| {
                name.starts_with("CHRONO_RELEASE_")
                    || [
                        "GITHUB_ACTIONS",
                        "GITHUB_REPOSITORY",
                        "GITHUB_WORKFLOW_SHA",
                        "GITHUB_WORKFLOW_REF",
                    ]
                    .contains(&name.as_str())
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({"release":release,"keep":std::env::var("CHRONO_FIXTURE_KEEP").ok()})
        );
        return Ok(0);
    }
    if args == ["--process-output-bound"] {
        io::stdout()
            .write_all(&[7u8; 100_000])
            .map_err(|e| e.to_string())?;
        return Ok(0);
    }
    if args == ["--process-parent"] {
        let child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .arg("--process-tail")
            .spawn()
            .map_err(|e| e.to_string())?;
        println!("original descendant {}", child.id());
        return Ok(0);
    }
    if args == ["--process-tail"] {
        std::thread::sleep(std::time::Duration::from_millis(500));
        fs::write("late-descendant", b"unexpected write after parent exit")
            .map_err(|e| e.to_string())?;
        println!("late descendant tail");
        return Ok(0);
    }
    if args == ["--fixture-asset"] {
        println!("fixture executable b");
        return Ok(0);
    }
    if executable_name() == Some("probe".into())
        && args == ["verify none", "space value", "$literal", "λ"]
        && !Path::new(".chrono-harness/release/tool-fixture.json").exists()
    {
        println!("verify none space value $literal λ");
        return Ok(0);
    }
    let fixture: Value = serde_json::from_slice(
        &fs::read(".chrono-harness/release/tool-fixture.json").map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = executable
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("missing tool fixture name")?;
    match (name, args.as_slice()) {
        ("cargo" | "rustc", [arg]) if arg == "--version" => {
            if name == "rustc" && fixture["behavior"] == "wrong-compiler" {
                println!("rustc wrong-version");
            } else {
                println!("{name} 1.95.0 (fixture)");
            }
            Ok(0)
        }
        ("rustup", [which, flag, version, tool])
            if which == "which"
                && flag == "--toolchain"
                && version == "1.95.0"
                && (tool == "cargo" || tool == "rustc") =>
        {
            println!(
                "{}",
                executable
                    .parent()
                    .ok_or("missing fixture tool parent")?
                    .join(tool)
                    .display()
            );
            Ok(0)
        }
        ("rustup", _)
            if args
                == [
                    "toolchain",
                    "install",
                    "1.95.0",
                    "--profile",
                    "minimal",
                    "--component",
                    "rustfmt",
                ] =>
        {
            Ok(0)
        }
        ("cargo", [build, release, locked, flag, manifest])
            if build == "build"
                && release == "--release"
                && locked == "--locked"
                && flag == "--manifest-path" =>
        {
            let (source, target) = match manifest.as_str() {
                "a/Cargo.toml" => (
                    PathBuf::from(
                        fixture["distribution"]
                            .as_str()
                            .ok_or("missing real distribution executable")?,
                    ),
                    PathBuf::from("out/a"),
                ),
                "b/Cargo.toml" => (executable.clone(), PathBuf::from("out/b")),
                _ => return Err("unexpected manifest".into()),
            };
            fs::create_dir_all(target.parent().ok_or("missing asset parent")?)
                .map_err(|e| e.to_string())?;
            fs::copy(source, &target).map_err(|e| e.to_string())?;
            fs::set_permissions(target, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            Ok(0)
        }
        ("probe", _) => {
            let consuming = if args == ["verify a"] {
                true
            } else if args == ["verify none", "space value", "$literal", "λ"] {
                false
            } else {
                return Err(format!("changed original action: {args:?}"));
            };
            if consuming {
                let mut command = Command::new(
                    Path::new("consumer/a")
                        .canonicalize()
                        .map_err(|e| e.to_string())?,
                );
                command.arg("--version");
                chrono_harness::process_fds::forward_command(&mut command, &[])?;
                if !command.status().map_err(|e| e.to_string())?.success() {
                    return Err("selected distribution consumer failed".into());
                }
            }
            let behavior = fixture["behavior"].as_str().unwrap_or("pass");
            if consuming && matches!(behavior, "drift" | "fail-and-drift") {
                fs::set_permissions("consumer/a", fs::Permissions::from_mode(0o751))
                    .map_err(|e| e.to_string())?;
            }
            if matches!(behavior, "fail" | "fail-and-drift") {
                fs::create_dir_all("failures/test-a").map_err(|e| e.to_string())?;
                fs::write("failures/test-a/original.bin", [0u8, 254, 255])
                    .map_err(|e| e.to_string())?;
                io::stdout()
                    .write_all(&[0, 255, 10])
                    .map_err(|e| e.to_string())?;
                io::stderr()
                    .write_all(&[254, 0, 10])
                    .map_err(|e| e.to_string())?;
                if let Some(nested) = fixture["nested"].as_str() {
                    eprintln!("original native publication process evidence: {nested}");
                }
                return Ok(37);
            }
            println!("original literal verifier passed");
            Ok(0)
        }
        _ => Err(format!("unknown external tool invocation: {name} {args:?}")),
    }
}

fn executable_name() -> Option<String> {
    std::env::current_exe().ok()?.file_name()?.to_str().map(str::to_owned)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(91)
        }
    }
}
