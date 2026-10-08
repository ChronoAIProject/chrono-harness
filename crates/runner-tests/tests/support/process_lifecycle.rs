use std::{
    fs::{self, File},
    io::Write,
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn now() -> f64 {
    #[cfg(target_vendor = "apple")]
    let clock = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_vendor = "apple"))]
    let clock = libc::CLOCK_MONOTONIC;
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::clock_gettime(clock, &mut value) }, 0);
    value.tv_sec as f64 + value.tv_nsec as f64 / 1e9
}

fn publish(path: &str, contents: &str) {
    let temporary = format!("{path}.tmp");
    fs::write(&temporary, contents).unwrap();
    fs::rename(temporary, path).unwrap();
}

fn await_file(path: &str, seconds: f64, interval_ms: u64) {
    let start = now();
    while !Path::new(path).exists() {
        assert!(
            now() - start < seconds,
            "{path} was not published in {seconds} seconds"
        );
        thread::sleep(Duration::from_millis(interval_ms));
    }
}

fn child() {
    publish("ready", &std::process::id().to_string());
    await_file("release", 5.0, 5);
    let release = fs::read_to_string("release").unwrap();
    let (delay, overflow) = release.split_once(',').unwrap();
    thread::sleep(Duration::from_secs_f64(delay.parse().unwrap()));
    if overflow == "yes" {
        std::io::stdout().write_all(&[b'x'; 8192]).unwrap();
    }
    unsafe {
        drop(File::from_raw_fd(1));
        drop(File::from_raw_fd(2));
        libc::_exit(7);
    }
}

struct Resume(libc::pid_t);
impl Drop for Resume {
    fn drop(&mut self) {
        assert_eq!(unsafe { libc::kill(self.0, libc::SIGCONT) }, 0);
    }
}

// This observer is a separate process from the suspended launcher and uses
// kernel exit events directly, without the production runner's observer.
struct Exit(File);
impl Exit {
    fn register(pid: libc::pid_t) -> Self {
        #[cfg(target_os = "macos")]
        {
            let descriptor = unsafe { libc::kqueue() };
            assert!(descriptor >= 0, "{}", std::io::Error::last_os_error());
            let file = unsafe { File::from_raw_fd(descriptor) };
            let event = libc::kevent {
                ident: pid as _,
                filter: libc::EVFILT_PROC,
                flags: libc::EV_ADD | libc::EV_ONESHOT,
                fflags: libc::NOTE_EXIT,
                data: 0,
                udata: std::ptr::null_mut(),
            };
            assert_eq!(
                unsafe {
                    libc::kevent(
                        descriptor,
                        &event,
                        1,
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null(),
                    )
                },
                0,
                "{}",
                std::io::Error::last_os_error()
            );
            Self(file)
        }
        #[cfg(target_os = "linux")]
        {
            let descriptor = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
            assert!(descriptor >= 0, "{}", std::io::Error::last_os_error());
            Self(unsafe { File::from_raw_fd(descriptor) })
        }
    }

    fn wait(self, pid: libc::pid_t, seconds: u32) {
        #[cfg(target_os = "macos")]
        {
            let timeout = libc::timespec {
                tv_sec: seconds.into(),
                tv_nsec: 0,
            };
            let mut event: libc::kevent = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe {
                    libc::kevent(
                        self.0.as_raw_fd(),
                        std::ptr::null(),
                        0,
                        &mut event,
                        1,
                        &timeout,
                    )
                },
                1,
                "missing kernel exit: {}",
                std::io::Error::last_os_error()
            );
            let observed_pid = event.ident;
            assert_eq!(observed_pid, pid as libc::uintptr_t);
            assert_eq!(event.flags & libc::EV_ERROR, 0);
            assert_ne!(event.fflags & libc::NOTE_EXIT, 0);
        }
        #[cfg(target_os = "linux")]
        {
            let _ = pid;
            let mut event = libc::pollfd {
                fd: self.0.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let milliseconds = i32::try_from(seconds * 1000).unwrap();
            assert_eq!(unsafe { libc::poll(&mut event, 1, milliseconds) }, 1);
            assert_ne!(event.revents & libc::POLLIN, 0);
        }
    }
}

fn controller(case: &str) {
    assert!(matches!(case, "on-time" | "late" | "overflow"));
    File::create_new("controller-ready").unwrap();
    await_file("ready", 5.0, 5);
    let launcher: serde_json::Value =
        serde_json::from_slice(&fs::read("launcher").unwrap()).unwrap();
    let started = launcher["started"].as_f64().unwrap();
    let launcher_pid: libc::pid_t = launcher["pid"].as_i64().unwrap().try_into().unwrap();
    let pid = fs::read_to_string("ready").unwrap().parse().unwrap();
    let exit = Exit::register(pid);
    thread::sleep(Duration::from_millis(150));
    let stopped = now();
    assert!(
        (0.0..0.75).contains(&(stopped - started)),
        "fixture missed the original deadline: started={started}, stopped={stopped}"
    );
    assert_eq!(unsafe { libc::kill(launcher_pid, libc::SIGSTOP) }, 0);
    let _resume = Resume(launcher_pid);
    publish(
        "release",
        &format!(
            "{},{}",
            if case == "late" { "1.3" } else { ".05" },
            if case == "overflow" { "yes" } else { "no" }
        ),
    );
    exit.wait(pid, 3);
    let terminal = now();
    if case == "late" {
        assert!(terminal - stopped > 1.0);
    } else {
        assert!(
            (0.0..1.0).contains(&(terminal - started)),
            "child did not exit inside the deadline: started={started}, terminal={terminal}"
        );
    }
    thread::sleep(Duration::from_secs_f64((stopped + 2.1 - now()).max(0.0)));
    fs::write(
        "kernel-terminal.json",
        serde_json::to_vec(&serde_json::json!({
            "pid": pid,
            "started": started,
            "stopped": stopped,
            "terminal": terminal,
            "resumed": now(),
        }))
        .unwrap(),
    )
    .unwrap();
}

fn marker(path: &str) {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut file = File::create_new(path).unwrap();
    write!(file, "{}", time.as_nanos()).unwrap();
}

fn closed_streams(delay: u64) {
    marker("started");
    // Closing the runner's pipes must not signal process exit.
    unsafe {
        drop(File::from_raw_fd(1));
        drop(File::from_raw_fd(2));
    }
    thread::sleep(Duration::from_millis(delay));
    marker("completed");
    std::process::exit(7);
}

fn interrupted_child(complete: &str) {
    assert!(matches!(complete, "yes" | "no"));
    publish("ready", &std::process::id().to_string());
    await_file("observing", 3.0, 10);
    assert_eq!(unsafe { libc::kill(libc::getppid(), libc::SIGKILL) }, 0);
    if complete == "yes" {
        fs::write("child-original", "original child bytes").unwrap();
        fs::write("completed", "child reached exit").unwrap();
    } else {
        thread::sleep(Duration::from_secs(30));
        fs::write("escaped", "escaped").unwrap();
    }
}

fn interrupted_observer() {
    await_file("ready", 5.0, 10);
    let pid = fs::read_to_string("ready").unwrap().parse().unwrap();
    let exit = Exit::register(pid);
    fs::write("observing", pid.to_string()).unwrap();
    exit.wait(pid, 7);
    fs::write("kernel-terminal", pid.to_string()).unwrap();
}

fn terminated(with_streams: bool) {
    fs::write("terminated.pid", std::process::id().to_string()).unwrap();
    if with_streams {
        let mut stdout = std::io::stdout();
        stdout.write_all(b"original stdout\xff").unwrap();
        stdout.flush().unwrap();
        let mut stderr = std::io::stderr();
        stderr.write_all(b"original stderr\xfe").unwrap();
        stderr.flush().unwrap();
    }
    assert_eq!(unsafe { libc::kill(libc::getpid(), libc::SIGKILL) }, 0);
    unreachable!("SIGKILL must terminate the fixture");
}

fn retained(case: &str) {
    fs::write("retained.pid", std::process::id().to_string()).unwrap();
    std::io::stdout()
        .write_all(b"original stdout\xff\n")
        .unwrap();
    std::io::stdout().flush().unwrap();
    std::io::stderr().write_all(b"CHRONO_CARGO_PHASE {\"phase\":\"consumer\",\"event\":\"begin\"}\noriginal stderr\xfe\n").unwrap();
    std::io::stderr().flush().unwrap();
    if case == "pause-launcher" {
        // Suspend only after the engine has actually retained both originals.
        let start = now();
        while fs::read("original.stderr").unwrap_or_default().is_empty()
            || fs::read("original.stdout").unwrap_or_default().is_empty()
            || !Path::new("launch.json").exists()
        {
            assert!(now() - start < 5.0);
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(unsafe { libc::kill(libc::getppid(), libc::SIGSTOP) }, 0);
    }
    if case != "normal" {
        assert!(matches!(case, "stall" | "pause-launcher"));
        thread::sleep(Duration::from_secs(30));
        fs::write("escaped", "escaped").unwrap();
    }
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().unwrap().as_str() {
        "marker" => fs::write(arguments.next().unwrap(), "yes").unwrap(),
        "terminated" => terminated(arguments.next().as_deref() == Some("streams")),
        "retained" => retained(&arguments.next().unwrap()),
        "exit" => std::process::exit(arguments.next().unwrap().parse().unwrap()),
        "child" => child(),
        "controller" => controller(&arguments.next().unwrap()),
        "closed-streams" => closed_streams(arguments.next().unwrap().parse().unwrap()),
        "interrupted-child" => interrupted_child(&arguments.next().unwrap()),
        "interrupted-observer" => interrupted_observer(),
        "nested" => {
            fs::write("nested.pid", std::process::id().to_string()).unwrap();
            thread::sleep(Duration::from_secs(30));
            fs::write("escaped", "escaped").unwrap();
        }
        "slow" => thread::sleep(Duration::from_secs(5)),
        "sibling" => {
            thread::sleep(Duration::from_secs(2));
            fs::write("sibling", "joined").unwrap();
            println!("original");
        }
        "burst" => {
            let index: u32 = arguments.next().unwrap().parse().unwrap();
            File::create_new(format!("child-{index}"))
                .unwrap()
                .write_all(b"once")
                .unwrap();
            println!("original-{index}");
        }
        mode => panic!("unknown process lifecycle fixture mode: {mode}"),
    }
}
