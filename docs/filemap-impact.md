# FILEMAP impact contract

FILEMAP v2 may opt into `configuration_edges` with schema
`chrono-filemap-configuration-edges/v1` and a nonempty `configurations` array.
Each row names one registered direct policy file as `config` and a nonempty
`edges` array using the existing explicit edge shape. Registration's shared
`select_filemap` projection appends only the row matching the endpoint's effective
direct policy path to unconditional `project_edges`. A nonmatching policy keeps
the shared graph. Missing configuration context, unknown table versions,
unregistered policy paths, repeated configurations and duplicate active edges
fail; legacy v1 rejects the extension. No directory, platform name or executable
observation infers an edge. Retained registry snapshots keep the source bytes;
graph/record consumers use the projection independently at both endpoints, so
adding, removing or changing active edges keeps the existing union impact rules.

**Product and host policy change together:** the isolated macOS 27 input edges
move from the common table into this explicit scope. Their required bindings and
expected bytes remain intact; pinned native policies retain their shared graph.
Schema/projection Rust behavior and endpoint impact tests, source bootstrap,
canonical DELTA and native detector/consumer validation are required. Additional
build, validation and maintenance costs remain unmeasured. Git v2 and worktree v3
identity selection are unchanged.

`chrono-judge-filemap --protocol chrono-judge/v1` is a candidate executable. It reuses runner facts, wire identity and transport, consumes registration's single bound interpreted view, including historical conversion, and publishes `outputs.impact`. It never calls registration's judge in place of a process. Registration still owns schema, readiness, checkout and direct-reference admissibility. Its immutable `node_data()` is the sole full-format node inventory; loading data alone does not certify semantic validity or input completeness.

The dependency direction is `judge-ci → judge-filemap → judge-mixed → judge-registration → runner`, with direct runner dependencies for facts/transport. The independent filemap test project has its own manifest, lock and target. There is no workspace or generic registry project.

The closure adapter calls the executable Verus FIFO owner in
`crates/judge-filemap/src/verified_graph.rs`. It indexes the explicit node names
and preserves each existing sorted typed-edge adjacency slot. Its Verus byte
comparator and exact binary lookup replace the separate name-to-index map.
The proved merge constructor replaces BTreeSet name sorting/deduplication: its
output is strictly ordered, has exactly the supplied byte-name set and has length
equal to that set's cardinality. A separate validator retains exact admission of
untrusted inventories, using strictness/transitivity across adjacent names.
Lookup returns only a byte-identical name slot, or proves absence. Empty,
duplicate/descending, prefix-related, Unicode and missing-name boundaries are
verified independently, including exact construction from descending duplicates.
The same owner's `index_names` constructs all destination and seed indices from
the supplied name rows. It proves row/slot dimensions and exact byte identity at
every position, with all target/root indices in range. Duplicate targets and
roots retain their positions. It rejects exactly mismatched row counts or a
missing target/root name. The actual closure uses these indices for every seed,
replacing its independent mapping loops and subsequent destination-validation
scan. The core proves exact reachability, real predecessor edges, acyclic
predecessor distances and minimum hop counts, including
zero-length root paths. Its complete result also equals the in-source `fifo`
contract: queue order and first adjacency slots fix every predecessor, and later
discoveries cannot overwrite one. Independent exact inventories verify parallel
slots, reordered ties, longer alternatives and cycle returns. The adapter still
owns extraction of the supplied edge/seed names and target-name rows, edge kinds,
cause merging and the mapping from sorted typed edges to indexed slots;
these and union/test selection remain unmet Verus
obligations. The independent test project checks the actual adapter's ties,
parallel typed edges, cycles, Unicode and duplicate seed IDs. Two registered
proof operations verify the source and independent boundary examples. Name
construction uses O(N log N) complete-byte comparisons and O(N) reference
storage for N supplied names. Dense per-seed arrays cost O(V) initialization and
memory; traversal visits each reached
edge once. Total runtime/build/storage costs and sparse-closure regression costs
remain unmeasured; this is a proof migration, without a performance-gain claim.
Index construction performs one binary lookup per supplied target and seed.
Its temporary target-name rows and seed references cost O(V + E + S) storage,
released before the per-seed traversals; the returned root indices retain O(S)
storage. The same proof roots and vstd contracts suffice, without an additional
operation, dependency or external assumption.

## Bounded internal causal consumption

The current scoped check policy may opt in with `causal_consumption.schema` equal to
`chrono-internal-causal-consumption/v1`. Its nonempty `shared_startup` array
contains unique `{id, units, startup, consumer, producer, caller, evidence}`
references; `evidence` is `{test, operation}`. Paths name existing registered host
documents, and operation/test identities resolve through projects and execution
plans. Unknown versions/fields, duplicate chains and empty identities fail.
This contract currently covers internal shared-startup unit consumption only.

A chain may explicitly declare `preparation: {operation, config, verifier}` when
its unit invokes a composed bootstrap profile. The current unit argv must equal
the registered preparation operation and exactly invoke that profile's declared
entrypoint at the host root. The profile must use `chrono-bootstrap/v1`, with the
named verifier as its sole prerequisite. The verifier's registered command still
must select a built profile imported by this consumer. Moving verification into
ordinary operations, omitting or duplicating it, or changing the script, config
or invocation fails. The explicit preparation path and both operation owners
participate in endpoint DELTA impact. This is configuration consistency; the
existing real consumer remains required to establish actual execution.

Product and host policy change together: runner startup now adopts this explicit
composition for Verus package acquisition after core verification. Its exact
preparation predicate is executable Verus in registration's
`verified_consumption.rs`, with separately verified boundary examples. Text/byte
identity comparisons share the runner's proved implementation with the CI judge.
The surrounding JSON/registration adapter remains an unmet proof obligation.
Two whole-crate proof operations join the registration test plan; runtime and
total validation/storage costs remain unmeasured. Existing producer, caller,
source/output, profile and real-consumer evidence requirements remain enforced.

The host adopts `runner-startup`: the existing detector production operation →
`host.bootstrap-shared-install-unit_runner` → installed runner/core tools →
current runner unit check. Need and output-use text remain in the selected units
consumer. Profiles, build operations and installation source/destination pairs
remain in bootstrap registrations; caller argv and ownership remain in projects;
build output claims and evidence plans remain in FILEMAP. No second inventory or
discovery is introduced. The existing `host.bootstrap-real-consumer` retains the
actual startup manifest and original current-check report and exercises the
subsequent runner tests. Registration checks its declared operation, test owner,
plan and output claim; existing projects execution and report collection must
still establish actual success.

The current scoped canonical path delegates to registration before unit
selection. Contract changes, explicit source-path changes and reached operation
owners from either fixed endpoint select the chain. Old-only references still
select after removal or rebinding. A selected chain must resolve in the candidate;
removing its opt-in is an uncertified retirement and fails. Missing need/use,
inputs, callers, profiles, producer claims, installed tool roles, current check
binding or required evidence fail with `E_CONSUMPTION`; retired chains use
`E_CONSUMPTION_RETIRED`. Selected chains add their explicitly registered evidence
test to the same global check obligations. Unit execution and collection keep
their existing assignment and original-evidence rules. Unreached chains are
reported as historical context, and consumers outside this opt-in keep their
prior meaning.

Product/policy visibility: registration gains this bounded consistency rule and
the host opts in one existing chain. Additional fixed input reads, regression
tests and execution of the already registered evidence test are the validation
cost; complete build/CI/storage costs remain unknown. The contract cannot prove
usefulness or fidelity to a real need: those remain accountable engineering
judgments. External/all-consumer adoption, full-path causal enforcement and
complete SPEC activation remain pending. The existing macos-26 path was observed
at production `72b1d08` in run `39690377913` attempt 1 and scenario `c515cd05` in
run `39695762671` attempt 1: current detector manifests and clean source-bound
cache/core reports were imported, `unit_runner` installed/verified the tools,
the canonical runner unit passed and original collection verified its report.
Scenario startup artifact `12391076536`, runner `12390726564` and collection
`12391598633` retain the actual producer/attempt and matching manifest/tool
identities. These observations establish this internal chain on Darwin only.

Native handoff uses the existing PR-to-dev macos-26 parent workflow. Require the
detector's current shared artifact, original clean producer reports and manifest,
then the real `unit_runner` install/verify and
`.chrono-harness/bin/chrono-harness check --unit runner`, followed by the existing
collection. Retain startup binding, exact artifact ID and actual producing
run/attempt, `startup-install.json`, imported bootstrap provenance, manifest and
file identities, sealed canonical request/report, original stdout/stderr and
uploaded unit artifact binding. Match the checked-out PR head across them;
event merge SHA is a separate role. On job reruns, carried detector results must
resolve to their original producer/artifact attempt, not the current attempt by
assumption. Collection must consume the actual selected unit originals and
passing business response. A local real-consumer fixture or a generated workflow
cannot establish any of these remote observations. Artifact retention follows
the existing finite provider retention; the caller joins consumers and harvests
necessary evidence before protected lifecycle cleanup.

## Versioned output

`schema` is `chrono-filemap-impact/v2`. Consumers may deserialize `chrono_judge_filemap::Impact`; JSON consumers must check this schema. Endpoint commits, trees and registry digest are bound by the enclosing v1 request/report. Arrays and maps are emitted deterministically. Unknown cost values remain JSON null. This output is declared graph impact, including validated effective environment changes when supplied; it is not operation evidence or retirement approval.

| Field | Meaning |
| --- | --- |
| `workflow` | Explicit stability/integration requirements, shared rule classification, endpoint source pointers and lowered test edges; selection only |
| `delta` | Complete factual A/M/D paths, old/new blobs and modes; no similarity rename inference |
| `records` | Stable record ID → `{base, candidate}`, each nullable; a record contains its registry `path`, explicit `targets`, and normalized `value` |
| `changes` | `{record, fields}` with relative JSON pointers for changed fields; empty pointer means the whole record was added/deleted; `/$registry_path` denotes relocation |
| `nodes` | Node ID → uniquely resolved nullable `base`/`candidate` values, plus complete `base_definitions`/`candidate_definitions` arrays of `{identity, value}`; includes old-only nodes and all ambiguous definitions |
| `edges` | Flat `{from, kind, to, origin}` for the full explicit union; origin is `base`, `candidate` or `both` |
| `seeds` | Sorted unique list of seed node IDs |
| `seed_causes` | Additive `{id, node, reference, reason}` records; reference is an actual DELTA path, stable record ID or explicit workflow requirement pointer; reason is `delta-path`, `record-change` or `edge-change` |
| `closure.reached` | Node → all contributing seed IDs |
| `closure.predecessors` | Seed ID → node → one shortest predecessor edge, or null at the seed; supports finite consecutive witness reconstruction |
| `closure.traversed` | Every traversed edge and all contributing seed IDs; retains alternate diamond routes and cycle edges without enumerating infinitely many walks |
| `tests` | Sorted unique selected test-node IDs present in candidate, including ambiguous IDs; presence is not unique resolution or execution |
| `retired_tests` | Sorted unique selected required test-node IDs absent from candidate; removal facts only, never retirement approval |
| `required_tests` | All selected test IDs, `candidate_present`, `candidate_ambiguous`, actual selecting `execution_edges`, nullable `base_cost` and `candidate_cost` containing the cost reference and value |
| `judges` | Each candidate every-delta judge once, plus its reached `judge-trigger` edges as extra explanations |
| `historical_context` | Unreached semantic edge defects with endpoint and edge; they are not findings for this DELTA |
| `historical_ambiguities` | Unreached ambiguous node IDs with endpoint and defining identities; factual context, not findings |
| `limits` | Explicit boundaries: no execution/retirement/cost verdict, unretained external bytes, unproven completeness/locality |

Stable record IDs begin `/records/<registry>/<collection>/<key>`, using JSON Pointer escaping for the key. They are identities within `records`, not array-index pointers into the source JSON. File keys are paths, project/script/judge/input/tool/stability keys are IDs, owner keys are names, test cost keys are unprefixed test IDs, and cost model keys are cost IDs. Project-edge keys are compact JSON `[from,kind,to]` tuples. Other singleton fields (including arrays without an explicit unique identity) use `fields/<field-name>`. Input declarations are separate `config/inputs/<id>` records; environment variables have separate `config/environment/<name>` records and explicit consumer edges. All identity arrays are compared as sorted sets of declarations (duplicate declarations are retained in normalized values); `argv`, `version_argv` and plan `operations` preserve order. Record targets never manufacture graph edges. Full evaluation adds registration-validated retained `effective` values to environment records at both endpoints. Null means absent and differs from an empty string; configured values override inheritance. `produce` remains declaration-only, while `produce_with_inputs` validates and consumes retained effective facts through registration's input API. Their graph/record implementation and historical view are shared; missing or inconsistent effective facts cannot be treated as unchanged.

Registration's shared inventory retains every defining record and its identity: for example `project:t` and `script:t` both define `test:t`. `NodeView::unique()` returns a definition only for exactly one definition. In the wire, a null `base`/`candidate` value means absent or ambiguous: the corresponding definitions array distinguishes zero, one and multiple definitions. No arbitrary executable value is exposed on collision. Both project and script records retain their test alias in `targets`; filemap consumes the same inventory and does not rediscover aliases.

The complete base/candidate records retain owner declarations, old cost references and cost model values. A removed required test remains in `required_tests` with `candidate_present:false` and in `retired_tests`. The `tests`/`retired_tests` arrays partition selected requirements by candidate presence; they do not contain executed-test or approved-retirement statuses. In particular, membership in `retired_tests` must never waive `E_REQUIRED_TEST_REMOVED`: downstream projects/workflow judges still need the SPEC retirement/replacement evidence and must fail its absence. Projects verifies actual replacement execution; workflow retirement certification remains pending. Missing cost declarations remain null, not zero. Later projects/workflow/cost judges must consume these facts and establish their own results.

## Policy and witnesses

File A/M/D changes seed `file:<path>`. Changed normalized records seed both endpoint targets, including executable test aliases supplied by registration for project/script records. Changed input declarations seed `input:<id>`. Added/removed/retargeted edges seed both affected endpoints; removed edges remain in the union. Explicit workflow paths/tests and conditional integration suites are lowered to typed edges before the same closure; see [workflow selection](workflow-selection.md). The finite forward closure traverses these declared edges only. No pairing, directory, import, Cargo metadata or runtime discovery adds a dependency.

| Kind | Permitted source types | Target |
| --- | --- | --- |
| compile | file, project | project |
| build-input / runtime-input | file, input, tool, environment, project, script | project, script |
| test-execution | file, input, tool, environment, project, script | test |
| judge-trigger | file, project | judge |

Only traversing a `test-execution` edge selects a full-policy test. Reaching a test seed, changing test costs or owning a paired test does not select execution. Every-delta judges remain selected for an empty DELTA; triggers add explanations once. No actual input retention is implied by comparing external input declarations. The real registration process still rejects nonempty, unretained external inputs with `E_EVIDENCE_UNRESOLVED`; direct input-node tests stop at the declaration boundary.

Reached/changed invalid references and endpoint types produce `E_DANGLING_EDGE` / `E_EDGE_TYPE`. A reached ambiguous ID produces `E_NODE_AMBIGUOUS` unless a finite historical repair accounts for all old definitions and one unique current identity (candidate ambiguity always fails), even if it is only a changed-record seed and no execution edge was traversed. Unaffected base/candidate ambiguities remain factual context; loading all nodes does not globally adjudicate them. A finding's `delta_refs` identifies a seed path or changed record. `causes` is a consecutive node sequence from that seed, following real union edges, ending at the defective endpoint or ambiguous ID; a direct record seed has a one-node witness. The affected registry endpoint and definitions or edge appear in the message. The closure preserves all contributing cause IDs from `seed_causes` even when a finding chooses one deterministic witness. Required unparsable or missing registries/objects, forged empty DELTA and digest disagreement return error without an empty-success impact.

## Scope of graph work

Impact computes the endpoint union and finite seed closure used by current
selection consumers. It retains typed edges, all contributing seeds, alternate
paths and cycle edges. It does not calculate a second structural report: SCC
partitions, condensed-graph depths, relation counts, cross-owner rankings and
execution conflict summaries have no current production reader.

**Product and policy change:** SPEC §5.2 no longer requires these observations on
every impact. Their report-only self-check and two exclusive tests retire with
the output. The existing cycle/diamond/multiple-seed regression still checks the
real edges and witnesses, and the real runner consumer still checks the declared
impact and selected tests. Routes owns execution-DAG admission and resource/output
claims; its checks and the execution receipt self-diagnostics remain required.
The ordinary impact schema and authoritative selection fields are unchanged.

A refactoring must address an actual consumer and preserve its explicit test
obligations. Use the existing graph/selection facts and owner checks for that
DELTA; additional measurements need a concrete purpose. Runtime, payload and
build savings from this deletion remain unmeasured until candidate validation.

## Operational CI adapter and retirement boundary

`judge-ci` retains its existing scoped snapshots, adoption/initial semantics, operation/binding/tool/environment/bounds seeds, validation, argv execution and exit propagation. It calls filemap's `graph::union`, `graph::closure`, and `Closure::selected_tests`; the former private traversal is removed. This mechanics API takes explicit edges/seeds and does not load or require full-format registration for legacy hosts.

CI evidence adds `selection_explanation` with scope `chrono-ci-check/v1-adapter`, flat union edges, node-ID `seeds`, additive `seed_causes`, closure, `extra_selections` and `legacy_only_selections`. Extra reasons explicitly identify changed operations, bindings, bound tools, environment or operation limits. The adapter does not turn those reasons into whitelist edges or full-policy authority. Existing selected commands and execution semantics remain covered by the original CI suite; additional tests cover empty-graph legacy selection and changed execution bounds.

This adapter expires when full routes/projects and retained input/migration evidence replace its actual local/native consumers. Routes/projects now own shared execution and receipt checking. See [execution](execution.md) for the bounded input and finite ci-verify transition. Workflow certification/freshness, complete Cargo/SDK inputs, activation and native parity remain subsequent work. The host remains proposed/incomplete.

## Direct evidence

The dedicated `impact` suite specifies small endpoint graphs independently, checks witness edges against those graphs, and covers content/mode/binary DELTA, D+A rename, nonancestor and multi-commit endpoints, record/field/edge changes, all edge origins, removed owners/costs/tests, no-op/docs, typed propagation, paired nonselection, judge triggers, cycles/diamonds/multiple seeds and array versus argv ordering. `consumer` runs the actual built runner, registration and filemap subprocesses on committed temporary hosts with spaces in their paths, from another cwd, without HOME; historical binary bindings are absent. It checks exact report pointers, missing/malformed inputs, false DELTA/object/digest inputs and the retained-input boundary. These are single-worker direct checks, not independent review or native CI evidence.

The shared-inventory regression checks distinct definitions and safe unique resolution directly. Real consumer regressions cover same-ID project/script operations, distinct-ID success, reached and unrelated historical collisions, empty DELTA, alias-only changes, both record targets, base-only ambiguity, and the SPEC wire fields including removed required tests without approval.

Build prerequisites are the registered `build.runner`, `build.judge-registration`, `build.judge-filemap`, `build.judge-routes`, `build.judge-projects` operations. Then run `cargo test --locked --manifest-path crates/judge-filemap-tests/Cargo.toml`. CI's explicit binding contains these prerequisites. Tests use local synthetic commits; target candidate lifecycle remains caller-owned.

Impact v2 adds workflow selection; v1 did not enforce these requirements. Consumers reject unsupported output schemas. The Git FILEMAP registry remains schema version 2, and historical registry decoding is unchanged. Shared mixed classification and fixed-object loading now run before operation planning, so malformed selected semantic input stops downstream execution. New `workflow_selection` and `workflow_consumer` suites verify this contract through real processes.
