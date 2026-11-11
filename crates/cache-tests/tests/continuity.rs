//! Independent attempt judgments and Rust behavior at the IO boundary.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::{expect_cache_true, policy};
#[cfg(not(verus_keep_ghost))]
use chrono_cache::verified_planner as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn request_identity_keeps_full_text_and_missing_distinct() {
    proof {
        reveal_strlit("attempt"); reveal_strlit("attempt\0"); reveal_strlit("Attempt");
        assert("attempt"@.len() != "attempt\0"@.len());
        assert("attempt"@[0] != "Attempt"@[0]);
    }
    expect_cache_true(policy::same_attempt("attempt", Some("attempt")));
    expect_cache_true(!policy::same_attempt("attempt", Some("attempt\0")));
    expect_cache_true(!policy::same_attempt("attempt", Some("Attempt")));
    expect_cache_true(!policy::same_attempt("attempt", None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn custody_needs_original_terminal_publication_and_finished_consumer() {
    expect_cache_true(policy::accept_original(true, true, true));
    expect_cache_true(!policy::accept_original(false, true, true));
    expect_cache_true(!policy::accept_original(true, false, true));
    expect_cache_true(!policy::accept_original(true, true, false));
    expect_cache_true(!policy::accept_original(false, false, false));
    expect_cache_true(policy::release_custody(true, true, true, true));
    expect_cache_true(!policy::release_custody(false, true, true, true));
    expect_cache_true(!policy::release_custody(true, false, true, true));
    expect_cache_true(!policy::release_custody(true, true, false, true));
    expect_cache_true(!policy::release_custody(true, true, true, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_bound_terminal_attempts_replay() {
    expect_cache_true(policy::replay_attempt(true, true));
    expect_cache_true(!policy::replay_attempt(true, false));
    expect_cache_true(!policy::replay_attempt(false, true));
    expect_cache_true(!policy::replay_attempt(false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_terminal_process_facts_are_required() {
    expect_cache_true(policy::confirmed_probe(true, 0, None, true));
    expect_cache_true(policy::confirmed_probe(true, 7, None, true));
    expect_cache_true(policy::confirmed_probe(true, i32::MAX, None, true));
    expect_cache_true(policy::confirmed_probe(true, -1, Some(9), true));
    expect_cache_true(!policy::confirmed_probe(false, 0, None, true));
    expect_cache_true(!policy::confirmed_probe(true, -1, None, true));
    expect_cache_true(!policy::confirmed_probe(true, i32::MIN, None, true));
    expect_cache_true(!policy::confirmed_probe(true, 0, None, false));
    expect_cache_true(!policy::confirmed_probe(false, -1, None, true));
    expect_cache_true(!policy::confirmed_probe(false, 0, None, false));
    expect_cache_true(!policy::confirmed_probe(true, -1, None, false));
    expect_cache_true(!policy::confirmed_probe(false, -1, None, false));
    expect_cache_true(!policy::confirmed_probe(false, -1, Some(9), true));
    expect_cache_true(!policy::confirmed_probe(true, -1, Some(9), false));
}



#[cfg(not(verus_keep_ghost))]
fn expect_cache_true(actual: bool)
    requires actual,
{ assert!(actual); }
}

#[cfg(not(verus_keep_ghost))]
mod behavior {
    use super::super::*;

    fn probe_config(root: &Path, config: &mut Value, args: Value) {
        config["inputs"]["compiler"] = json!({"kind":"command","command":{
        "program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),"args":args,"env":{},
        "timeout_seconds":5,"output_limit_bytes":4096},"inherit":[],"result":"stdout"});
        fs::write(
            root.join(".chrono-harness/cache.json"),
            serde_json::to_vec(config).unwrap(),
        )
        .unwrap();
    }

    fn plan_args(root: &Path, output: &str) -> Vec<String> {
        [
            "plan",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            ".chrono-harness/cache.json",
            "--consumer",
            "check.one",
            "--plan-output",
            output,
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn exact_plan_attempt_reentry_does_not_dispatch_another_probe() {
        let (root, mut config) = fixture();
        let count = root.path().join("invocations");
        probe_config(root.path(), &mut config, json!(["count", count]));
        let args = plan_args(root.path(), ".chrono-harness/state/cache/one/plan.json");
        let binary = env!("CARGO_BIN_EXE_chrono-cache-test-cli");
        let first = Command::new(binary).args(&args).output().unwrap();
        assert!(
            first.status.success(),
            "{}",
            String::from_utf8_lossy(&first.stderr)
        );
        let second = Command::new(binary).args(&args).output().unwrap();
        assert!(
            second.status.success(),
            "{}",
            String::from_utf8_lossy(&second.stderr)
        );
        assert_eq!(fs::read(&count).unwrap(), b"x");
        assert_eq!(first.stdout, second.stdout);
        config["namespace"] = json!("changed-request");
        fs::write(
            root.path().join(".chrono-harness/cache.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let changed = Command::new(binary).args(&args).output().unwrap();
        assert!(!changed.status.success());
        assert!(String::from_utf8_lossy(&changed.stderr).contains("original request differs"));
        assert_eq!(fs::read(&count).unwrap(), b"x");
    }

    fn attempt_path(root: &Path, output: &str) -> std::path::PathBuf {
        root.join(format!(
            ".chrono-harness/state/cache-probes/plan-{:x}",
            Sha256::digest(output.as_bytes())
        ))
    }
    fn value(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }
    fn invoke(root: &Path, output: &str) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli"))
            .args(plan_args(root, output))
            .output()
            .unwrap()
    }

    fn seal(root: &Path, output: &str, cache: Option<&str>) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli"));
        command.args([
            "seal",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            ".chrono-harness/cache.json",
            "--plan",
            output,
            "--steps-env",
            "CACHE_STEPS",
            "--work",
            "build",
        ]);
        if let Some(cache) = cache {
            command.args(["--cache", cache]);
        }
        command
            .env("CACHE_STEPS", "{\"build\":{\"outcome\":\"success\"}} ")
            .output()
            .unwrap()
    }

    #[test]
    fn plan_rejects_missing_changed_and_other_attempt_original_binding() {
        for field in ["path", "sha256", "attempt_id"] {
            let (root, mut config) = fixture();
            let count = root.path().join("invocations");
            probe_config(root.path(), &mut config, json!(["count", count]));
            let output = ".chrono-harness/state/cache/binding/plan.json";
            let run = invoke(root.path(), output);
            assert!(
                run.status.success(),
                "{}",
                String::from_utf8_lossy(&run.stderr)
            );
            let original: Value = serde_json::from_slice(&run.stdout).unwrap();
            for changed in [Value::Null, json!("another-attempt")] {
                let mut plan = original.clone();
                plan["inputs"]["compiler"]["original"][field] = changed;
                fs::write(root.path().join(output), serde_json::to_vec(&plan).unwrap()).unwrap();
                let rejected = seal(root.path(), output, None);
                assert!(!rejected.status.success());
                assert!(
                    String::from_utf8_lossy(&rejected.stderr).contains("E_CACHE_ATTEMPT"),
                    "{}",
                    String::from_utf8_lossy(&rejected.stderr)
                );
                assert_eq!(fs::read(&count).unwrap(), b"x");
            }
        }
    }

    #[test]
    fn resolve_spawn_and_nonzero_errors_preserve_distinct_original_facts() {
        for mode in ["resolve", "spawn", "exit"] {
            let (root, mut config) = fixture();
            probe_config(root.path(), &mut config, json!(["exit"]));
            if mode != "exit" {
                let executable = root.path().join("unlaunchable");
                if mode == "spawn" {
                    fs::write(&executable, b"not executable").unwrap();
                }
                config["inputs"]["compiler"]["command"]["program"] = json!(executable);
                fs::write(
                    root.path().join(".chrono-harness/cache.json"),
                    serde_json::to_vec(&config).unwrap(),
                )
                .unwrap();
            }
            let output = ".chrono-harness/state/cache/errors/plan.json";
            let run = invoke(root.path(), output);
            assert!(!run.status.success());
            let attempt = attempt_path(root.path(), output);
            assert!(attempt.join("intent.json").is_file());
            let result = value(&attempt.join("result.json"));
            if mode == "exit" {
                let process = value(&attempt.join("compiler/process.json"));
                assert_eq!(process["exit_code"], 7);
                assert_eq!(process["stderr"], "original probe failure\n");
                assert_eq!(result["terminal"], true);
            } else {
                let error = value(&attempt.join("compiler/error.json"));
                assert!(error["returned_task_identity"].is_null());
                assert!(error["terminal_result"].is_null());
                assert_eq!(error["engine_invoked"], mode == "spawn");
                assert_eq!(error["launch_original"].is_object(), mode == "spawn");
                assert_eq!(result["terminal"], mode == "resolve");
                assert!(!attempt.join("compiler/process.json").exists());
            }
            let repeated = invoke(root.path(), output);
            assert!(!repeated.status.success());
            assert_eq!(value(&attempt.join("result.json")), result);
        }
    }

    #[test]
    fn missing_adopted_probe_policy_refuses_dispatch() {
        for mode in ["absent", "limits"] {
            let (root, mut config) = fixture();
            let count = root.path().join("invocations");
            probe_config(root.path(), &mut config, json!(["count", count]));
            let path = root.path().join(".chrono-harness/retention.json");
            if mode == "absent" {
                fs::remove_file(path).unwrap();
            } else {
                let mut policy = value(&path);
                policy["terminal_limits"]
                    .as_object_mut()
                    .unwrap()
                    .remove("cache-probe-attempt");
                fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
            }
            let run = invoke(root.path(), ".chrono-harness/state/cache/policy/plan.json");
            assert!(!run.status.success());
            assert!(String::from_utf8_lossy(&run.stderr).contains("E_CACHE_ATTEMPT"));
            assert!(!count.exists());
        }
    }

    #[test]
    #[cfg(unix)]
    fn environment_error_is_not_dispatched_and_original_error_replays() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let (root, mut config) = fixture();
        let count = root.path().join("invocations");
        probe_config(root.path(), &mut config, json!(["count", count]));
        config["inputs"]["compiler"]["inherit"] = json!(["CACHE_TEST_INVALID_TEXT"]);
        fs::write(
            root.path().join(".chrono-harness/cache.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let output = ".chrono-harness/state/cache/environment/plan.json";
        let run = Command::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli"))
            .args(plan_args(root.path(), output))
            .env("CACHE_TEST_INVALID_TEXT", OsString::from_vec(vec![255]))
            .output()
            .unwrap();
        assert!(!run.status.success());
        let attempt = attempt_path(root.path(), output);
        let error = value(&attempt.join("compiler/error.json"));
        assert_eq!(error["engine_invoked"], false);
        assert_eq!(error["completion"], "not-dispatched");
        assert_eq!(value(&attempt.join("result.json"))["terminal"], true);
        let replay = invoke(root.path(), output);
        assert!(!replay.status.success());
        assert!(String::from_utf8_lossy(&replay.stderr).contains(error["error"].as_str().unwrap()));
        assert!(!count.exists());
    }

    #[test]
    fn failed_original_reentry_finishes_interrupted_reference_release() {
        let (root, mut config) = fixture();
        let count = root.path().join("invocations");
        probe_config(root.path(), &mut config, json!(["exit", count]));
        let output = ".chrono-harness/state/cache/failed-reentry/plan.json";
        let first = invoke(root.path(), output);
        assert!(!first.status.success());
        let attempt = attempt_path(root.path(), output);
        let receipt = fs::read(attempt.join("result.json")).unwrap();
        assert_eq!(value(&attempt.join("result.json"))["terminal"], true);
        let intent = value(&attempt.join("intent.json"));
        let role = intent["consumer_role"].as_str().unwrap();
        let relative = attempt.strip_prefix(root.path()).unwrap().to_str().unwrap();
        let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(root.path())
            .unwrap().unwrap();
        // Model the exact interruption boundary: the actual sealed failure
        // exists, but its existing plan-root release has not completed. Never
        // fabricate a terminal result or execute another probe to recover it.
        inventory.reference_paths(role, &[relative.to_owned()]).unwrap();
        inventory.maintain_at(u64::MAX).unwrap();
        assert!(attempt.is_dir(), "the pending recovery root protects its original");
        let replay = invoke(root.path(), output);
        assert_eq!(replay.status.code(), first.status.code());
        assert_eq!(replay.stderr, first.stderr);
        assert_eq!(fs::read(&count).unwrap(), b"x");
        assert_eq!(fs::read(attempt.join("result.json")).unwrap(), receipt);
        let mut rounds = Vec::new();
        for _ in 0..19 { rounds.push(inventory.maintain_at(u64::MAX).unwrap()); }
        assert!(!attempt.exists(), "replay must finish the same failed attempt's root release: {rounds:?}");
    }

    #[test]
    fn process_receipt_publication_failure_keeps_acquired_original_and_unknown() {
        let (root, mut config) = fixture();
        let output = ".chrono-harness/state/cache/receipt/plan.json";
        let attempt = attempt_path(root.path(), output);
        probe_config(
            root.path(),
            &mut config,
            json!(["receipt-conflict", attempt.join("compiler/process.json")]),
        );
        let run = invoke(root.path(), output);
        assert!(!run.status.success());
        assert!(String::from_utf8_lossy(&run.stderr).contains("original receipt publication"));
        let process = value(&attempt.join("compiler/process.json.writing"));
        assert_eq!(process["exit_code"], 0);
        assert_eq!(process["stdout"], "original observed output\n");
        assert_eq!(process["owned_work_joined"], true);
        assert_eq!(value(&attempt.join("result.json"))["terminal"], false);
        let replay = invoke(root.path(), output);
        assert!(!replay.status.success());
        assert!(String::from_utf8_lossy(&replay.stderr).contains("unknown"));
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root.path())
                .unwrap()
                .unwrap();
        inventory.maintain_at(u64::MAX).unwrap();
        assert!(attempt.join("compiler/process.json.writing").is_file());
    }

    #[test]
    fn plan_handoff_failure_reuses_original_attempt_and_independent_keys_match() {
        let (root, mut config) = fixture();
        let count = root.path().join("invocations");
        probe_config(root.path(), &mut config, json!(["count", count]));
        let output = ".chrono-harness/state/cache/handoff/plan.json";
        let target = root.path().join(output);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"conflicting publication").unwrap();
        let first = invoke(root.path(), output);
        assert!(!first.status.success());
        assert!(
            String::from_utf8_lossy(&first.stderr).contains("addressed original bytes mismatch")
        );
        let result = value(&attempt_path(root.path(), output).join("result.json"));
        assert_eq!(result["terminal"], true);
        assert!(result["plan"].is_object());
        fs::remove_file(target).unwrap(); // fixture owns the rejected destination
        let repaired = invoke(root.path(), output);
        assert!(
            repaired.status.success(),
            "{}",
            String::from_utf8_lossy(&repaired.stderr)
        );
        assert_eq!(fs::read(&count).unwrap(), b"x");
        let second = invoke(
            root.path(),
            ".chrono-harness/state/cache/independent/plan.json",
        );
        assert!(
            second.status.success(),
            "{}",
            String::from_utf8_lossy(&second.stderr)
        );
        let first: Value = serde_json::from_slice(&repaired.stdout).unwrap();
        let second: Value = serde_json::from_slice(&second.stdout).unwrap();
        assert_eq!(first["caches"], second["caches"]);
        assert_ne!(
            first["inputs"]["compiler"]["original"],
            second["inputs"]["compiler"]["original"]
        );
    }

    #[cfg(unix)]
    struct Owner(Option<std::thread::JoinHandle<Result<chrono_harness::ProcessResult, String>>>);
    #[cfg(unix)]
    impl Drop for Owner {
        fn drop(&mut self) {
            if let Some(task) = self.0.take() {
                let _ = task.join();
            }
        }
    }
    #[cfg(unix)]
    impl Owner {
        fn join(mut self) -> chrono_harness::ProcessResult {
            self.0.take().unwrap().join().unwrap().unwrap()
        }
    }

    #[cfg(unix)]
    fn present(path: &Path) {
        use std::os::fd::AsRawFd;
        let directory = fs::File::open(path.parent().unwrap()).unwrap();
        #[cfg(target_os = "macos")]
        unsafe {
            let queue = libc::kqueue();
            assert!(queue >= 0);
            let change = libc::kevent {
                ident: directory.as_raw_fd() as _,
                filter: libc::EVFILT_VNODE,
                flags: libc::EV_ADD | libc::EV_CLEAR,
                fflags: libc::NOTE_WRITE,
                data: 0,
                udata: std::ptr::null_mut(),
            };
            assert_eq!(
                libc::kevent(queue, &change, 1, std::ptr::null_mut(), 0, std::ptr::null()),
                0
            );
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !path.exists() {
                let remaining = deadline
                    .checked_duration_since(std::time::Instant::now())
                    .unwrap();
                let timeout = libc::timespec {
                    tv_sec: remaining.as_secs() as _,
                    tv_nsec: remaining.subsec_nanos() as _,
                };
                let mut event = std::mem::zeroed();
                assert_eq!(
                    libc::kevent(queue, std::ptr::null(), 0, &mut event, 1, &timeout),
                    1
                );
            }
            libc::close(queue);
        }
        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::unix::ffi::OsStrExt;
            let queue = libc::inotify_init1(libc::IN_CLOEXEC);
            assert!(queue >= 0);
            let name =
                std::ffi::CString::new(path.parent().unwrap().as_os_str().as_bytes()).unwrap();
            assert!(
                libc::inotify_add_watch(queue, name.as_ptr(), libc::IN_CREATE | libc::IN_MOVED_TO)
                    >= 0
            );
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !path.exists() {
                let remaining = deadline
                    .checked_duration_since(std::time::Instant::now())
                    .unwrap();
                let mut event = libc::pollfd {
                    fd: queue,
                    events: libc::POLLIN,
                    revents: 0,
                };
                assert_eq!(libc::poll(&mut event, 1, remaining.as_millis() as _), 1);
                let mut bytes = [0u8; 4096];
                assert!(libc::read(queue, bytes.as_mut_ptr().cast(), bytes.len()) > 0);
            }
            libc::close(queue);
        }
    }

    // Synchronize on acquired file bytes with one native blocking filesystem event.
    // This is a fixture acquisition barrier, not a production completion watcher.
    #[cfg(unix)]
    fn acquired(path: &Path, expected: &[u8]) {
        #[cfg(target_os = "macos")]
        unsafe {
            use std::os::fd::AsRawFd;
            let file = fs::File::open(path).unwrap();
            let queue = libc::kqueue();
            assert!(queue >= 0);
            let change = libc::kevent {
                ident: file.as_raw_fd() as _,
                filter: libc::EVFILT_VNODE,
                flags: libc::EV_ADD | libc::EV_CLEAR,
                fflags: libc::NOTE_WRITE,
                data: 0,
                udata: std::ptr::null_mut(),
            };
            assert_eq!(
                libc::kevent(queue, &change, 1, std::ptr::null_mut(), 0, std::ptr::null()),
                0
            );
            if fs::read(path).unwrap() != expected {
                let mut event = std::mem::zeroed();
                let timeout = libc::timespec {
                    tv_sec: 5,
                    tv_nsec: 0,
                };
                assert_eq!(
                    libc::kevent(queue, std::ptr::null(), 0, &mut event, 1, &timeout),
                    1
                );
            }
            libc::close(queue);
        }
        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::unix::ffi::OsStrExt;
            let queue = libc::inotify_init1(libc::IN_CLOEXEC);
            assert!(queue >= 0);
            let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
            assert!(libc::inotify_add_watch(queue, name.as_ptr(), libc::IN_MODIFY) >= 0);
            if fs::read(path).unwrap() != expected {
                let mut event = libc::pollfd {
                    fd: queue,
                    events: libc::POLLIN,
                    revents: 0,
                };
                assert_eq!(libc::poll(&mut event, 1, 5000), 1);
            }
            libc::close(queue);
        }
        assert_eq!(fs::read(path).unwrap(), expected);
    }

    #[test]
    #[cfg(unix)]
    fn live_originals_survive_launcher_interruption_without_redispatch() {
        use std::{
            io::{Read, Write},
            os::unix::net::UnixListener,
        };
        let (root, mut config) = fixture();
        let socket_directory = tempfile::tempdir_in("/tmp").unwrap();
        let socket = socket_directory.path().join("ready.sock");
        // Keep the native socket address within sockaddr_un on macOS.
        let listener = UnixListener::bind(&socket).unwrap();
        probe_config(root.path(), &mut config, json!(["live", socket]));
        let output = ".chrono-harness/state/cache/live/plan.json";
        let args = plan_args(root.path(), output);
        let actual_root = root.path().to_owned();
        let owner = Owner(Some(std::thread::spawn(move || {
            let command = chrono_harness::CommandSpec {
                program: env!("CARGO_BIN_EXE_chrono-cache-test-cli").into(),
                args,
                env: Default::default(),
                timeout_seconds: 10,
                output_limit_bytes: 65536,
            };
            chrono_harness::run_process_observed(
                &actual_root,
                &command,
                &[],
                &chrono_harness::file_identity(Path::new(&command.program))
                    .unwrap()
                    .0,
            )
        })));
        use std::os::fd::AsRawFd;
        unsafe {
            let mut event = libc::pollfd {
                fd: listener.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            assert_eq!(libc::poll(&mut event, 1, 5000), 1);
        }
        let (mut connection, _) = listener.accept().unwrap();
        connection
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut ready = [0; 5];
        connection.read_exact(&mut ready).unwrap();
        assert_eq!(&ready, b"ready");
        let attempt = attempt_path(root.path(), output);
        let intent = value(&attempt.join("compiler/intent.json"));
        assert!(intent["task_identity"].is_null());
        present(&attempt.join("compiler/launch.json"));
        let launch = value(&attempt.join("compiler/launch.json"));
        assert!(launch["child_pid"].as_u64().unwrap() > 0);
        acquired(&attempt.join("compiler/stdout"), b"live stdout");
        acquired(&attempt.join("compiler/stderr"), b"live stderr");
        assert!(!attempt.join("compiler/process.json").exists());
        // A concurrent exact caller cannot dispatch; unrelated addresses still work.
        let repeated = invoke(root.path(), output);
        assert!(!repeated.status.success());
        assert!(String::from_utf8_lossy(&repeated.stderr).contains("unknown"));
        let original_config = config.clone();
        let independent_count = root.path().join("independent-invocations");
        probe_config(
            root.path(),
            &mut config,
            json!(["count", independent_count]),
        );
        let independent = invoke(
            root.path(),
            ".chrono-harness/state/cache/independent-live/plan.json",
        );
        assert!(
            independent.status.success(),
            "{}",
            String::from_utf8_lossy(&independent.stderr)
        );
        assert_eq!(fs::read(&independent_count).unwrap(), b"x");
        fs::write(
            root.path().join(".chrono-harness/cache.json"),
            serde_json::to_vec(&original_config).unwrap(),
        )
        .unwrap();
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root.path())
                .unwrap()
                .unwrap();
        inventory.maintain_at(u64::MAX).unwrap();
        assert!(attempt.join("compiler/stdout").is_file());
        unsafe {
            assert_eq!(
                libc::kill(launch["launcher_pid"].as_u64().unwrap() as _, libc::SIGKILL),
                0
            );
        }
        connection.write_all(b"x").unwrap();
        let result = owner.join();
        assert_eq!(result.exit_signal, Some(libc::SIGKILL));
        assert!(!attempt.join("result.json").exists());
        let after = invoke(root.path(), output);
        assert!(!after.status.success());
        assert!(String::from_utf8_lossy(&after.stderr).contains("no redispatch"));
        inventory.maintain_at(u64::MAX).unwrap();
        assert_eq!(
            fs::read(attempt.join("compiler/stdout")).unwrap(),
            b"live stdout"
        );
    }

    fn mock_git(root: &Path) {
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(
            root.join(".chrono-harness/config.json"),
            serde_json::to_vec(&json!({
        "schema_version":4,"facts_git":{"tool":"git"},
        "tools":[{"id":"git","program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),
            "resolution":"PATH-once","version_argv":["--version"],"expected_version":null}],
        "environment":{"inherit":[],"values":{},"inputs":[]},
        "protocol":{"timeout_seconds":5,"stdout_limit_bytes":65536}}))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn unregistered_plan_handoff_failure_keeps_the_original_attempt_root() {
        let (root, mut config) = fixture();
        mock_git(root.path());
        config["require_primary_checkout"] = json!(true);
        config["recover_failed_restores"] = json!(true);
        config["recover_unconfirmed_restores"] = json!(true);
        config["caches"]["project"]["verify_contents"] = json!(true);
        let count = root.path().join("invocations");
        probe_config(root.path(), &mut config, json!(["count", count]));
        let output = ".chrono-harness/state/restore/plan.json";
        let mut args = plan_args(root.path(), output);
        args.truncate(args.len() - 2); // stdout-only API/CLI has no plan publisher
        let run = Command::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let plan: Value = serde_json::from_slice(&run.stdout).unwrap();
        let original = Path::new(
            plan["inputs"]["compiler"]["original"]["path"]
                .as_str()
                .unwrap(),
        );
        let attempt = original
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_str()
            .unwrap();
        fs::create_dir_all(root.path().join(output).parent().unwrap()).unwrap();
        fs::write(root.path().join(output), &run.stdout).unwrap();
        fs::create_dir_all(root.path().join("out/project")).unwrap();
        fs::write(
            root.path().join("out/project/object"),
            b"mock producer output",
        )
        .unwrap();
        let completed = seal(root.path(), output, None);
        assert!(!completed.status.success());
        assert!(
            String::from_utf8_lossy(&completed.stderr)
                .contains("consumer plan publication not bound"),
            "{}",
            String::from_utf8_lossy(&completed.stderr)
        );
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root.path())
                .unwrap()
                .unwrap();
        let role = format!("cache-plan:{:x}", Sha256::digest(attempt.as_bytes()));
        assert!(
            inventory
                .rooted_original("cache-probe-attempt", &role)
                .unwrap()
                .is_some(),
            "a failed consumer handoff must retain the attempt's recovery obligation"
        );
        inventory.maintain_at(u64::MAX).unwrap();
        assert!(root.path().join(attempt).is_dir());
        assert_eq!(fs::read(count).unwrap(), b"x");
    }

    #[test]
    fn recovery_and_seal_keep_probe_custody_until_final_consumer_releases() {
        let (root, mut config) = fixture();
        mock_git(root.path());
        config["require_primary_checkout"] = json!(true);
        config["recover_failed_restores"] = json!(true);
        config["recover_unconfirmed_restores"] = json!(true);
        config["caches"]["project"]["verify_contents"] = json!(true);
        let count = root.path().join("invocations");
        probe_config(root.path(), &mut config, json!(["count", count]));
        let output = ".chrono-harness/state/restore/plan.json";
        let run = invoke(root.path(), output);
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let plan: Value = serde_json::from_slice(&run.stdout).unwrap();
        let attempt = attempt_path(root.path(), output);
        let original = plan["inputs"]["compiler"]["original"]["path"]
            .as_str()
            .unwrap();
        let original_bytes = fs::read(root.path().join(original)).unwrap();
        let recovered = recovery::recover(
            root.path(),
            &json!({"cache_0_restore":{"outcome":"success","outputs":{"cache-matched-key":plan["caches"]["project"]["key"]}}}),
        );
        assert!(
            recovered.status.success(),
            "{}",
            String::from_utf8_lossy(&recovered.stderr)
        );
        assert_eq!(fs::read(&count).unwrap(), b"x");
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root.path())
                .unwrap()
                .unwrap();
        inventory.maintain_at(u64::MAX).unwrap();
        assert!(
            attempt.is_dir(),
            "the seal consumer still needs the original attempt"
        );
        fs::create_dir_all(root.path().join("out/project")).unwrap();
        fs::write(
            root.path().join("out/project/object"),
            b"actual mock build bytes",
        )
        .unwrap();
        let partial = seal(root.path(), output, Some("project"));
        assert!(
            partial.status.success(),
            "{}",
            String::from_utf8_lossy(&partial.stderr)
        );
        inventory.maintain_at(u64::MAX).unwrap();
        assert!(
            attempt.is_dir(),
            "partial seal keeps the plan's whole consumer"
        );
        let completed = seal(root.path(), output, None);
        assert!(
            completed.status.success(),
            "{}",
            String::from_utf8_lossy(&completed.stderr)
        );
        assert_eq!(fs::read(&count).unwrap(), b"x");
        assert_eq!(
            fs::read(root.path().join(original)).unwrap(),
            original_bytes
        );
        let mut rounds = Vec::new();
        // Two retained outputs, eleven declared physical nodes, and up to four
        // phase/edge transitions per output. The original 25 ms round budget stays
        // unchanged, including rounds that only advance bounded metadata work.
        for _ in 0..(11 + 4 * 2) {
            let round = inventory.maintain_at(u64::MAX).unwrap();
            for error in round["accounting_errors"].as_array().unwrap() {
                // Nonblocking maintenance may honestly preserve a busy local
                // component. Corruption is not accepted, and the final physical
                // retirement assertions still require all busy work to resolve.
                assert!(
                    error["reason"]
                        .as_str()
                        .unwrap()
                        .contains("metadata is busy"),
                    "{round}"
                );
                assert!(
                    !round["protected"].as_array().unwrap().is_empty(),
                    "{round}"
                );
            }
            rounds.push(round);
        }
        fs::write(
            root.path().join("maintenance.json"),
            serde_json::to_vec(&rounds).unwrap(),
        )
        .unwrap();
        assert!(
            !attempt.exists(),
            "confirmed terminal originals retire after consumer release: {rounds:?}"
        );
        assert!(
            !root.path().join(output).exists(),
            "the released plan retires through its own finite policy"
        );
        // Reusing an entirely retired address creates a different attempt. A stale
        // plan must not qualify that later generation, even with equal semantic keys.
        let next = invoke(root.path(), output);
        assert!(
            next.status.success(),
            "{}",
            String::from_utf8_lossy(&next.stderr)
        );
        let next_plan: Value = serde_json::from_slice(&next.stdout).unwrap();
        assert_eq!(plan["caches"], next_plan["caches"]);
        assert_ne!(
            plan["inputs"]["compiler"]["original"]["attempt_id"],
            next_plan["inputs"]["compiler"]["original"]["attempt_id"]
        );
        fs::write(root.path().join(output), serde_json::to_vec(&plan).unwrap()).unwrap();
        let stale = seal(root.path(), output, None);
        assert!(!stale.status.success());
        assert!(
            String::from_utf8_lossy(&stale.stderr)
                .contains("original digest or attempt identity differs"),
            "{}",
            String::from_utf8_lossy(&stale.stderr)
        );
        assert_eq!(fs::read(&count).unwrap(), b"xx");
    }

    #[test]
    fn owned_process_evidence_rejects_outside_files_before_dispatch() {
        use chrono_harness::retained_artifacts::retention::Inventory;
        let (root, _) = fixture();
        let output = ".chrono-harness/state/cache-probes/capability";
        let state = Inventory::adopted(root.path()).unwrap().unwrap();
        fs::create_dir_all(root.path().join(".chrono-harness/state/cache-probes")).unwrap();
        let publication = state
            .plan_rooted(
                "cache-probe-attempt",
                output,
                json!({"completion":"not-established"}),
                "fixture-original",
            )
            .unwrap();
        publication.materialize().unwrap();
        let count = root.path().join("invocations");
        let command = chrono_harness::CommandSpec {
            program: env!("CARGO_BIN_EXE_chrono-cache-test-probe").into(),
            args: vec!["count".into(), count.to_str().unwrap().into()],
            env: Default::default(),
            timeout_seconds: 5,
            output_limit_bytes: 4096,
        };
        let directory = root.path().join(output);
        let error = chrono_harness::run_process_observed_retained_owned(
            root.path(),
            &command,
            &[],
            &chrono_harness::file_identity(Path::new(&command.program))
                .unwrap()
                .0,
            &chrono_harness::ProcessEvidence {
                stdout: &root.path().join("outside"),
                stderr: &directory.join("stderr"),
                launch: &directory.join("launch.json"),
            },
            &publication,
        )
        .unwrap_err();
        assert!(error.contains("outside exact output"), "{error}");
        assert!(!count.exists());
        assert!(!root.path().join("outside").exists());
    }
}
