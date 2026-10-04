mod execution_fixture;

use execution_fixture::Step;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::time::Duration;

fn require(path: &str, expected: &[u8]) -> io::Result<()> {
    let actual = fs::read(path)?;
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{path}: expected {expected:?}, observed {actual:?}"),
        ));
    }
    Ok(())
}

fn run(steps: Vec<Step>) -> io::Result<u8> {
    for step in steps {
        match step {
            Step::Stdout(bytes) => {
                let mut out = io::stdout().lock();
                out.write_all(&bytes)?;
                out.flush()?;
            }
            Step::Stderr(bytes) => {
                let mut err = io::stderr().lock();
                err.write_all(&bytes)?;
                err.flush()?;
            }
            Step::Exit(code) => return Ok(u8::try_from(code).expect("fixture exit code")),
            Step::SleepMillis(ms) => std::thread::sleep(Duration::from_millis(ms)),
            Step::Append { path, bytes } => OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?
                .write_all(&bytes)?,
            Step::Create { path, bytes } => OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)?
                .write_all(&bytes)?,
            Step::Require { path, bytes } => require(&path, &bytes)?,
            Step::Rendezvous { address, id } => {
                let mut stream = TcpStream::connect(address)?;
                stream.write_all(id.as_bytes())?;
                stream.write_all(b"\n")?;
                let mut byte = [0];
                stream.read_exact(&mut byte)?;
                if byte != *b"!" {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "rendezvous did not release the child",
                    ));
                }
            }
            Step::RequireWithWitness {
                path,
                bytes,
                witness,
            } => {
                if let Err(original) = require(&path, &bytes) {
                    if original.kind() == io::ErrorKind::NotFound {
                        // Readers may observe the witness only after all bytes are written.
                        let pending = format!("{witness}.pending");
                        fs::write(&pending, b"observed")?;
                        fs::rename(pending, witness)?;
                    }
                    return Err(original);
                }
            }
        }
    }
    Ok(0)
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("chrono-test-project-execution/v1");
        return ExitCode::SUCCESS;
    }
    assert_eq!(args.len(), 2, "expected run and encoded Rust fixture steps");
    assert_eq!(args[0], "run");
    match run(serde_json::from_str(&args[1]).expect("valid fixture steps")) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error:?}");
            ExitCode::FAILURE
        }
    }
}
