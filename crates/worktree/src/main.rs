fn run() -> u8 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let output = chrono_worktree::run(&args);
    if !output.stderr.is_empty() {
        chrono_harness::log_cli_error(&output.stderr);
    }
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    output.exit_code
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-worktree", run))
}
