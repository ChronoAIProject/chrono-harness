//! frontend 的高频 Git／探针替身；只依赖标准库，不引入发布程序或进程引擎。
use std::process::ExitCode;

fn run() -> Result<u8, String> {
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

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(91)
        }
    }
}
