use serde_json::Value;
use std::{
    fs,
    os::unix::fs::symlink,
    process::{Command, Output},
};

const ARCHIVE: &str = ".chrono-harness/cache/bootstrap-verus/package.zip";
const CHECKSUM: &str = ".chrono-harness/ci/package.sha256";
const HEADERS: &str = ".chrono-harness/state/download.headers";
const BODY: &[u8] = b"verified archive\x00\xff\n";

struct Fixture(tempfile::TempDir);
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.disable_cleanup(true);
            eprintln!("Retained download fixture: {}", self.0.path().display());
        }
    }
}
impl Fixture {
    fn new() -> Self {
        let fixture = Self(tempfile::tempdir().unwrap());
        for directory in [
            ".chrono-harness/ci",
            ".chrono-harness/state",
            ".chrono-harness/cache/bootstrap-verus",
        ] {
            fs::create_dir_all(fixture.0.path().join(directory)).unwrap();
        }
        fs::write(fixture.0.path().join("download-body"), BODY).unwrap();
        fixture.checksum(&format!("{}  {ARCHIVE}\n", chrono_harness::sha256(BODY)));
        fixture
    }
    fn checksum(&self, value: &str) {
        fs::write(self.0.path().join(CHECKSUM), value).unwrap();
    }
    fn invoke(&self, curl: &str, url: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
        command.args([
            "download",
            self.0.path().to_str().unwrap(),
            CHECKSUM,
            curl,
            HEADERS,
            url,
        ]);
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        command.output().unwrap()
    }
    fn run(&self) -> Output {
        self.invoke(
            env!("CARGO_BIN_EXE_chrono-test-bootstrap-download"),
            "https://example.invalid/archive.zip",
        )
    }
}

#[test]
fn concurrent_preparation_publishes_only_verified_archives() {
    let fixture = Fixture::new();
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| fixture.run());
        let second = fixture.run();
        (first.join().unwrap(), second)
    });
    for output in [first, second] {
        assert!(output.status.success(), "{:?}", output);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["sha256"], chrono_harness::sha256(BODY));
    }
    assert_eq!(fs::read(fixture.0.path().join(ARCHIVE)).unwrap(), BODY);
    assert_eq!(
        fs::read_dir(fixture.0.path().join(ARCHIVE).parent().unwrap())
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn verified_cache_never_starts_the_downloader() {
    let fixture = Fixture::new();
    fs::write(fixture.0.path().join(ARCHIVE), BODY).unwrap();
    let output = fixture.invoke("/no/such/downloader", "https://example.invalid/archive.zip");
    assert!(output.status.success(), "{:?}", output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "cache-hit");
    assert_eq!(report["sha256"], chrono_harness::sha256(BODY));
    assert_eq!(fs::read(fixture.0.path().join(ARCHIVE)).unwrap(), BODY);
}

#[test]
fn missing_or_corrupt_archive_is_verified_before_replacement_then_reused() {
    for old in [None, Some(b"partial archive".as_slice())] {
        let fixture = Fixture::new();
        if let Some(old) = old {
            fs::write(fixture.0.path().join(ARCHIVE), old).unwrap();
        }
        let output = fixture.run();
        assert!(output.status.success(), "{:?}", output);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"], "downloaded");
        assert_eq!(fs::read(fixture.0.path().join(ARCHIVE)).unwrap(), BODY);
        let calls = fs::read(fixture.0.path().join("download-calls.json")).unwrap();
        let second = fixture.run();
        assert!(second.status.success(), "{:?}", second);
        assert_eq!(
            fs::read(fixture.0.path().join("download-calls.json")).unwrap(),
            calls
        );
        assert_eq!(
            fs::read(fixture.0.path().join(HEADERS)).unwrap(),
            b"fixture response headers\n"
        );
    }
}

#[test]
fn download_failure_keeps_original_exit_headers_and_previous_archive() {
    let fixture = Fixture::new();
    fs::write(fixture.0.path().join(ARCHIVE), b"previous archive").unwrap();
    fs::write(fixture.0.path().join("download-exit"), b"56").unwrap();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(56), "{:?}", output);
    assert!(
        output
            .stderr
            .windows(b"fixture download failed".len())
            .any(|s| s == b"fixture download failed")
    );
    assert_eq!(
        fs::read(fixture.0.path().join(ARCHIVE)).unwrap(),
        b"previous archive"
    );
    assert_eq!(
        fs::read(fixture.0.path().join(HEADERS)).unwrap(),
        b"fixture response headers\n"
    );
}

#[test]
fn wrong_download_content_is_rejected_without_publishing_or_overwriting() {
    for old in [None, Some(b"previous archive".as_slice())] {
        let fixture = Fixture::new();
        if let Some(old) = old {
            fs::write(fixture.0.path().join(ARCHIVE), old).unwrap();
        }
        fs::write(fixture.0.path().join("download-body"), b"wrong download").unwrap();
        let output = fixture.run();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("checksum mismatch"));
        assert_eq!(
            fs::read(fixture.0.path().join(ARCHIVE)).ok().as_deref(),
            old
        );
    }
}

#[test]
fn invalid_declarations_and_symlinks_fail_before_download() {
    let hash = chrono_harness::sha256(BODY);
    for checksum in [
        format!("{}  {ARCHIVE}\n", hash.to_uppercase()),
        format!("{hash}  ../escape.zip\n"),
        format!("{hash}  .chrono-harness/state/evidence.zip\n"),
        format!("{hash}  {ARCHIVE}\n{hash}  {ARCHIVE}\n"),
        format!("{hash} {ARCHIVE}\n"),
    ] {
        let fixture = Fixture::new();
        fixture.checksum(&checksum);
        assert!(!fixture.run().status.success(), "{checksum}");
        assert!(!fixture.0.path().join("download-calls.json").exists());
    }
    let fixture = Fixture::new();
    assert!(
        !fixture
            .invoke(
                env!("CARGO_BIN_EXE_chrono-test-bootstrap-download"),
                "http://example.invalid/archive.zip"
            )
            .status
            .success()
    );
    assert!(!fixture.0.path().join("download-calls.json").exists());
    symlink(
        fixture.0.path().join("download-body"),
        fixture.0.path().join(ARCHIVE),
    )
    .unwrap();
    assert!(!fixture.run().status.success());
    assert!(!fixture.0.path().join("download-calls.json").exists());
    assert_eq!(
        fs::read(fixture.0.path().join("download-body")).unwrap(),
        BODY
    );
}
