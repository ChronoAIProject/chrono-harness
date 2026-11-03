use std::{env, fs::OpenOptions, io::Write, process::Command};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let output = Command::new(env::var_os("CHRONO_REAL_CARGO").unwrap())
        .args(&args)
        .output()
        .unwrap();
    let phase = env::var("CHRONO_MUTATION_PHASE").unwrap();
    if args.first().is_some_and(|arg| arg == std::ffi::OsStr::new(&phase)) {
        OpenOptions::new()
            .append(true)
            .open(env::var_os("CHRONO_MUTATION_TARGET").unwrap())
            .unwrap()
            .write_all(b" ")
            .unwrap();
    }
    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();
    std::process::exit(output.status.code().unwrap());
}
