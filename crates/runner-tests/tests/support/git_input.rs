use serde_json::Value;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::{fs, path::Path, process::Command};

fn emit(stdout: &[u8], stderr: &[u8], code: i32) -> ! {
    io::stdout().write_all(stdout).unwrap();
    io::stdout().flush().unwrap();
    io::stderr().write_all(stderr).unwrap();
    io::stderr().flush().unwrap();
    std::process::exit(code);
}

fn main() {
    let executable = std::env::current_exe().unwrap();
    let root = executable.parent().unwrap();
    let case: Value =
        serde_json::from_slice(&fs::read(root.join("git-case.json")).unwrap()).unwrap();
    let args: Vec<_> = std::env::args().skip(1).collect();
    let joined = args.join(" ");
    writeln!(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("trace"))
            .unwrap(),
        "{joined}"
    )
    .unwrap();
    let mode = case["mode"].as_str().unwrap();
    let exists = |name: &str| Path::new(name).exists();
    let drift = || fs::write(".git/config.worktree", "changed").unwrap();
    if joined.contains("rev-parse") {
        match mode {
            "input-drift" => {
                drift();
                emit(b"original output", b"original error", 17);
            }
            "selector-drift" => {
                fs::write(".chrono-harness/git-platforms.json", "changed").unwrap();
                emit(b"original", b"", 17);
            }
            "identity" => {
                if exists("identity-fail") {
                    emit(b"", b"original error", 17);
                }
                if joined.contains("^{tree}") {
                    if exists("tree-fail") {
                        emit(b"", b"", 17);
                    }
                    if exists("tree-response") {
                        emit(&fs::read("tree-response").unwrap(), b"", 0);
                    }
                }
                if exists("object-response") {
                    emit(&fs::read("object-response").unwrap(), b"", 0);
                }
            }
            _ => {}
        }
    }
    if joined.contains(" ls-tree ") && exists("fail-tree") {
        match mode {
            "tree-drift" => drift(),
            "tree-malformed" => emit(b"malformed", b"", 0),
            _ => {}
        }
    }
    if joined.contains(" show ") {
        match mode {
            "blob-drift" if !exists("retry-ready") => drift(),
            "blob-fail" if !exists("retry-ready") => {
                emit(b"original output", b"original error", 17)
            }
            "bound-value" => emit(std::env::var("BOUND_VALUE").unwrap().as_bytes(), b"", 0),
            _ => {}
        }
    }
    if joined.ends_with(" cat-file --batch") {
        match mode {
            "batch-drift" if !exists("retry-ready") => drift(),
            "batch-fail" if !exists("retry-ready") => {
                emit(b"original partial", b"original error", 17)
            }
            "batch-output" => emit(&fs::read("oversized").unwrap(), b"", 0),
            "batch-time" => std::thread::sleep(std::time::Duration::from_secs(60)),
            _ => {}
        }
    }
    if mode == "batch-response"
        && joined.ends_with(&format!(" cat-file {}", case["phase"].as_str().unwrap()))
        && exists("response")
    {
        emit(&fs::read("response").unwrap(), b"", 0);
    }
    let error = Command::new(case["git"].as_str().unwrap())
        .args(args)
        .exec();
    panic!("execute real Git: {error}");
}
