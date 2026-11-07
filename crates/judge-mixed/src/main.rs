use std::io::{Read, Write};
fn run() -> u8 {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("chrono-judge-mixed {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    if args != ["--protocol", "chrono-judge/v1"] {
        chrono_harness::log_cli_error(&format!("E_USAGE: --protocol chrono-judge/v1"));
        eprintln!("E_USAGE: --protocol chrono-judge/v1");
        return 2;
    }
    let mut bytes = vec![];
    if std::io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 64 * 1024 * 1024
    {
        chrono_harness::log_cli_error(&format!("E_PROTOCOL: request read/bound"));
        eprintln!("E_PROTOCOL: request read/bound");
        return 2;
    }
    let req: chrono_harness::wire::Request = match chrono_harness::decode(&bytes) {
        Ok(r) => r,
        Err(e) => {
            chrono_harness::log_cli_error(&format!("E_PROTOCOL: {e}"));
            eprintln!("E_PROTOCOL: {e}");
            return 2;
        }
    };
    let response = chrono_judge_mixed::judge(&req);
    match chrono_harness::wire::canonical(&response) {
        Ok(bytes) => {
            if std::io::stdout().write_all(&bytes).is_err() {
                return 2;
            }
        }
        Err(e) => {
            chrono_harness::log_cli_error(&format!("E_PROTOCOL: {e}"));
            eprintln!("E_PROTOCOL: {e}");
            return 2;
        }
    }
    response.status.exit_code() as u8
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-judge-mixed", run))
}
