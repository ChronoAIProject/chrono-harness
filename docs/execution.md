# Registered operation execution

FILEMAP v2 owns `execution_plans`: a map from `test:ID` to
`{operations: [operation-ID, ...], timeout_seconds, output_limit_bytes}`.
Sequences are nonempty, ordered and duplicate-free. Actions, tools and argv remain
in projects/scripts. Plans must include the unique test execute method. Prerequisites
exist only when listed. Routes merges selected sequences as precedence constraints,
runs shared operations once, and rejects cycles or conflicting shared bounds before
tool observation or operation launch. Bounds are positive; each output stream is
limited to 64 MiB. A timeout is an infrastructure failure, not a functional result.

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

Projects validates affected reciprocal exclusive project/test and script/test
pairs, file ownership, distinct manifest/lock/root/target ownership, registered
pair execution edges and absence of Cargo workspace aggregation. Affected Rust
manifests and their registered ancestor Cargo manifests (including intermediate
ancestors) must have no workspace association; this bounded check rejects ancestor
workspace declarations without trying to interpret membership globs/exclusions.
Script-only and docs-only changes do not adjudicate unrelated Cargo manifests.
Its TOML consumer
checks path dependencies against explicit compile edges without selecting work.
The bounded full consumer currently rejects registry dependencies without retained
closure support. This is a full Cargo/SDK closure limitation, not Cargo discovery.
The positive project fixture exercises registered Python methods associated with
standalone Cargo manifests; it validates method execution and TOML pairing, not
Cargo compilation or SDK completeness. The migration consumer additionally runs
this repository's actual registered ci-tests Cargo plan through the scoped adapter.

The context may name a root-relative `retained_inputs` JSON file. Its `base` and
`candidate` objects each contain `commit`, `environment` and `files`. Environment
snapshots map each inherited variable to a string (including empty) or null
(absent). Each `files[ID]` contains `bytes`, an array of bytes. Runner transports
these supplied snapshots. Registration checks every declared external file's
retained digest at both endpoints and current candidate disk bytes, and compares
the candidate environment with the runner observation. It never reconstructs old
bytes from current disk. Reports publish input identities and lengths; supplied
snapshots remain evidence. Interpreter/delegated toolchain/config/fixture bytes
must be declared as effective inputs. A declaration and hashes do not prove no
undisclosed inputs exist. This host remains proposed and explicitly incomplete.

## Historical transition and the scoped CI consumer

Strict v1 readers reject v2. Candidate readers retain v1 structure as historical
data, without inventing plans. The sole implemented decoder profile is
`chrono-ci-check/v1` with FILEMAP v1, declared in workflow v2
`historical_profiles`. Each entry names the original profile path, candidate script
and dedicated script test, exact malformed legacy records, identity mappings and
finite ambiguity repairs. Existing workflow `migrations` and `retirements` retain
the transition and replacement facts. Other registry schema versions remain 1.

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

`ci.verify` has moved unchanged into `ci.actions.execute`; `ci` ↔ `ci-tests` remains
the real dedicated pair. `chrono-ci --version` observes that producer's version.
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
