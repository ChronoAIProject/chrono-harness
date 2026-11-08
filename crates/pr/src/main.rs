fn main() -> std::process::ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let output = chrono_pr::run(&args);
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    std::process::ExitCode::from(output.exit_code)
}
