use serde_json::Value;
use std::path::{Path, PathBuf};

/// Actual native fixture routes forward the scopes already held by their host.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    command
}

pub fn fixture_git() -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry = source.join(".chrono-harness/tests/ci-tools.json");
    let tools: Value =
        serde_json::from_slice(&std::fs::read(registry).expect("registered CI test tools"))
            .unwrap();
    assert_eq!(tools["schema"], "chrono-ci-test-tools/v1");
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let program = tools["platforms"][platform]["git"]
        .as_str()
        .expect("Git must be explicitly registered for this CI test platform");
    assert!(Path::new(program).is_absolute());
    chrono_harness::resolve_program(&source, program, None).unwrap()
}

pub fn temporary_host(prefix: &str) -> chrono_worktree::TemporaryHost {
    static STORE: std::sync::OnceLock<(std::path::PathBuf, Vec<String>)> =
        std::sync::OnceLock::new();
    let (directory, outputs) = STORE.get_or_init(|| {
        chrono_worktree::TemporaryHost::registered_store(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            ".chrono-harness/worktree.json",
            "ci-test-host",
        )
        .expect("committed temporary producer adoption")
    });
    chrono_worktree::TemporaryHost::allocate(directory, "ci-test-host", outputs, prefix)
        .expect("prospective temporary host custody")
}
