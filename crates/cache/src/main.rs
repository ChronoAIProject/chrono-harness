fn run() -> u8 {
    match chrono_cache::dispatch(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(plan) => {
            print!("{plan}");
            0
        }
        Err(error) => {
            chrono_harness::log_cli_error(&format!("{error}"));
            eprintln!("{error}");
            2
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-cache", run))
}
