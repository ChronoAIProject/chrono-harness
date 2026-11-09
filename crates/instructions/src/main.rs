use std::io::{self, Write};
use std::process::ExitCode;

fn run() -> u8 {
    let result = chrono_instructions::dispatch(&std::env::args_os().skip(1).collect::<Vec<_>>());
    if !result.stderr.is_empty() {
        chrono_diagnostics::runtime::diagnostic(result.stderr.trim_end());
    }
    if io::stdout().write_all(result.stdout.as_bytes()).is_err()
        || io::stderr().write_all(result.stderr.as_bytes()).is_err()
    {
        return 1;
    }
    result.exit_code
}

fn main() -> ExitCode {
    ExitCode::from(chrono_diagnostics::runtime::cli("chrono-instructions", run))
}
