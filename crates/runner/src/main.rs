use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(arguments) = arguments
        .iter()
        .map(|argument| argument.to_str())
        .collect::<Option<Vec<_>>>()
    else {
        let index = arguments
            .iter()
            .position(|argument| argument.to_str().is_none())
            .unwrap();
        let error =
            chrono_diagnostics::error::Failure::new("E_USAGE", "arguments must be valid UTF-8.")
                .at("argument_index", index as u64)
                .at(
                    "argument_bytes",
                    serde_json::json!(arguments[index].as_encoded_bytes()),
                );
        let logger = chrono_diagnostics::logging::Logger::stderr("chrono-harness");
        let published = logger.run(|| {
            chrono_diagnostics::logging::failure(
                "cli.invalid_argument",
                &error.to_string(),
                &error.record(),
            )
        });
        let flushed = logger.finish();
        // The failed CLI operation remains failed if its diagnostic cannot be published.
        if published.is_err() || flushed.is_err() {
            return ExitCode::from(2);
        }
        return ExitCode::from(2);
    };
    let output = chrono_harness::dispatch(&arguments);
    match chrono_harness::publish_output(&output, &mut io::stdout().lock(), io::stderr()) {
        Ok(exit_code) => ExitCode::from(exit_code),
        Err(_failure) => ExitCode::from(2),
    }
}
