use std::io::{Read, Write};
fn main() {
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
            std::process::exit(2)
        }
    };
    let response = chrono_judge_ci::judge(&request);
    let failed = matches!(
        response.status,
        chrono_harness::Status::Failed | chrono_harness::Status::Blocked
    );
    if serde_json::to_writer(std::io::stdout(), &response).is_err()
        || std::io::stdout().write_all(b"\n").is_err()
    {
        std::process::exit(2)
    }
    std::process::exit(if failed { 1 } else { 0 });
}
