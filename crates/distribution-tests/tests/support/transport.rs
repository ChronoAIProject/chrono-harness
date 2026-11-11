//! Rust command-boundary fixtures for the generated launcher, assets and HTTPS.
mod bootstrap;
use std::{env, fs, io::{self, Write}, path::Path, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str)==Some("bootstrap") { process::exit(bootstrap::dispatch(&args[1..])); }
    if args.first().map(String::as_str)==Some("asset") { return asset(&args[1..]); }
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
    assert_eq!(args.len(), 17);
    assert_eq!(args[..14], expected);
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


fn asset(args: &[String]) {
    assert_eq!(args.len(),1,"asset fixture accepts only the installed path");
    let bytes=fs::read(&args[0]).unwrap();
    let cases: &[(&[u8], &[u8], i32)] = &[
        (include_bytes!("../fixtures/alpha-v1.sh"), b"alpha-v1\n",0),
        (include_bytes!("../fixtures/alpha-v2.sh"), b"alpha-v2\n",0),
        (include_bytes!("../fixtures/beta-v1.sh"), b"beta-v1\n",0),
        (include_bytes!("../fixtures/beta-v2.sh"), b"beta-v2\n",0),
        (include_bytes!("../fixtures/installer-ok.sh"), b"",0),
        (include_bytes!("../fixtures/installer-exit-23.sh"), b"",23),
    ];
    for (source,stdout,code) in cases {if bytes==*source {io::stdout().write_all(stdout).unwrap();io::stdout().flush().unwrap();process::exit(*code);}}
    eprintln!("unknown distribution asset fixture bytes");process::exit(93);
}
