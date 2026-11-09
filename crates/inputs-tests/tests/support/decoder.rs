#[path = "../fixtures/transition.rs"]
mod transition;
#[path = "../fixtures/transition_checks.rs"]
mod transition_checks;

use serde_json::Value;
use std::{env, fs, io, process::ExitCode};

fn run(mode: &str) -> Result<(), String> {
    match mode {
        "--version" => println!("chrono-inputs-transition 0.1.0"),
        "direct-check" => transition_checks::direct(),
        "selected-check" => {
            let input = fs::read(".chrono-harness/state/decoder-input.json")
                .map_err(|error| error.to_string())?;
            let request = serde_json::from_slice(&input).map_err(|error| error.to_string())?;
            transition_checks::selected(&request);
        }
        "compatibility-without-conversion" => {
            println!("compatibility command passed without conversion");
        }
        "direct-compatibility-fails" => return Err("incompatible migration".into()),
        "selected-compatibility-fails" => return Err("selected compatibility failed".into()),
        "direct" | "selected" | "rewrite-direct" | "rewrite-selected" => {
            let request: Value =
                serde_json::from_reader(io::stdin().lock()).map_err(|error| error.to_string())?;
            let selected = matches!(mode, "selected" | "rewrite-selected");
            let mut result = if selected {
                fs::write(
                    ".chrono-harness/state/decoder-input.json",
                    serde_json::to_vec(&request).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                transition::convert_selected(&request)?
            } else {
                transition::convert_direct(&request)?
            };
            if mode == "rewrite-direct" {
                let path = request["config_path"]
                    .as_str()
                    .ok_or("missing config path")?;
                result["values"][path] = request["candidate"][path].clone();
            } else if mode == "rewrite-selected" {
                let path = request["config_bindings"]["base"]["effective_path"]
                    .as_str()
                    .ok_or("missing effective path")?;
                result["values"][path]["environment"]["values"]["EMPTY"] = "rewritten".into();
            }
            serde_json::to_writer(io::stdout().lock(), &result)
                .map_err(|error| error.to_string())?;
        }
        _ => return Err(format!("unsupported decoder fixture mode: {mode}")),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    let result = match args.as_slice() {
        [mode] => run(mode),
        _ => Err("expected one decoder fixture mode".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("E_MIGRATION: {error}");
            ExitCode::FAILURE
        }
    }
}
