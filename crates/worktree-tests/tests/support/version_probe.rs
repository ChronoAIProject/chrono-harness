use std::{
    fs,
    io::{self, Write},
    os::unix::fs::symlink,
};

fn main() {
    match std::env::var("CHRONO_TEST_PROBE_RETAIN").unwrap().as_str() {
        "true" => (),
        "false" => {
            fs::rename(
                ".chrono-harness/state/preparation",
                ".chrono-harness/state/saved-preparation",
            )
            .unwrap();
            symlink("saved-preparation", ".chrono-harness/state/preparation").unwrap();
        }
        value => panic!("unknown version probe fixture setting: {value}"),
    }
    writeln!(
        io::stdout(),
        "{}",
        "original-full-version-probe".repeat(20000)
    )
    .unwrap();
}
