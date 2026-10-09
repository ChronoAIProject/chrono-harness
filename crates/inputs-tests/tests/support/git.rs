use std::{
    env,
    ffi::OsStr,
    fs::OpenOptions,
    io::{self, Write},
    os::unix::{ffi::OsStrExt, process::CommandExt},
    process::{self, Command},
};

fn main() -> io::Result<()> {
    if env::var_os("SNAPSHOT_GIT").as_deref() != Some(OsStr::new("declared")) {
        process::exit(81);
    }
    if env::var_os("SHOULD_NOT_LEAK").is_some_and(|value| !value.is_empty()) {
        process::exit(82);
    }
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(env::var_os("SNAPSHOT_TRACE").expect("registered trace path"))?;
    for (index, argument) in arguments.iter().enumerate() {
        if index != 0 {
            trace.write_all(b" ")?;
        }
        trace.write_all(argument.as_bytes())?;
    }
    trace.write_all(b"\n")?;
    drop(trace);
    Err(
        Command::new(env::var_os("SNAPSHOT_REAL_GIT").expect("registered real Git"))
            .args(arguments)
            .exec(),
    )
}
