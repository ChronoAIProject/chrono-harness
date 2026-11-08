use chrono_host_release::{execution, recipe::Scope};
use std::{path::Path, process::ExitCode};

fn run() -> Result<i32, String> {
    let args = std::env::args_os()
        .skip(1)
        .map(|s| {
            s.into_string()
                .map_err(|_| "non UTF-8 release argument".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    match args.as_slice() {
        [root, output, flag, id] if flag == "--unit" => execution::scoped(
            Path::new(root),
            Path::new(output),
            Scope::Unit(id),
            std::env::vars_os().collect(),
        ),
        [root, output, flag] if flag == "--collect" => execution::scoped(
            Path::new(root),
            Path::new(output),
            Scope::Collect,
            std::env::vars_os().collect(),
        ),
        _ => Err("usage: chrono-host-release ROOT ABSENT_OUTPUT (--unit ID | --collect)".into()),
    }
}

fn main() -> ExitCode {
    let code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}
