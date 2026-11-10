fn run(args: &[String]) -> u8 {
    match chrono_ci::dispatch(args) {
        Ok(s) => {
            print!("{s}");
            0
        }
        Err(e) => {
            chrono_harness::log_cli_error(&format!("E_CI: {e}"));
            eprintln!("E_CI: {e}");
            return 1;
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_text_main("chrono-ci", run))
}
