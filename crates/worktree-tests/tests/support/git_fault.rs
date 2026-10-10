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

fn collide_with_intents(suffix: &str) {
    let directory = Path::new(".chrono-harness/state/worktrees");
    let mut reports: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_str().unwrap();
            name.starts_with("start-") && name.ends_with(".json")
        })
        .collect();
    reports.sort();
    assert!(
        !reports.is_empty(),
        "fixture requires the original start report"
    );
    for path in reports {
        let mut collision = path.into_os_string();
        collision.push(suffix);
        fs::write(PathBuf::from(collision), "occupied").unwrap();
    }
}

fn has_rebind_intent() -> bool {
    fs::read_dir(".chrono-harness/state/worktrees").is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry.path().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".rebind-intent.json")
        })
    })
}

fn update_remote(git: &str, home: &Path, reference: &str) {
    let head = fs::read_to_string(home.join("new-head")).unwrap();
    let mut directory = OsString::from("--git-dir=");
    directory.push(home.join("declared upstream.git"));
    require_success(
        Command::new(git)
            .arg(directory)
            .args(["update-ref", reference, head.trim_end_matches('\n')])
            .status()
            .unwrap(),
    );
}

fn main() {
    let path = std::env::var_os("CHRONO_TEST_GIT_FIXTURE").expect("explicit Git fixture input");
    let config: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").expect("fixture HOME"));
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let git = config["git"].as_str().unwrap();
    let arg = |index: usize, expected: &str| args.get(index).is_some_and(|value| value == expected);
    match config["mode"].as_str().unwrap() {
        "temporary-source-check" => {
            if std::env::current_dir().unwrap() == Path::new(config["root"].as_str().unwrap())
                && arg(1, "ls-tree")
            {
                fs::write(
                    home.join("temporary-check-active"),
                    "source check holds exclusion",
                )
                .unwrap();
                while !home.join("temporary-check-release").is_file() {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
        "remote" => match config["fault"].as_str().unwrap() {
            "remote-lease" if arg(1, "push") => {
                update_remote(git, &home, "refs/heads/integration/owned");
            }
            "post-delete" if arg(1, "push") => {
                require_success(Command::new(git).args(&args).status().unwrap());
                io::stderr()
                    .write_all(b"\xfforiginal post-delete failure\n")
                    .unwrap();
                std::process::exit(73);
            }
            fault @ ("remote-target" | "remote-plan") if arg(1, "push") => {
                require_success(Command::new(git).args(&args).status().unwrap());
                if fault == "remote-target" {
                    update_remote(git, &home, "refs/heads/dev");
                } else {
                    fs::write(".chrono-harness/state/maintenance.json", "changed").unwrap();
                }
                return;
            }
            "index-failure" if arg(1, "read-tree") => {
                io::stderr()
                    .write_all(b"\xfforiginal index preparation failure\n")
                    .unwrap();
                std::process::exit(29);
            }
            "donor-failure" if arg(1, "worktree") && arg(2, "add") && arg(3, "--detach") => {
                if !has_rebind_intent() {
                    std::process::exit(78);
                }
                if !Path::new(".chrono-harness/state/original-metadata/metadata/index").is_file() {
                    std::process::exit(79);
                }
                require_success(Command::new(git).args(&args).status().unwrap());
                eprintln!("original failure after donor creation");
                std::process::exit(31);
            }
            "repair-failure" if arg(1, "worktree") && arg(2, "repair") => {
                require_success(Command::new(git).args(&args).status().unwrap());
                eprintln!("original failure after repair");
                std::process::exit(32);
            }
            "backup-change"
                if arg(1, "worktree")
                    && arg(2, "unlock")
                    && Path::new(".chrono-harness/state/original-metadata/metadata").is_dir() =>
            {
                fs::write(
                    ".chrono-harness/state/original-metadata/metadata/index",
                    "changed backup",
                )
                .unwrap();
            }
            "rebind-interruption" if home.join("rebind-stage").is_file() => {
                let stage = fs::read_to_string(home.join("rebind-stage")).unwrap();
                let (trigger, after) = match stage.trim_end_matches('\n') {
                    "before-preservation" => (
                        arg(1, "rev-parse")
                            && arg(2, "--verify")
                            && has_rebind_intent()
                            && !Path::new(".chrono-harness/state/original-metadata").is_dir(),
                        false,
                    ),
                    "before-add" => (
                        arg(1, "worktree") && arg(2, "add") && arg(3, "--detach"),
                        false,
                    ),
                    "after-add" => (
                        arg(1, "worktree") && arg(2, "add") && arg(3, "--detach"),
                        true,
                    ),
                    "after-branch" => (
                        arg(1, "symbolic-ref") && arg(2, "HEAD") && args.len() == 4,
                        true,
                    ),
                    "after-index" => (arg(1, "read-tree"), true),
                    "before-repair" => (arg(1, "worktree") && arg(2, "repair"), false),
                    "after-repair" => (arg(1, "worktree") && arg(2, "repair"), true),
                    "before-unlock" => (arg(1, "worktree") && arg(2, "unlock"), false),
                    "after-unlock" => (arg(1, "worktree") && arg(2, "unlock"), true),
                    _ => (false, false),
                };
                if trigger {
                    if after {
                        require_success(Command::new(git).args(&args).status().unwrap());
                    }
                    fs::write(home.join("rebind-interrupted"), "stopped").unwrap();
                    if home.join("rebind-fail").is_file() {
                        io::stderr()
                            .write_all(b"\xfforiginal failed rebind\n")
                            .unwrap();
                        std::process::exit(83);
                    }
                    interrupt_parent();
                    return;
                }
            }
            "ignore-race"
                if std::env::current_dir().unwrap() == home.join("source with spaces")
                    && arg(1, "ls-files")
                    && arg(2, "--others")
                    && !args.iter().any(|arg| arg == "--ignored") =>
            {
                fs::write(".git/info/exclude", "unregistered-work\n").unwrap();
                let status = Command::new(git).args(&args).status().unwrap();
                fs::write(".git/info/exclude", "").unwrap();
                require_success(status);
                return;
            }
            "remote-lease"
            | "post-delete"
            | "remote-target"
            | "remote-plan"
            | "index-failure"
            | "donor-failure"
            | "repair-failure"
            | "backup-change"
            | "rebind-interruption"
            | "ignore-race" => (),
            fault => panic!("unknown remote/rebind fixture: {fault}"),
        },
        "interruption" => {
            let stage = home.join("interrupt-stage");
            if stage.is_file() {
                let stage = fs::read_to_string(stage).unwrap();
                let observed = format!(
                    "{} {}",
                    args.get(1).map_or("", |v| v.to_str().unwrap()),
                    args.get(2).map_or("", |v| v.to_str().unwrap())
                );
                if stage.trim_end_matches('\n') == observed {
                    require_success(Command::new(git).args(&args).status().unwrap());
                    fs::write(home.join("interruption-observed"), "completed").unwrap();
                    interrupt_parent();
                    return;
                }
            }
            match config["extra"].as_str().unwrap() {
                "none" => (),
                "mark-invoked" => {
                    fs::write(home.join("unexpected-plan-git"), "invoked").unwrap();
                }
                "checkout-collision" => {
                    if arg(1, "show-ref") && arg(4, "refs/heads/feature/new") {
                        collide_with_intents(".intent.json");
                    }
                }
                "fetch-collision" => {
                    if arg(1, "worktree") && arg(2, "list") {
                        collide_with_intents(".fetch-intent.json");
                    }
                    if arg(1, "fetch") {
                        fs::write(home.join("unexpected-fetch"), "invoked").unwrap();
                    }
                }
                "result-change" => {
                    let result = home.join("change-result");
                    if result.is_file()
                        && arg(1, "update-ref")
                        && arg(2, "--no-deref")
                        && arg(3, "-d")
                    {
                        require_success(Command::new(git).args(&args).status().unwrap());
                        let path = fs::read_to_string(result).unwrap();
                        fs::write(path.trim_end_matches('\n'), "changed").unwrap();
                        return;
                    }
                }
                extra => panic!("unknown interruption fixture: {extra}"),
            }
        }
        "maintenance-remove" => {
            if config["concurrent"] == true {
                if arg(1, "update-ref") && arg(3, "-d") {
                    let path = home.join("new-head");
                    let head = match fs::read_to_string(&path) {
                        Ok(head) => head,
                        Err(error) => {
                            eprintln!("fixture head input {}: {error}", path.display());
                            String::new()
                        }
                    };
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
        "local-check-head-drift" => {
            let target = Path::new(config["target"].as_str().unwrap());
            if std::env::current_dir().unwrap() == target && home.join("arm-head-drift").is_file() {
                if arg(1, "update-ref") && arg(2, "-d") {
                    fs::write(home.join("local-fetch-complete"), "observed").unwrap();
                }
                if arg(1, "rev-parse")
                    && args.last().is_some_and(|arg| arg == "HEAD")
                    && home.join("local-fetch-complete").is_file()
                    && !home.join("head-drift-applied").exists()
                {
                    let observed = Command::new(git).args(&args).output().unwrap();
                    require_success(observed.status);
                    require_success(
                        Command::new(git)
                            .args(["reset", "--hard", config["replacement"].as_str().unwrap()])
                            .stdout(Stdio::null())
                            .status()
                            .unwrap(),
                    );
                    fs::write(home.join("head-drift-applied"), "committed replacement").unwrap();
                    io::stdout().write_all(&observed.stdout).unwrap();
                    return;
                }
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
        "inventory-failure" => {
            if home.join("fail-inventory").is_file() && arg(1, "worktree") && arg(2, "list") {
                eprintln!("original inventory failure");
                std::process::exit(71);
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
