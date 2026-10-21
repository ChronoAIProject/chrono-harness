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

On the supported Unix platforms, the actual CLI supplies the same runner-owned
`NativeOutput` adapter for stdout and stderr. It writes through the supplied live
handle with the native write interface, preserving native errors rather than
using Rust stdio's successful acknowledgement of EBADF. An open read-only output
descriptor therefore fails publication even though it is not a closed socket.
The adapter neither duplicates nor closes borrowed stdio descriptors. It has no
userspace buffer, so flush is a no-op, as for the native stdio backend; it makes
no fsync or durable-storage claim. Existing publication policy handles short
writes and Interrupted errors. Non-Unix builds retain the previous standard
output adapters; native error transparency there has not been verified.

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
The read-only stdout regression runs a successful nonempty help command and
requires exit 2 with exactly one framed stderr event carrying native EBADF.
Failure evidence retains the original argv, executable digest, descriptor
scenario, raw streams and exit. Direct tests exercise the same native adapter
with each read-only channel and their combination, including primary, secondary
output and diagnostic IO causes and the independent available channel.

Product and registration changes are coupled in this increment: FILEMAP lists
`runner/src/publication.rs` as a runner compile input and explicitly records the
runner binary as a runtime input of runner-tests. The existing runner and
diagnostics execution plans remain the direct check owners. Canonical validation
uses the declared two-end DELTA closure, which may be broad; no local selection
or verification-cost reduction is claimed. Marginal build, CI and storage cost
remain unmeasured.

## Judge invocation diagnostics

Full judge invocations now retain a typed process setup cause through the wire
owner. Executable reads, cwd acquisition, descriptor transfer, process ownership
setup and spawn keep their original native IO values in the live Rust source
chain, including the available errno. A digest mismatch is a direct domain
rejection. The compatibility `transport_failure` string remains present.

Each new full failure row additionally carries `transport_diagnostic`, an
explicit `chrono-runner-transport-failure/v1` record containing a
`chrono-error/v1` error. It binds the sealed request ID, judge ID,
`operation: judge.invoke` and whether an actual child was started. Unstarted
rows keep `process: null` and `exit_code: null`; no streams, PID or child exit
are invented. Spawn failures separately retain private pre-exec diagnostic
availability and raw stage/reason/errno, plus shared observer counters and
available receive errno with `attributed_to_child: false`. Shared observations
are never promoted to a child-specific cause.

Started processes preserve their original bytes, digests, PID and terminal
result. New process `failure_record` fields keep stream read/publication and
launch-publication IO causes; secondary evidence and ownership cleanup failures
remain related errors. The live process also retains its typed cause until the
transport boundary. Historical process records without this optional field and
full rows without the extension remain readable. Strict decoders reject unknown
diagnostic/error schemas and fields; the full audit checks invocation context
and the relationship to the retained process cause. Judge-v1 requests and
responses remain strict and do not acquire a new error field.

Late full-report publication attempts both existing destinations. Failure returns
`E_REPORT_EVIDENCE` with the explicit
`chrono-full-report-publication-failure/v1` record, the unchanged attempted
report and only the writes that actually succeeded. An original judge transport
failure remains the source; failed report writes are related native IO errors.
The boundary does not claim durable retention when both destinations fail.

Rust behavior regressions exercise real missing executables, permission-denied
spawn, digest rejection, native source downcasts, serialized version/context
mutations, actual full reports with both stored copies, healthy execution,
blocked dependents, binary protocol-failure streams and real file-size-limited
secondary stream publication errors. Existing ownership exhaustion regressions
continue checking original pre-exec errors and fork allocation boundaries.

**Product and host registration change:** the existing runner-tests project adds
`test.runner.diagnostics`, a protected feedback operation over its Rust
diagnostics cases. It uses the existing cargo tool, project inputs and isolated
target. It supplements the current full runner test operation; canonical local
and CI checks still select the declared two-end DELTA closure. No FILEMAP edge
changes are needed because all changed units and consumers are already explicit.
Full build/check, remote CI and storage costs remain unmeasured; focused local
elapsed observations belong to the implementation handoff.

This increment does not complete SPEC §7.1 or §7.2. Other runner domain paths and
other programs retain their existing diagnostics. General native-error adapters,
typed errors throughout the existing products, a common emergency publication
entry, registered formatting/sink choices, persisted record references,
subprocess context transfer and non-Rust adapters remain required. Failed
diagnostic publication must not be reported as successful evidence retention.


For native candidate acceptance, the existing macos-26 `unit_runner` path must
execute the canonical runner unit after current shared-startup install/verify.
Retain actual full invocation regression outputs, original process/stream/error
records, current binary/source binding and the original unit/collection artifact
production identities. Linux process/descriptor/pre-exec coverage still requires
the registered Linux native verification path; local Darwin results do not
establish Linux errno or ownership behavior. A successful native business check
without these retained originals is insufficient for the corresponding evidence
claim. The [shared-startup handoff](filemap-impact.md#bounded-internal-causal-consumption)
owns the existing source/artifact/run/attempt predicates; no new workflow or
alternate diagnostic test recipe is introduced.
