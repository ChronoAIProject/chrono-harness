#[test]
fn native_holder() {
    if let Some(allocation) = std::env::var_os("CHRONO_TEMPORARY_CARGO_ROOT") {
        let allocation = std::path::PathBuf::from(allocation);
        assert_eq!(std::env::temp_dir(), allocation);
        let mut child = std::process::Command::new(std::env::var_os("CHRONO_TEMPORARY_HELPER").unwrap())
            .arg("temporary-descendant").arg(&allocation).spawn().unwrap();
        assert!(child.wait().unwrap().success());
        std::fs::write(allocation.join("cargo-native-joined"), "actual nested command joined").unwrap();
        return;
    }
    std::env::set_current_dir(std::env::var("CHRONO_NATIVE_FIXTURE_ROOT").unwrap()).unwrap();
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
    while !std::path::Path::new(".chrono-harness/state/native-release").exists() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::fs::write(".chrono-harness/state/native-done", "joined").unwrap();
}
