use chrono_host_release::{delivery, execution, frontend, local, recipe::Scope};
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
        [root, output, flag, config, native] if flag == "--install" => delivery::install(
            Path::new(root), Path::new(output), config, native, &std::env::vars_os().collect()
        ),
        [root, output, flag, config] if flag == "--deliver" => delivery::run(
            Path::new(root), Path::new(output), config, &std::env::vars_os().collect()
        ),
        [root, output] => local::run(
            Path::new(root),
            Path::new(output),
            std::env::vars_os().collect(),
        ),
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
        [root, output, flag] if flag == "--export-frontend" => {
            frontend::export(Path::new(root), Path::new(output), &std::env::vars_os().collect())?;
            Ok(0)
        },
        _ => Err("usage: chrono-host-release ROOT ABSENT_OUTPUT [--unit ID | --collect | --export-frontend | --deliver CONFIG | --install CONFIG NATIVE]".into()),
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
