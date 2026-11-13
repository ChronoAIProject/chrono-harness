//! Native fixture for the selected CI judge and its real operation process.
#[path = "../../../runner-tests/tests/support/git_model.rs"]
mod git_model;
use std::io::{Read, Write};
use std::process::ExitCode;

fn monotonic_ns() -> u64 {
    // Use the runner deadline's cross-process clock; Instant has no public epoch.
    #[cfg(target_vendor = "apple")]
    let clock = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_vendor = "apple"))]
    let clock = libc::CLOCK_MONOTONIC;
    let mut now: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::clock_gettime(clock, &mut now) }, 0);
    u64::try_from(now.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(now.tv_nsec).unwrap())
        .unwrap()
}

fn fixture_operation(path: &str) -> ExitCode {
    let input = std::fs::read_to_string(path).unwrap();
    let case = input.trim_end_matches('\n');
    let state = std::path::Path::new(".chrono-harness/state");
    match case {
        "called" | "sleep-2-done" | "zeros-128-done" | "large-invalid-output" => {
            std::fs::create_dir_all(state).unwrap();
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(state.join("calls"))
                .unwrap()
                .write_all(b"called\n")
                .unwrap();
            match case {
                "sleep-2-done" => std::thread::sleep(std::time::Duration::from_secs(2)),
                "zeros-128-done" => std::io::stdout().write_all(&[b'0'; 128]).unwrap(),
                "large-invalid-output" => {
                    std::io::stdout().write_all(&vec![b'0'; 80_000]).unwrap();
                    std::io::stdout().write_all(&[255, 0]).unwrap();
                    std::io::stdout().write_all("λ\n".as_bytes()).unwrap();
                    std::io::stderr()
                        .write_all(b"original diagnostic\n")
                        .unwrap();
                }
                "called" => {}
                _ => unreachable!(),
            }
            if matches!(case, "sleep-2-done" | "zeros-128-done") {
                std::fs::write(state.join("done"), b"done").unwrap();
            }
        }
        "other" => {
            std::fs::create_dir_all(state).unwrap();
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(state.join("other"))
                .unwrap()
                .write_all(b"other")
                .unwrap();
        }
        "failed-other" => {
            std::fs::create_dir_all(state).unwrap();
            std::fs::write(state.join("other"), b"failed").unwrap();
            return ExitCode::from(19);
        }
        "extra" => {
            std::fs::create_dir_all(state).unwrap();
            std::fs::write(state.join("extra"), b"extra").unwrap();
        }
        "exit-19" => return ExitCode::from(19),
        "dirty-input" => std::fs::write("src.txt", b"dirty").unwrap(),
        "changed-input" => std::fs::write("src.txt", b"changed").unwrap(),
        "require-good-tab-input" => {
            if std::fs::read_to_string("source with\ttab.txt")
                .unwrap()
                .trim_end_matches('\n')
                != "good"
            {
                return ExitCode::from(7);
            }
        }
        "index-assume-unchanged" | "index-skip-worktree" => {
            let flag = format!("--{}", case.strip_prefix("index-").unwrap());
            git_model::run(
                std::env::current_dir().unwrap(),
                &["update-index".into(), flag, "src.txt".into()],
                &[],
            )
            .unwrap();
            std::fs::write("src.txt", b"hidden").unwrap();
        }
        #[cfg(unix)]
        "executable-input" => {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata("src.txt").unwrap().permissions();
            permissions.set_mode(permissions.mode() | 0o111);
            std::fs::set_permissions("src.txt", permissions).unwrap();
        }
        _ => panic!("unknown test stimulus: {case}"),
    }
    ExitCode::SUCCESS
}

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
    if args.first().map(String::as_str) == Some("fixture-operation") {
        return fixture_operation(&args[1]);
    }
    if args == ["production"] {
        print!("fixture-production");
        return ExitCode::SUCCESS;
    }
    if args == ["fail-seven"] {
        return ExitCode::from(7);
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
    if args == ["judge-debug"] {
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).unwrap();
        let request: chrono_harness::Request = chrono_harness::decode(&input).unwrap();
        let log = std::fs::File::create(
            request
                .host_root
                .join(".chrono-harness/state/diagnostics.jsonl"),
        )
        .unwrap();
        let logger = chrono_diagnostics::logging::Logger::new(
            "ci-diagnostics-fixture",
            chrono_diagnostics::logging::Level::DEBUG,
            log,
        );
        let response = logger.run(|| chrono_judge_ci::judge(&request));
        logger.finish().unwrap();
        serde_json::to_writer(std::io::stdout(), &response).unwrap();
        return ExitCode::from(if response.status == chrono_harness::Status::Passed {
            0
        } else {
            1
        });
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
            .write_all(format!("ready:{}:{}\n", std::process::id(), monotonic_ns()).as_bytes())
            .unwrap();
        socket.read_exact(&mut [0]).unwrap();
    }
    ExitCode::from(7)
}
