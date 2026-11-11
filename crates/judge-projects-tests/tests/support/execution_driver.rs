//! Separate process boundary for the real interrupted-execution regression.
use std::{
    fs,
    io::{self, Read, Write},
    net::TcpStream,
    process::ExitCode,
    time::Duration,
};

struct InterruptBoundary(String);

impl Write for InterruptBoundary {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        io::stderr().write_all(bytes)?;
        let event: serde_json::Value = serde_json::from_slice(bytes)?;
        if event["event"] == "process.completed" && event["context"]["operation"] == "b" {
            // The existing event follows joined readers and retained streams,
            // but the result has not returned to projects for publication.
            let mut stream = TcpStream::connect(&self.0)?;
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            stream.write_all(bytes)?;
            let mut release = [0];
            stream.read_exact(&mut release)?;
            return Err(io::Error::other(
                "producer was released instead of interrupted",
            ));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()
    }
}

fn main() -> ExitCode {
    let run = || -> Result<(), String> {
        let mut args = std::env::args().skip(1);
        let path = args.next().ok_or("execution plan path missing")?;
        let address = args.next().ok_or("interruption socket address missing")?;
        let plan = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let logger = chrono_diagnostics::logging::Logger::new(
            "projects-interruption-test",
            chrono_diagnostics::logging::Level::DEBUG,
            InterruptBoundary(address),
        );
        let results = logger.run(|| chrono_judge_projects::execute(&plan))?;
        logger.finish().map_err(|errors| format!("{errors:?}"))?;
        serde_json::to_writer(std::io::stdout().lock(), &results).map_err(|e| e.to_string())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
