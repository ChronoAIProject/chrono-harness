//! The CLI boundary owns output policy; diagnostics owns records and logging.
use crate::CliOutput;
use chrono_diagnostics::{
    error::{Failure, NativeIoError},
    logging::{self, LogFailure, Logger},
};
use std::io::{self, Write};

/// Write through the supplied output handle without duplicating its descriptor.
/// Unlike Rust's standard output writers, native write failures (including EBADF)
/// reach the publication owner unchanged. This adapter has no userspace buffer.
#[cfg(unix)]
pub struct NativeOutput<W>(W);

#[cfg(unix)]
impl<W: std::os::fd::AsFd> NativeOutput<W> {
    pub fn new(writer: W) -> Self {
        Self(writer)
    }
}

#[cfg(unix)]
impl<W: std::os::fd::AsFd> Write for NativeOutput<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        use std::os::fd::AsRawFd;
        let count = bytes.len().min(isize::MAX as usize);
        // The handle keeps the borrowed descriptor live; the slice remains valid
        // for this synchronous call. No close or ownership transfer occurs here.
        let written =
            unsafe { libc::write(self.0.as_fd().as_raw_fd(), bytes.as_ptr().cast(), count) };
        if written < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(written as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // Native stdout/stderr have no buffer to flush. This is not fsync.
        Ok(())
    }
}

/// The actual CLI uses the same native adapter for both output channels.
pub fn publish_native_output(output: &CliOutput) -> Result<u8, Failure> {
    #[cfg(unix)]
    {
        publish_output(
            output,
            &mut NativeOutput::new(io::stdout().lock()),
            NativeOutput::new(io::stderr()),
        )
    }
    #[cfg(not(unix))]
    publish_output(output, &mut io::stdout().lock(), io::stderr())
}

struct ObservedWriter<W> {
    writer: W,
    published: usize,
    last_byte: Option<u8>,
}

impl<W> ObservedWriter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            published: 0,
            last_byte: None,
        }
    }
}

impl<W: Write> Write for ObservedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.writer.write(bytes)?;
        self.published += written;
        if written > 0 {
            self.last_byte = Some(bytes[written - 1]);
        }
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

fn publish_channel<W: Write>(
    writer: &mut ObservedWriter<W>,
    stream: &'static str,
    bytes: &str,
    output: &CliOutput,
) -> Result<(), Failure> {
    let (source, phase) = match writer.write_all(bytes.as_bytes()) {
        Err(source) => (source, "write"),
        Ok(()) => match writer.flush() {
            Ok(()) => return Ok(()),
            Err(source) => (source, "flush"),
        },
    };
    Err(Failure::wrap(
        "E_OUTPUT_PUBLISH",
        "runner output publication failed",
        NativeIoError::new(source),
    )
    .at("stream", stream)
    .at("phase", phase)
    .at("operation_exit_code", output.exit_code)
    .at("operation_succeeded", output.exit_code == 0)
    .at("output_bytes", bytes.len() as u64)
    .at("published_bytes", writer.published as u64))
}

fn attempt_output<WOut: Write, WErr: Write>(
    output: &CliOutput,
    stdout: &mut ObservedWriter<WOut>,
    stderr: &mut ObservedWriter<WErr>,
) -> Result<(), Failure> {
    let stdout_failure = publish_channel(stdout, "stdout", &output.stdout, output).err();
    let stderr_failure = publish_channel(stderr, "stderr", &output.stderr, output).err();
    match (stdout_failure, stderr_failure) {
        (None, None) => Ok(()),
        (Some(failure), None) | (None, Some(failure)) => Err(failure),
        (Some(stdout), Some(stderr)) => {
            Err(stdout.with_related("secondary_output_publication", stderr))
        }
    }
}

/// Attempt both existing channels, returning original typed IO failures.
pub fn publish_output_to<WOut: Write, WErr: Write>(
    output: &CliOutput,
    stdout: &mut WOut,
    stderr: &mut WErr,
) -> Result<(), Failure> {
    attempt_output(
        output,
        &mut ObservedWriter::new(stdout),
        &mut ObservedWriter::new(stderr),
    )
}

struct BoundaryWriter<W> {
    writer: W,
    needs_newline: bool,
}
impl<W: Write> Write for BoundaryWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.needs_newline {
            self.writer.write_all(b"\n")?;
            self.needs_newline = false;
        }
        self.writer.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Publish CLI output and one failure event on stderr. On unavailable stderr,
/// return all causes without claiming that the diagnostic was retained durably.
pub fn publish_output<WOut: Write, WErr: Write + Send + 'static>(
    output: &CliOutput,
    stdout: &mut WOut,
    stderr: WErr,
) -> Result<u8, Failure> {
    let mut stdout = ObservedWriter::new(stdout);
    let mut stderr = ObservedWriter::new(stderr);
    let Err(mut error) = attempt_output(output, &mut stdout, &mut stderr) else {
        return Ok(output.exit_code);
    };
    let writer = BoundaryWriter {
        needs_newline: stderr.last_byte.is_some_and(|byte| byte != b'\n'),
        writer: stderr.writer,
    };
    let logger = Logger::new("chrono-harness", logging::Level::INFO, writer);
    if let Err(source) = logger.run(|| {
        logging::failure(
            "runner.output_publication_failed",
            &error.to_string(),
            &error.record(),
        )
    }) {
        error = error.with_related(
            "diagnostic_serialization",
            LogFailure::Json {
                source,
                observed_fields: Default::default(),
            },
        );
    }
    if let Err(failures) = logger.finish() {
        for failure in failures {
            error = error.with_related("diagnostic_publication", failure);
        }
    }
    Err(error)
}
