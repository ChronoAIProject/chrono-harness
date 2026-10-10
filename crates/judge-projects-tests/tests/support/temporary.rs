pub fn temporary_host(prefix: &str) -> chrono_worktree::TemporaryHost {
    static STORE: std::sync::OnceLock<(std::path::PathBuf, Vec<String>)> =
        std::sync::OnceLock::new();
    let (directory, outputs) = STORE.get_or_init(|| {
        chrono_worktree::TemporaryHost::registered_store(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            ".chrono-harness/worktree.json",
            "projects-test-host",
        )
        .expect("committed temporary producer adoption")
    });
    chrono_worktree::TemporaryHost::allocate(directory, "projects-test-host", outputs, prefix)
        .expect("prospective temporary host custody")
}

pub fn copied_fixture_input(
    root: &std::path::Path,
    source: &std::path::Path,
    destination: &std::path::Path,
) {
    static STORE: std::sync::OnceLock<(std::path::PathBuf, Vec<String>)> =
        std::sync::OnceLock::new();
    let (directory, _) = STORE.get_or_init(|| {
        chrono_worktree::TemporaryHost::registered_store(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            ".chrono-harness/worktree.json",
            "projects-test-host",
        )
        .unwrap()
    });
    chrono_worktree::TemporaryHost::register_copy(
        directory,
        "projects-test-host",
        source,
        destination,
        Some(root),
        "crates/judge-projects-tests/tests/support/host.rs::install_program",
    )
    .unwrap();
}

/// The Git checkout may be absent during offline collection, while its
/// prospective allocation, TMPDIR and captured originals retain their lifetime.
pub struct FixtureDirectory {
    allocation: chrono_worktree::TemporaryHost,
    root: std::path::PathBuf,
}
impl FixtureDirectory {
    pub fn path(&self) -> &std::path::Path {
        &self.root
    }
    pub fn temporary_path(&self) -> &std::path::Path {
        self.allocation.path()
    }
    /// The caller has joined the checkout's producers and transported their
    /// reports. Preserve original bytes while making the old Git path unavailable.
    pub fn make_checkout_unavailable(&self) -> std::io::Result<std::path::PathBuf> {
        let retained = self.allocation.path().join("retained-offline-checkout");
        chrono_worktree::TemporaryHost::relocate_copies(
            self.allocation.path().parent().unwrap(),
            "projects-test-host",
            &self.root,
            &retained,
        )
        .map_err(std::io::Error::other)?;
        std::fs::rename(&self.root, &retained)?;
        Ok(retained)
    }
    pub fn capture_output(
        &self,
        command: &mut std::process::Command,
    ) -> Result<std::process::Output, String> {
        self.allocation.capture_output(command)
    }
}
pub fn fixture_directory(prefix: &str) -> FixtureDirectory {
    let allocation = temporary_host(prefix);
    let root = allocation.path().join("fixture");
    std::fs::create_dir(&root).expect("declared fixture Git root");
    FixtureDirectory { allocation, root }
}
