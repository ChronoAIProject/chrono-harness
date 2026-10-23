use serde_json::Value;
use std::path::{Path, PathBuf};

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

/// Only explicitly selected executing fixtures use the retained producer.
pub fn temporary_host(prefix: &str) -> FixtureDirectory {
    let (directory, outputs) = temporary_store();
    FixtureDirectory::Retained(chrono_worktree::TemporaryHost::allocate(directory, "ci-test-host", outputs, prefix).unwrap())
}
fn temporary_store() -> &'static (PathBuf, Vec<String>) {
    static STORE: std::sync::OnceLock<(PathBuf, Vec<String>)> = std::sync::OnceLock::new();
    STORE.get_or_init(|| chrono_worktree::TemporaryHost::registered_store(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        ".chrono-harness/worktree.json", "ci-test-host",
    ).expect("committed selected CI fixture producer"))
}
pub fn copied_fixture_input(root: &Path, source: &Path, destination: &Path) {
    chrono_worktree::TemporaryHost::register_copy(
        &temporary_store().0, "ci-test-host", source, destination, Some(root),
        "crates/ci-tests/tests/units.rs::install_retained_tools",
    ).unwrap();
}

pub enum FixtureDirectory {
    Ephemeral(tempfile::TempDir),
    Retained(chrono_worktree::TemporaryHost),
}
impl FixtureDirectory {
    pub fn path(&self) -> &Path {
        match self {
            Self::Ephemeral(host) => host.path(),
            Self::Retained(host) => host.path(),
        }
    }
    pub fn bind_command(&self, command: &mut std::process::Command) -> Result<(), String> {
        match self {
            Self::Ephemeral(_) => Ok(()),
            Self::Retained(host) => host.bind_command(command),
        }
    }
    pub fn capture_output(&self, command: &mut std::process::Command) -> Result<std::process::Output, String> {
        match self {
            Self::Ephemeral(_) => command.output().map_err(|e| e.to_string()),
            Self::Retained(host) => host.capture_output(command),
        }
    }
    pub fn retain_diagnostic(&self) -> Result<(), String> {
        if let Self::Retained(host) = self {
            chrono_worktree::RetainedArtifact::resume(
                host.path().parent().unwrap(), "ci-test-host", host.path(),
            )?.acquire("fixture-diagnostic", "actual failed short fixture diagnostics remain to be consumed")?;
        }
        Ok(())
    }
}
