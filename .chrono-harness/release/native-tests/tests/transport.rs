//! 执行物的原 tar 接口；预期来自独立文件内容及 mode，不使用新实现生成预期。
#[path = "support/archive.rs"]
mod archive;

use archive::Entry;
use chrono_host_release::transport::{self, ArchiveIdentity, ExecutableIdentity};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

fn executable(root: &Path, relative: &str, bytes: &[u8], mode: u32) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    path
}

fn archive_identity(path: &Path) -> ArchiveIdentity {
    let (sha256, size) = chrono_harness::file_identity(path).unwrap();
    ArchiveIdentity { sha256, size }
}

fn expected(bytes: &[u8], mode: u32) -> ExecutableIdentity {
    ExecutableIdentity {
        sha256: chrono_harness::sha256(bytes),
        size: bytes.len() as u64,
        mode,
    }
}

#[test]
fn large_executable_streams_between_independent_roots_with_exact_mode() {
    let source = tempfile::tempdir().unwrap();
    let handoff = tempfile::tempdir().unwrap();
    let consumer = tempfile::tempdir().unwrap();
    let size = 64 * 1024 * 1024 + 4097;
    let source_path = executable(source.path(), "out/tool", b"", 0o751);
    let mut file = OpenOptions::new().write(true).open(&source_path).unwrap();
    file.set_len(size).unwrap();
    file.write_all(&[0, 255, 10]).unwrap();
    file.seek(SeekFrom::End(-3)).unwrap();
    file.write_all(&[254, 1, 0]).unwrap();
    drop(file);
    let source_hash = chrono_harness::file_identity(&source_path).unwrap().0;
    let package = handoff.path().join("executable.tar");
    let packed = transport::pack(source.path(), "out/tool", &package).unwrap();
    assert_eq!(packed.executable.sha256, source_hash);
    assert_eq!(packed.executable.size, size);
    assert_eq!(packed.executable.mode, 0o751);
    assert!(packed.archive.size > 64 * 1024 * 1024);
    let destination = executable(consumer.path(), "out/tool", b"previous", 0o700);
    let received = transport::import(
        &package, &packed.archive, &packed.executable, consumer.path(), "out/tool",
    ).unwrap();
    assert_eq!(received.size, size);
    assert_eq!(received.sha256, source_hash);
    assert_eq!(fs::metadata(&destination).unwrap().permissions().mode() & 0o7777, 0o751);
    assert_eq!(chrono_harness::file_identity(&destination).unwrap().0, source_hash);
    let mut file = File::open(destination).unwrap();
    let mut bytes = [0; 3];
    file.read_exact(&mut bytes).unwrap();
    assert_eq!(bytes, [0, 255, 10]);
    file.seek(SeekFrom::End(-3)).unwrap();
    file.read_exact(&mut bytes).unwrap();
    assert_eq!(bytes, [254, 1, 0]);
}

#[test]
fn existing_archive_and_non_executable_or_symlink_sources_are_preserved() {
    let root = tempfile::tempdir().unwrap();
    executable(root.path(), "tool", b"tool", 0o755);
    let package = root.path().join("executable.tar");
    fs::write(&package, b"original archive").unwrap();
    assert!(transport::pack(root.path(), "tool", &package).is_err());
    assert_eq!(fs::read(&package).unwrap(), b"original archive");
    executable(root.path(), "data", b"data", 0o644);
    let absent = root.path().join("absent.tar");
    assert!(transport::pack(root.path(), "data", &absent).is_err());
    assert!(!absent.exists());
    symlink(root.path().join("tool"), root.path().join("alias")).unwrap();
    assert!(transport::pack(root.path(), "alias", &absent).is_err());
    assert!(!absent.exists());
}

#[test]
fn incorrect_archive_or_executable_identity_never_replaces_the_consumer() {
    let root = tempfile::tempdir().unwrap();
    executable(root.path(), "tool", b"source", 0o751);
    let package = root.path().join("executable.tar");
    let packed = transport::pack(root.path(), "tool", &package).unwrap();
    let consumer = executable(root.path(), "consumer/tool", b"previous", 0o700);
    for variant in 0..5 {
        let mut archive = packed.archive.clone();
        let mut identity = packed.executable.clone();
        match variant {
            0 => archive.sha256 = "0".repeat(64),
            1 => archive.size += 1,
            2 => identity.sha256 = "0".repeat(64),
            3 => identity.size += 1,
            4 => identity.mode = 0o755,
            _ => unreachable!(),
        }
        assert!(transport::import(&package, &archive, &identity, root.path(), "consumer/tool").is_err(), "variant {variant}");
        assert_eq!(fs::read(&consumer).unwrap(), b"previous");
        assert_eq!(fs::metadata(&consumer).unwrap().permissions().mode() & 0o7777, 0o700);
    }
}

#[test]
fn archive_members_must_be_one_regular_literal_executable() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("executable.tar");
    let previous = executable(root.path(), "consumer", b"previous", 0o700);
    let valid = || Entry { name: b"executable", kind: tar::EntryType::Regular, mode: 0o751, bytes: b"source" };
    for variant in 0..7 {
        let entries = match variant {
            0 => vec![],
            1 => vec![Entry { name: b"other", ..valid() }],
            2 => vec![Entry { name: b"dir/executable", ..valid() }],
            3 => vec![Entry { name: b"../executable", ..valid() }],
            4 => vec![Entry { kind: tar::EntryType::Symlink, bytes: b"", ..valid() }],
            5 => vec![valid(), valid()],
            6 => vec![Entry { kind: tar::EntryType::Directory, bytes: b"", ..valid() }],
            _ => unreachable!(),
        };
        archive::write(&package, &entries);
        assert!(transport::import(&package, &archive_identity(&package), &expected(b"source", 0o751), root.path(), "consumer").is_err(), "variant {variant}");
        assert_eq!(fs::read(&previous).unwrap(), b"previous");
    }
}

#[test]
fn changed_archive_and_rehashed_wrong_payload_are_both_rejected() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("executable.tar");
    executable(root.path(), "tool", b"source", 0o751);
    let packed = transport::pack(root.path(), "tool", &package).unwrap();
    let previous = executable(root.path(), "consumer", b"previous", 0o700);
    let mut file = OpenOptions::new().write(true).open(&package).unwrap();
    file.seek(SeekFrom::Start(512)).unwrap();
    file.write_all(b"broken").unwrap();
    drop(file);
    for archive in [&packed.archive, &archive_identity(&package)] {
        assert!(transport::import(&package, archive, &packed.executable, root.path(), "consumer").is_err());
        assert_eq!(fs::read(&previous).unwrap(), b"previous");
    }
}

#[test]
fn truncated_payload_and_symlink_destination_leave_existing_work_intact() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("executable.tar");
    archive::write(&package, &[Entry { name: b"executable", kind: tar::EntryType::Regular, mode: 0o751, bytes: b"source" }]);
    OpenOptions::new().write(true).open(&package).unwrap().set_len(517).unwrap();
    let previous = executable(root.path(), "consumer", b"previous", 0o700);
    assert!(transport::import(&package, &archive_identity(&package), &expected(b"source", 0o751), root.path(), "consumer").is_err());
    assert_eq!(fs::read(&previous).unwrap(), b"previous");
    archive::write(&package, &[Entry { name: b"executable", kind: tar::EntryType::Regular, mode: 0o751, bytes: b"source" }]);
    symlink(root.path().join("consumer"), root.path().join("alias")).unwrap();
    assert!(transport::import(&package, &archive_identity(&package), &expected(b"source", 0o751), root.path(), "alias").is_err());
    assert_eq!(fs::read(previous).unwrap(), b"previous");
}
