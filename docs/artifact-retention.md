# Explicit output retention

The retained-artifact publisher owns output identities and reference lifetimes. It reuses the lifecycle kernel `ownership::Lease`, existing state publication and physical `artifact_disposal` effects. Absence of `.chrono-harness/retention.json` preserves old behavior. Unknown historical directories are never discovered or silently enrolled.

**Mixed source and host policy change:** the host opts in fixture, preparation, full report/blob and worktree report producers. It retains the seven-day window, 512 outputs, 128 summaries, 256 steps, 64 MiB original verification reads and 1000 ms rounds. It adds 1 GiB per-output reservation, 8 GiB total admission, 16384 descendant identities per output and 64 MiB round metadata IO. Blob copies, full baseline exports, full reports and lifecycle report writes check their reservations before writing. Direct fixture tree writes and creation before enrollment remain unfinished source obligations; the policy does not establish a hard physical ceiling for those writers. Protected overflow refuses admission and preserves originals.

`Inventory::begin` records an exact physical output before its producer writes content. A publication capability protects the producer and inherited descendants. `complete` retains its actual outcome; interruption cannot create success. Current report slots, recovery and native delivery are explicit roots. Dependencies alone are not roots, so released historical cycles can retire. Slot publication protects both old and candidate originals until its pointer is published. Actual original readers acquire the same owner's shared capability.

Maintenance first accounts inactive publications, including rooted outputs. A bounded directory-name snapshot and durable cursor produce a sealed manifest of every descendant identity and file/link length/stamp. Disposal walks that sealed list backwards, never enrolling new children during deletion. Replacements, modified content, unknown late children and special files remain protected. Every effect rechecks policy, capability, exact output/entry identities and committed/index source through the Git fact owner. Internal symlinks are unlinked without following their target. Logical removed bytes and still-readable evidence are separate facts; physical allocation reclaimed remains unknown.

The inventory, roots, graph, directory snapshots, metadata reads/writes and summary ring have finite bounds. Partial accounting and deletion resume on ordinary adopted start/reconstruct/check/bootstrap/use/report entries. No daemon, idle scanner or additional lifecycle platform is introduced. The existing independent worktree maintain entry continues its lifecycle role; the retention command delegates to this same output owner.

| Actual owner | Publication and consumer binding | Release |
| --- | --- | --- |
| worktree/cache/CI/migration fixtures | exact fixture output and inherited probe/test protection | joined producer, declared references, retention window |
| preparation/scoped checks | immutable originals, acquisition and stdout/stderr dependencies, fixed report slots | slot replacement and explicit delivery acknowledgement |
| full checks | original report, base snapshot, staged blobs and current report closure | report replacement; recovery role after original report publication |
| worktree lifecycle | intent/result originals and actual recovery readers | terminal result releases recovery; failed rebind keeps its resume root |
| CI provider upload | opt-in `retention_delivery` in the existing provider configuration | successful upload action supplies artifact ID/digest to matching delivery root |

The full/lifecycle additions and native release protocol still need complete integration acceptance. Upload metadata alone does not prove content validity or remote rerun availability. Public beta21's v2 process encoding, decoders and full-input producer behavior are carried forward; legacy inline/v1 originals remain readable. Compression does not constitute retirement.

Historical adoption uses one supported owner entry:

```sh
.chrono-harness/bin/chrono-harness retention --host-root H --request P
```

`P` is an exact root-relative JSON request, limited to 64 KiB. Operations are `maintain`, `migrate-policy`, `enroll`, `release` and `acknowledge-delivery`. Success prints a producer-generated JSON result and exits 0; invalid input, changed identity, unavailable capability or policy refusal exits 2. No operation declares the historical work completed.

For policy migration, preserve the exact prior policy as an ordinary registered artifact, install the candidate binary and candidate policy under compatible original-owner exclusion, then use:

```json
{"operation":"migrate-policy","previous_policy":{"path":".chrono-harness/state/prior-retention-policy.json","sha256":"EXACT_PRIOR_POLICY_SHA256"}}
```

The owner validates the old binding, candidate scopes/capacities and all producer/reader exclusions, then changes only the metadata policy binding. Original outcomes, paths, expiry, roots and partial effects survive. Run bounded accounting with the previous adopted policy before introducing smaller capacities; unknown/unaccounted storage must not be declared within budget. No raw inventory editing is supported.

Historical enrollment is explicitly host-selected by `historical.KEY` in the same policy. Each rule supplies `producer`, literal `path`, `producer_source` and original `receipt` as `{path,sha256}`, `receipt_schema`, `output_pointer` (or null for a literal legacy owner declaration), `outcome_pointer`, the original `owner_lease` and `owner_lease_identity`, required `roots`, and optional `release` evidence. Source/receipt inputs are bounded regular files with exact digest checks and must remain outside the disposable output, including the consumer release receipt. The original kernel capability must be exclusively available. The caller must deploy compatibly and join old writers/readers, including legacy implementations which do not participate in the capability contract.

```json
{"operation":"enroll","enrollment":"KEY","expected_identity":"EXACT_DEVICE:INODE:TYPE"}
```

The CLI reads the outcome from the original receipt, binds the exact output and publishes required recovery/delivery roots in the initial reservation. It does not remove bytes. Neither failure, age nor size is release permission. A rule's `release` contains an exact `receipt`, JSON `pointer` and `expected` value from the responsible consumer. Only then may the caller use:

```json
{"operation":"release","enrollment":"KEY"}
```

Release removes only that enrollment's registered roots. Subsequent ordinary maintenance applies the same expiry, source/index and identity checks. Producer source and original receipt remain readable independently of retired output bytes. The Rust CLI regression exercises refusal for a live original owner, enrollment, explicit release and 2048 logical bytes removed while the exit-101 receipt stays intact. Actual beta19 historical enrollment and cleanup remain caller-owned and unperformed.

For native upload, the existing CI provider may set `retention_delivery:true`. Its generated acknowledgement executes after a successful `actions/upload-artifact` result and passes the action's artifact ID/digest through environment variables. The acknowledgement consumer accepts the current typed report reference, verifies its bound original digest with the existing original reader, and rejects drift. The retention owner verifies the matching `native-delivery:<request_id>` root. The receipt preserves the original check outcome, including failure. Local unit tests and generated YAML do not establish remote delivery/rerun acceptance.

Remaining source work: eliminate creation-to-enrollment gaps for temporary files/directories and blob staging; enforce physical tree-write capacity in direct fixture/build writers; complete native rerun and all recovery release paths; validate full/lifecycle integration and the actual historical producer adapter. Unknown ownership and unfinished recovery remain protected. Caller owns committed clean-candidate canonical/native validation, compatible two-end deployment, real historical cleanup, exact dev landing and public release. The old 3636/3722-byte cleanup-policy mismatch and 2022211/1048576-byte full-config reader failure remain explicit. Neither public pins nor protected full-adoption source/allowance are changed.
