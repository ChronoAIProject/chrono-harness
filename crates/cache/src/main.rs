fn main() -> std::process::ExitCode {
    match chrono_cache::dispatch(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(plan) => {
            print!("{plan}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(2)
        }
    }
}
