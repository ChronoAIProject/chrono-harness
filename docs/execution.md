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
Runner transports the supplied snapshot metadata. Registration checks every declared external file's
retained digest at both endpoints and current candidate disk bytes, and compares
the candidate environment with the runner observation. It never reconstructs old
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
the transition and replacement facts. Config additionally supports explicit v2 presence declarations on both endpoints (see [input snapshots](inputs.md)); projects and judges remain v1.

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
