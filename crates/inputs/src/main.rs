fn run() -> u8 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match chrono_inputs::run(&std::env::current_dir().unwrap(), &args) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            (2)
        }
    }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-inputs", run))
}
