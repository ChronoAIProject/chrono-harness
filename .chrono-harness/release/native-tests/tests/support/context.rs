//! 临时发布宿主只接收样本显式提供的发布输入。
use std::{ffi::OsStr, process::Command};

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    for name in [
        "CHRONO_RELEASE_LOCAL",
        "CHRONO_RELEASE_RUN",
        "CHRONO_RELEASE_ATTEMPT",
        "CHRONO_RELEASE_JOB",
        "CHRONO_RELEASE_DEPENDENCIES",
        "CHRONO_RELEASE_CHANNEL",
        "CHRONO_RELEASE_SOURCE",
        "CHRONO_RELEASE_CI_RUN_ID",
        "CHRONO_RELEASE_CI_ATTEMPT",
        "CHRONO_RELEASE_VERSION",
        "GITHUB_ACTIONS",
        "GITHUB_REPOSITORY",
        "GITHUB_WORKFLOW_SHA",
        "GITHUB_WORKFLOW_REF",
    ] {
        command.env_remove(name);
    }
    command
}

// 高频 frontend Git／探针与命令上下文共用标准库边界。

#[allow(dead_code)]
fn run_frontend_tool() -> Result<u8, String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut input = Vec::new();
    std::io::Read::read_to_end(&mut std::io::stdin(), &mut input).map_err(|e| e.to_string())?;
    if !input.is_empty() {
        return Err("unexpected frontend tool stdin".into());
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = executable
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("missing frontend tool fixture name")?;
    match name {
        "git" => {
            let args = args
                .strip_prefix(&["--no-optional-locks".into(), "--no-replace-objects".into()])
                .unwrap_or(&args);
            let args = args.iter().map(String::as_str).collect::<Vec<_>>();
            match args.as_slice() {
                ["rev-parse", "--show-toplevel", "--verify", "HEAD"] => {
                    println!(
                        "{}\n{}",
                        std::env::current_dir()
                            .map_err(|e| e.to_string())?
                            .display(),
                        "a".repeat(40)
                    );
                }
                ["rev-parse", "--verify", "HEAD" | "HEAD^{tree}"]
                | ["rev-parse", "HEAD" | "HEAD^{tree}"]
                | [
                    "rev-parse",
                    "--verify",
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa^{tree}",
                ] => {
                    println!(
                        "{}",
                        if args.last().unwrap().ends_with("^{tree}") {
                            "b"
                        } else {
                            "a"
                        }
                        .repeat(40)
                    );
                }
                ["status", "--porcelain", "--untracked-files=normal"]
                | [
                    "status",
                    "--porcelain=v1",
                    "--untracked-files=normal",
                    "--ignore-submodules=none",
                ] => {}
                _ => return Err(format!("unexpected frontend Git invocation: {args:?}")),
            }
            return Ok(0);
        }
        "probe" if args == ["verify none", "space value", "$literal", "λ"] => {
            println!("original literal verifier passed");
            Ok(0)
        }
        _ => Err(format!(
            "unexpected frontend tool invocation: {name} {args:?}"
        )),
    }
}

#[allow(dead_code)]
fn main() -> std::process::ExitCode {
    use std::process::ExitCode;
    match run_frontend_tool() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(91)
        }
    }
}
