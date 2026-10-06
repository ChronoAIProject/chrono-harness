use super::*;
use std::os::unix::fs::PermissionsExt;

fn payload() -> (tempfile::TempDir, String) {
    let (dir, _, _) = fixture();
    fs::write(dir.path().join("payload"), b"canonical\n").unwrap();
    let candidate = commit(dir.path());
    (dir, candidate)
}

#[test]
fn raw_checkout_rejects_mode_hidden_by_local_git_configuration() {
    let (dir, candidate) = payload();
    let root = dir.path();
    git(root, &["config", "core.filemode", "false"]);
    fs::set_permissions(root.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(git(root, &["diff", "--name-only", &candidate, "--"]).is_empty());
    let observed = facts::checkout(root, &candidate).unwrap();
    assert!(observed.tracked.contains(&"payload".into()), "{observed:?}");
}

#[test]
fn raw_checkout_rejects_bytes_hidden_by_clean_filter_without_running_filter() {
    let (dir, _) = payload();
    let root = dir.path();
    fs::write(root.join(".gitattributes"), "payload filter=hide\n").unwrap();
    let filter = env!("CARGO_BIN_EXE_chrono-test-cli-child");
    let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
    git(
        root,
        &[
            "config",
            "filter.hide.clean",
            &format!("{} filter-clean", quote(filter)),
        ],
    );
    let candidate = commit(root);
    fs::write(root.join("payload"), b"uncommitted actual bytes\0\xff").unwrap();
    assert!(git(root, &["diff", "--name-only", &candidate, "--"]).is_empty());
    // A snapshot check must not delegate byte identity to a clean filter.
    git(
        root,
        &[
            "config",
            "filter.hide.clean",
            &format!("{} filter-fail", quote(filter)),
        ],
    );
    git(root, &["config", "filter.hide.required", "true"]);
    let observed = facts::checkout(root, &candidate)
        .expect("raw facts must not run the configured clean filter");
    assert!(observed.tracked.contains(&"payload".into()), "{observed:?}");
}

#[test]
fn raw_checkout_rejects_line_endings_hidden_by_git_conversion() {
    let (dir, _) = payload();
    let root = dir.path();
    fs::write(root.join(".gitattributes"), "payload text\n").unwrap();
    let candidate = commit(root);
    fs::write(root.join("payload"), b"canonical\r\n").unwrap();
    assert!(git(root, &["diff", "--name-only", &candidate, "--"]).is_empty());
    let observed = facts::checkout(root, &candidate).unwrap();
    assert!(observed.tracked.contains(&"payload".into()), "{observed:?}");
}

#[test]
fn raw_checkout_checks_symlinks_types_and_literal_parent_paths() {
    use std::os::unix::fs::symlink;
    let (dir, _) = payload();
    let root = dir.path();
    fs::create_dir(root.join("parent")).unwrap();
    fs::write(root.join("parent/file"), b"contents\0\xff").unwrap();
    symlink("missing target", root.join("alias")).unwrap();
    let candidate = commit(root);
    assert!(
        facts::checkout(root, &candidate)
            .unwrap()
            .tracked
            .is_empty()
    );
    fs::remove_file(root.join("alias")).unwrap();
    fs::write(root.join("alias"), "missing target").unwrap();
    git(root, &["config", "core.symlinks", "false"]);
    assert!(git(root, &["diff", "--name-only", &candidate, "--"]).is_empty());
    assert!(
        facts::checkout(root, &candidate)
            .unwrap()
            .tracked
            .contains(&"alias".into())
    );
    fs::rename(root.join("parent"), root.join("elsewhere")).unwrap();
    symlink("elsewhere", root.join("parent")).unwrap();
    fs::remove_file(root.join("payload")).unwrap();
    fs::create_dir(root.join("payload")).unwrap();
    let observed = facts::checkout(root, &candidate).unwrap();
    for name in ["alias", "parent/file", "payload"] {
        assert!(observed.tracked.contains(&name.into()), "{observed:?}");
    }
}

#[test]
fn raw_checkout_supports_sha256_and_keeps_index_changes_separate() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "--object-format=sha256"]);
    let bytes = vec![0xabu8; 1024 * 1024 + 13];
    fs::write(root.join("payload"), &bytes).unwrap();
    let candidate = commit(root);
    assert_eq!(candidate.len(), 64);
    assert!(
        facts::checkout(root, &candidate)
            .unwrap()
            .tracked
            .is_empty()
    );
    fs::write(root.join("payload"), b"staged replacement").unwrap();
    git(root, &["add", "payload"]);
    fs::write(root.join("payload"), &bytes).unwrap();
    assert_eq!(
        facts::checkout(root, &candidate).unwrap().tracked,
        vec!["payload"]
    );
    git(root, &["reset", "--mixed", &candidate]);
    fs::remove_file(root.join("payload")).unwrap();
    assert_eq!(
        facts::checkout(root, &candidate).unwrap().tracked,
        vec!["payload"]
    );
}
