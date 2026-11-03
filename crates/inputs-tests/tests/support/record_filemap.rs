use std::{
    env, fs,
    io::{self, Read},
    os::unix::process::CommandExt,
    process::Command,
};

fn main() -> io::Result<()> {
    let mut request = Vec::new();
    io::stdin().read_to_end(&mut request)?;
    let original = ".chrono-harness/state/filemap-request.json";
    fs::write(original, request)?;
    // Preserve the registered process identity and inherited ownership channels.
    // The real judge receives the exact recorded request and original outputs.
    Err(Command::new(".chrono-harness/bin/chrono-judge-filemap")
        .args(env::args_os().skip(1))
        .stdin(fs::File::open(original)?)
        .exec())
}
