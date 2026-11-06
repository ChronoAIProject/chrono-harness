//! Native fixture for the selected CI judge and its real operation process.
use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        if let Ok(address) = std::env::var("CHRONO_TEST_ADMISSION") {
            let mut socket = std::net::TcpStream::connect(address).unwrap();
            socket.write_all(b"admission").unwrap();
            socket.read_exact(&mut [0]).unwrap();
        }
        println!("chrono-test-ci-diagnostics/v1");
        return ExitCode::SUCCESS;
    }
    #[cfg(unix)]
    if args.first().map(String::as_str) == Some("infrastructure-operation") {
        use std::os::unix::process::ExitStatusExt;
        let state = std::path::Path::new(".chrono-harness/state");
        std::fs::create_dir_all(state).unwrap();
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(state.join("calls"))
            .unwrap()
            .write_all(b"called\n")
            .unwrap();
        let original = std::fs::File::create(state.join("infra-original")).unwrap();
        let status = std::process::Command::new(&args[1])
            .args([
                "--exact",
                "units::exit_zero_infrastructure_launcher_helper",
                "--nocapture",
            ])
            .env("CHRONO_CI_INFRASTRUCTURE", std::env::current_dir().unwrap())
            .stdout(original.try_clone().unwrap())
            .stderr(original)
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(9));
        return ExitCode::SUCCESS;
    }
    #[cfg(unix)]
    if args.first().map(String::as_str) == Some("interrupt-launcher") {
        unsafe extern "C" {
            fn getppid() -> std::ffi::c_int;
            fn kill(pid: std::ffi::c_int, signal: std::ffi::c_int) -> std::ffi::c_int;
        }
        let parent: std::ffi::c_int = args[1].parse().unwrap();
        assert!(parent > 1);
        assert_eq!(unsafe { getppid() }, parent);
        std::fs::write(
            ".chrono-harness/state/infra-ready",
            std::process::id().to_string(),
        )
        .unwrap();
        assert_eq!(unsafe { kill(parent, 9) }, 0);
        std::thread::sleep(std::time::Duration::from_secs(30));
        return ExitCode::SUCCESS;
    }
    if args == ["execute-debug"] {
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).unwrap();
        let prepared: chrono_judge_ci::PreparedExecution = chrono_harness::decode(&input).unwrap();
        let logger = chrono_diagnostics::logging::Logger::new(
            "ci-diagnostics-fixture",
            chrono_diagnostics::logging::Level::DEBUG,
            std::io::stderr(),
        );
        let response = logger.run(|| prepared.execute().unwrap());
        logger.finish().unwrap();
        serde_json::to_writer(std::io::stdout(), &response).unwrap();
        return ExitCode::from(if response.passed() { 0 } else { 1 });
    }
    assert_eq!(args.first().map(String::as_str), Some("operation"));
    std::io::stdout().write_all(&[255, 0, 10]).unwrap();
    std::io::stderr().write_all(&[254, 0, 10]).unwrap();
    if args.get(1).map(String::as_str) == Some("success") {
        return ExitCode::SUCCESS;
    }
    if let Some(address) = args.get(1) {
        let mut socket = std::net::TcpStream::connect(address).unwrap();
        socket
            .write_all(format!("ready:{}\n", std::process::id()).as_bytes())
            .unwrap();
        socket.read_exact(&mut [0]).unwrap();
    }
    ExitCode::from(7)
}
