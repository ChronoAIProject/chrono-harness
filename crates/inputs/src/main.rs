fn run(args: &[String]) -> u8 {
    match chrono_inputs::run(&std::env::current_dir().unwrap(), args) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(e) => {
            chrono_harness::log_cli_error(&format!("{e}"));
            eprintln!("{e}");
            2
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_text_main("chrono-inputs", run))
}
