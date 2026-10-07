# FILEMAP impact contract

`chrono-judge-filemap --protocol chrono-judge/v1` is a candidate executable. It reuses runner facts, wire identity and transport, consumes registration's single bound interpreted view, including historical conversion, and publishes `outputs.impact`. It never calls registration's judge in place of a process. Registration still owns schema, readiness, checkout and direct-reference admissibility. Its immutable `node_data()` is the sole full-format node inventory; loading data alone does not certify semantic validity or input completeness.

The dependency direction is `judge-ci → judge-filemap → judge-mixed → judge-registration → runner`, with direct runner dependencies for facts/transport. The independent filemap test project has its own manifest, lock and target. There is no workspace or generic registry project.

## Versioned output

`schema` is `chrono-filemap-impact/v2`. Consumers may deserialize `chrono_judge_filemap::Impact`; JSON consumers must check this schema. Endpoint commits, trees and registry digest are bound by the enclosing v1 request/report. Arrays and maps are emitted deterministically. Unknown cost values remain JSON null. This output is declared graph impact, including validated effective environment changes when supplied; it is not operation evidence or retirement approval. The additive `structure` field is `chrono-filemap-structure/v1` and is a deterministic diagnostic view of the same explicit inputs.

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
| `structure` | Base, candidate and union graph snapshots with typed SCCs, original cycle witnesses, longest compressed-DAG depth, direct relation counts, explicit cross-owner edges, affected components, and execution-plan/resource conflict diagnostics; absent only in historical v2 reports produced before this field existed |
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

## Structural diagnostics

`structure` makes the failure modes which a plain impact closure cannot expose
visible to the same FILEMAP consumer. For each endpoint and their explicit union
it retains typed edges, direct outgoing relation counts, SCC membership, an
original-edge cycle witness for every cyclic component, and longest-path depth
on the SCC condensation DAG. A component is marked affected only when the
current DELTA closure reaches one of its registered nodes; unrelated historical
cycles remain context and do not turn a local change into a whole-repository
finding. Compressing an SCC is a reporting operation and never removes its
cycle or changes execution semantics.

The execution view lowers each registered plan sequence to operation
precedence edges and records the tests which declared each edge. It reports
operation cycles and the explicit resource/output claim pairs which serialize
otherwise ready operations. Claims are read from `execution_scheduling`; no
resource, output or prerequisite is inferred. An empty conflict list is a
measured absence for the registered claims, not a proof that undeclared effects
do not exist. Routes remains the execution authority and rejects a malformed
operation cycle before launching work.

Before publishing this diagnostic, the filemap judge runs a bounded
consistency check over the report it just produced. It checks schema and edge
counts, node partitioning, SCC identities and closed witnesses, component
depths, DELTA affected-component membership, and execution-plan precedence
edges. A violation is an `E_SELF_DIAGNOSTIC` error with the offending
invariant; the report is not treated as a successful impact result. This
detects an internally inconsistent judge result, but does not prove the
algorithm complete, the declarations necessary, or the host inputs complete.

This output is intended to expose a concrete candidate for an autonomous
refactoring round. It does not rank candidates, invent a consumer, claim a
runtime speedup or declare an optimization delivered. A real refactoring still
requires a fixed before/after mapping, preserved behavior and tests, updated
registrations, and the ordinary local/CI check.

The runner already aggregates `outputs.impact`. For the registration→filemap fixture it is exactly `/judges/1/response/outputs/impact`, recorded in `sources["/impact"]`; full transport also forwards predecessor impact to configured successors. Routes supplies bounded tool/input identities and projects supplies actual operation/test receipts; complete inputs, costs and remaining governance producers remain unresolved. A bounded host pass is not full SPEC or current-host activation.

## Operational CI adapter and retirement boundary

`judge-ci` retains its existing scoped snapshots, adoption/initial semantics, operation/binding/tool/environment/bounds seeds, validation, argv execution and exit propagation. It calls filemap's `graph::union`, `graph::closure`, and `Closure::selected_tests`; the former private traversal is removed. This mechanics API takes explicit edges/seeds and does not load or require full-format registration for legacy hosts.

CI evidence adds `selection_explanation` with scope `chrono-ci-check/v1-adapter`, flat union edges, node-ID `seeds`, additive `seed_causes`, closure, `extra_selections` and `legacy_only_selections`. Extra reasons explicitly identify changed operations, bindings, bound tools, environment or operation limits. For large DELTAs, derived predecessor/path material is bounded with an explicit omission marker, count and digest; consumers can recompute it from the registered graph and seeds. The adapter does not turn those reasons into whitelist edges or full-policy authority. Existing selected commands and execution semantics remain covered by the original CI suite; additional tests cover empty-graph legacy selection and changed execution bounds.

This adapter expires when full routes/projects and retained input/migration evidence replace its actual local/native consumers. Routes/projects now own shared execution and receipt checking. See [execution](execution.md) for the bounded input and finite ci-verify transition. Workflow certification/freshness, complete Cargo/SDK inputs, activation and native parity remain subsequent work. The host remains proposed/incomplete.

## Direct evidence

The dedicated `impact` suite specifies small endpoint graphs independently, checks witness edges against those graphs, and covers content/mode/binary DELTA, D+A rename, nonancestor and multi-commit endpoints, record/field/edge changes, all edge origins, removed owners/costs/tests, no-op/docs, typed propagation, paired nonselection, judge triggers, cycles/diamonds/multiple seeds and array versus argv ordering. `consumer` runs the actual built runner, registration and filemap subprocesses on committed temporary hosts with spaces in their paths, from another cwd, without HOME; historical binary bindings are absent. It checks exact report pointers, missing/malformed inputs, false DELTA/object/digest inputs and the retained-input boundary. These are single-worker direct checks, not independent review or native CI evidence.

The shared-inventory regression checks distinct definitions and safe unique resolution directly. Real consumer regressions cover same-ID project/script operations, distinct-ID success, reached and unrelated historical collisions, empty DELTA, alias-only changes, both record targets, base-only ambiguity, and the SPEC wire fields including removed required tests without approval.

Build prerequisites are the registered `build.runner`, `build.judge-registration`, `build.judge-filemap`, `build.judge-routes`, `build.judge-projects` operations. Then run `cargo test --locked --manifest-path crates/judge-filemap-tests/Cargo.toml`. CI's explicit binding contains these prerequisites. Tests use local synthetic commits; target candidate lifecycle remains caller-owned.

Impact v2 adds workflow selection; v1 did not enforce these requirements. Consumers reject unsupported output schemas. The Git FILEMAP registry remains schema version 2, and historical registry decoding is unchanged. Shared mixed classification and fixed-object loading now run before operation planning, so malformed selected semantic input stops downstream execution. New `workflow_selection` and `workflow_consumer` suites verify this contract through real processes.
