fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-pr", || {
        let args = std::env::args_os().skip(1).collect::<Vec<_>>();
        let output = chrono_pr::run(&args);
        if !output.stderr.is_empty() {
            chrono_harness::log_cli_error(&output.stderr);
        }
        match chrono_harness::publish_native_output(&output) {
            Ok(exit_code) => exit_code,
            Err(_failure) => 2,
        }
    }))
}
