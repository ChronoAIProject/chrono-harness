use std::io::{self, Read, Write};
use std::time::Duration;

fn main() {
    let mode = std::env::args().nth(1).expect("transport mode");
    let mut request = Vec::new();
    io::stdin().lock().read_to_end(&mut request).unwrap();
    let mut stdout = io::stdout().lock();
    match mode.as_str() {
        "timeout-partial" => {
            stdout.write_all(b"{\n").unwrap();
            stdout.flush().unwrap();
            std::thread::sleep(Duration::from_secs(8));
        }
        "output-limit" => {
            stdout.write_all(&[b'x'; 8192]).unwrap();
            stdout.write_all(b"\n").unwrap();
            stdout.flush().unwrap();
        }
        _ => panic!("unknown transport mode: {mode}"),
    }
}
