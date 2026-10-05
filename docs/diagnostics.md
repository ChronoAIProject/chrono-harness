# Structured diagnostics

`chrono-diagnostics` owns the Rust `chrono-log/v1` and `chrono-error/v1`
records. It is an independent library with a dedicated `diagnostics-tests`
project and no dependency on the runner, host layout or Git. The host explicitly
registers its build, test plan, inputs, outputs and CI unit.

## Error values and records

`RecordedError` connects a concrete Rust error to the record supplied by its
owner. `Failure::wrap` retains that original value as `Error::source` and adds a
domain code, message and named context. `Failure::new` represents a direct domain
rejection with no invented source. `with_related` attaches secondary failures,
such as cleanup or publication failures, without replacing the primary cause.

`ErrorRecord` is a versioned tree. Each node contains its type, stable code,
original message, context and source. Native codes, obtained traceback text and
related failures have separate fields. The owner supplies these details; the
library does not recover types, codes or causes by parsing messages. Native
upstream errors require an explicit owner adapter before wrapping.

`ReceivedError` imports a record while preserving its source chain. Importing
does not claim that the receiving process executed the original operation.
Unknown schema versions and fields are rejected. This record is independent of
the strict judge v1 protocol; adding it to a judge request or response requires
that protocol's explicit evolution.

## Logging and context

`Logger` installs a scoped `tracing` dispatcher with an explicit producer, level
and `Write` sink. `Logger::stderr` selects stderr and info. Each event produces
one UTF-8 JSON line with an actual UTC RFC3339 timestamp. The layer preserves
typed fields and merges context from enclosing spans, with nearer spans taking
precedence. Events without an explicit `event_id` use `tracing.external`.

`logging::failure` emits an error record at error level. `record_error` supports
other levels, including a recovery event which retains the original exception.
Successful handling does not rewrite or erase an earlier error. Level filtering
does not change a judge's verdict.

The caller keeps the logger alive, explicitly transfers `dispatch()` and the
parent span to child work, joins that work and inspects `finish()`. The library
does not infer thread ancestry or subprocess context. It does not rewrite child
stdout/stderr or use stderr content as a success criterion.

Serialization, formatting and sink errors remain typed failures. A failed write
retains its native IO error and attempted record bytes. After an IO failure,
further writes are blocked with a reference to that failure; a partially written
record is not retried. Repeated `finish()` calls cannot erase the failure. The
caller remains responsible for publishing failures through its declared evidence
or emergency output boundary.

## Adoption boundary

The runner's invalid UTF-8 argument path uses the shared logger, emits no stdout
and exits with status 2. Other runner paths and other programs still use their
existing diagnostics. The dedicated tests cover error wrapping and import,
source and related-error preservation, UTF-8 JSON lines, level filtering,
explicit thread context transfer, recovery events and partial sink failures.

This increment does not complete SPEC §7.1 or §7.2. General native-error adapters,
typed errors throughout the existing products, a common emergency publication
entry, registered formatting/sink choices, persisted record references,
subprocess context transfer and non-Rust adapters remain required. Failed
diagnostic publication must not be reported as successful evidence retention.
