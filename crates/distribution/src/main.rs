fn run() -> u8 {
    match chrono_distribution::run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(value) => {
            println!("{value}");
            0
        }
        Err(error) => {
            eprintln!("E_DISTRIBUTION: {error}");
            (1)
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-distribution", run))
}
