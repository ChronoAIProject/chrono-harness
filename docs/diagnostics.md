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
library does not recover types, codes or causes by parsing messages.
`NativeIoError` adopts the original `std::io::Error`, retains its kind, message
and available OS code, and exposes the unchanged native value through its source
and accessors. A custom IO payload remains typed in memory; without its own
recording adapter, its wire cause is explicitly `custom_source: not_recorded`.
This adapter does not serialize arbitrary custom cause chains or obtain a stack.
Other native upstream errors still require an explicit owner adapter.

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

`LogFailure` implements `RecordedError`, including shared retained failures, so
a boundary can attach the actual logging exception as a related error. Native IO
recording uses the same diagnostics owner as `NativeIoError`; attempted bytes and
the producer's explicit write/flush phase remain available for event publication
and final flushing. Recording adds no successful publication
claim.

## Adoption boundary

The runner's invalid UTF-8 argument path uses the shared logger, emits no stdout
and exits with status 2. All post-dispatch `CliOutput` consumers now use the
runner-owned publication boundary, including unestablished parity's machine
stdout plus stderr explanation. Both existing channels are attempted and flushed
independently. Successful publication preserves the bytes and operation exit
status. A write or flush failure exits 2, retains the original typed IO value,
stream, phase, acknowledged byte count and original operation outcome, and keeps
the other channel's failure separately. No whole payload is replayed.

The boundary attempts one `runner.output_publication_failed` event through the
existing stderr logger. It adds a newline only when necessary to frame the event
after already accepted stderr bytes; diagnostic bytes never enter stdout.
Logger serialization and finish failures become related typed failures in the
returned error. If stderr is unavailable or a diagnostic is only partially
written, the boundary cannot promise durable retention and does not retry the
whole record. A logged publication failure records the operation's exit status;
it does not reconstruct a business cause from existing free-form stderr.

The dedicated Rust tests cover error wrapping/import, native and custom IO value
retention, source and related-error preservation, UTF-8 JSON lines, level
filtering, explicit thread context transfer, recovery and failed sinks. Runner
owner tests cover independent output attempts, both failures, partial write and
flush boundaries, recoverable interrupted writes, actual zero writes with native
`WriteZero` errors and acknowledged-byte counts, healthy bytes/status, diagnostic
framing and an unavailable diagnostic sink. Existing help/domain outputs supply
the empty-channel cases: healthy binary publication and direct empty-channel
flush-failure observations, without adding a new output producer. An actual
runner binary regression also covers help/version,
domain failure, unestablished parity, stdout rejection and both unavailable
descriptors, detecting a main that still uses the former publication path.

Product and registration changes are coupled in this increment: FILEMAP lists
`runner/src/publication.rs` as a runner compile input and explicitly records the
runner binary as a runtime input of runner-tests. The existing runner and
diagnostics execution plans remain the direct check owners. Canonical validation
uses the declared two-end DELTA closure, which may be broad; no local selection
or verification-cost reduction is claimed. Marginal build, CI and storage cost
remain unmeasured.

This increment does not complete SPEC §7.1 or §7.2. Other runner domain paths and
other programs retain their existing diagnostics. General native-error adapters,
typed errors throughout the existing products, a common emergency publication
entry, registered formatting/sink choices, persisted record references,
subprocess context transfer and non-Rust adapters remain required. Failed
diagnostic publication must not be reported as successful evidence retention.
