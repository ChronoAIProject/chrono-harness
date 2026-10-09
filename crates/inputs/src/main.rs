fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match chrono_inputs::run(&std::env::current_dir().unwrap(), &args) {
        Ok(text) => {
            print!("{text}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::from(2)
        }
    }
}
