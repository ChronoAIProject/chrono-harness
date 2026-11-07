fn run() -> u8 {
    match chrono_ci::dispatch(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(s) => { print!("{s}"); 0 },
        Err(e) => {
            eprintln!("E_CI: {e}");
            return 1
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-ci", run))
}
