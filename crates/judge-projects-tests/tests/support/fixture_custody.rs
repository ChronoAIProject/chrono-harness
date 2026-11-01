//! Expectations at the real shared fixture publisher boundary, in consumer.
use super::*;
use chrono_worktree::RetainedArtifact;
use std::path::Path;

fn captures(allocation: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(allocation.join(".chrono-harness/state/command-captures"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect()
}

#[test]
fn prospective_host_preserves_source_external_input_and_copy_original_offline() {
    let directory = host::fixture_directory("projects custody offline λ ");
    let allocation = directory.temporary_path().to_owned();
    let root = directory.path().to_owned();
    git(&root, &["init", "-q"]);
    fs::write(root.join("source.txt"), b"unique source\0\xff").unwrap();
    fs::write(root.join(".gitignore"), ".chrono-harness/\n").unwrap();
    let head = commit(&root);
    let input = allocation.join("declared-external-input");
    fs::write(&input, b"external\0\xfe").unwrap();
    let source = allocation.join("copy-source");
    fs::write(&source, b"fixture bytes\0\xfd").unwrap();
    let destination = root.join(".chrono-harness/bin/copied-input");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    host::copied_fixture_input(&root, &source, &destination);
    // Publication is prospective: a missing destination is not invented.
    assert!(!destination.exists());
    fs::copy(&source, &destination).unwrap();
    fs::remove_file(&source).unwrap();
    let retained = directory.make_checkout_unavailable().unwrap();
    assert!(!root.exists());
    assert_eq!(git(&retained, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read(retained.join("source.txt")).unwrap(),
        b"unique source\0\xff"
    );
    assert_eq!(fs::read(&input).unwrap(), b"external\0\xfe");
    assert_eq!(
        fs::read(retained.join(".chrono-harness/bin/copied-input")).unwrap(),
        b"fixture bytes\0\xfd"
    );
    let duplicate = directory.make_checkout_unavailable().unwrap_err();
    assert_eq!(duplicate.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(retained.join(".git").is_dir());
}

#[test]
fn native_spawn_failure_keeps_original_empty_streams_and_no_joined_result() {
    let directory = host::fixture_directory("projects custody spawn failure ");
    let result =
        directory.capture_output(&mut Command::new(directory.path().join("absent-program")));
    assert!(result.is_err());
    let original = captures(directory.temporary_path());
    assert_eq!(original.len(), 1);
    assert_eq!(fs::read(original[0].join("stdout.bytes")).unwrap(), b"");
    assert_eq!(fs::read(original[0].join("stderr.bytes")).unwrap(), b"");
    assert!(original[0].join("intent.json").is_file());
    assert!(!original[0].join("started.json").exists());
    assert!(!original[0].join("result.json").exists());
}

#[test]
fn failed_assertion_retains_diagnostic_before_allocation_custody_releases() {
    let directory = host::fixture_directory("projects custody diagnostic ");
    let allocation = directory.temporary_path().to_owned();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _directory = directory;
        panic!("controlled fixture assertion failure");
    }));
    assert!(outcome.is_err());
    let producer = RetainedArtifact::resume(
        allocation.parent().unwrap(),
        "projects-test-host",
        &allocation,
    )
    .unwrap();
    // Release must find an actual unreleased reference produced during unwind.
    producer
        .release(
            "fixture-diagnostic",
            "controlled diagnostic assertions consumed",
        )
        .unwrap();
    assert!(allocation.is_dir());
}
