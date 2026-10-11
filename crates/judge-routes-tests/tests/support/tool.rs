use std::{env, fs::OpenOptions, io::Write};

fn version() {
    println!(
        "chrono-judge-routes-test-tool {}",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") => version(),
        Some("count-version") => {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open("version-count")
                .unwrap()
                .write_all(b"v")
                .unwrap();
            version();
        }
        Some("observe") => println!(
            "{}",
            serde_json::json!([
                args.collect::<Vec<_>>(),
                env::current_dir().unwrap(),
                env::var("EMPTY").ok(),
                env::var("ABSENT").ok()
            ])
        ),
        Some("bound") => println!("bound-rust"),
        mode => panic!("unknown route test helper mode: {mode:?}"),
    }
}
