//! Separate process boundary for the real interrupted-execution regression.
use std::{fs, process::ExitCode};

fn main() -> ExitCode {
    let run = || -> Result<(), String> {
        let path = std::env::args().nth(1).ok_or("execution plan path missing")?;
        let plan = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let results = chrono_judge_projects::execute(&plan)?;
        serde_json::to_writer(std::io::stdout().lock(), &results).map_err(|e| e.to_string())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
