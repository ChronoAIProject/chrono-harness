//! Declared construction entrypoint adapter; all fixture assertions are Rust.
use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Command, ExitCode},
};
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("owner probe");
        return ExitCode::SUCCESS;
    }
    if args.first().map(String::as_str) == Some("--hold-descendants") {
        let depth: u32 = args[1].parse().unwrap();
        if depth > 0 {
            let mut child = Command::new(std::env::current_exe().unwrap());
            child.args(["--hold-descendants", &(depth - 1).to_string()]);
            chrono_harness::process_fds::forward_command(&mut child, &[]).unwrap();
            return ExitCode::from(
                u8::try_from(child.status().unwrap().code().unwrap_or(1)).unwrap(),
            );
        }
    }
    if args == ["--hold"] || args.first().map(String::as_str) == Some("--hold-descendants") {
        println!("ready");
        io::stdout().flush().unwrap();
        let mut byte = [0];
        io::stdin().read_exact(&mut byte).unwrap();
        return ExitCode::SUCCESS;
    }
    let root = Path::new(&args[1]);
    if root
        .join(".chrono-harness/state/owner-probe-unavailable")
        .exists()
    {
        eprintln!("construction entrypoint unavailable");
        return ExitCode::from(42);
    }
    // The registered producer is a pure exec adapter; bind its exact bytes
    // and exercise the real Rust producer with the original descriptors.
    assert_eq!(
        std::fs::read(root.join(&args[0])).unwrap(),
        include_bytes!("../../../../.chrono-harness/ci/native-tests/tests/support/producer.sh")
    );
    let mut child = Command::new(
        std::env::var_os("CHRONO_TEST_BOOTSTRAP_PRODUCER").expect("explicit Rust producer"),
    );
    child.args(&args[1..]);
    chrono_harness::process_fds::forward_command(&mut child, &[]).unwrap();
    let status = child.status().unwrap();
    ExitCode::from(u8::try_from(status.code().unwrap_or(1)).unwrap_or(1))
}
