use std::{
    fs::File,
    io::Write,
    os::fd::FromRawFd,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn marker(path: &str) {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut file = File::create_new(path).unwrap();
    write!(file, "{}", time.as_nanos()).unwrap();
}

fn main() {
    let delay: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    marker("started");
    // The runner supplies both pipes. Closing them must not signal process exit.
    unsafe {
        drop(File::from_raw_fd(1));
        drop(File::from_raw_fd(2));
    }
    std::thread::sleep(Duration::from_millis(delay));
    marker("completed");
    std::process::exit(7);
}
