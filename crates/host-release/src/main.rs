#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_identifiers as identifiers;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn select_preview_command(flag: &str) -> (selected: bool)
    ensures selected == (flag@ == "--select-preview"@),
{
    identifiers::equal_text(flag, "--select-preview")
}
}
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::{delivery, execution, frontend, local, recipe::Scope};
#[cfg(not(verus_keep_ghost))]
use std::{path::Path, process::ExitCode};

#[cfg(not(verus_keep_ghost))]
fn run() -> Result<i32, String> {
    let args = std::env::args_os()
        .skip(1)
        .map(|s| {
            s.into_string()
                .map_err(|_| "non UTF-8 release argument".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    match args.as_slice() {
        [root, output, flag, config, collection] if select_preview_command(flag) => delivery::select_preview(
            Path::new(root), Path::new(output), config, collection, &std::env::vars_os().collect(),
        ),
        [root, output, flag, config, request] if flag == "--plan-preview" => {
            chrono_host_release::recipe::preview::write_plan(
                Path::new(root), Path::new(output), config, request, &std::env::vars_os().collect()
            )?;
            Ok(0)
        },
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
        _ => Err("usage: chrono-host-release ROOT ABSENT_OUTPUT [--unit ID | --collect | --export-frontend | --deliver CONFIG | --install CONFIG NATIVE | --plan-preview CONFIG REQUEST | --select-preview CONFIG COLLECTION]".into()),
    }
}

#[cfg(not(verus_keep_ghost))]
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
