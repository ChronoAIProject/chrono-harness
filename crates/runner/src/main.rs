fn run(arguments: &[String]) -> u8 {
    let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    let output = chrono_harness::dispatch(&arguments);
    if !output.stderr.is_empty() {
        chrono_harness::log_cli_error(&output.stderr);
    }
    match chrono_harness::publish_native_output(&output) {
        Ok(exit_code) => exit_code,
        Err(_failure) => 2,
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_text_main("chrono-harness", run))
}
