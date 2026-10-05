use std::{
    env,
    ffi::OsStr,
    fs::OpenOptions,
    io::Write,
    os::unix::{ffi::OsStrExt, process::CommandExt},
    path::Path,
    process::{Command, ExitCode},
};

fn append(path: &Path, bytes: &[u8]) {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

fn main() -> ExitCode {
    let invocation = env::args_os().next().unwrap();
    let executable = Path::new(&invocation);
    let parent = executable.parent().unwrap();
    let args: Vec<_> = env::args_os().skip(1).collect();
    let name = executable.file_name().unwrap();
    if name == "git" {
        eprintln!("ambient-git-must-not-run");
        return ExitCode::from(83);
    }
    let (variable, trace) = match name.to_str().unwrap() {
        "selected Git λ" => ("CHRONO_WORKFLOW_REAL_GIT", Some("git-trace")),
        "facts-git" => ("CHRONO_WORKFLOW_REAL_GIT", None),
        "cargo-fixture" => ("CHRONO_WORKFLOW_REAL_CARGO", Some("business-launches")),
        "python-migration" => ("CHRONO_WORKFLOW_REAL_PYTHON", Some("business-launches")),
        other => panic!("unknown registered test tool: {other}"),
    };
    if variable == "CHRONO_WORKFLOW_REAL_GIT"
        && env::var_os("FACTS_SENTINEL").as_deref() != Some(OsStr::new("declared"))
    {
        return ExitCode::from(81);
    }
    if let Some(trace) = trace {
        let mut line = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            if index != 0 {
                line.push(b' ');
            }
            line.extend_from_slice(arg.as_bytes());
        }
        line.push(b'\n');
        append(&parent.join(trace), &line);
    }
    if name == "cargo-fixture" {
        if args.first().is_some_and(|arg| arg == "fixture-prepare") {
            append(Path::new(".chrono-harness/state/order"), b"s");
            return ExitCode::SUCCESS;
        }
        if Path::new(".chrono-harness/state/fail").is_file()
            && args.get(2).is_some_and(|arg| arg == "t2/Cargo.toml")
        {
            return ExitCode::from(7);
        }
    }
    let real = env::var_os(variable).expect("explicit native fixture tool binding");
    panic!(
        "test tool exec failed: {}",
        Command::new(real).args(args).exec()
    );
}
