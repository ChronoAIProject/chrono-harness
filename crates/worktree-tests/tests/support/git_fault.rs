use serde_json::Value;
use std::{
    ffi::OsString,
    fs::{self, File},
    os::{fd::AsRawFd, raw::c_int, unix::process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
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

fn main() {
    let path = std::env::var_os("CHRONO_TEST_GIT_FIXTURE").expect("explicit Git fixture input");
    let config: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").expect("fixture HOME"));
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match config["mode"].as_str().unwrap() {
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
    let error = Command::new(config["git"].as_str().unwrap())
        .args(args)
        .exec();
    panic!("fixture Git exec failed: {error}");
}
