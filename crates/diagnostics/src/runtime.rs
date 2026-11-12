//! Explicit host adoption of runtime logging; absent adoption preserves the caller's sink.
use crate::{
    error::{Failure, NativeIoError},
    logging::{self, Level, Logger},
    rolling::RollingFile,
    verified_runtime,
};
use serde::Deserialize;
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

pub const CONFIG_PATH: &str = ".chrono-harness/logging.json";

/// Record an existing CLI diagnostic without inventing a typed domain cause from its text.
/// The command still publishes its original diagnostic through its existing output boundary.
pub fn diagnostic(message: &str) {
    tracing::error!(event_id = "cli.diagnostic", message = message);
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub level: String,
    pub directory: String,
    pub max_bytes: u64,
    pub max_files: usize,
    pub stderr: bool,
}

impl Policy {
    fn level(&self) -> Result<Level, Failure> {
        match self.level.as_str() {
            "trace" => Ok(Level::TRACE),
            "debug" => Ok(Level::DEBUG),
            "info" => Ok(Level::INFO),
            "warn" => Ok(Level::WARN),
            "error" => Ok(Level::ERROR),
            _ => Err(Failure::new("E_LOG_CONFIG", "unknown runtime log level")),
        }
    }

    fn validate(&self) -> Result<(), Failure> {
        if self.schema != "chrono-runtime-logging/v1"
            || !self.directory.starts_with(".chrono-harness/state/")
            || !self.directory.ends_with('/')
            || !Path::new(&self.directory)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            || Path::new(&self.directory)
                .components()
                .collect::<PathBuf>()
                .to_str()
                != Some(self.directory.trim_end_matches('/'))
            || Path::new(&self.directory).components().count() < 3
            || self.max_bytes == 0
            || self.max_bytes > 64 * 1024 * 1024
            || !(1..=crate::rolling::MAX_FILES).contains(&self.max_files)
        {
            return Err(Failure::new(
                "E_LOG_CONFIG",
                "invalid runtime logging schema, directory or finite bounds",
            ));
        }
        self.level()?;
        Ok(())
    }
}

pub struct RuntimeSink<W> {
    file: RollingFile,
    stderr: Option<W>,
}

impl<W: Write> Write for RuntimeSink<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let (_, _, result) = verified_runtime::write_mirrors(
            &mut self.file,
            self.stderr.as_mut(),
            bytes.len(),
            |file| {
                let result = file.write_all(bytes);
                (file, result)
            },
            |stderr| {
                verified_runtime::write_and_flush(
                    stderr,
                    |stderr| {
                        let result = stderr.write_all(bytes);
                        (stderr, result)
                    },
                    |stderr| {
                        let result = stderr.flush();
                        (stderr, result)
                    },
                )
            },
            |file, stderr| {
                io::Error::other(io_failure(file).with_related("stderr", io_failure(stderr)))
            },
        );
        result
    }

    fn flush(&mut self) -> io::Result<()> {
        let (_, _, result) = verified_runtime::flush_mirrors(
            &mut self.file,
            self.stderr.as_mut(),
            |file| {
                let result = file.flush();
                (file, result)
            },
            |stderr| {
                let result = stderr.flush();
                (stderr, result)
            },
        );
        result
    }
}

fn io_failure(error: io::Error) -> Failure {
    Failure::wrap(
        "E_RUNTIME_LOG",
        "runtime logging IO failed",
        NativeIoError::new(error),
    )
}

fn load_policy(root: &Path) -> Result<Option<Policy>, Failure> {
    let config = root.join(CONFIG_PATH);
    match fs::symlink_metadata(&config) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_failure(error)),
        Ok(metadata) if metadata.is_file() => (),
        Ok(_) => {
            return Err(Failure::new(
                "E_LOG_CONFIG",
                "runtime log policy must be a regular file",
            ));
        }
    }
    let mut bytes = Vec::new();
    File::open(&config)
        .map_err(io_failure)?
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(io_failure)?;
    if bytes.len() > 16384 {
        return Err(Failure::new(
            "E_LOG_CONFIG",
            "runtime log policy exceeds 16384 bytes",
        ));
    }
    let policy: Policy = serde_json::from_slice(&bytes).map_err(|source| {
        Failure::wrap(
            "E_LOG_CONFIG",
            "invalid runtime log policy",
            logging::LogFailure::Json {
                source,
                observed_fields: Default::default(),
            },
        )
    })?;
    policy.validate()?;
    Ok(Some(policy))
}

fn sink<W>(root: &Path, policy: &Policy, stderr: W) -> Result<RuntimeSink<W>, Failure> {
    let root = fs::canonicalize(root).map_err(io_failure)?;
    let file = RollingFile::open(
        &root.join(&policy.directory),
        policy.max_bytes,
        policy.max_files,
    )
    .map_err(io_failure)?;
    Ok(RuntimeSink {
        file,
        stderr: policy.stderr.then_some(stderr),
    })
}

pub fn configured_logger<W: Write + Send + 'static>(
    root: &Path,
    producer: &str,
    stderr: W,
) -> Result<Option<Logger<RuntimeSink<W>>>, Failure> {
    let Some(policy) = load_policy(root)? else {
        return Ok(None);
    };
    Ok(Some(Logger::new(
        producer,
        policy.level()?,
        sink(root, &policy, stderr)?,
    )))
}

/// Preserve a library caller's chosen sink and level when the host has not adopted file logs.
pub fn logger<W: Write + Send + 'static>(
    root: &Path,
    producer: &str,
    default_level: Level,
    stderr: W,
) -> Result<Logger<Box<dyn Write + Send>>, Failure> {
    match load_policy(root)? {
        Some(mut policy) => {
            // An explicitly requested evidence sink remains active; file logging mirrors it.
            policy.stderr = true;
            Ok(Logger::new(
                producer,
                policy.level()?,
                Box::new(sink(root, &policy, stderr)?) as Box<dyn Write + Send>,
            ))
        }
        None => Ok(Logger::new(
            producer,
            default_level,
            Box::new(stderr) as Box<dyn Write + Send>,
        )),
    }
}

fn emergency(producer: &str, failure: &Failure) {
    let logger = Logger::stderr(producer);
    let _ = logger.run(|| {
        logging::failure(
            "runtime.logging_failed",
            &failure.to_string(),
            &failure.record(),
        )
    });
    let _ = logger.finish();
    // An unavailable emergency stderr cannot be advertised as persisted evidence.
}

/// Main-boundary adapter. Explicit --host-root wins; otherwise use the actual cwd.
/// No parent-directory search, environment activation, background task or global subscriber.
pub fn cli(producer: &str, work: impl FnOnce() -> u8) -> u8 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let root = args
        .windows(2)
        .find(|pair| pair[0] == "--host-root")
        .map(|pair| PathBuf::from(&pair[1]))
        .map(Ok)
        .unwrap_or_else(std::env::current_dir);
    let root = match root {
        Ok(root) => root,
        Err(error) => {
            emergency(producer, &io_failure(error));
            return 2;
        }
    };
    let operation = args.first().and_then(|arg| arg.to_str()).unwrap_or("");
    cli_at(&root, producer, operation, work)
}

/// Use a host and operation already selected by an entry with positional arguments.
pub fn cli_at(root: &Path, producer: &str, operation: &str, work: impl FnOnce() -> u8) -> u8 {
    let logger = match configured_logger(root, producer, io::stderr()) {
        Ok(Some(logger)) => logger,
        Ok(None) => return work(),
        Err(error) => {
            emergency(producer, &error);
            return 2;
        }
    };
    let root = match fs::canonicalize(root) {
        Ok(root) => root,
        Err(error) => {
            emergency(producer, &io_failure(error));
            return 2;
        }
    };
    let exit = logger.run(|| {
        let span = tracing::info_span!("cli", pid = std::process::id(), operation, host_root = %root.display());
        let _entered = span.enter();
        tracing::info!(event_id = "cli.started", "command started");
        let exit = work();
        if exit == 0 {
            tracing::info!(event_id = "cli.finished", exit_code = exit, "command returned");
        } else {
            tracing::error!(event_id = "cli.finished", exit_code = exit, "command returned with failure");
        }
        exit
    });
    if let Err(errors) = logger.finish() {
        let mut failure = Failure::new("E_RUNTIME_LOG", "runtime log publication failed")
            .at("operation_exit_code", u64::from(exit));
        for error in errors {
            failure = failure.with_related("logging", error);
        }
        emergency(producer, &failure);
        return 2;
    }
    exit
}
