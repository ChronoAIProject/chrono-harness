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
