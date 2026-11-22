#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::{policy::download as archive, tar_fixture};
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::download as archive;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn only_external_absolute_storage_is_eligible() {
    let valid = archive::shared_location_valid(true, false);
    let local = archive::shared_location_valid(true, true);
    let relative = archive::shared_location_valid(false, false);
    let both = archive::shared_location_valid(false, true);
    assert(valid && !local && !relative && !both);
    #[cfg(not(verus_keep_ghost))]
    assert!(valid && !local && !relative && !both);
}
#[cfg(verus_keep_ghost)]
fn mock_tar_requires_the_registered_extraction_arguments() {
    let valid = tar_fixture::valid_tar_command(7, true, true);
    let extra = tar_fixture::valid_tar_command(8, true, true);
    let missing_root = tar_fixture::valid_tar_command(7, false, true);
    let options = tar_fixture::valid_tar_command(7, true, false);
    assert(valid && !extra && !missing_root && !options);
}
}

#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    os::unix::fs::symlink,
    path::Path,
    process::{Command, Output},
};

#[cfg(not(verus_keep_ghost))]
const ARCHIVE: &str = "package.zip";
#[cfg(not(verus_keep_ghost))]
const CHECKSUM: &str = ".chrono-harness/ci/package.sha256";
#[cfg(not(verus_keep_ghost))]
const HEADERS: &str = ".chrono-harness/state/download.headers";
#[cfg(not(verus_keep_ghost))]
const TEMPORARIES: &str = ".chrono-harness/cache/bootstrap-owner-tooling/tmp";
#[cfg(not(verus_keep_ghost))]
const BODY: &[u8] = b"verified archive\x00\xff\n";

#[cfg(not(verus_keep_ghost))]
struct Fixture(tempfile::TempDir, tempfile::TempDir);
#[cfg(not(verus_keep_ghost))]
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.disable_cleanup(true);
            eprintln!("Retained download fixture: {}", self.0.path().display());
        }
    }
}
#[cfg(not(verus_keep_ghost))]
impl Fixture {
    fn new() -> Self {
        let fixture = Self(tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fs::create_dir_all(fixture.archive().parent().unwrap()).unwrap();
        for directory in [".chrono-harness/ci", ".chrono-harness/state", TEMPORARIES] {
            fs::create_dir_all(fixture.0.path().join(directory)).unwrap();
        }
        fs::write(fixture.0.path().join("download-body"), BODY).unwrap();
        fixture.checksum(&format!("{}  {ARCHIVE}\n", chrono_harness::sha256(BODY)));
        fixture
    }
    fn archive(&self) -> std::path::PathBuf {
        self.1
            .path()
            .canonicalize()
            .unwrap()
            .join("verus")
            .join(chrono_harness::sha256(BODY))
            .join(ARCHIVE)
    }
    fn checksum(&self, value: &str) {
        fs::write(self.0.path().join(CHECKSUM), value).unwrap();
    }
    fn invoke(&self, curl: &str, url: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
        command.env("CHRONO_TOOLCHAIN_CACHE", self.1.path());
        command.env("TMPDIR", self.0.path().join(TEMPORARIES));
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
    fn assert_disposable_download(&self) {
        let args: Vec<String> =
            serde_json::from_slice(&fs::read(self.0.path().join("download-calls.json")).unwrap())
                .unwrap();
        let output = Path::new(&args[args.iter().position(|arg| arg == "--output").unwrap() + 1]);
        assert_eq!(output.parent(), self.archive().parent());
        assert!(!output.exists());
        assert_eq!(
            fs::read_dir(self.0.path().join(TEMPORARIES))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
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
    assert_eq!(fs::read(fixture.archive()).unwrap(), BODY);
    fixture.assert_disposable_download();
    assert_eq!(
        fs::read_dir(fixture.archive().parent().unwrap())
            .unwrap()
            .count(),
        1
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn verified_cache_never_starts_the_downloader() {
    let fixture = Fixture::new();
    fs::write(fixture.archive(), BODY).unwrap();
    let output = fixture.invoke("/no/such/downloader", "https://example.invalid/archive.zip");
    assert!(output.status.success(), "{:?}", output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "cache-hit");
    assert_eq!(report["sha256"], chrono_harness::sha256(BODY));
    assert_eq!(fs::read(fixture.archive()).unwrap(), BODY);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn missing_or_corrupt_archive_is_verified_before_replacement_then_reused() {
    for old in [None, Some(b"partial archive".as_slice())] {
        let fixture = Fixture::new();
        if let Some(old) = old {
            fs::write(fixture.archive(), old).unwrap();
        }
        let output = fixture.run();
        assert!(output.status.success(), "{:?}", output);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"], "downloaded");
        assert_eq!(fs::read(fixture.archive()).unwrap(), BODY);
        fixture.assert_disposable_download();
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
#[cfg(not(verus_keep_ghost))]
fn download_failure_keeps_original_exit_headers_and_previous_archive() {
    let fixture = Fixture::new();
    fs::write(fixture.archive(), b"previous archive").unwrap();
    fs::write(fixture.0.path().join("download-exit"), b"56").unwrap();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(56), "{:?}", output);
    fixture.assert_disposable_download();
    assert!(
        output
            .stderr
            .windows(b"fixture download failed".len())
            .any(|s| s == b"fixture download failed")
    );
    assert_eq!(fs::read(fixture.archive()).unwrap(), b"previous archive");
    assert_eq!(
        fs::read(fixture.0.path().join(HEADERS)).unwrap(),
        b"fixture response headers\n"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn wrong_download_content_is_rejected_without_publishing_or_overwriting() {
    for old in [None, Some(b"previous archive".as_slice())] {
        let fixture = Fixture::new();
        if let Some(old) = old {
            fs::write(fixture.archive(), old).unwrap();
        }
        fs::write(fixture.0.path().join("download-body"), b"wrong download").unwrap();
        let output = fixture.run();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("checksum mismatch"));
        fixture.assert_disposable_download();
        assert_eq!(fs::read(fixture.archive()).ok().as_deref(), old);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
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
    symlink(fixture.0.path().join("download-body"), fixture.archive()).unwrap();
    assert!(!fixture.run().status.success());
    assert!(!fixture.0.path().join("download-calls.json").exists());
    assert_eq!(
        fs::read(fixture.0.path().join("download-body")).unwrap(),
        BODY
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn two_checkouts_use_one_install_after_an_interrupted_extraction() {
    let first = Fixture::new();
    let second = Fixture::new();
    fs::write(first.archive(), BODY).unwrap();
    let extract = |checkout: &Fixture, program: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
        command.env("CHRONO_TOOLCHAIN_CACHE", first.1.path());
        command.args([
            "extract",
            checkout.0.path().to_str().unwrap(),
            CHECKSUM,
            program,
        ]);
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        command.output().unwrap()
    };
    let tar = env!("CARGO_BIN_EXE_chrono-test-bootstrap-download");
    fs::write(first.0.path().join("fail-extraction"), b"").unwrap();
    assert_eq!(extract(&first, tar).status.code(), Some(42));
    let target = first.archive().parent().unwrap().join("toolchain/verus");
    assert!(!target.exists());
    fs::remove_file(first.0.path().join("fail-extraction")).unwrap();
    assert!(extract(&first, tar).status.success());
    assert!(extract(&second, "/unused-on-reuse").status.success());
    assert_eq!(fs::read(&target).unwrap(), BODY);
    for checkout in [&first, &second] {
        assert_eq!(
            fs::read_link(checkout.0.path().join(".chrono-harness/bin/verus-compiler")).unwrap(),
            target
        );
        assert!(
            !checkout
                .0
                .path()
                .join(".chrono-harness/bin/verus.zip")
                .exists()
        );
    }
}
