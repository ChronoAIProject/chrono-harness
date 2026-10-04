#![cfg(unix)]
use chrono_harness::{CommandSpec, process_fds::Scope, run_process_observed, sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::fd::{AsFd, AsRawFd},
    path::Path,
};
fn spec(program: &str, args: Vec<String>) -> CommandSpec {
    CommandSpec {
        program: program.into(),
        args,
        env: BTreeMap::new(),
        timeout_seconds: 10,
        output_limit_bytes: 65536,
    }
}
fn inherited_count() -> usize {
    std::env::var("CHRONO_PROCESS_FDS")
        .map(|value| value.split(',').count())
        .unwrap_or(0)
}
fn capability_probe(count: usize) -> CommandSpec {
    let mut command = spec(
        std::env::current_exe().unwrap().to_str().unwrap(),
        vec![
            "--exact".into(),
            "capability_child_helper".into(),
            "--nocapture".into(),
        ],
    );
    command
        .env
        .insert("CHRONO_FD_EXPECT".into(), count.to_string());
    command
}
#[test]
fn capability_child_helper() {
    let Ok(expected) = std::env::var("CHRONO_FD_EXPECT") else {
        return;
    };
    let carrier = std::env::var("CHRONO_PROCESS_FDS").ok();
    let fds: Vec<i32> = carrier
        .as_deref()
        .map(|v| v.split(',').map(|fd| fd.parse().unwrap()).collect())
        .unwrap_or_default();
    assert_eq!(fds.len(), expected.parse::<usize>().unwrap());
    for fd in &fds {
        let flags = unsafe { libc::fcntl(*fd, libc::F_GETFD) };
        assert!(flags >= 0, "{}", std::io::Error::last_os_error());
        assert_eq!(flags & libc::FD_CLOEXEC, 0);
    }
    if let Ok(previous) = std::env::var("CHRONO_FD_SUPERSEDED") {
        for fd in previous.split(',').map(|fd| fd.parse::<i32>().unwrap()) {
            assert_eq!(
                unsafe { libc::fcntl(fd, libc::F_GETFD) },
                -1,
                "superseded capability descriptor still inherited: {fd}"
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
        }
    }
    if let Ok(bytes) = std::env::var("CHRONO_FD_WRITE") {
        assert_eq!(
            unsafe { libc::write(*fds.last().unwrap(), bytes.as_ptr().cast(), bytes.len()) },
            bytes.len() as isize,
            "{}",
            std::io::Error::last_os_error()
        );
    }
    if let Ok(expected) = std::env::var("CHRONO_FD_CONTENT") {
        for fd in &fds {
            let mut bytes = [0u8; 64];
            let len = unsafe { libc::pread(*fd, bytes.as_mut_ptr().cast(), bytes.len(), 0) };
            assert!(len >= 0, "{}", std::io::Error::last_os_error());
            assert_eq!(&bytes[..len as usize], expected.as_bytes());
        }
    }
    println!("FORWARDED_FDS={}", carrier.as_deref().unwrap_or(""));
}
#[test]
fn scoped_descriptor_survives_bound_env_clear_and_parent_keeps_cloexec() {
    let dir = tempfile::tempdir().unwrap();
    let file = tempfile::tempfile().unwrap();
    let raw = file.as_raw_fd();
    assert_ne!(
        unsafe { libc::fcntl(raw, libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    let scope = Scope::new(&[file.as_fd()]).unwrap();
    let mut command = capability_probe(inherited_count() + 1);
    command
        .env
        .insert("CHRONO_FD_WRITE".into(), "child-capability".into());
    command
        .env
        .insert("CHRONO_PROCESS_FDS".into(), "not-a-descriptor".into());
    let result = run_process_observed(
        dir.path(),
        &command,
        &[],
        &sha256(&fs::read(&command.program).unwrap()),
    )
    .unwrap();
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert!(!result.environment.contains_key("CHRONO_PROCESS_FDS"));
    let observed = serde_json::to_value(&result).unwrap();
    assert!(observed["ownership_fds"].is_string());
    let forwarded = result
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("FORWARDED_FDS="))
        .unwrap();
    assert_eq!(observed["ownership_fds"], forwarded);
    assert_eq!(
        result.environment_digest,
        chrono_harness::wire::digest(&result.environment).unwrap()
    );
    assert_ne!(
        unsafe { libc::fcntl(raw, libc::F_GETFD) } & libc::FD_CLOEXEC,
        0,
        "owner descriptor flags must stay local"
    );
    drop(scope);
    let command = capability_probe(inherited_count());
    assert_eq!(
        run_process_observed(
            dir.path(),
            &command,
            &[],
            &sha256(&fs::read(&command.program).unwrap())
        )
        .unwrap()
        .exit_code,
        0
    );
}
#[test]
fn nested_runner_helper() {
    if std::env::var_os("CHRONO_FD_NESTED").is_none() {
        return;
    }
    let inherited = std::env::var("CHRONO_PROCESS_FDS").unwrap();
    let descriptors: Vec<i32> = inherited.split(',').map(|v| v.parse().unwrap()).collect();
    let flags: Vec<i32> = descriptors
        .iter()
        .map(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) })
        .collect();
    let mut command = capability_probe(inherited_count());
    command.env.insert("CHRONO_FD_SUPERSEDED".into(), inherited);
    command
        .env
        .insert("CHRONO_FD_WRITE".into(), "nested-env-clear".into());
    let result = run_process_observed(
        Path::new("/"),
        &command,
        &[],
        &sha256(&fs::read(&command.program).unwrap()),
    )
    .unwrap();
    assert_eq!(result.exit_code, 0, "{} {}", result.stdout, result.stderr);
    assert_eq!(
        descriptors
            .iter()
            .map(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) })
            .collect::<Vec<_>>(),
        flags,
        "engine calls must not mutate inherited descriptors used by other threads or native children"
    );
    let native = capability_probe(inherited_count());
    let direct = std::process::Command::new(&native.program)
        .args(&native.args)
        .envs(&native.env)
        .env("CHRONO_FD_WRITE", "native-after-engine")
        .output()
        .unwrap();
    assert!(
        direct.status.success(),
        "{}",
        String::from_utf8_lossy(&direct.stderr)
    );
    println!("nested-forwarded");
}
#[test]
fn nested_engine_forwards_opaque_capability_after_environment_clear() {
    use std::io::{Read, Seek};
    let dir = tempfile::tempdir().unwrap();
    let mut file = tempfile::tempfile().unwrap();
    let _scope = Scope::new(&[file.as_fd()]).unwrap();
    let exe = std::env::current_exe().unwrap();
    let mut command = spec(
        exe.to_str().unwrap(),
        vec![
            "--exact".into(),
            "nested_runner_helper".into(),
            "--nocapture".into(),
        ],
    );
    command.env.insert("CHRONO_FD_NESTED".into(), "1".into());
    let result =
        run_process_observed(dir.path(), &command, &[], &sha256(&fs::read(&exe).unwrap())).unwrap();
    assert_eq!(result.exit_code, 0, "{} {}", result.stdout, result.stderr);
    assert!(result.stdout.contains("nested-forwarded"));
    file.rewind().unwrap();
    let mut bytes = String::new();
    file.read_to_string(&mut bytes).unwrap();
    assert_eq!(bytes, "nested-env-clearnative-after-engine");
}

#[test]
fn reserved_descriptor_helper() {
    let Ok(slot) = std::env::var("CHRONO_FD_RESERVED_SLOT") else {
        return;
    };
    let slot: i32 = slot.parse().unwrap();
    assert!((198..=200).contains(&slot));
    let dir = tempfile::tempdir().unwrap();
    let warm = spec("/usr/bin/true", vec![]);
    let result = run_process_observed(
        dir.path(),
        &warm,
        &[],
        &sha256(&fs::read(&warm.program).unwrap()),
    )
    .unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.failure.is_none());
    let file = tempfile::tempfile().unwrap();
    use std::os::unix::fs::FileExt;
    file.write_all_at(b"declared-ownership-capability", 0)
        .unwrap();
    let mut held = Vec::new();
    loop {
        let filler = fs::File::open("/dev/null").unwrap();
        let fd = filler.as_raw_fd();
        held.push(filler);
        if fd >= slot - 2 {
            assert_eq!(fd, slot - 2);
            break;
        }
    }
    let _scope = Scope::new(&[file.as_fd()]).unwrap();
    let mut command = capability_probe(1);
    command.env.insert(
        "CHRONO_FD_CONTENT".into(),
        "declared-ownership-capability".into(),
    );
    let result = run_process_observed(
        dir.path(),
        &command,
        &[],
        &sha256(&fs::read(&command.program).unwrap()),
    )
    .unwrap();
    assert_eq!(
        result.exit_code, 0,
        "slot {slot}: {} {}",
        result.stdout, result.stderr
    );
    assert!(result.failure.is_none(), "{:?}", result.failure);
}

#[test]
fn forwarded_capabilities_preserve_identity_across_reserved_descriptor_slots() {
    for slot in [198, 199, 200] {
        // Isolate this namespace experiment from the enclosing launch tree.
        // Inherited lease descriptors remain open; only their carrier is absent.
        use std::os::unix::process::CommandExt;
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "reserved_descriptor_helper", "--nocapture"])
            .env("CHRONO_FD_RESERVED_SLOT", slot.to_string())
            .env_remove("CHRONO_PROCESS_FDS");
        unsafe {
            command.pre_exec(|| {
                for fd in [198, 199, 200] {
                    if libc::close(fd) < 0 {
                        let error = std::io::Error::last_os_error();
                        if error.raw_os_error() != Some(libc::EBADF) {
                            return Err(error);
                        }
                    }
                }
                Ok(())
            });
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "slot {slot}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
