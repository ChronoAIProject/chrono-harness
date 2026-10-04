use crate::error::{Context, ErrorRecord};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::{
    Dispatch, Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use tracing_subscriber::{
    Layer, Registry, layer::Context as LayerContext, prelude::*, registry::LookupSpan,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum LogSchema {
    #[serde(rename = "chrono-log/v1")]
    V1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogRecord {
    pub schema: LogSchema,
    pub timestamp: String,
    pub level: String,
    pub producer: String,
    pub event: String,
    pub message: String,
    pub context: Context,
    pub fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorRecord>,
}

#[derive(Debug)]
pub enum LogFailure {
    Io {
        source: io::Error,
        attempted_record: Option<Vec<u8>>,
    },
    Json {
        source: serde_json::Error,
        observed_fields: Map<String, Value>,
    },
    Time(time::error::Format),
    InvalidField {
        field: &'static str,
        observed: Value,
    },
    Blocked {
        source: Arc<LogFailure>,
        attempted_record: Vec<u8>,
    },
    Closed {
        attempted_record: Vec<u8>,
    },
}

impl fmt::Display for LogFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { source, .. } => write!(f, "log sink failed: {source}"),
            Self::Json { source, .. } => write!(f, "log record JSON failed: {source}"),
            Self::Time(source) => write!(f, "log timestamp failed: {source}"),
            Self::InvalidField { field, .. } => write!(f, "invalid structured log field {field}"),
            Self::Blocked { .. } => {
                f.write_str("log sink is unavailable after its original failure")
            }
            Self::Closed { .. } => f.write_str("log event arrived after the sink was finalized"),
        }
    }
}

impl Error for LogFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Time(source) => Some(source),
            Self::InvalidField { .. } => None,
            Self::Blocked { source, .. } => Some(source.as_ref()),
            Self::Closed { .. } => None,
        }
    }
}

#[derive(Default)]
struct Fields(Map<String, Value>);

impl Visit for Fields {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(
            field.name().into(),
            serde_json::Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(value.to_string())),
        );
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0
            .insert(field.name().into(), format!("{value:?}").into());
    }
}

struct Sink<W> {
    writer: W,
    failures: Vec<Arc<LogFailure>>,
    stopped: Option<Arc<LogFailure>>,
    finished: bool,
}

struct LogLayer<W> {
    producer: String,
    max_level: Level,
    sink: Arc<Mutex<Sink<W>>>,
}

fn take_text(
    fields: &mut Map<String, Value>,
    key: &'static str,
) -> Result<Option<String>, LogFailure> {
    match fields.remove(key) {
        None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text)),
        Some(observed) => Err(LogFailure::InvalidField {
            field: key,
            observed,
        }),
    }
}

impl<W: Write> LogLayer<W> {
    fn publish(&self, record: Result<LogRecord, LogFailure>) {
        let mut sink = self.sink.lock().expect("diagnostic sink mutex");
        let result = record.and_then(|record| {
            let mut bytes = serde_json::to_vec(&record).map_err(|source| LogFailure::Json {
                source,
                observed_fields: record.fields,
            })?;
            bytes.push(b'\n');
            if sink.finished {
                return Err(LogFailure::Closed {
                    attempted_record: bytes,
                });
            }
            if let Some(source) = &sink.stopped {
                return Err(LogFailure::Blocked {
                    source: source.clone(),
                    attempted_record: bytes,
                });
            }
            sink.writer
                .write_all(&bytes)
                .and_then(|()| sink.writer.flush())
                .map_err(|source| LogFailure::Io {
                    source,
                    attempted_record: Some(bytes),
                })
        });
        if let Err(error) = result {
            let error = Arc::new(error);
            if matches!(error.as_ref(), LogFailure::Io { .. }) {
                sink.stopped = Some(error.clone());
            }
            sink.failures.push(error);
        }
    }
}

impl<S, W> Layer<S> for LogLayer<W>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    W: Write + Send + 'static,
{
    fn enabled(&self, metadata: &Metadata<'_>, _context: LayerContext<'_, S>) -> bool {
        metadata.is_span() || *metadata.level() <= self.max_level
    }

    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: LayerContext<'_, S>) {
        let mut fields = Fields::default();
        attributes.record(&mut fields);
        if let Some(span) = context.span(id) {
            span.extensions_mut().insert(fields);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, context: LayerContext<'_, S>) {
        if let Some(span) = context.span(id) {
            if let Some(fields) = span.extensions_mut().get_mut::<Fields>() {
                values.record(fields);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, context: LayerContext<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let mut inherited = Context::new();
        if let Some(scope) = context.event_scope(event) {
            for span in scope.from_root() {
                if let Some(fields) = span.extensions().get::<Fields>() {
                    inherited.extend(fields.0.clone());
                }
            }
        }
        self.publish((|| {
            let identity =
                take_text(&mut fields.0, "event_id")?.unwrap_or_else(|| "tracing.external".into());
            let message = take_text(&mut fields.0, "message")?.unwrap_or_default();
            let error = take_text(&mut fields.0, "chrono_error")?
                .map(|raw| {
                    serde_json::from_str(&raw).map_err(|source| LogFailure::Json {
                        source,
                        observed_fields: Map::from_iter([("chrono_error".into(), raw.into())]),
                    })
                })
                .transpose()?;
            fields
                .0
                .insert("target".into(), event.metadata().target().into());
            Ok(LogRecord {
                schema: LogSchema::V1,
                timestamp: OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .map_err(LogFailure::Time)?,
                level: event.metadata().level().as_str().to_ascii_lowercase(),
                producer: self.producer.clone(),
                event: identity,
                message,
                context: inherited,
                fields: fields.0,
                error,
            })
        })());
    }
}

/// Keep the logger until all instrumented work joins, then inspect finish().
/// A dispatcher clone is the explicit handoff for another thread or task.
pub struct Logger<W> {
    dispatch: Dispatch,
    sink: Arc<Mutex<Sink<W>>>,
}

impl<W: Write + Send + 'static> Logger<W> {
    pub fn new(producer: impl Into<String>, max_level: Level, writer: W) -> Self {
        let sink = Arc::new(Mutex::new(Sink {
            writer,
            failures: Vec::new(),
            stopped: None,
            finished: false,
        }));
        let layer = LogLayer {
            producer: producer.into(),
            max_level,
            sink: sink.clone(),
        };
        Self {
            dispatch: Dispatch::new(Registry::default().with(layer)),
            sink,
        }
    }

    pub fn dispatch(&self) -> Dispatch {
        self.dispatch.clone()
    }

    pub fn run<T>(&self, work: impl FnOnce() -> T) -> T {
        tracing::dispatcher::with_default(&self.dispatch, work)
    }

    /// Retain every original failure; repeated inspection cannot erase one.
    pub fn finish(&self) -> Result<(), Vec<Arc<LogFailure>>> {
        let mut sink = self.sink.lock().expect("diagnostic sink mutex");
        if !sink.finished {
            sink.finished = true;
            if sink.stopped.is_none() {
                if let Err(source) = sink.writer.flush() {
                    let failure = Arc::new(LogFailure::Io {
                        source,
                        attempted_record: None,
                    });
                    sink.stopped = Some(failure.clone());
                    sink.failures.push(failure);
                }
            }
        }
        if sink.failures.is_empty() {
            Ok(())
        } else {
            Err(sink.failures.clone())
        }
    }
}

impl Logger<io::Stderr> {
    pub fn stderr(producer: impl Into<String>) -> Self {
        Self::new(producer, Level::INFO, io::stderr())
    }
}

/// Publish an already structured domain failure through the same tracing layer.
pub fn failure(event: &str, message: &str, record: &ErrorRecord) -> Result<(), serde_json::Error> {
    record_error(Level::ERROR, event, message, record)
}

/// Recovery and handling events keep their original error at the actual event level.
pub fn record_error(
    level: Level,
    event: &str,
    message: &str,
    record: &ErrorRecord,
) -> Result<(), serde_json::Error> {
    let encoded = serde_json::to_string(record)?;
    macro_rules! emit {
        ($level:expr) => {
            tracing::event!(target: "chrono", $level, event_id = event, message = message, chrono_error = encoded.as_str())
        };
    }
    match level {
        Level::ERROR => emit!(Level::ERROR),
        Level::WARN => emit!(Level::WARN),
        Level::INFO => emit!(Level::INFO),
        Level::DEBUG => emit!(Level::DEBUG),
        Level::TRACE => emit!(Level::TRACE),
    }
    Ok(())
}
