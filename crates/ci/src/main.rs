fn main() {
    match chrono_ci::dispatch(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(s) => print!("{s}"),
        Err(e) => {
            eprintln!("E_CI: {e}");
            std::process::exit(1)
        }
    }
}
