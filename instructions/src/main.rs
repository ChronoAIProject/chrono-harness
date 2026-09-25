use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let result = chrono_instructions::dispatch(&std::env::args_os().skip(1).collect::<Vec<_>>());
    if io::stdout().write_all(result.stdout.as_bytes()).is_err()
        || io::stderr().write_all(result.stderr.as_bytes()).is_err()
    {
        return ExitCode::from(1);
    }
    ExitCode::from(result.exit_code)
}
