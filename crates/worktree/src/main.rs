fn run(args: &[String]) -> u8 {
    let output = chrono_worktree::run(args);
    if !output.stderr.is_empty() {
        chrono_harness::log_cli_error(&output.stderr);
    }
    match chrono_harness::publish_native_output(&output) {
        Ok(exit_code) => exit_code,
        Err(_failure) => 2,
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_text_main("chrono-worktree", run))
}
