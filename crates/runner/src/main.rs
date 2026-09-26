use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(arguments) = arguments
        .iter()
        .map(|argument| argument.to_str())
        .collect::<Option<Vec<_>>>()
    else {
        let _ = writeln!(io::stderr(), "E_USAGE: arguments must be valid UTF-8.");
        return ExitCode::from(2);
    };
    let output = chrono_harness::dispatch(&arguments);
    if io::stdout().write_all(output.stdout.as_bytes()).is_err()
        || io::stderr().write_all(output.stderr.as_bytes()).is_err()
    {
        return ExitCode::from(2);
    }
    ExitCode::from(output.exit_code)
}
