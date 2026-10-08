# Explicit output retention

The retained-artifact publisher owns output identities and reference lifetimes. It reuses the lifecycle kernel `ownership::Lease`, existing state publication and physical `artifact_disposal` effects. Absence of `.chrono-harness/retention.json` preserves old behavior. Unknown historical directories are never discovered or silently enrolled.

**Mixed source and host policy change:** this candidate adds explicit fixture-consumer protection/release to the retained-artifact owner and adopts native delivery acknowledgement in the host CI provider. The host policy selects `keep_seconds:0` for future publications and 25 ms maintenance rounds. Existing seven-day expiries survive explicit migration unchanged. Required/live roots protect originals regardless of their expiry. The observed inventory was full at 512 outputs despite 469 released records because no extant original had expired; releasing a root does not remove an old hold.

The host retains its other limits: 512 outputs, 128 summaries, 256 steps, 64 MiB verification reads and metadata IO per round, 1 GiB per output, 8 GiB total admission and 16384 descendant identities per output. The 45 default-concurrency cache tests compete with maintenance under the existing five-second admission window. Earlier 1000 ms and 100 ms maintenance trials retained admission failures; the final 25 ms source fixture passed all 45 tests. Shorter rounds reserve more of that window for publication, with durable cursors carrying unfinished cleanup to later ordinary entries. Recovery and retired-lease draining now check the same time predicate as accounting/disposal. This is an explicit host scheduling choice; it does not raise capacity or promise an exact wall-clock ceiling for an individual filesystem operation.

Cache fixtures reserve 32 MiB each and use cumulative controlled copy/write/directory operations, within the unchanged 1 GiB per-output ceiling and 8 GiB total. Blob copies, full baseline exports, full reports and lifecycle report writes check reservations before writing. Worktree, CI and migration fixture captures charge cumulative bytes and descendant count before their effects. Preparation originals, full snapshots/reports and lifecycle report creation use durable creation intents. External builders are accounted after ownership exclusion; protected overflow refuses admission and preserves originals. These contracts do not promise a universal physical filesystem ceiling.

Typed inventory decoding reuses the existing strict JSON visitor for opaque fields and rejects duplicate/unknown members without constructing a second whole-ledger JSON value. Diagnostic runs with 155 decodes of inventories larger than 650000 bytes reduced median decode time from 13174 to 6771 microseconds; median encoding remained about 17.7 ms. This measures decoding cost, not overall test or host savings. The actual cache recheck holds the owner read capability, verifies stable cache keys and compiler bytes, and preserves independent originals for different ownership descriptor transfers. Its expected-failure consumer releases only after verifying the original exit-7/stderr. Rust ordinary-entry checks aggregate partial reclamation across bounded rounds while retaining the original byte/outcome assertions.

`Inventory::begin` remains the explicit existing-output enrollment API. New producers use `plan`/`create_adopted`: persist the exact destination, parent identity and one empty owner staging slot, attach the staging inode durably, then publish it with a kernel no-replace rename. No content write precedes attachment. Ordinary maintenance recovers abandoned reservations, empty staging slots and interrupted publication, preserving occupied/replaced destinations and the original unknown outcome. The count and metadata limits include these intents. A publication capability protects the producer and inherited descendants. `complete` retains its actual outcome; interruption cannot create success. Current report slots, recovery and native delivery are explicit roots. Dependencies alone are not roots, so released historical cycles can retire. Slot publication protects both old and candidate originals until its pointer is published. Actual original readers acquire the same owner's shared capability.

Maintenance first accounts inactive publications, including rooted outputs. A bounded directory-name snapshot and durable cursor produce a sealed manifest of every descendant identity and file/link length/stamp. Disposal walks a sealed list backwards. Byte overflow is accounted rather than preventing sealing. A released tree which exceeds the finite manifest capacity uses a durable depth-first cursor: observe one exact descendant under the enrolled tree identity, persist its identity/content stamp before an effect, and recheck it under existing policy/reference/ownership exclusion. This fallback never scans outside that explicitly producer-owned tree. Previously observed entries are revalidated; sealed trees keep the existing refusal for late unknown descendants. The cursor has at most 128 entries, and larger depths and special entries remain protected. Replacements, modified content, unknown late children and special files remain protected. Every effect rechecks policy, capability, exact output/entry identities and committed/index source through the Git fact owner. Internal symlinks are unlinked without following their target. Regular originals larger than a verification round use the sealed inode/content stamp for deletion; the summary explicitly says their digest was not reread. Smaller originals retain their bounded SHA-256 recheck. Logical removed bytes and still-readable evidence are separate facts. Manifest observations report allocated blocks separately from logical lengths, identify incomplete/legacy allocation as unknown, and never call either value physical bytes reclaimed; reclaimed allocation remains unknown.

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

`P` is an exact root-relative JSON request, limited to 64 KiB. Operations are `maintain`, `migrate-policy`, `enroll`, `release`, `protect-consumer`, `release-consumer` and `acknowledge-delivery`. Success prints a producer-generated JSON result and exits 0; invalid input, changed identity, unavailable capability or policy refusal exits 2. No operation declares the historical work completed.

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

Remaining acceptance: CI gather download/transport, complete native rerun and full/lifecycle integration remain caller/owner obligations. The existing standalone CI prerequisite failure at initial_projection.rs:286 and later unrun targets belong to the next CI assignment. A generated upload acknowledgement does not establish native execution. Caller owns clean-candidate canonical/native checks, compatible deployment, historical cleanup and byte measurements, exact dev landing and public release. Source1 supports the immutable 2022211-byte input through its explicitly registered 2097152-byte bound; its actual coordinator adoption is unperformed here. The old cache migration mismatch remains a distinct lifecycle-owner obligation. Public pins and protected full-adoption source are unchanged.

Deployment boundary: this source adds optional inventory creation/cursor/allocation fields under the existing inventory schema. Candidate readers accept older inventories; old strict readers reject inventories written with the new fields. Join/exclude old writers and deploy compatible binaries before activating these producers. This is source compatibility evidence, not an executed coordinator deployment or a policy migration receipt.


## Required repair/recheck consumer

The actual fixture owners attach `consumer:fixture-recheck:<output-id>` through `create_adopted_for_consumer` in the existing durable publication before original writes, without an extra inventory transaction. Cache fixture interruption and failed cache/worktree/CI/migration captures leave that durable root. Successful joined cache fixtures and successful requested worktree/CI captures use `complete_releasing_consumer`, releasing their own single-output root in the same durable publication as their actual outcome. Failure, partial capture, or missing completion is never inferred as release. The ordinary adopted maintenance entry resumes original accounting/disposal; it does not declare the repair completed.

For already enrolled originals, the host explicitly supplies a digest-bound declaration outside the disposable outputs:

```json
{"schema":"chrono-retention-consumer/v1","consumer":"REPAIR_ID","purpose":"Actual required repair/recheck inputs and release responsibility","originals":[{"id":"OUTPUT_ID","path":"EXACT_OWNER_PATH","identity":"EXACT_DEVICE:INODE:TYPE"}]}
```

Use the registered retention owner entry with these request shapes:

```json
{"operation":"protect-consumer","declaration":{"path":"DECLARATION_PATH","sha256":"EXACT_DECLARATION_SHA256"}}
```

```json
{"operation":"release-consumer","declaration":{"path":"DECLARATION_PATH","sha256":"EXACT_DECLARATION_SHA256"},"receipt":{"path":"ACTUAL_CONSUMER_RESULT_PATH","sha256":"EXACT_RESULT_SHA256"},"pointer":"/retention_release"}
```

A consumer ID denotes one consumption activity; a new activity after release uses a new ID. Interrupted reentry repeats the same identity. The responsible consumer must actually publish `{"consumer":"REPAIR_ID","declaration_sha256":"EXACT_DECLARATION_SHA256","state":"released"}` at that pointer after accounting its inputs and preserving any still-needed originals under their actual consumers. This is a domain state, not a tool-written successful recheck. An unresolved result is not release. The request/declaration/result are ordinary explicit host inputs, not a second registry. The existing inventory roots remain authoritative. Declaration/result reads are bounded to 1 MiB each; requests to 64 KiB; original and root counts to the adopted policy. Protection validates every enrolled ID/path/inode under the existing admission exclusion and shared capabilities, fails atomically on missing/drifted/disposing inputs and cannot silently shrink an existing root. Release validates the exact root and exclusive original capabilities, refuses live readers or changed identities, then publishes a bounded digest/count receipt. It changes no expiry or original outcome. Exact retries use that receipt while it remains in the bounded summary ring; once evicted, a retry reports insufficient release evidence.

No inventory schema change is required for these consumers: old root vectors and inventories remain readable, and all policy defaults remain optional for old hosts. Absence of retention adoption keeps old behavior. Existing creation/manifest compatibility limits above still apply to older strict binaries.

## Current host transition handoff

The host declarations under `.chrono-harness/retention-consumers/` name exact original identities from inventory SHA-256 `a6961bd289c81abe7f814987e6062ab499bfd20a9da76b303b7b94742b1418f1`: 512 records, 493 extant originals and 19 explicitly recorded absent outputs. `current-repair-recheck.json` protects the extant set during actual current failure/recheck and retention accounting. It is temporary transition protection, with no assertion that every historical failure remains necessary. `required-os39.json` independently protects the exact d909/bac200 trees containing check-kDhXA0/check-lvpNl3. Passing later regressions have not accounted for their original os39/os54 cause. Original source1/2/3/4 failure logs, source snapshots and caller-owned evidence outside these enrolled scopes must continue under their own existing owners; the declarations do not claim to manage `.sshx` or coordinator files.

Caller sequence, under compatible deployment and joined/excluded old writers/readers:

1. Install the accepted candidate binary while retaining the exact currently bound policy `f91bcd38f9031f4fa4ce17a77a118f98a4fe2e7e136d1581216f33d7d7f356cb`. Recheck the declaration against actual owner IDs; drift is an error, never permission to regenerate it by scanning. Invoke the same retention entry with `.chrono-harness/retention-consumers/protect-current.json`, then `protect-os39.json`. This attaches roots without enrolling a duplicate output or requiring a new output slot. Check both actual receipts.
2. Under that old policy, invoke registered bounded maintenance to account the 43 sealing records and recover the 19 absent records. Required originals stay rooted; absent metadata recovery is not measured byte reclamation. Preserve any partial/error results. Unknown or unsealed capacity can still prevent migration.
3. Preserve the old policy as an exact registered input outside disposable outputs. Adopt this candidate's policy with `keep_seconds:0` and use `migrate-policy` with that input's path/digest. The migration must preserve original paths, outcomes, roots and expiries. It does not override either current count admission or seven-day old holds. The 19 absent records may provide limited headroom; if old extant records still fill admission, further work honestly refuses until eligible originals retire. Do not raise capacity or backdate expiry.
4. Recheck actual source1/2/3/4 and report originals. Establish separate exact consumer declarations for every still-needed original, including prepared/full/lifecycle closures and any residual causal failures, before the responsible transition consumer releases its 493-original declaration. Publish real release state/results, use `release-consumer`, preserve its receipt and only then run ordinary owner maintenance/measurement. A failed outcome alone neither keeps everything forever nor permits release. The existing seven-day old windows must still elapse. The candidate CI provider opts into `retention_delivery`; generated successful-upload acknowledgements release only matching native-delivery roots. Verify that path natively after the next CI prerequisite assignment.
5. Keep actual future build/test/bootstrap/check callers under registered use/participation. Successful future fixture consumers release their roots; interrupted/failed captures await genuine repair/recheck release. Later ordinary entries reclaim eligible outputs without finish. Record logical removed bytes separately from before/after allocated blocks and unknown physical savings, with units, scope and observation window. No current host cleanup, deployment or native acceptance was executed by these source fixtures.

`ci-persistent-cache` at fixed 3219b9d remains separate: the new coordinator requires a startup disposal artifact absent from that old source. The present migration insists on matching committed policies and both-endpoint registration. It has no supported endpoint-subset migration, so copying declarations or manufacturing unused startup consumption would bypass its real contract. The lifecycle owner must supply the smallest explicit two-endpoint disposal selection path or complete a separately authorized actual adoption; preserve the fixed commit, source/bin/evidence and original failed migration. This assignment changes neither endpoint nor that migration owner.
