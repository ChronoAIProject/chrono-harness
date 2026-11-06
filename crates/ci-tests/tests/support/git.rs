use std::{
    env,
    ffi::OsStr,
    fs::OpenOptions,
    io::{self, BufRead, Write},
    os::unix::{ffi::OsStrExt, process::CommandExt},
    path::Path,
    process::{self, Command},
};

fn failure(bytes: &[u8], code: i32) -> ! {
    io::stderr().write_all(bytes).unwrap();
    process::exit(code);
}

fn main() -> io::Result<()> {
    let mut arguments = env::args_os();
    let program = arguments.next().unwrap();
    if Path::new(&program).file_name() == Some(OsStr::new("git")) {
        failure(b"ambient-git\n", 83);
    }
    let (bound, ambient, trace_name, mode_name, real_git) =
        match env::var("CHRONO_TEST_GIT_PROFILE").as_deref() {
            Ok("event") => (
                "BOUND_EVENT_VALUE",
                "CHRONO_EVENT_AMBIENT",
                "EVENT_TRACE",
                "EVENT_GIT_MODE",
                "EVENT_REAL_GIT",
            ),
            Ok("scoped") => (
                "BOUND_SCOPED_VALUE",
                "SCOPED_AMBIENT",
                "SCOPED_TRACE",
                "SCOPED_GIT_MODE",
                "SCOPED_REAL_GIT",
            ),
            Ok("registration") => (
                "BOUND_FACTS_VALUE",
                "CHRONO_AMBIENT_PROBE",
                "CHRONO_FACTS_TRACE",
                "FACTS_GIT_MODE",
                "FACTS_REAL_GIT",
            ),
            _ => failure(b"unknown registered Git fixture profile\n", 84),
        };
    if env::var_os(bound).as_deref() != Some(OsStr::new("declared")) {
        process::exit(81);
    }
    if env::var_os(ambient).is_some_and(|value| !value.is_empty()) {
        process::exit(82);
    }
    let arguments: Vec<_> = arguments.collect();
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(env::var_os(trace_name).expect("registered trace path"))?;
    for (index, argument) in arguments.iter().enumerate() {
        if index != 0 {
            trace.write_all(b" ")?;
        }
        trace.write_all(argument.as_bytes())?;
    }
    trace.write_all(b"\n")?;
    drop(trace);

    let mode = env::var(mode_name).expect("registered Git fixture mode");
    let probe = arguments
        .iter()
        .any(|argument| argument.as_bytes().starts_with(b"--batch-check"));
    let fetch = arguments.iter().any(|argument| argument == "fetch");
    let version = arguments
        .first()
        .is_some_and(|argument| argument == "--version");
    let tree = arguments.iter().any(|argument| argument == "ls-tree");
    let registration = env::var("CHRONO_TEST_GIT_PROFILE").as_deref() == Ok("registration");
    match mode.as_str() {
        "pass" => {}
        "read-failure" if tree => {
            io::stdout().write_all(b"a\0b")?;
            io::stdout().flush()?;
            failure(
                if registration {
                    b"selected-facts-failure"
                } else {
                    b"chosen-failure"
                },
                17,
            );
        }
        "version-exit" if version => {
            io::stdout().write_all(b"a\0b")?;
            io::stdout().flush()?;
            failure(b"version-failed", 19);
        }
        "version-timeout" if version => {
            if registration {
                io::stdout().write_all(b"before-timeout")?;
                io::stdout().flush()?;
            }
            std::thread::sleep(std::time::Duration::from_secs(if registration {
                10
            } else {
                60
            }));
            return Ok(());
        }
        "read-timeout" if tree => {
            io::stdout().write_all(b"before-timeout")?;
            io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
        "read-limit" if tree && registration => {
            for _ in 0..400 {
                io::stdout().write_all(b"0123456789012345678901234567890123456789")?;
            }
            io::stdout().flush()?;
        }
        "read-limit" if tree => loop {
            io::stdout().write_all(b"0123456789abcdef")?;
        },
        "read-drift" if arguments.iter().any(|argument| argument == "rev-parse") => {
            let status = Command::new(env::var_os(real_git).expect("registered real Git"))
                .args(arguments)
                .status()?;
            if status.success() {
                // Replace this test's private copied image after Git exits.
                // Reuse the transport fixture's permission-preserving rename
                // method on platforms that forbid writing an executing image.
                let executable = env::current_exe()?;
                let mut bytes = std::fs::read(&executable)?;
                bytes.extend_from_slice(b"\n# persistent drift\n");
                let replacement = executable.with_extension("replacement");
                std::fs::write(&replacement, bytes)?;
                std::fs::set_permissions(
                    &replacement,
                    std::fs::metadata(&executable)?.permissions(),
                )?;
                std::fs::rename(replacement, executable)?;
            }
            process::exit(status.code().unwrap_or(1));
        }
        "probe-failure" if probe => {
            io::copy(&mut io::stdin(), &mut io::sink())?;
            failure(b"\xfforiginal-probe\n", 73);
        }
        "malformed-probe" if probe => {
            io::copy(&mut io::stdin(), &mut io::sink())?;
            io::stdout().write_all(b"not-the-request missing\n")?;
            return Ok(());
        }
        "wrong-type-probe" | "extra-probe-output" if probe => {
            let mut oid = Vec::new();
            io::stdin().lock().read_until(b'\n', &mut oid)?;
            if oid.last() == Some(&b'\n') {
                oid.pop();
            }
            io::stdout().write_all(&oid)?;
            io::stdout().write_all(if mode == "wrong-type-probe" {
                b" blob\n"
            } else {
                b" commit\nextra\n"
            })?;
            return Ok(());
        }
        "full-probe-failure" if probe => failure(b"\xffbroken", 29),
        "full-fetch-failure" if fetch => failure(b"\xfefetch-failure", 31),
        "scope-probe-failure" if probe => failure(b"\xffscope-probe-failure", 29),
        "probe-failure"
        | "malformed-probe"
        | "wrong-type-probe"
        | "extra-probe-output"
        | "full-probe-failure"
        | "full-fetch-failure"
        | "scope-probe-failure"
        | "read-failure"
        | "read-drift"
        | "read-timeout"
        | "version-exit"
        | "version-timeout"
        | "read-limit" => {}
        _ => failure(b"unknown registered Git fixture mode\n", 84),
    }
    Err(
        Command::new(env::var_os(real_git).expect("registered real Git"))
            .args(arguments)
            .exec(),
    )
}
