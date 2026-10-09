use std::io::Write;
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "echo" | "file" => println!("{}", args[1]),
        "github" => {
            std::fs::write(
                "backend-call.txt",
                format!(
                    "{}\ncredential={}",
                    args[1],
                    std::env::var_os("CACHE_TEST_SECRET").is_some()
                ),
            )
            .unwrap();
            std::io::stdout()
                .write_all(&std::fs::read(&args[2]).unwrap())
                .unwrap();
        }
        "exit" => {
            eprintln!("original probe failure");
            std::process::exit(7);
        }
        "sleep" => std::thread::sleep(std::time::Duration::from_secs(4)),
        "flood" => std::io::stdout().write_all(&vec![b'x'; 4096]).unwrap(),
        _ => std::process::exit(99),
    }
}
