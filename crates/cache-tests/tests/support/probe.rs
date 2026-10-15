use std::io::Write;

#[cfg(unix)]
fn observation() -> Option<std::fs::File> {
    use std::os::{fd::BorrowedFd, unix::fs::FileExt};

    // observed_plan appends exactly one scoped file after inherited ownership
    // capabilities. Check only that last descriptor, without pathname IO or
    // changing its shared offset while recognizing the fixture's header.
    let carrier = std::env::var("CHRONO_PROCESS_FDS").ok()?;
    let fd: i32 = carrier.rsplit(',').next()?.parse().ok()?;
    if fd < 3 {
        return None;
    }
    // The runner supplies live inherited descriptors in its reserved carrier.
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let file = std::fs::File::from(borrowed.try_clone_to_owned().ok()?);
    const HEADER: &[u8] = b"chrono-cache-test-probe-observation/v2\n";
    let mut header = [0u8; HEADER.len()];
    file.read_exact_at(&mut header, 0).ok()?;
    (header.as_slice() == HEADER).then_some(file)
}

fn main() {
    let entered = std::time::SystemTime::now();
    let started = std::time::Instant::now();
    #[cfg(unix)]
    let mut observation = observation();
    #[cfg(not(unix))]
    let mut observation: Option<std::fs::File> = None;
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
    observe("args-ready");
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
            observe("exit-request");
            eprintln!("original probe failure");
            std::process::exit(7);
        }
        "sleep" => {
            observe("sleep-start");
            std::thread::sleep(std::time::Duration::from_secs(4));
        }
        "flood" => std::io::stdout().write_all(&vec![b'x'; 4096]).unwrap(),
        _ => std::process::exit(99),
    }
    observe("main-end");
}
