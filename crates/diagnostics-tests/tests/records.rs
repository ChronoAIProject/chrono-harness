use chrono_diagnostics::{
    error::{ErrorNode, ErrorRecord, Failure, ReceivedError, RecordedError},
    logging::{self, LogFailure, LogRecord, Logger},
};
use serde_json::{Value, json};
use std::{
    error::Error,
    io::{self, Write},
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);
impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Captured {
    fn records(&self) -> Vec<LogRecord> {
        let bytes = self.0.lock().unwrap();
        assert!(bytes.ends_with(b"\n"));
        std::str::from_utf8(&bytes)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

#[derive(Debug)]
struct OriginalIo(io::Error);
impl std::fmt::Display for OriginalIo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
impl Error for OriginalIo {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
impl RecordedError for OriginalIo {
    fn error_node(&self) -> ErrorNode {
        let io_node = ErrorNode {
            error_type: "std::io::Error".into(),
            code: "os".into(),
            message: self.0.to_string(),
            context: Default::default(),
            source: None,
            native_code: self.0.raw_os_error().map(i64::from),
            traceback: None,
            related: Vec::new(),
        };
        ErrorNode {
            error_type: std::any::type_name::<Self>().into(),
            code: "fixture.io".into(),
            message: self.to_string(),
            context: Default::default(),
            source: Some(Box::new(io_node)),
            native_code: None,
            traceback: Some("obtained upstream traceback".into()),
            related: Vec::new(),
        }
    }
}

#[test]
fn wrapping_and_round_trip_keep_actual_causes_native_code_and_related_error() {
    let failure = Failure::wrap(
        "E_READ",
        "reading declared input failed",
        OriginalIo(io::Error::from_raw_os_error(2)),
    )
    .at("path", "input λ")
    .with_related("cleanup", Failure::new("E_CLEANUP", "cleanup unavailable"));
    let original = failure
        .source()
        .unwrap()
        .downcast_ref::<OriginalIo>()
        .unwrap();
    assert_eq!(
        original
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(2)
    );
    assert_eq!(failure.related().next().unwrap().0, "cleanup");
    let record = failure.record();
    assert_eq!(record.error.context["path"], "input λ");
    assert_eq!(
        record
            .error
            .source
            .as_ref()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .native_code,
        Some(2)
    );
    let bytes = serde_json::to_vec(&record).unwrap();
    let received: ErrorRecord = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(received, record);
    let imported = ReceivedError::from(received);
    assert_eq!(imported.error_node(), record.error);
    assert_eq!(
        imported.source().unwrap().source().unwrap().to_string(),
        io::Error::from_raw_os_error(2).to_string()
    );
    let outer = Failure::wrap("E_CHECK", "check could not complete", imported);
    assert_eq!(*outer.record().error.source.unwrap(), record.error);
}

#[test]
fn domain_rejection_does_not_invent_a_source_and_wire_version_is_strict() {
    let error = Failure::new("E_USAGE", "invalid argument");
    assert!(error.source().is_none());
    let mut value = serde_json::to_value(error.record()).unwrap();
    assert_eq!(value["schema"], "chrono-error/v1");
    assert_eq!(value["error"]["source"], Value::Null);
    value["schema"] = json!("chrono-error/v999");
    assert!(serde_json::from_value::<ErrorRecord>(value.clone()).is_err());
    value["schema"] = json!("chrono-error/v1");
    value["error"]["unregistered_field"] = json!(true);
    assert!(serde_json::from_value::<ErrorRecord>(value).is_err());
}

#[test]
fn structured_events_preserve_unicode_newlines_span_context_and_level_filter() {
    let output = Captured::default();
    let logger = Logger::new("fixture", tracing::Level::INFO, output.clone());
    logger.run(|| {
        let parent = tracing::info_span!("request", request = "r-7", candidate = "fixed");
        let _parent = parent.enter();
        let operation = tracing::debug_span!("operation", operation = tracing::field::Empty);
        operation.record("operation", "compile");
        let _operation = operation.enter();
        tracing::debug!(event_id = "diagnostic.hidden", message = "filtered");
        tracing::info!(
            event_id = "compile.started",
            message = "λ\nsecond line",
            amount = 7_u64,
            ready = true
        );
    });
    logger.finish().unwrap();
    let rows = output.records();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.producer, "fixture");
    assert_eq!(row.event, "compile.started");
    assert_eq!(row.message, "λ\nsecond line");
    assert_eq!(row.context["request"], "r-7");
    assert_eq!(row.context["operation"], "compile");
    assert_eq!(row.fields["amount"], 7);
    assert_eq!(row.fields["ready"], true);
    assert!(row.timestamp.ends_with('Z'));
    assert!(row.error.is_none());
}

#[test]
fn explicit_dispatch_and_parent_span_preserve_context_across_threads() {
    let output = Captured::default();
    let logger = Logger::new("fixture", tracing::Level::INFO, output.clone());
    let span = logger.run(|| tracing::info_span!("request", request = "r-thread"));
    let dispatch = logger.dispatch();
    std::thread::spawn(move || {
        tracing::dispatcher::with_default(&dispatch, || {
            let _entered = span.enter();
            tracing::info!(event_id = "worker.joinable", message = "actual child");
        });
    })
    .join()
    .unwrap();
    logger.finish().unwrap();
    assert_eq!(output.records()[0].context["request"], "r-thread");
}

#[test]
fn error_event_preserves_the_full_record_and_success_does_not_erase_it() {
    let output = Captured::default();
    let logger = Logger::new("fixture", tracing::Level::INFO, output.clone());
    let record = Failure::wrap(
        "E_INPUT",
        "failed input",
        Failure::new("E_PARSE", "bad field"),
    )
    .record();
    logger.run(|| {
        logging::failure("input.failed", "original error", &record).unwrap();
        logging::record_error(
            tracing::Level::INFO,
            "input.recovered",
            "recovery completed",
            &record,
        )
        .unwrap();
    });
    logger.finish().unwrap();
    let rows = output.records();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].error.as_ref(), Some(&record));
    assert_eq!(rows[1].error.as_ref(), Some(&record));
    assert_eq!(rows[1].level, "info");
    assert_eq!(rows[1].event, "input.recovered");
}

#[test]
fn invalid_error_record_keeps_raw_input_and_later_success_does_not_clear_failure() {
    let output = Captured::default();
    let logger = Logger::new("fixture", tracing::Level::INFO, output.clone());
    let raw = r#"{"schema":"chrono-error/unknown","error":null}"#;
    logger.run(|| {
        tracing::error!(event_id = "input.invalid", chrono_error = raw);
        tracing::info!(event_id = "other.completed", message = "independent event");
    });
    let errors = logger.finish().unwrap_err();
    assert_eq!(errors.len(), 1);
    let LogFailure::Json {
        source,
        observed_fields,
    } = errors[0].as_ref()
    else {
        panic!("expected the original decoder failure")
    };
    assert!(source.is_data());
    assert_eq!(observed_fields["chrono_error"], raw);
    assert!(errors[0].source().unwrap().is::<serde_json::Error>());
    let rows = output.records();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event, "other.completed");
    assert!(Arc::ptr_eq(&errors[0], &logger.finish().unwrap_err()[0]));
}

#[test]
fn events_after_finish_are_retained_as_failures_without_writing_to_the_closed_sink() {
    let output = Captured::default();
    let logger = Logger::new("fixture", tracing::Level::INFO, output.clone());
    logger.finish().unwrap();
    logger.run(|| tracing::info!(event_id = "late.event", message = "caller did not join"));
    assert!(output.0.lock().unwrap().is_empty());
    let errors = logger.finish().unwrap_err();
    let [error] = errors.as_slice() else {
        panic!("expected one rejected late event")
    };
    let LogFailure::Closed { attempted_record } = error.as_ref() else {
        panic!("expected a closed sink failure")
    };
    let record: LogRecord = serde_json::from_slice(attempted_record).unwrap();
    assert_eq!(record.event, "late.event");
    assert!(error.source().is_none());
}

struct FailingSink;
impl Write for FailingSink {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::from_raw_os_error(32))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn sink_failure_keeps_native_error_and_attempted_original_record() {
    let logger = Logger::new("fixture", tracing::Level::INFO, FailingSink);
    let primary = Failure::new("E_PRIMARY", "original operation failed").record();
    logger.run(|| logging::failure("operation.failed", "failed", &primary).unwrap());
    let errors = logger.finish().unwrap_err();
    assert_eq!(errors.len(), 1);
    let LogFailure::Io {
        source,
        attempted_record: Some(bytes),
    } = errors[0].as_ref()
    else {
        panic!("missing original failure")
    };
    assert_eq!(source.raw_os_error(), Some(32));
    assert_eq!(
        errors[0]
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(32)
    );
    let row: LogRecord = serde_json::from_slice(bytes).unwrap();
    assert_eq!(row.error.as_ref(), Some(&primary));
    assert!(Arc::ptr_eq(&logger.finish().unwrap_err()[0], &errors[0]));
}

#[test]
fn partial_sink_failure_blocks_further_writes_without_retrying_or_replacing_the_cause() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct PartialSink(Arc<AtomicUsize>);
    impl Write for PartialSink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(bytes.len().min(3))
            } else {
                Err(io::Error::from_raw_os_error(32))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            panic!("a failed sink must not be retried implicitly")
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let logger = Logger::new("fixture", tracing::Level::INFO, PartialSink(calls.clone()));
    logger.run(|| {
        tracing::info!(event_id = "first", message = "first attempted record");
        tracing::info!(event_id = "next", message = "retain blocked record");
    });
    let failures = logger.finish().unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(failures.len(), 2);
    let LogFailure::Blocked {
        source,
        attempted_record,
    } = failures[1].as_ref()
    else {
        panic!("expected blocked publication")
    };
    assert!(Arc::ptr_eq(source, &failures[0]));
    assert_eq!(
        serde_json::from_slice::<LogRecord>(attempted_record)
            .unwrap()
            .event,
        "next"
    );
    assert!(Arc::ptr_eq(&logger.finish().unwrap_err()[0], &failures[0]));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
