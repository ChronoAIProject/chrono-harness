use std::{env, process::ExitCode};

fn main() -> ExitCode {
    if env::args().skip(1).eq(["--version"]) {
        println!("chrono-inputs-transition 0.1.0");
        return ExitCode::SUCCESS;
    }
    eprintln!("historical decoder must never execute");
    ExitCode::FAILURE
}
