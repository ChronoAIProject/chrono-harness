use std::io::{Read, Write};
fn run() -> u8 {
    let mut bytes = Vec::new();
    let result = std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            if bytes.len() > 1024 * 1024 {
                Err("request too large".into())
            } else {
                chrono_harness::decode::<chrono_harness::Request>(&bytes)
            }
        });
    let request = match result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("E_REQUEST: {e}");
            return 2;
        }
    };
    let response = chrono_judge_ci::judge(&request);
    let failed = matches!(
        response.status,
        chrono_harness::Status::Failed | chrono_harness::Status::Blocked
    );
    let stdout = std::io::stdout();
    let mut output = std::io::BufWriter::new(stdout.lock());
    if serde_json::to_writer(&mut output, &response).is_err()
        || output.write_all(b"\n").is_err()
        || output.flush().is_err()
    {
        return 2;
    }
    if failed { 1 } else { 0 }
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-judge-ci", run))
}
