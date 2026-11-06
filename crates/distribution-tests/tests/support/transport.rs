//! A native fixture for the real shell/Rust HTTPS command boundary.
use std::{env, fs, path::Path, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let expected = [
        "--disable",
        "--fail",
        "--location",
        "--silent",
        "--show-error",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "30",
        "--max-time",
        "300",
        "--output",
    ];
    assert_eq!(args[..14], expected);
    assert_eq!(args.len(), 17);
    assert_eq!(args[15], "--url");
    let member = args[16]
        .strip_prefix("https://release.example.invalid/v1/")
        .unwrap();
    assert!(!member.contains('/'));
    if let Ok(code) = env::var("CHRONO_FIXTURE_CURL_FAILURE") {
        process::exit(code.parse().unwrap());
    }
    let source = env::var_os("CHRONO_FIXTURE_RELEASE").unwrap();
    fs::copy(Path::new(&source).join(member), &args[14]).unwrap();
}
