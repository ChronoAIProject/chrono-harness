use std::io::{Read, Write};
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("chrono-judge-registration {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    let initial = args == ["--protocol", chrono_harness::initial::PROTOCOL];
    if !initial && args != ["--protocol", "chrono-judge/v1"] {
        eprintln!("E_USAGE: --protocol chrono-judge/v1 or chrono-initial-judge/v1");
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
    let decoded = if initial {
        chrono_harness::decode(&bytes).map(|r| chrono_judge_registration::initial::judge(&r))
    } else {
        chrono_harness::decode(&bytes).map(|r| chrono_judge_registration::judge(&r))
    };
    let response = match decoded {
        Ok(r) => r,
        Err(e) => {
            eprintln!("E_PROTOCOL: {e}");
            return std::process::ExitCode::from(2);
        }
    };
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
