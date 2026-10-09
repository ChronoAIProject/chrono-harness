fn main() -> std::process::ExitCode {
    match chrono_distribution::run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(value) => {
            println!("{value}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("E_DISTRIBUTION: {error}");
            std::process::ExitCode::from(1)
        }
    }
}
