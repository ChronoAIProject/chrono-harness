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
    if env::var_os("BOUND_EVENT_VALUE").as_deref() != Some(OsStr::new("declared")) {
        process::exit(81);
    }
    if env::var_os("CHRONO_EVENT_AMBIENT").is_some_and(|value| !value.is_empty()) {
        process::exit(82);
    }
    let arguments: Vec<_> = arguments.collect();
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(env::var_os("EVENT_TRACE").expect("registered trace path"))?;
    for (index, argument) in arguments.iter().enumerate() {
        if index != 0 {
            trace.write_all(b" ")?;
        }
        trace.write_all(argument.as_bytes())?;
    }
    trace.write_all(b"\n")?;
    drop(trace);

    let mode = env::var("EVENT_GIT_MODE").expect("registered Git fixture mode");
    let probe = arguments
        .iter()
        .any(|argument| argument.as_bytes().starts_with(b"--batch-check"));
    let fetch = arguments.iter().any(|argument| argument == "fetch");
    match mode.as_str() {
        "pass" => {}
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
        | "scope-probe-failure" => {}
        _ => failure(b"unknown registered Git fixture mode\n", 84),
    }
    Err(
        Command::new(env::var_os("EVENT_REAL_GIT").expect("registered real Git"))
            .args(arguments)
            .exec(),
    )
}
