use std::io::{Read, Write};
fn run(args: &[String]) -> u8 {
    if args == ["--version"] {
        println!("chrono-judge-registration {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let initial = args == ["--protocol", chrono_harness::initial::PROTOCOL];
    if !initial && args != ["--protocol", "chrono-judge/v1"] {
        chrono_harness::log_cli_error(&format!(
            "E_USAGE: --protocol chrono-judge/v1 or chrono-initial-judge/v1"
        ));
        eprintln!("E_USAGE: --protocol chrono-judge/v1 or chrono-initial-judge/v1");
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
    let decoded = if initial {
        chrono_harness::decode(&bytes).map(|r| chrono_judge_registration::initial::judge(&r))
    } else {
        chrono_harness::decode(&bytes).map(|r| chrono_judge_registration::judge(&r))
    };
    let response = match decoded {
        Ok(r) => r,
        Err(e) => {
            chrono_harness::log_cli_error(&format!("E_PROTOCOL: {e}"));
            eprintln!("E_PROTOCOL: {e}");
            return 2;
        }
    };
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
    std::process::ExitCode::from(chrono_harness::logged_text_main(
        "chrono-judge-registration",
        run,
    ))
}
