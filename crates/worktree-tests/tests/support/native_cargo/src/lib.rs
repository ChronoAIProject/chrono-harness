#[test]
fn native_holder() {
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
