use serde_json::Value;
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, Write},
    os::{
        fd::AsRawFd,
        raw::c_int,
        unix::process::{CommandExt, ExitStatusExt},
    },
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::Duration,
};

unsafe extern "C" {
    fn getppid() -> c_int;
    fn kill(pid: c_int, signal: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
}

fn interrupt_parent() {
    assert_eq!(unsafe { kill(getppid(), 9) }, 0);
}

fn redirect(path: &Path, descriptor: c_int) {
    let file = File::create(path).unwrap();
    assert_eq!(unsafe { dup2(file.as_raw_fd(), descriptor) }, descriptor);
}

fn require_success(status: ExitStatus) {
    if !status.success() {
        std::process::exit(
            status
                .code()
                .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)),
        );
    }
}

fn capture(program: &str, args: &[OsString]) -> String {
    let output = Command::new(program)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

fn main() {
    let path = std::env::var_os("CHRONO_TEST_GIT_FIXTURE").expect("explicit Git fixture input");
    let config: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").expect("fixture HOME"));
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let git = config["git"].as_str().unwrap();
    let arg = |index: usize, expected: &str| args.get(index).is_some_and(|value| value == expected);
    match config["mode"].as_str().unwrap() {
        "maintenance-remove" => {
            if config["concurrent"] == true {
                if arg(1, "update-ref") && arg(3, "-d") {
                    let head = fs::read_to_string(home.join("new-head")).unwrap();
                    Command::new(git)
                        .args([
                            "update-ref",
                            "refs/heads/feature/saved",
                            head.trim_end_matches('\n'),
                        ])
                        .status()
                        .unwrap();
                }
            } else if arg(1, "worktree") && arg(2, "remove") {
                eprintln!("original-remove-failure");
                std::process::exit(71);
            }
        }
        "retention-race" => {
            if arg(1, "worktree") && arg(2, "lock") {
                require_success(Command::new(git).args(&args).status().unwrap());
                let head = fs::read_to_string(home.join("new-head")).unwrap();
                require_success(
                    Command::new(git)
                        .args(["update-ref", "refs/heads/dev", head.trim_end_matches('\n')])
                        .status()
                        .unwrap(),
                );
                return;
            }
        }
        "unsaved-after-unlock" => {
            if home.join("arm").is_file() && arg(1, "worktree") && arg(2, "unlock") {
                require_success(Command::new(git).args(&args).status().unwrap());
                fs::write(Path::new(&args[3]).join("payload"), "new unsaved work").unwrap();
                return;
            }
        }
        "fetch-failure" => {
            if arg(1, "fetch") {
                let action = fs::read_to_string(home.join("fetch-action")).unwrap();
                if action.trim_end_matches('\n') == "fetch-block" {
                    require_success(Command::new(git).args(&args).status().unwrap());
                    eprintln!("failure-after-fetch");
                    std::process::exit(72);
                }
            }
            if arg(1, "update-ref")
                && arg(3, "-d")
                && args.get(4).is_some_and(|v| {
                    v.to_string_lossy()
                        .starts_with("refs/chrono-harness/fetch/")
                })
            {
                let action = fs::read_to_string(home.join("fetch-action")).unwrap();
                match action.trim_end_matches('\n') {
                    "block" => {
                        eprintln!("original-fetch-ref-removal-failure");
                        std::process::exit(71);
                    }
                    "race" => {
                        let head = fs::read_to_string(home.join("fetch-new-head")).unwrap();
                        require_success(
                            Command::new(git)
                                .arg("update-ref")
                                .arg(&args[4])
                                .arg(head.trim_end_matches('\n'))
                                .status()
                                .unwrap(),
                        );
                    }
                    "remove-fail" => {
                        require_success(Command::new(git).args(&args).status().unwrap());
                        eprintln!("failure-after-ref-removal");
                        std::process::exit(73);
                    }
                    _ => (),
                }
            }
        }
        "artifact-removal" => {
            if arg(1, "worktree") && arg(2, "remove") {
                let failure = match config["fault"].as_str().unwrap() {
                    "must-be-absent"
                        if Path::new(&args[5])
                            .join(".chrono-harness/state/output")
                            .exists() =>
                    {
                        Some("artifacts-reached-git-removal")
                    }
                    "fail-marker" if home.join("fail-remove").exists() => {
                        Some("original-remove-failure")
                    }
                    "must-be-absent" | "fail-marker" => None,
                    fault => panic!("unknown artifact removal fixture: {fault}"),
                };
                if let Some(message) = failure {
                    eprintln!("{message}");
                    std::process::exit(71);
                }
            }
        }
        "batch-failure" => {
            if args.ends_with(&["cat-file".into(), "--batch".into()]) {
                io::stdout()
                    .write_all(config["stdout"].as_str().unwrap().as_bytes())
                    .unwrap();
                io::stdout().flush().unwrap();
                io::stderr()
                    .write_all(b"original batch diagnostic")
                    .unwrap();
                std::process::exit(i32::try_from(config["exit"].as_u64().unwrap()).unwrap());
            }
        }
        "mark-executed" => {
            fs::write(home.join("must-not-exist"), "executed").unwrap();
            return;
        }
        "stage-after-tree" => {
            if home.join("stage-after-tree").is_file()
                && std::env::current_dir().unwrap() == home.join("tree-index")
                && arg(1, "ls-tree")
            {
                require_success(Command::new(git).args(&args).status().unwrap());
                fs::create_dir_all("output λ").unwrap();
                fs::write("output λ/staged-source", "source").unwrap();
                require_success(
                    Command::new(git)
                        .args(["add", "-f", "--", "output λ/staged-source"])
                        .status()
                        .unwrap(),
                );
                return;
            }
        }
        "identity-fault" => {
            let fault = home.join("identity-fault");
            let cwd = std::env::current_dir().unwrap();
            if fault.is_file()
                && cwd == home.join("observed")
                && arg(1, "rev-parse")
                && arg(2, "--show-toplevel")
                && arg(3, "--git-common-dir")
            {
                let fault = fs::read_to_string(fault).unwrap();
                match fault.trim_end_matches('\n') {
                    "truncated" => println!("{}", cwd.display()),
                    "malformed-head" | "foreign-common" => {
                        let malformed = fault.trim_end_matches('\n') == "malformed-head";
                        let common = home.join(if malformed {
                            "source with spaces/.git"
                        } else {
                            "foreign-common"
                        });
                        let metadata =
                            capture(git, &["rev-parse".into(), "--absolute-git-dir".into()]);
                        let head = if malformed {
                            "not-an-oid".into()
                        } else {
                            capture(git, &["rev-parse".into(), "HEAD".into()])
                        };
                        println!(
                            "{}\n{}\n{}\n{}\nrefs/heads/feature/observed",
                            cwd.display(),
                            common.display(),
                            metadata.trim_end_matches('\n'),
                            head.trim_end_matches('\n')
                        );
                    }
                    "foreign-branch" => print!(
                        "{}",
                        capture(git, &args)
                            .replace("refs/heads/feature/observed", "refs/heads/feature/other")
                    ),
                    "foreign-metadata" => {
                        for line in capture(git, &args).lines() {
                            if let Some(index) = line.rfind("/worktrees/observed") {
                                println!(
                                    "{}{}",
                                    home.join("foreign-common").display(),
                                    &line[index + "/worktrees/observed".len()..]
                                );
                            } else {
                                println!("{line}");
                            }
                        }
                    }
                    "failure" => {
                        eprintln!("original identity failure");
                        std::process::exit(71);
                    }
                    fault => panic!("unknown identity fixture fault: {fault}"),
                }
                io::stdout().flush().unwrap();
                return;
            }
        }
        "automatic-remove" => {
            if home.join("fail-remove").is_file() && arg(1, "worktree") && arg(2, "remove") {
                if config["after_effect"] == true {
                    require_success(Command::new(git).args(&args).status().unwrap());
                }
                eprintln!("original automatic removal failure");
                std::process::exit(71);
            }
        }
        "cache-interrupt" | "cache-fail" => {
            let fail = config["mode"] == "cache-fail";
            let marker = if fail {
                "fail-cache"
            } else {
                "interrupt-cache"
            };
            if home.join(marker).is_file()
                && !home.join(config["output"].as_str().unwrap()).is_dir()
            {
                if fail {
                    eprintln!("original cache failure");
                    std::process::exit(71);
                }
                if let Some(name) = config["observed"].as_str() {
                    fs::write(home.join(name), "interrupted").unwrap();
                }
                interrupt_parent();
            }
        }
        "birth-interrupt" => {
            let ledger = Path::new(config["ledger"].as_str().unwrap());
            if home.join("interrupt-birth").is_file()
                && ledger.is_file()
                && fs::read_to_string(ledger)
                    .unwrap()
                    .contains("\"kind\": \"birth\"")
            {
                interrupt_parent();
            }
        }
        "recovery-interrupt" => {
            if home.join("interrupt-recovery").is_file()
                && Path::new(config["recovery"].as_str().unwrap()).is_file()
            {
                interrupt_parent();
            }
        }
        "admission-interrupt" => {
            if home.join("interrupt-admission").is_file()
                && args.get(1).is_some_and(|arg| arg == "worktree")
                && args.get(2).is_some_and(|arg| arg == "add")
            {
                fs::write(
                    home.join("admission-holder"),
                    std::process::id().to_string(),
                )
                .unwrap();
                redirect(&home.join("git-mutation.stdout"), 1);
                redirect(&home.join("git-mutation.stderr"), 2);
                interrupt_parent();
                while !home.join("release-mutation").is_file() {
                    thread::sleep(Duration::from_millis(20));
                }
            }
        }
        mode => panic!("unknown Git fixture mode: {mode}"),
    }
    let error = Command::new(git).args(args).exec();
    panic!("fixture Git exec failed: {error}");
}
