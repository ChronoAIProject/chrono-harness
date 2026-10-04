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
    let script = "import os,sys,fcntl; fds=[int(v) for v in os.environ['CHRONO_PROCESS_FDS'].split(',')]; assert len(fds)==int(sys.argv[1]); assert all(fcntl.fcntl(fd,fcntl.F_GETFD)&fcntl.FD_CLOEXEC==0 for fd in fds); os.write(fds[-1],b'child-capability'); print(os.environ['CHRONO_PROCESS_FDS'])";
    let mut command = spec(
        "/usr/bin/python3",
        vec![
            "-c".into(),
            script.into(),
            (inherited_count() + 1).to_string(),
        ],
    );
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
    assert_eq!(observed["ownership_fds"], result.stdout.trim());
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
    let command = spec("/usr/bin/python3", vec!["-c".into(), "import os,sys; value=os.environ.get('CHRONO_PROCESS_FDS'); assert (len(value.split(',')) if value else 0)==int(sys.argv[1])".into(), inherited_count().to_string()]);
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
    let script = "import os,sys; fds=[int(v) for v in os.environ['CHRONO_PROCESS_FDS'].split(',')]; assert len(fds)==int(sys.argv[1]); os.write(fds[-1],b'nested-env-clear'); print('nested-forwarded')";
    let command = spec(
        "/usr/bin/python3",
        vec!["-c".into(), script.into(), inherited_count().to_string()],
    );
    let result = run_process_observed(
        Path::new("/"),
        &command,
        &[],
        &sha256(&fs::read(&command.program).unwrap()),
    )
    .unwrap();
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(
        descriptors
            .iter()
            .map(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) })
            .collect::<Vec<_>>(),
        flags,
        "engine calls must not mutate inherited descriptors used by other threads or native children"
    );
    let direct = std::process::Command::new("/usr/bin/python3")
        .args(["-c", "import os; fds=[int(v) for v in os.environ['CHRONO_PROCESS_FDS'].split(',')]; os.write(fds[-1],b'native-after-engine')"])
        .output().unwrap();
    assert!(
        direct.status.success(),
        "{}",
        String::from_utf8_lossy(&direct.stderr)
    );
    println!("{}", result.stdout);
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
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert!(result.stdout.contains("nested-forwarded"));
    file.rewind().unwrap();
    let mut bytes = String::new();
    file.read_to_string(&mut bytes).unwrap();
    assert_eq!(bytes, "nested-env-clearnative-after-engine");
}
