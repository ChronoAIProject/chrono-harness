# FILEMAP impact contract

`chrono-judge-filemap --protocol chrono-judge/v1` is a candidate executable. It reuses runner facts, wire identity and transport, loads both fixed registry snapshots with registration's strict `Registrations::load`, and publishes `outputs.impact`. It never calls registration's judge in place of a process. Registration still owns schema, readiness, checkout and direct-reference admissibility. Its immutable `node_data()` is the sole full-format node inventory; loading data alone does not certify semantic validity or input completeness.

The dependency direction is `judge-ci → judge-filemap → judge-registration → runner`, with direct runner dependencies for facts/transport. The independent filemap test project has its own manifest, lock and target. There is no workspace or generic registry project.

## Versioned output

`schema` is `chrono-filemap-impact/v1`. Consumers may deserialize `chrono_judge_filemap::Impact`; JSON consumers must check this schema. Endpoint commits, trees and registry digest are bound by the enclosing v1 request/report. Arrays and maps are emitted deterministically. Unknown cost values remain JSON null. This output is declaration impact, not operation evidence or retirement approval.

| Field | Meaning |
| --- | --- |
| `delta` | Complete factual A/M/D paths, old/new blobs and modes; no similarity rename inference |
| `records` | Stable record ID → `{base, candidate}`, each nullable; a record contains its registry `path`, explicit `targets`, and normalized `value` |
| `changes` | `{record, fields}` with relative JSON pointers for changed fields; empty pointer means the whole record was added/deleted; `/$registry_path` denotes relocation |
| `nodes` | Authoritative node ID → nullable base/candidate registration values, including old-only files, projects, tests, inputs and judges |
| `edges` | `{edge: {from, kind, to}, origin: base\|candidate\|both}` for the full explicit union |
| `seeds` | `{id, node, reference, reason}`; reference is an actual DELTA path or a stable record ID; reason is `delta-path`, `record-change` or `edge-change` |
| `closure.reached` | Node → all contributing seed IDs |
| `closure.predecessors` | Seed ID → node → one shortest predecessor edge, or null at the seed; supports finite consecutive witness reconstruction |
| `closure.traversed` | Every traversed edge and all contributing seed IDs; retains alternate diamond routes and cycle edges without enumerating infinitely many walks |
| `required_tests` | Test node, `candidate_present`, actual selecting `execution_edges`, nullable `base_cost` and `candidate_cost` containing the cost reference and value |
| `judges` | Each candidate every-delta judge once, plus its reached `judge-trigger` edges as extra explanations |
| `historical_context` | Unreached semantic edge defects with endpoint and edge; they are not findings for this DELTA |
| `limits` | Explicit boundaries: no execution/retirement/cost verdict, unretained external bytes, unproven completeness/locality |

Stable record IDs begin `/records/<registry>/<collection>/<key>`, using JSON Pointer escaping for the key. They are identities within `records`, not array-index pointers into the source JSON. File keys are paths, project/script/judge/input/tool/stability keys are IDs, owner keys are names, test cost keys are unprefixed test IDs, and cost model keys are cost IDs. Project-edge keys are compact JSON `[from,kind,to]` tuples. Other singleton fields (including arrays without an explicit unique identity) use `fields/<field-name>`. Input declarations are separate `config/inputs/<id>` records; the remaining environment is a singleton. All identity arrays are compared as sorted sets of declarations (duplicate declarations are retained in normalized values); `argv` and `version_argv` preserve order. Record targets never manufacture graph edges.

The complete base/candidate records retain owner declarations, old cost references and cost model values. A removed required test remains in `required_tests` with `candidate_present:false`; neither `executed` nor `retired` is invented. Missing cost declarations remain null, not zero. Later projects/workflow/cost judges must consume these facts and establish their own results.

## Policy and witnesses

File A/M/D changes seed `file:<path>`. Changed normalized records seed both endpoint targets, including executable test aliases supplied by registration for project/script records. Changed input declarations seed `input:<id>`. Added/removed/retargeted edges seed both affected endpoints; removed edges remain in the union. The finite forward closure traverses registered edges only. No pairing, directory, import, Cargo metadata or runtime discovery adds a dependency.

| Kind | Permitted source types | Target |
| --- | --- | --- |
| compile | file, project | project |
| build-input / runtime-input | file, input, project, script | project |
| test-execution | file, input, project, script | test |
| judge-trigger | file, project | judge |

Only traversing a `test-execution` edge selects a full-policy test. Reaching a test seed, changing test costs or owning a paired test does not select execution. Every-delta judges remain selected for an empty DELTA; triggers add explanations once. No actual input retention is implied by comparing external input declarations. The real registration process still rejects nonempty, unretained external inputs with `E_EVIDENCE_UNRESOLVED`; direct input-node tests stop at the declaration boundary.

Reached/changed invalid references and endpoint types produce `E_DANGLING_EDGE` / `E_EDGE_TYPE`. A finding's `delta_refs` identifies a seed path or changed record. `causes` is a consecutive node sequence from that seed, following the real union edges, ending at the defective edge endpoint. The edge and affected registry endpoint appear in the message. The closure preserves all contributing seeds even when a finding chooses one deterministic witness. Unreached historical semantic defects are context. Required unparsable or missing registries/objects, forged empty DELTA and digest disagreement return error without an empty-success impact.

The runner already aggregates `outputs.impact`. For the registration→filemap fixture it is exactly `/judges/1/response/outputs/impact`, recorded in `sources["/impact"]`; full transport also forwards predecessor impact to configured successors. Missing tools/effective-inputs/tests/cost producers remain unresolved. A bounded host pass is not full SPEC or current-host activation.

## Operational CI adapter and retirement boundary

`judge-ci` retains its existing scoped snapshots, adoption/initial semantics, operation/binding/tool/environment/bounds seeds, validation, argv execution and exit propagation. It calls filemap's `graph::union`, `graph::closure`, and `Closure::selected_tests`; the former private traversal is removed. This mechanics API takes explicit edges/seeds and does not load or require full-format registration for legacy hosts.

CI evidence adds `selection_explanation` with scope `chrono-ci-check/v1-adapter`, union edges, seeds, closure, `extra_selections` and `legacy_only_selections`. Extra reasons explicitly identify changed operations, bindings, bound tools, environment or operation limits. The adapter does not turn those reasons into whitelist edges or full-policy authority. Existing selected commands and execution semantics remain covered by the original CI suite; additional tests cover empty-graph legacy selection and changed execution bounds.

This adapter expires when full routes/projects and retained input/migration evidence replace its actual local/native consumers. Full execution, cost/mixed, retirement/workflow/freshness, effective tool/environment inputs, activation/bootstrap provenance and native parity are later increments. The current host remains proposed/incomplete, and its actual ci-verify script/pair/tool defects remain visible. No fake script or placeholder judge is supplied.

## Direct evidence

The dedicated `impact` suite specifies small endpoint graphs independently, checks witness edges against those graphs, and covers content/mode/binary DELTA, D+A rename, nonancestor and multi-commit endpoints, record/field/edge changes, all edge origins, removed owners/costs/tests, no-op/docs, typed propagation, paired nonselection, judge triggers, cycles/diamonds/multiple seeds and array versus argv ordering. `consumer` runs the actual built runner, registration and filemap subprocesses on committed temporary hosts with spaces in their paths, from another cwd, without HOME; historical binary bindings are absent. It checks exact report pointers, missing/malformed inputs, false DELTA/object/digest inputs and the retained-input boundary. These are single-worker direct checks, not independent review or native CI evidence.

Build prerequisites are the registered `build.runner`, `build.judge-registration`, `build.judge-filemap` operations. Then run `cargo test --locked --manifest-path crates/judge-filemap-tests/Cargo.toml`. CI's explicit binding contains these prerequisites. Tests use local synthetic commits; target candidate lifecycle remains caller-owned.
