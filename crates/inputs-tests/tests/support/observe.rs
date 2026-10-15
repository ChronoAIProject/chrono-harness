use std::io::Write;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "path" => println!("{}", args[1]),
        "fail" => {
            eprintln!("actual failure");
            std::process::exit(17)
        }
        "timeout" => {
            eprintln!("before timeout");
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        "overflow" => {
            let _ = std::io::stdout().write_all(&[b'x'; 4096]);
        }
        _ => panic!("unknown observation fixture"),
    }
}
