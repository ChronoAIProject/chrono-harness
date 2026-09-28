# Workflow verdicts and integration evidence

`chrono-judge-workflow` owns branch freshness, explicit retirement/migration and
integration certification. It is a separate production project with a dedicated
test project. Configure it after every other judge; it consumes registration's
interpreted endpoints, FILEMAP selection, routes' plan, projects' actual results,
costs and runner process observations. It reuses the existing executor and receipt
comparison. The judge does not create worktrees, PRs or merges. The separate
[worktree producer](worktree.md) now creates fresh registered worktrees and
retains actual Git observations. Explicit reconstruction plans now stage selected old changes on a freshly fetched target. AI reconciliation, recovery, PR/merge and
landing orchestration remain caller-owned pending their producers.

Use the same registered `chrono-harness check` invocation locally and in CI.
Workflow requires context schema 2, with all existing context fields plus
`run_kind: "integration" | "delivery"`. Registration still reads context v1 for
bounded consumers without this judge. An integration run requires the registered
integration prefix. Delivery from an integration-named branch still consumes
completed evidence; its name cannot turn delivery into an integration run.

Freshness uses fixed candidate/dev/fork objects and explicit UTC timestamps.
`base` must equal `dev_tip`; fork must be a common ancestor. Behind counts commits
reachable from dev but not fork. Age is observed time minus declared branch start,
never the wall clock or a commit date. Strict excess of either registered limit
returns `E_BRANCH_STALE`; equality passes. Missing objects, shallow boundaries
beyond the proven fork, invalid branch names and negative ages fail explicitly.
The caller supplies source-branch context for the dev landing commit.

An integration run executes selected tests before publishing the registered
state certificate. Its nine mandatory bindings are:

| Field | Evidence |
| --- | --- |
| `base`, `candidate_tree` | Fixed Git endpoints |
| `registry_digest` | Original endpoint registry bytes |
| `executables` | Actual runner and all configured judge paths/digests |
| `tools` | Resolved paths, bytes and observed version results |
| `environment` | Effective values, including empty values |
| `effective_inputs` | Validated endpoint input identities, including explicit v2 file presence/absence |
| `required_tests` | Old/new obligations, selected plans, methods and bounds |
| `results_digest` | Original successful execution results |

The certificate retains producer context, plan, results, inputs and preceding
judge observations. Runner retains each completed full report at a unique
`.chrono-harness/state/run-<identity>.json`, also publishes the latest report at
`report.json`, and transports the retained path to judges. Observations bind the
actual judge request digest to process stdin; configured metadata is not execution.

Delivery supplies the certificate byte digest in `integration_evidence` and
transports its matching retained report into the consumer's state directory.
The report must be finalized successfully and contain the workflow response
publishing that exact certificate digest. A certificate left by an incomplete or
failed run is insufficient. Every required process/receipt must match its plan;
configured-but-unexecuted judges cannot be certified. Producer freshness is also
checked at the delivery observation time.

Candidate commit metadata may differ when base, tree and other bindings match.
Comparison normalizes declared host paths and candidate commit identity only;
environment values remain exact. Original evidence is preserved. Delivery still
runs its selected tests; certificate consumption is not a test cache. Rule/product
changes retain mixed's warning and costs, without acknowledgement or human gates.

Retirement requires explicit declarations. A replacement test must actually pass;
null retirement requires removal of its paired producer and a matching producer
declaration. Removed judges need declarations, never execution of old binaries.
Historical test aliases may be repaired by naming every old definition and a
unique replacement, including the same surviving alias. Old costs and definitions
remain visible. Routes/projects defer changed historical methods only when the
configured downstream migration validator consumes their results; standalone and
scoped consumers keep their stricter preservation contract.

Schema changes are compared using original versions before decoding. FILEMAP
selects explicit compatibility tests and integration requirements. Workflow checks
retained candidate conversion and actual compatibility-test success against the
declared version transition and mappings. The shipped host conversion remains the
finite `chrono-ci-check/v1` plus FILEMAP v1 decoder. Workflow v3 additionally allows
explicit `from_versions`/`to_versions` profile selection across all five registries,
including config v1→v2; it runs the registered candidate script and requires its
dedicated compatibility test. Ambiguous matches fail, unmatched profiles never
certify a transition, and each changed version pair still requires a migration
declaration. Historical config must retain its original interpretation and snapshot
binding. See [execution](execution.md) for the v2 decoder input; no migration or
dependency is discovered automatically.

Bounded fixtures exercise the complete seven-judge chain. This host remains
proposed with incomplete Cargo/SDK input closure and bindings; native CI still
uses its documented scoped adapter. Certificates explicitly report
`input_completeness_proven: false` in workflow verdicts. Full activation, complete
inputs, deterministic parity and complete AI reconciliation/PR/landing orchestration remain
separate unfinished obligations. Worktree creation alone does not discharge them.
