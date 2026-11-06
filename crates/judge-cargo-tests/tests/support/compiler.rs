use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    os::unix::{ffi::OsStrExt, process::CommandExt},
    process::Command,
};

fn main() -> io::Result<()> {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(env::var_os("CHRONO_TEST_COMPILER_TRACE").expect("registered compiler trace"))?;
    for argument in &arguments {
        trace.write_all(argument.as_bytes())?;
        trace.write_all(b"\n")?;
    }
    drop(trace);
    Err(
        Command::new(env::var_os("CHRONO_TEST_REAL_COMPILER").expect("registered actual compiler"))
            .args(arguments)
            .exec(),
    )
}
