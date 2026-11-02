//! Native fixture for the selected CI judge and its real operation process.
use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("chrono-test-ci-diagnostics/v1");
        return ExitCode::SUCCESS;
    }
    if args == ["judge"] {
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).unwrap();
        let request = chrono_harness::decode(&input).unwrap();
        let response = chrono_judge_ci::judge(&request);
        serde_json::to_writer(std::io::stdout(), &response).unwrap();
        return ExitCode::from(if response.status == chrono_harness::Status::Passed {
            0
        } else {
            1
        });
    }
    assert_eq!(args.first().map(String::as_str), Some("operation"));
    std::io::stdout().write_all(&[255, 0, 10]).unwrap();
    std::io::stderr().write_all(&[254, 0, 10]).unwrap();
    if args.get(1).map(String::as_str) == Some("success") {
        return ExitCode::SUCCESS;
    }
    if let Some(address) = args.get(1) {
        let mut socket = std::net::TcpStream::connect(address).unwrap();
        socket.write_all(b"ready").unwrap();
        socket.read_exact(&mut [0]).unwrap();
    }
    ExitCode::from(7)
}
