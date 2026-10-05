mod provider;

use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;

fn main() {
    let executable = std::env::args_os().next().unwrap();
    match Path::new(&executable).file_name().unwrap().as_bytes() {
        b"mock-gh" => return provider::units(&std::env::args().skip(1).collect::<Vec<_>>()),
        b"parent-gh" => return provider::parent(&std::env::args().skip(1).collect::<Vec<_>>()),
        b"$CHRONO_BASE" => {
            let mut stdout = io::stdout().lock();
            for arg in std::env::args_os().skip(1) {
                stdout.write_all(arg.as_bytes()).unwrap();
                stdout.write_all(b"\n").unwrap();
            }
            stdout.flush().unwrap();
            let failure: &[u8] = match std::env::var("CHRONO_TEST_RECORDING").as_deref() {
                Ok("full") => b"original-failure\n",
                Ok("scope") => b"\xffscope-consumer-failure",
                _ => panic!("missing recording fixture mode"),
            };
            io::stderr().write_all(failure).unwrap();
            std::process::exit(23);
        }
        _ => {}
    }
    let mode = std::env::args().nth(1).expect("transport mode");
    io::copy(&mut io::stdin().lock(), &mut io::sink()).unwrap();
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
