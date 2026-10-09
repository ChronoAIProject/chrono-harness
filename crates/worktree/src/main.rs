fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let output = chrono_worktree::run(&args);
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    std::process::ExitCode::from(output.exit_code)
}
