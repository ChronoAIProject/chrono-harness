#[test]
fn native_holder() {
    if let Some(root) = std::env::var_os("CHRONO_TEMPORARY_CARGO_ROOT") {
        let status =
            std::process::Command::new(std::env::var_os("CHRONO_TEMPORARY_HELPER").unwrap())
                .arg("custody-holder")
                .status()
                .unwrap();
        assert!(status.success());
        std::fs::write(
            std::path::Path::new(&root).join("cargo-native-joined"),
            "actual native child joined",
        )
        .unwrap();
        return;
    }
    std::env::set_current_dir(std::env::var("CHRONO_NATIVE_FIXTURE_ROOT").unwrap()).unwrap();
    // Cargo and its managed launcher deliberately die in this fixture. Keep
    // this native process's actual output, including libtest's late progress,
    // in its own originals before acknowledging readiness.
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn dup2(old: i32, new: i32) -> i32;
    }
    for (path, descriptor) in [
        (".chrono-harness/state/native.stdout", 1),
        (".chrono-harness/state/native.stderr", 2),
    ] {
        let original = std::fs::File::create_new(path).unwrap();
        assert_eq!(
            unsafe { dup2(original.as_raw_fd(), descriptor) },
            descriptor,
            "native output ownership: {}",
            std::io::Error::last_os_error()
        );
    }
    std::fs::write(
        ".chrono-harness/state/native-pid",
        std::process::id().to_string(),
    )
    .unwrap();
    std::fs::write(
        ".chrono-harness/state/native-holder",
        std::env::var("CHRONO_PROCESS_FDS").expect("native capability carrier"),
    )
    .unwrap();
    use std::io::{Read, Write};
    let mut synchronization = std::net::TcpStream::connect(
        std::fs::read_to_string(".chrono-harness/state/native-sync").unwrap(),
    )
    .unwrap();
    synchronization.write_all(b"ready").unwrap();
    let mut release = [0; 4];
    synchronization.read_exact(&mut release).unwrap();
    assert_eq!(&release, b"show");
    let progress = || -> std::io::Result<()> {
        std::io::stdout().write_all(b"original native progress after launcher exit\n")?;
        std::io::stdout().flush()?;
        std::io::stderr().write_all(b"original native diagnostic after launcher exit\n")?;
        std::io::stderr().flush()
    };
    if let Err(error) = progress() {
        std::fs::write(
            ".chrono-harness/state/native-io-error",
            format!("{error:?}"),
        )
        .unwrap();
        synchronization.write_all(b"error").unwrap();
        panic!("native output: {error:?}");
    }
    synchronization.write_all(b"shown").unwrap();
    synchronization.read_exact(&mut release).unwrap();
    assert_eq!(&release, b"join");
    assert!(std::path::Path::new(".chrono-harness/state/native-release").exists());
    std::fs::write(".chrono-harness/state/native-done", "joined").unwrap();
    synchronization.write_all(b"done").unwrap();
}
