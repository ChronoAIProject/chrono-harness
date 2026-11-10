fn run(args: &[String]) -> u8 {
    let output = chrono_worktree::run(args);
    if !output.stderr.is_empty() {
        chrono_harness::log_cli_error(&output.stderr);
    }
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    output.exit_code
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_text_main("chrono-worktree", run))
}
