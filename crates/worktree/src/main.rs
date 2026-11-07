fn run() -> u8 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let output = chrono_worktree::run(&args);
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    output.exit_code
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-worktree", run))
}
