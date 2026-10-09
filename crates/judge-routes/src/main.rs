use std::io::{Read, Write};
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("chrono-judge-routes {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    if args != ["--protocol", "chrono-judge/v1"] {
        eprintln!("E_USAGE: --protocol chrono-judge/v1");
        return std::process::ExitCode::from(2);
    }
    let mut bytes = vec![];
    if std::io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 64 * 1024 * 1024
    {
        eprintln!("E_PROTOCOL: request read/bound");
        return std::process::ExitCode::from(2);
    }
    let req: chrono_harness::wire::Request = match chrono_harness::decode(&bytes) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("E_PROTOCOL: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let response = chrono_judge_routes::judge(&req);
    match chrono_harness::wire::canonical(&response) {
        Ok(bytes) => {
            if std::io::stdout().write_all(&bytes).is_err() {
                return std::process::ExitCode::from(2);
            }
        }
        Err(e) => {
            eprintln!("E_PROTOCOL: {e}");
            return std::process::ExitCode::from(2);
        }
    }
    std::process::ExitCode::from(response.status.exit_code() as u8)
}
