use std::io::Write;
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let args = if args.first().map(String::as_str) == Some("--no-optional-locks") {
        assert_eq!(
            &args[..3],
            ["--no-optional-locks", "--no-replace-objects", "-C"]
        );
        assert_eq!(
            std::fs::canonicalize(&args[3]).unwrap(),
            std::env::current_dir().unwrap()
        );
        args[4..].to_vec()
    } else {
        args
    };
    match args[0].as_str() {
        "--version" => {
            assert_eq!(args.len(), 1);
            println!("cache-test git mock");
        }
        "--literal-pathspecs" => {
            assert!(
                args.last()
                    .unwrap()
                    .starts_with(".chrono-harness/state/cache-probes/")
                    || args.last().unwrap() == ".chrono-harness/state/restore/plan.json"
            );
            let prefix: Vec<_> = args[..args.len() - 1].iter().map(String::as_str).collect();
            assert!(
                prefix == ["--literal-pathspecs", "ls-files", "-z", "--"]
                    || prefix == ["--literal-pathspecs", "ls-tree", "-rz", "HEAD", "--"]
            );
        }

        "cargo" => {
            assert_eq!(
                &args[1..],
                [
                    "test",
                    "--locked",
                    "--manifest-path",
                    "crates/cache-tests/Cargo.toml"
                ]
            );
            assert_eq!(std::env::var("CARGO_INCREMENTAL").unwrap(), "1");
            let project = std::path::Path::new("crates/cache-tests");
            let target = project.join("target/debug");
            std::fs::create_dir_all(target.join("incremental")).unwrap();
            let source = std::fs::read_to_string(project.join("src/main.rs")).unwrap();
            let output = source
                .split("println!(\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            std::fs::write(target.join("mock-output"), format!("{output}\n")).unwrap();
            println!("mock producer complete");
        }
        #[cfg(unix)]
        "live" => {
            use std::io::Read;
            std::io::stdout().write_all(b"live stdout").unwrap();
            std::io::stdout().flush().unwrap();
            std::io::stderr().write_all(b"live stderr").unwrap();
            let mut socket = std::os::unix::net::UnixStream::connect(&args[1]).unwrap();
            socket.write_all(b"ready").unwrap();
            let mut release = [0];
            socket.read_exact(&mut release).unwrap();
        }
        "count" => {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&args[1])
                .unwrap()
                .write_all(b"x")
                .unwrap();
            println!("stable compiler");
        }
        "receipt-conflict" => {
            std::fs::write(&args[1], b"fixture publication conflict").unwrap();
            println!("original observed output");
        }
        "echo" | "file" => println!("{}", args[1]),
        "exit" => {
            if let Some(count) = args.get(1) {
                std::fs::OpenOptions::new().create(true).append(true).open(count)
                    .unwrap().write_all(b"x").unwrap();
            }
            eprintln!("original probe failure");
            std::process::exit(7);
        }
        "sleep" => std::thread::sleep(std::time::Duration::from_secs(4)),
        "flood" => std::io::stdout().write_all(&vec![b'x'; 4096]).unwrap(),
        #[cfg(unix)]
        "socket" => {
            let _listener = std::os::unix::net::UnixListener::bind(&args[1]).unwrap();
        }
        _ => std::process::exit(99),
    }
}
