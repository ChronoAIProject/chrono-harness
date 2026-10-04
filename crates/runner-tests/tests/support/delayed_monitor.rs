use std::{
    fs::{self, File},
    io::Write,
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
    thread,
    time::Duration,
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

fn await_file(path: &str) {
    let start = now();
    while !Path::new(path).exists() {
        assert!(
            now() - start < 5.0,
            "{path} was not published in five seconds"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn child() {
    publish("ready", &std::process::id().to_string());
    await_file("release");
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

    fn wait(self, pid: libc::pid_t) {
        #[cfg(target_os = "macos")]
        {
            let timeout = libc::timespec {
                tv_sec: 3,
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
            assert_eq!(unsafe { libc::poll(&mut event, 1, 3000) }, 1);
            assert_ne!(event.revents & libc::POLLIN, 0);
        }
    }
}

fn controller(case: &str) {
    assert!(matches!(case, "on-time" | "late" | "overflow"));
    File::create_new("controller-ready").unwrap();
    await_file("ready");
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
    exit.wait(pid);
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

fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().unwrap().as_str() {
        "child" => child(),
        "controller" => controller(&arguments.next().unwrap()),
        mode => panic!("unknown delayed-monitor fixture mode: {mode}"),
    }
}
