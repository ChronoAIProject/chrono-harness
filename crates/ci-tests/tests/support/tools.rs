use std::path::{Path, PathBuf};

pub fn fixture_git() -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    chrono_harness::resolve_program(&source, "git", None).expect("Git fixture tool on PATH")
}

/// Produce the fixed workflow merge object without changing the fixture checkout.
pub fn pr_workflow_merge(root: &Path, base: &str, head: &str) -> String {
    let git = fixture_git();
    let tree = std::process::Command::new(&git)
        .arg("-C")
        .arg(root)
        .args(["rev-parse", &format!("{head}^{{tree}}")])
        .output()
        .unwrap();
    assert!(
        tree.status.success(),
        "{}",
        String::from_utf8_lossy(&tree.stderr)
    );
    let tree = String::from_utf8(tree.stdout).unwrap();
    let merge = std::process::Command::new(&git)
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit-tree",
            tree.trim(),
            "-p",
            base,
            "-p",
            head,
            "-m",
            "fixed PR workflow merge",
        ])
        .output()
        .unwrap();
    assert!(
        merge.status.success(),
        "{}",
        String::from_utf8_lossy(&merge.stderr)
    );
    String::from_utf8(merge.stdout).unwrap().trim().into()
}

/// Event fixtures commit this policy before either endpoint is created.
pub fn write_event_facts(root: &Path) {
    let program = fixture_git();
    let version = std::process::Command::new(&program)
        .arg("--version")
        .output()
        .unwrap();
    assert!(version.status.success());
    let facts = serde_json::json!({"schema_version":3,
        "facts_git":{"tool":"git","input":"git-bytes"},
        "tools":[{"id":"git","program":program,"resolution":"PATH-once",
            "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
        "environment":{"inherit":["PATH"],"values":{"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
            "inputs":[{"id":"git-bytes","presence":"present","location":program,
                "sha256":chrono_harness::sha256(&std::fs::read(&program).unwrap())}]},
        "protocol":{"timeout_seconds":10,"stdout_limit_bytes":1048576}});
    std::fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    std::fs::write(
        root.join(".chrono-harness/event-facts.json"),
        serde_json::to_vec_pretty(&facts).unwrap(),
    )
    .unwrap();
}

/// Only explicitly selected executing fixtures use the retained producer.
pub fn temporary_host(prefix: &str) -> FixtureDirectory {
    FixtureDirectory::Retained(
        chrono_worktree::TemporaryHost::registered_allocate(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            ".chrono-harness/worktree.json",
            "ci-test-host",
            prefix,
        )
        .unwrap(),
    )
}
fn temporary_store() -> &'static (PathBuf, Vec<String>) {
    static STORE: std::sync::OnceLock<(PathBuf, Vec<String>)> = std::sync::OnceLock::new();
    STORE.get_or_init(|| {
        chrono_worktree::TemporaryHost::registered_store(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            ".chrono-harness/worktree.json",
            "ci-test-host",
        )
        .expect("committed selected CI fixture producer")
    })
}
pub fn copied_fixture_input(root: &Path, source: &Path, destination: &Path, recipe: &str) {
    chrono_worktree::TemporaryHost::register_copy(
        &temporary_store().0,
        "ci-test-host",
        source,
        destination,
        Some(root),
        recipe,
    )
    .unwrap();
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
    pub fn capture_output(
        &self,
        command: &mut std::process::Command,
    ) -> Result<std::process::Output, String> {
        match self {
            Self::Ephemeral(_) => command.output().map_err(|e| e.to_string()),
            Self::Retained(host) => host.capture_output(command),
        }
    }
    pub fn retain_diagnostic(&self) -> Result<(), String> {
        if let Self::Retained(host) = self {
            host.retain_diagnostic()?;
        }
        Ok(())
    }
}
impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        if let Self::Retained(host) = self {
            match host.finish(std::thread::panicking()) {
                Ok(result) if result["status"] == "finished" => (),
                result => eprintln!("CI fixture handoff failed; allocation preserved: {result:?}"),
            }
        }
    }
}
