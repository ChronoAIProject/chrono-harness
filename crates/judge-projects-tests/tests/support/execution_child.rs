mod execution_fixture;

use execution_fixture::{Product, Step};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::{Command, ExitCode};
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

fn run(steps: Vec<Step>, arguments: &[String]) -> io::Result<u8> {
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
            Step::Write { path, bytes } => fs::write(path, bytes)?,
            Step::Require { path, bytes } => require(&path, &bytes)?,
            Step::RequireSuffix { path, bytes } => {
                if !fs::read(&path)?.ends_with(&bytes) {
                    return Err(io::Error::other(format!("{path}: required suffix absent")));
                }
            }
            Step::RequireCount { path, byte, count } => {
                if fs::read(&path)?
                    .iter()
                    .filter(|&&value| value == byte)
                    .count()
                    != count
                {
                    return Err(io::Error::other(format!("{path}: required count differs")));
                }
            }
            Step::CheckProduct {
                path,
                input,
                expected,
            } => {
                let product: Product = serde_json::from_slice(&fs::read(&path)?)?;
                let actual = product.evaluate(input);
                if actual != expected {
                    return Err(io::Error::other(format!(
                        "{path}: expected {expected}, observed {actual}"
                    )));
                }
            }
            Step::ObserveProcess => {
                serde_json::to_writer(
                    io::stdout().lock(),
                    &serde_json::json!([
                        arguments,
                        std::env::current_dir()?,
                        std::env::var("EMPTY").ok(),
                        std::env::var("ABSENT").ok()
                    ]),
                )?;
                println!();
            }
            Step::ObserveEnvironment { names } => {
                let values: Vec<_> = names.iter().map(|name| std::env::var(name).ok()).collect();
                serde_json::to_writer(io::stdout().lock(), &values)?;
                println!();
            }
            Step::CheckNativeInputs { path } => {
                let tool = std::env::var_os("FACTS_GIT")
                    .ok_or_else(|| io::Error::other("FACTS_GIT absent"))?;
                let version = Command::new(tool).arg("--version").output()?;
                if !version.status.success() {
                    return Err(io::Error::other(format!(
                        "declared Git version failed: {version:?}"
                    )));
                }
                if std::env::var_os("DECLARED_EMPTY") != Some("".into())
                    || std::env::var_os("DECLARED_ABSENT").is_some()
                {
                    return Err(io::Error::other(
                        "declared native input environment differs",
                    ));
                }
                println!(
                    "data-present {}",
                    if std::path::Path::new(&path).exists() {
                        "True"
                    } else {
                        "False"
                    }
                );
            }
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
    if args.len() == 1 && ["encoding-valid", "encoding-invalid"].contains(&args[0].as_str()) {
        return match encoding(args[0] == "encoding-valid") {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        };
    }
    let execute = || -> io::Result<u8> {
        match args.as_slice() {
            [mode, encoded] if mode == "run" => run(serde_json::from_str(encoded)?, &[]),
            [mode, path, arguments @ ..] if mode == "file" => {
                run(serde_json::from_slice(&fs::read(path)?)?, arguments)
            }
            _ => Err(io::Error::other("expected run JSON or file PATH [ARG ...]")),
        }
    };
    match execute() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error:?}");
            ExitCode::FAILURE
        }
    }
}

fn encoding(valid: bool) -> Result<(), String> {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .map_err(|e| e.to_string())?;
    let request: chrono_harness::wire::Request =
        serde_json::from_value(chrono_harness::json(&input)?).map_err(|e| e.to_string())?;
    request.validate()?;
    fs::create_dir_all(".chrono-harness/state").map_err(|e| e.to_string())?;
    fs::write(".chrono-harness/state/encoding.request.json", &input).map_err(|e| e.to_string())?;
    let mut response = request.response(chrono_harness::wire::Status::Pass);
    response
        .outputs
        .insert("note".into(), serde_json::json!("MARKER"));
    let encoded = serde_json::to_vec(&response).map_err(|e| e.to_string())?;
    let marker = b"\"note\":\"MARKER\"";
    let start = encoded
        .windows(marker.len())
        .position(|bytes| bytes == marker)
        .ok_or("missing note slot")?
        + b"\"note\":\"".len();
    let replacement: &[u8] = if valid { &[0xef, 0xbf, 0xbd] } else { &[0xff] };
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&encoded[..start])
        .map_err(|e| e.to_string())?;
    stdout.write_all(replacement).map_err(|e| e.to_string())?;
    stdout
        .write_all(&encoded[start + b"MARKER".len()..])
        .map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())
}
