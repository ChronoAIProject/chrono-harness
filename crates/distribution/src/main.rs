fn run(args: &[String]) -> u8 {
    match chrono_distribution::run(args) {
        Ok(value) => {
            println!("{value}");
            0
        }
        Err(error) => {
            chrono_harness::log_cli_error(&format!("E_DISTRIBUTION: {error}"));
            eprintln!("E_DISTRIBUTION: {error}");
            1
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_text_main("chrono-distribution", run))
}
