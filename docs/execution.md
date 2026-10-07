# Registered operation execution

Unix process observations record the engine's actual capability carrier in optional
`ownership_fds`. The `environment` map and `environment_digest` describe business
inputs, excluding reserved `CHRONO_PROCESS_FDS`; that child environment variable
is set from live inherited/scoped descriptors by the process engine. Explicit
caller values cannot replace it. Receipts retain the carrier separately so input
preparation, tool binding and operation validation compare the registered business
environment without treating descriptor allocation as input drift. Observations
without a carrier omit the field; existing observations remain readable.

Execution plans also exclude the reserved carrier from business tool bindings.
The generated native CI Python adapter explicitly forwards the inherited
descriptors to its Rust bridge. Host adapters that launch further consumers must
forward the same descriptors; forwarding only the environment variable does not
keep the corresponding kernel ownership alive.
Engine calls leave already inherited descriptors and their flags unchanged, so
other threads and subsequent native children retain the original capability.
Engine-owned duplicates remain CLOEXEC until their child launch; closing the final
inherited or owned descriptor still releases the kernel lease.

FILEMAP v2 owns `execution_plans`: a map from `test:ID` to
`{operations: [operation-ID, ...], timeout_seconds, output_limit_bytes}`.
Sequences are nonempty, ordered and duplicate-free. Actions, tools and argv remain
in projects/scripts. Plans must include the unique test execute method. Prerequisites
exist only when listed. Routes merges selected sequences as precedence constraints,
runs shared operations once, and rejects cycles or conflicting shared bounds before
tool observation or operation launch. Bounds are positive; each output stream is
limited to 64 MiB. A timeout is an infrastructure failure, not a functional result.

FILEMAP v2 may additionally declare one `execution_scheduling` policy:

```json
{
  "max_running": 2,
  "priority": ["build.example", "test.example"],
  "resources": ["shared-inventory"],
  "claims": {
    "build.example": {"resources": [], "outputs": ["example/output/"]},
    "test.example": {"resources": ["shared-inventory"], "outputs": ["example/output/"]}
  }
}
```

`max_running` is a positive finite operation cap, independent of CPU count and
of tool-internal threads. Every operation participating in any registered plan
must have exactly one claim record, including explicit empty resource/output
lists. Resource names come only from `resources`; outputs name exact existing
`config.artifacts` paths (the scoped legacy adapter uses its declared artifact
paths). Missing, unknown, duplicate, ambiguous and invalid declarations fail
before business tool observation or launch. There is no language, directory,
call-graph or test discovery. All resource claims are exclusive; equal or nested
output namespaces conflict. The declaration remains responsible for completeness.

`priority` optionally lists distinct registered operation IDs in launch order.
Listed ready operations precede unlisted ready operations; unlisted operations
retain canonical plan order. Empty or absent priority retains the previous
serialized identity and ordering. The full policy may include operations outside
the selected unit; priority does not select them or infer durations or dependencies.
Unknown, empty and duplicate priority IDs fail before business effects.

Projects alone schedules the routes DAG. It chooses ready work by the registered
priority followed by canonical plan order, skips conflicts so disjoint ready work advances, and acquires all
claims together before launch. A pending prerequisite waits; an unsuccessful
terminal prerequisite blocks only its descendants. Shared operations run once
per selected invocation. All owned workers join, and executed/blocked report
rows retain canonical order independent of completion order. Original exits,
output bytes, timeout failures and receipt errors remain failures. Without the
policy, execution stays serial and the serialized plan and its identity retain
the legacy shape. With the policy, its complete declarations enter plan identity;
collection reconstructs and compares them against retained candidate registration
without executing business or version commands.

Scheduling cap/resource/priority changes affect registered plan consumers. Priority
order is semantic, and changes or removal reach consumers at both endpoints. Claim changes
also affect explicitly conflicting consumers at both endpoints; output declaration
changes retain declared output consumers and owners. Removal and reassignment
use the same union of endpoint records and edges. No selected obligation is
removed to obtain overlap.

On Unix, the runner process engine carries a bounded anonymous launch-tree
registry through inherited descriptors 198/199/200, without changing observed
argv or environment. Each known nested launch hands off its live child and
launcher identities before exec to one observer thread owned and joined by the
first engine. On macOS the observer uses kernel process exit events; on Linux it
uses pidfds (requiring kernel support). An interrupted launcher cannot run its
destructor. A surviving owner can instead reconcile that launch only after exact
kernel terminal observations for both identities and completion of its registered
subtree. PID disappearance, a kill request and successful enclosing exit alone
never acknowledge completion. Launch generations prevent delayed events or a
retired inherited context from referring to a reused slot. Native macOS behavior
is tested; Linux pidfd behavior requires native verification.

The observer also records when it actually receives a child's kernel exit event,
using the system clock underlying `Instant` (Apple `CLOCK_UPTIME_RAW`, otherwise
`CLOCK_MONOTONIC`). The monitor fixes its evidence cutoff before starting its
existing execution timer. If it resumes after the deadline, an observation
strictly before that cutoff permits an immediate child-status probe. Only an
actual completed status preserves the child result; missing/late observations,
clock failures or an incomplete probe still time out without another wait.
The timestamp bounds observed exit from above; it never infers an earlier exit
when the observer was delayed too. Cancellation, output limits and ownership
cleanup remain independent guards. The private mapping identity versions this
layout, so coupled nested binaries must be upgraded together; incompatible
inherited owners fail instead of silently reinterpreting shared memory.

An operation timeout cancels only that launch subtree. Nested engines terminate
their groups and reap their children before the enclosing engine joins its child
and stream workers. The existing one-second cleanup allowance follows the
unchanged execution deadline. An interrupted launcher's child still live halfway
through that allowance is terminated and its exact exit is observed during the
remaining allowance; the abandoned live child remains a cleanup failure even
when it is terminated. Unknown or unresponsive completion also remains failure.
No execution bound is increased. The registry has 4096 simultaneous launch slots;
slot or kernel descriptor exhaustion fails explicitly before exec. Descriptor
collisions and unavailable live-identity handoff fail explicitly. Native tools'
ordinary descendants remain in their launch group; a wrapper that closes the
inherited descriptors cannot transport nested runner ownership. Non-Unix teardown
retains direct-child behavior. There is no process scanner, daemon, arbitrary
undeclared-writer guarantee or cross-invocation lock. Callers retain disjoint
checkouts and established worktree lifecycle boundaries.

Failed live-identity registration returns the observer's actual kernel errno.
The child publishes a fixed stage/reason/errno record in its private context
descriptor with bounded checked writes. All pre-exec error paths use stack-only
records and raw OS errors; diagnostic text is constructed only by the parent
after a failed spawn. Unavailable or partial diagnostics do not replace the
actual errno; owner failures without an OS errno use EINVAL. A failed spawn still
has no executed process receipt; diagnostics do not invent an exit or completion.
This changes neither the one-second handoff/cleanup allowances nor execution
limits. The existing send loop retries EAGAIN, EINTR and ENOBUFS within that same
handoff allowance; exhaustion retains the last send errno. Parent diagnostics
distinguish an acknowledgement poll timeout from a short read.
Linux's pidfd syscall uses the explicit existing `libc::pid_t` ABI type;
successful compilation does not certify Linux runtime behavior.

Registration owns strict interpretation; FILEMAP owns record identities, targets
and impact. Plan changes target defining records of the test and listed operations.
`operations`, `argv` and `version_argv` preserve order. `tool:ID` and
`environment:NAME` records require explicit consumer edges, just like `input:ID`.
Only traversed `test-execution` edges select full-policy tests. A seeded test alias
alone never selects execution. Build/runtime input edges may target scripts as well
as projects. No Cargo dependency, ownership relation or directory creates an edge.
Full FILEMAP evaluation also compares registration-validated retained effective
environments at both fixed endpoints. Null/absent differs from an empty string;
configured values override inherited values before comparison. Changes seed only
the already-declared `environment:NAME` records and follow the union of explicit
endpoint edges, including old-only edges. Disconnected or unchanged effective
values select no consumers. Unregistered snapshot keys or inconsistent effective
facts fail. The direct `produce` API remains declaration-only;
`produce_with_inputs` additionally consumes validated retained facts. Both use the
same record comparison and graph closure over the same interpreted history.

## Full-v3 execution units and collection

Full registrations may opt into an `execution_units` block. It assigns every
candidate execution plan exactly once, names repeated shared operations, sets
bounded manifest/report byte limits, and gives each unit and the collected full
report a distinct state path. Registration rejects empty or unknown assignments,
overlapping paths, a canonical-report collision, and invalid bounds before a
judge can launch a business operation. Assignment and operation-selection
helpers are shared with the scoped route; v1/v2 and unsliced full requests keep
their original wire shape and behavior.

`--unit ID` still runs the full seven-judge DAG. Routes validates the global
DELTA selection, replacements, prerequisites, and operation order first, then
passes only the selected unit's complete plans to the existing projects
executor. A unit validates its local receipts and applicable context/freshness in
both integration and delivery runs. Completion-dependent migration, replacement,
retirement and certificate decisions remain pending collection. Reports record
`global_selected`, `own`, `assigned_elsewhere`, `required_units`, `not_required`,
and the declaration-derived global graph. Complete-report and certificate
consumers reject contribution-only results.

Selected reports retain one sealed first-judge request template. The runner's
shared `judge_request` rule reconstructs every original stdin from that template,
the configured DAG and the exact preceding records; each actual stdin digest is
checked. Predecessor observations never embed requests. The lossless
`chrono-retained-process/v1` encoding stores original stdout/stderr bytes as hex
and reconstructs their text fields without changing argv, cwd, roots, run IDs,
receipt identities or exits. Direct legacy reports retain their ordinary process
encoding. Addressed context, retained snapshots/blobs and referenced evidence
use external `chrono-retained-blob/v1` descriptors under their original logical
keys: `{schema, storage, sha256, length}`. `storage` is an explicit state path
beneath the selected upload root. Staging hashes and copies with bounded buffers,
deduplicating equal original bytes. Existing `{sha256, length, hex}` inline
originals remain readable. JSON reports and process streams keep their existing
bounds; external business blobs do not consume an inline hexadecimal budget.
JSON/process resolution remains limited to 64 MiB, while input identity and
transport validation stream large files. Immutable run records
preserve earlier attempts while the registered unit path identifies a new attempt.
This portable closure applies only to unit and collection scopes. Ordinary full
check keeps local retained blob references and registration streams their original
and candidate identities, including inputs larger than the portable report bound.
Malformed, missing or changed blobs still fail through the input owner before
business execution; unscoped reports do not embed those files as portable bytes.
Collector import restages every declared external original and preserves unit
report bytes unchanged. Completion records explicitly map original storage
addresses to imported descriptors. Finalization retains that entire nested
closure beneath the collector upload root; delivery resolves it through the
bound transport. Missing maps, lost nested bytes, truncation and conflicting
identities reject. Failed runs preserve their original failure and report
unavailable originals in `artifact_failures` and `unresolved./artifacts`.

Before a full report is assembled, the runner independently audits the records
returned by the execution loop. The audit reconstructs the registered binding
DAG with a separate topological walk, checks each request and stdin digest and
the process executable/argv/cwd receipt against its binding, matches response
identity/status/findings to the observed process exit, verifies that blocked
records name failed registered predecessors or reproduce a request-construction
failure, checks successful response bytes against the original process stdout,
checks pre-launch error records for the expected request identity and the
absence of process, response, exit, and stdin evidence,
and recomputes the aggregate status. Any contradiction is
`E_SELF_DIAGNOSTIC` and prevents report publication. This is a consistency
boundary over declared inputs; it does not discover dependencies, validate
host completeness, or prove that a judge's business result is semantically
correct.

`--collect MANIFEST` uses the same seven-judge entry and consumes a bounded
`chrono-full-collection/v1` manifest with `unit`, `path`, and `sha256` rows. Runner
and seven-judge pins come from the current bound invocation and candidate
registry. Routes plans the complete declaration graph without observing any
business tool, including its version command. Projects validates every required
unit once and every supplied nonrequired unit. It derives success and warnings
from the retained judge responses and successful processes, checks DAG/stdin
consistency, and rebuilds the declared inputs, entry, tool/version contracts,
plan/order/prerequisites/bounds, obligation partitions and receipts. A retained
receipt must also fit each declared output stream bound. A retained observation's
self-digest alone cannot satisfy these comparisons. Original roots
may be unavailable; only explicit declarations and transported bytes supply the
original evidence. External input locations and environment semantics remain
exact, and original process coordinates remain unchanged.
Runner host-root aliases are checked against the retained raw invocation and its
existing live path resolution; collection does not reopen or rewrite old roots.

Historical conversion also consumes retained original decoder observations during
collection. The current registration producer compares the fixed original bytes,
candidate decoder, declared tool/version/input and successful original process;
collection never launches the decoder. Every supplied unit's conversion is checked.
Explicit runtime-input and judge-trigger edges from declared tools, inputs and
environment nodes can reach their registered judge consumers. Unit assignment and
shared-operation changes reuse the same declared plan targets as plan changes.

Projects produces a typed `chrono-collected-completion/v1` with source-linked
coverage and no collector-owned business execution rows. It retains each original
report under its content digest. The finalized collector report includes those
original reports and their complete addressed evidence, plus the manifest and
actual collector processes. Workflow applies the ordinary full transition and
integration policy to direct or verified collected completion. Delivery checks
the real finalized pass/warn report, actual workflow output and original source
closure; provisional, truncated, failed or contribution-only evidence rejects.
Unit/collection report paths cannot overlap canonical reports, configured
certificate/context destinations, retained input artifacts or immutable evidence.
Reports and manifests remain subject to their registered byte bounds (at most
64 MiB); an oversized evidence set fails rather than silently truncating it.

The core supplies explicit unit execution and offline collection. The existing
CI owner supplies workflow generation and automatic artifact transport through
explicit full-context/upload mappings. Full native host activation,
bootstrap/input closure, actual provider adoption and public release remain pending.
These checks do not establish universal local/CI parity or a runtime speedup.

The dedicated workflow consumer exercises two real independent Rust production/test
pairs, shared prerequisites, failed units and selective retry, offline collection
with unavailable original roots and business executables, semantic mutations,
cross-unit replacement/migration and joint retirement, empty collection, and
collected integration followed by delivery. The FILEMAP and runner consumers also
check assignment DELTA and contribution rejection at completed-report boundaries.

The full process DAG is registration → filemap → routes → projects. Registration
publishes one immutable interpreted view bound to the fixed endpoints, registry
digest and context. Filemap and routes forward it; they do not reload an
unconverted historical registry. Routes checks entry argv/cwd captured by runner
before parsing. It accepts equivalent absolute references from a different cwd,
while parameter order, literal arguments and immutable endpoints must match the
registered canonical method. Only entire `{base}`/`{candidate}` arguments expand.

Routes publishes `chrono-execution-plan/v1`, binding endpoints, registries, context,
effective inputs, observed invocation and run identity. It resolves each selected
tool once against the declared environment, preserving the absolute invocation path
and basename. Bytes, version stdout and version exit are observed. The configured
expected version must equal observed stdout with trailing whitespace removed.
Migration's prior observation is validated and reused. Tool bytes are rechecked
before every launch. There is no ambient PATH fallback for bound execution.

Runner's existing process engine records launched argv, canonical cwd, effective
environment and its digest, stdin digest, executable identity, raw stdout/stderr
bytes and digests, exit and resource failures. Both judge protocols decode the
original stdout bytes with strict UTF-8/JSON validation. The lossy display string
is diagnostic only; arbitrary operation output bytes remain retained. Projects constructs receipts from
these observations. Routes' comparison API checks receipt/plan/method/run identity,
argv, cwd, tool, input/environment identities and output digests. A missing or
inconsistent receipt fails. Receipt validation does not turn a failing exit,
output bound or timeout into success. Independent branches can continue after a
failure; dependent operations are reported blocked. Selected, executed, blocked,
removed and replacement obligations remain separate. Candidate dirt is checked
before and after operations.

An observed Unix signal termination records its signal number and child PID in
`failure`, while retaining the legacy `exit_code: -1` and original stream bytes.
Prepared and explicit legacy check reports retain the launched process on failure.
Transport reports that process failure before attempting response JSON decoding.
Normal numeric exits keep their existing meaning; engine cancellation, timeout
and output-limit failures retain their original cause instead of being
reclassified by the cleanup signal. A signal observation does not identify who
sent it or why.

Projects validates affected reciprocal exclusive project/test and script/test
pairs, explicit pair execution edges, registered file ownership and isolation of
declared generated-output directories. A production project has registered files
and one dedicated test; scripts keep their explicit path and test pair. Projects
need no manifest, lockfile, source root or generated output. Optional legacy
manifest/lockfile fields are opaque owned references; root carries no language
or output inference. Shared/nested outputs belonging to different owners fail;
registered input files cannot live inside a generated-output exclusion. Artifact
record changes also target their explicitly declared project/script owners at
both endpoints; changing an output alone therefore cannot silently become no work.

Actions may have any nonempty explicit name. Each binds one globally unique
operation; test `execute` remains its execution contract. FILEMAP execution plans
continue to provide ordered prerequisites. No manifest, extension, import or
directory discovery changes ownership, dependencies or test selection.

Language checks live in separately selected adapters. [chrono-judge-cargo](cargo-projects.md)
retains the former workspace and TOML path-dependency consistency checks using
an explicit policy under the host `.chrono-harness/`. Full protocol fixtures
verify that its failure blocks operations and docs-only changes do not rejudge
unaffected Cargo history. Generic fixtures include manifest-free projects,
opaque legacy files, custom actions and arbitrary locations. Actual Go/TS/mix
project/FILEMAP declarations are accepted by the generic schema and pair consumer;
this is not complete SDK input closure or full governance activation.

The context may name a root-relative `retained_inputs` JSON file. Its `base` and
`candidate` objects each contain `commit`, `environment` and `files`. Environment
snapshots map each inherited variable to a string (including empty) or null
(absent). Each `files[ID]` contains either `bytes`, an array of bytes, or exactly
`{blob, sha256, length}` referencing retained state content. The [input snapshot
producer](inputs.md) can capture and transport these without inline large bytes.
Runner transports the supplied snapshot metadata. Unscoped checks validate every
declared external file's retained digest at both endpoints and current candidate
disk bytes. Full unit checks use registration/input's explicit prerequisite
projection, including shared and governance inputs; unrelated live SDK files are
not required. Collection validates original blobs and each original unit's input
projection without live business input reads. Structural and reference validation
remains global, and supplied unknown or malformed inputs still fail. Registration
compares the candidate environment with the runner observation. It never reconstructs old
bytes from current disk. Reports publish input identities and lengths; supplied
snapshots remain evidence. Interpreter/delegated toolchain/config/fixture bytes
must be declared as effective inputs. A declaration and hashes do not prove no
undisclosed inputs exist. This host remains proposed and explicitly incomplete.

## Initial inventory

A root commit can explicitly use a separate host profile under `.chrono-harness/`:

```json
{
  "schema": "chrono-initial-check/v1",
  "host_config": ".chrono-harness/config.json",
  "timeout_seconds": 30,
  "stdout_limit_bytes": 1048576,
  "judges": [{
    "id": "initial-registration",
    "executable": ".chrono-harness/bin/chrono-judge-registration",
    "version": "0.1.0",
    "sha256": "<SHA-256 of the installed candidate executable>",
    "argv": ["--protocol", "chrono-initial-judge/v1"],
    "selector": "every-initial",
    "modes": ["inventory"],
    "after": []
  }]
}
```

Register this profile itself in FILEMAP, commit the complete initial inventory,
and invoke the same entry locally and in CI:

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/initial.json --candidate FULL_OID --initial
```

This explicit profile rejects `--base` and `--context`. It does not change ordinary
full or scoped profiles. The runner reads actual commit headers, so a nonroot
commit cannot enter this mode merely because history is shallow or a Git replacement
overlay hides its original parents. Shared immutable Git facts disable replacement
objects; the scoped CI judge and event preparer reuse the same parent reader. Each declared
judge runs with the full host config's explicit environment. `after` is a declared
DAG; failed predecessors block dependents. Paths, languages and judge IDs do not
select extra checks. Existing process bounds, byte digests, strict response JSON,
request identities and exact status/exit validation remain in force. `{candidate}`
can expand in judge argv; `{base}` is an error.

The request uses `chrono-initial-judge/v1`, mode `inventory`, and a single candidate
endpoint. It has no base or DELTA field. `profile_sha256` binds exact profile bytes;
`registry_digest` is SHA-256 of the JCS map from each of the five candidate registry
paths to its parsed JSON value. The request also binds profile/config paths,
checkout, actual runner, observations and declared predecessor responses. Response
fields match the ordinary response structure with this protocol identifier; finding
references must be JSON pointers because there is no DELTA path collection.

The default registration inventory judge checks proposed registry schemas, all
current registration references and file membership, actual profile/executable
binding, fixed candidate bytes and checkout cleanliness, including ignored dirt
and unsupported index flags. These reuse ordinary registration policy in an
explicit inventory scope with no historical endpoint. The default judge refuses
active/enabled registries: later activation requires a real DELTA and the existing
readiness/integration obligations. This inventory does not execute project tests,
certify semantic pairing or toolchain/input completeness, or replace the other
six governance judges. Other inventory judges/scripts require explicit bindings.

The `chrono-initial-report/v1` report says `scope: initial-inventory`, has
`base: null`, `delta: null`, `previous_enforcement: none`, and
`governance: not-evaluated`. `status: complete` means the configured inventory
checks completed successfully (exit 0); `failed` and `error` retain nonzero exits.
Judge responses preserve pass/warn/fail/error and original process receipts.
Reports are retained by content identity under `.chrono-harness/state/`, with the
latest copy at `initial-report.json`; stdout contains the same JSON value. No
registry status or input declaration is rewritten. Full bootstrap provenance,
automatic initial-profile generation, activation and complete native parity remain
outstanding.

## Historical transition and the scoped CI consumer

Strict v1 readers reject v2. Candidate readers retain v1 structure as historical
data, without inventing plans. The built-in host decoder profile is
`chrono-ci-check/v1` with FILEMAP v1, declared in workflow v2
`historical_profiles`. Each entry names the original profile path, candidate script
and dedicated script test, exact malformed legacy records, identity mappings and
finite ambiguity repairs. Existing workflow `migrations` and `retirements` retain
the transition and replacement facts. Config additionally supports explicit v2 presence declarations on both endpoints (see [input snapshots](inputs.md)); projects support v1/v2 and judges use v1.

Workflow v3 adds an explicit version selector: `from_versions` and `to_versions`
each contain the five positive integer versions `config`, `filemap`, `projects`,
`judges`, and `workflow`. The maps must differ and exactly match the fixed endpoints.
The remaining profile fields are `id`, `script`, `test`, `mappings`, `legacy_records`,
and `ambiguities`. Legacy marker profiles remain supported without reinterpretation.
More than one matching profile fails. No matching version profile runs no decoder;
any changed schema still requires actual conversion and compatibility evidence.

The selected candidate script receives `chrono-historical-decode/v2` with
`config_path`, `original`, `original_bytes`, `candidate`, and `profile`.
It returns `values` (the historical interpretation), `historical` (retained old
definitions), and the declared `mappings`. Historical `config` must remain equal
to the original, so old input snapshots keep their original schema and digest.
This supports a host-registered v1→v2 config conversion/compatibility check without
relabeling old snapshots or treating an unbound v1 digest as absence. The runtime
does not generate a host's conversion algorithm. Its dedicated registered test
must actually execute and pass; an unrelated passing pair cannot certify it.
Unchanged v3 hosts do not repeat a version transition on documentation-only DELTAs.

For a native selector entry, the registration view is
`chrono-registration-view/v2` and carries `config_bindings` for the base and
candidate entry/effective paths. Its decoder input is
`chrono-historical-decode/v3`, which adds the same binding and retains selector
and target values under their original paths. A selected view is rejected if
those identities or original bytes are missing or substituted. The downstream view
reader also checks the retained decoder input’s schema, canonical entry and
endpoint bindings against the view. The inputs consumer executes a selected
workflow v1→v3 decoder and its dedicated compatibility test; the historical
selector/target remain original even when the old target is absent from the
candidate. Direct v1/v2 decoder contracts retain their existing semantics.

The registered `.chrono-harness/migrations/scoped-v1.py` receives original JSON and
exact bytes, the fixed historical profile bytes and candidate mapping declaration.
It copies ordered bindings into FILEMAP plans and retains the malformed pseudo-script
as explicitly named historical definitions. It does not create a fake current
script/pair. Registration retains original inputs, mappings, output digest, actual
candidate script digest and tool/process observation. Failed migration version
bindings retain the expected version and actual tool path/digest/version-process
observation (raw stdout/stderr and exit) in their error diagnostic. Candidate ambiguity or missing,
ambiguous or incomplete mappings fail. A finite historical ambiguity repair must
name every old defining identity and one unique current replacement; old definitions
remain in impact. Without a downstream migration validator, required removed or ambiguous execute
obligations must remain in the replacement plan. With the declared workflow validator,
explicit replacement or alias repair may change methods; the replacement must actually
pass. Joint producer/test removal can explicitly retire a test without executing it.
Original schema identities and historical costs remain available after decoding.
The [workflow contract](workflow.md) defines actual migration/retirement certification.

`ci.verify` belongs to `ci.actions.execute`; `ci` ↔ `ci-tests` remains
the real dedicated pair. `chrono-ci --version` observes that producer's version.
Its historical profile now declares the exact same-operation method replacement
from the old provider path to `units.json`, using the
[method replacement contract](workflow.md). The old legacy action is retained
verbatim. The real migration consumer rejects missing or mismatched replacements,
executes the current complete plan, detects workflow drift and verifies repair.
The pseudo-script/owner/test `ci-verify` is retired with `ci-tests` as replacement.
All six prior incoming verification edges already had equivalent same-source,
same-kind ci-tests edges. Their effect survives. Adding verification to the ci-tests
plan also runs it for pre-existing ci-tests-only triggers. Selection equivalence
and measured costs are not claimed; costs remain unknown.

Current host operation sequences live only in FILEMAP. Scoped CI consumes v2 plans
through routes' planner and projects' executor/receipt path. Historical v1 bindings
are a named decoder input and remain supported for existing v1 example profiles.
The legacy adapter's additional selection and ambient-environment behavior remain
explicitly scoped; missing legacy version contracts remain unverified. Current host
tools use full declarations. This adapter retires when full native CI consumes the
full contracts. Full host activation, Cargo/SDK closure,
initial adoption, native CI/parity and delivery lifecycle are subsequent work.

The dedicated routes/projects tests include actual processes, retained inputs,
spaced host paths, another caller cwd, literal metacharacters/empty/newline argv,
ordering/deduplication, rejection controls and real exit/output/effect observations.
Single-worker checks are not independent review, native CI or landing evidence.

Declared costs from the same impact now have a dedicated [cost judge](costs.md).

## Explicit groups in one test project

A test-project record may add `test_groups`, a mapping from opaque test-ID suffixes
(without the `test:` prefix) to action keys in that same record. The mapping must
include the legacy project ID. Without it, the existing `execute` alias applies;
scripts retain that legacy contract. For example, `{"suite":"core","detail":"detail"}`
binds `test:suite` and `test:detail` to two explicit actions in one dedicated project.
The registration owner resolves the identities, defining project and terminal
operations for both full and scoped consumers. Empty/repeated bindings, missing
local actions, production/script maps and explicit identity collisions fail.
Legacy ambiguous aliases remain visible for existing DELTA-local adjudication.

Each group has its own explicit FILEMAP execution plan, cost and dependency edges,
and its producer retains the legacy dedicated pair. Pair validation requires an
explicit producer execution edge to every group. Registration changes retain all
group identities as record targets at each endpoint; names, paths and Cargo filters
do not infer dependency selection. Plans must include their bound terminal action.
An original unfiltered action may remain available for release without being a
current independent-unit plan.

## Declared test languages

Projects registry v2 requires a nonempty `language` on every project and standalone script, including each dedicated test owner. The projects judge compares only affected production/test pairs. Equal literal identifiers pass; the sole additional pair is production `shell` with test `python`. Hosts explicitly use these reserved identifiers for Shell and Python. Other host language IDs remain literal, with no path, extension, tool-name or source scan used to infer or normalize them. Missing v2 declarations fail structure validation; mismatched affected pairs fail with `E_TEST_LANGUAGE` before business execution. V1 retains its original contract and rejects the new field.

Language edits live in the original project/script records, so both-endpoint FILEMAP impact includes metadata-only changes and existing explicit test edges. Unaffected historical semantic defects are not rejudged. This validates declared pair languages, not the actual contents or absence of hidden cross-language helpers. Existing route and receipt checks still bind observed operations to registered argv and tools. Full host adoption and migration of foreign test logic remain separate required work.
