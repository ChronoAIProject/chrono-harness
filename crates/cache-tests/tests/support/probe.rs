use std::io::Write;
fn main() {
    let entered = std::time::SystemTime::now();
    let started = std::time::Instant::now();
    // Enabled only by the two owner fixtures. Keep observations out of the
    // probe's stdout/stderr and declared command environment.
    let mut observation = std::path::Path::new("probe-observation.enabled")
        .exists()
        .then(|| {
            std::fs::File::create_new(format!("probe-observation-{}.bytes", std::process::id()))
                .unwrap()
        });
    let mut observe = |stage: &str| {
        if let Some(file) = &mut observation {
            writeln!(
                file,
                "{}\t{}\t{}\t{}",
                std::process::id(),
                stage,
                entered
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                started.elapsed().as_nanos()
            )
            .unwrap();
        }
    };
    observe("main-entry");
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "echo" | "file" => {
            println!("{}", args[1]);
            observe("println-return");
        }
        "github" => {
            std::fs::write(
                "backend-call.txt",
                format!(
                    "{}\ncredential={}",
                    args[1],
                    std::env::var_os("CACHE_TEST_SECRET").is_some()
                ),
            )
            .unwrap();
            std::io::stdout()
                .write_all(&std::fs::read(&args[2]).unwrap())
                .unwrap();
        }
        "exit" => {
            eprintln!("original probe failure");
            std::process::exit(7);
        }
        "sleep" => std::thread::sleep(std::time::Duration::from_secs(4)),
        "flood" => std::io::stdout().write_all(&vec![b'x'; 4096]).unwrap(),
        _ => std::process::exit(99),
    }
    observe("main-end");
}
