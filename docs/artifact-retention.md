# Final originals and report retention

The retained-artifact owner governs final originals, fixed report slots and their actual readers, upload and repair consumers. It uses the lifecycle's shared kernel `ownership::Lease`, the existing durable inventory and `artifact_disposal` physical effects. Fixture custody remains owned by worktree's retained producer, including prospective allocations, copied members, native captures and diagnostic consumers. Observation never enrolls an unknown historical path.

**Source and host policy change together:** this candidate selects `.chrono-harness/retention.json` for future report publications, including `keep_seconds:0` and 25 ms maintenance rounds. Count/byte capacities remain PR172's 512 outputs, 128 summaries, 256 steps, 64 MiB verification/metadata per round, 1 GiB per output, 8 GiB total admission and 16384 descendant identities per output. Existing outcomes and expiries require explicit migration and remain unchanged. This source selection is not historical adoption, deployment or release permission. Implementation, native, migration and total storage costs are unknown; per-command observations belong to the actual verification reports.

## Publishers and consumers

| Natural owner | Publication and current use | Release |
| --- | --- | --- |
| preparation and retained blobs | durable creation intent, original digest/length, explicit preparation dependencies and `publication-recheck:OUTPUT_ID` consumer | actual fixed-slot publication transfers the exact closure to its final report; interruption retains the consumer |
| scoped/native process capture | prospective stdout/stderr enrollment before native writes, inherited capabilities, separately published launch record and actual joined outcome | final report closure handoff; failed/missing publication retains its repair consumer |
| scoped/full check | immutable report original, explicit snapshot/blob/preparation/stream dependencies, fixed current report slot | pointer replacement releases prior current closure; failed final reports retain `report-recheck:OUTPUT_ID` independently |
| full baseline export | reserved output before export, cumulative controlled byte admission and explicit recovery role | published final report owns the snapshot closure; failed/interrupted work keeps original recovery state |
| lifecycle reports | exact intent/result originals and actual recovery readers | terminal result transfers failed diagnostics to an exact recheck consumer; interrupted work and failed rebind resume retain recovery roles |
| native gather | exact acquisition intent and original gather report; existing PR207 shared report aliases and all attempt paths are preserved | current gather report owns acquisitions; failed gather retains a distinct exact recheck consumer |
| generated CI upload | optional `retention_delivery` in the current provider; successful upload action supplies actual artifact ID/digest | acknowledgement verifies the matching original report and exact `native-delivery:REQUEST_ID` root, preserving the original check result |

Readers of original descriptors and external blobs acquire shared use protection. Projects, workflow certificate, parity and offline collection readers use that owner too. Adopted parity publishes a new original and returns its address, preserving both compared originals and their old outcomes; a distinct fixed pointer follows the new publication. No check status, current pointer or upload metadata alone proves all external consumers have released their originals. Required report roots retain their explicit dependency closure; historical references and cycles alone are not roots. New current-slot publication protects old and candidate originals until the actual pointer write succeeds. Unknown/unpublished outcomes never become success.

Retained process adoption preserves the existing original stream/launch interfaces. Unadopted hosts retain existing behavior. Adopted stream creation is reserved before writes; a failed spawn keeps its enrolled empty/partial originals and repair state instead of silently removing owner evidence. Native process ownership still joins or excludes actual descendants; no PID/age inference substitutes for capabilities.

Lifecycle outcome metadata records operation/status and an exact report digest plus `/error` pointer for a failure. The complete original keeps the entire error chain. This avoids duplicating a large diagnostic in the finite inventory; existing outcomes remain unchanged.

## Shared physical effects and overlap

Fixture custody and final-original retention have separate release meanings. The current fixture recovery adapter names its already declared nested Git hosts and effect paths, then holds final-original admission exclusion across its exact effect. Any overlapping enrolled original blocks that fixture effect until the final-original owner retires it. Copied bodies use the same guard and shared entry-unlink implementation. This is explicit registry membership, not directory discovery. Ordinary publication cannot silently enroll an existing custody body: addressed legacy reuse verifies bytes and preserves its existing ownership; historical enrollment needs its exact owner evidence and exclusion. Final cross-lane composition and surviving-coordinator acceptance remain separate obligations.

## Bounded recovery and migration

The supported owner entry is:

```sh
.chrono-harness/bin/chrono-harness retention --host-root H --request P
```

`P` is an exact root-relative JSON request bounded to 64 KiB. Operations are `maintain`, `migrate-policy`, `enroll`, `release`, `protect-consumer`, `release-consumer` and `acknowledge-delivery`. A producer-generated result exits 0; invalid input, changed identity, unavailable capability or policy refusal exits 2. The result never declares historical engineering work completed.

Publication persists an intent, exact destination and parent identity before creating an empty staging inode, attaches its identity durably and publishes by a kernel no-replace rename. Actual completion records the original outcome. Ordinary adopted entries resume bounded accounting, interrupted creation and partial effects. The inventory, graph, directory snapshots, summary ring and read/write budgets are finite. Over-capacity protected state refuses controllable publication; external builders remain accounted after exclusion rather than under an invented filesystem quota.

Every effect rechecks policy, exact output/entry identities, references, capabilities and committed/index source. Sealed manifests/cursors cover only the explicitly enrolled output. Replacements, changed content, unknown late descendants and special files are preserved. Links are unlinked without following their targets. Logical removed bytes, observed allocated blocks and unknown physical savings remain distinct facts. Maintenance summaries retain original outcomes without claiming the original bytes still exist after retirement.

Policy migration requires the exact previous policy bytes and exclusive original producer/reader protection. It updates only the policy binding, preserving paths, original outcomes, expiries, roots and partial effects. Missing scopes, incomplete accounting, incompatible capacity or live consumers refuse migration. No backdating, scanner, handwritten inventory edits or capacity increase is supported.

A repair consumer uses a digest-bound declaration outside its disposable originals:

```json
{"schema":"chrono-retention-consumer/v1","consumer":"ACTIVITY_ID","purpose":"Actual required recheck inputs and release responsibility","originals":[{"id":"OUTPUT_ID","path":"EXACT_PATH","identity":"EXACT_DEVICE:INODE:TYPE"}]}
```

`protect-consumer` takes `declaration:{path,sha256}`. `release-consumer` additionally takes `receipt:{path,sha256}` and a JSON `pointer`. The responsible consumer must publish `{consumer, declaration_sha256, state:"released"}` at that pointer after actually accounting its inputs and preserving any still-required originals under their real consumers. Release checks exact membership, identity and exclusive reader exclusion. It changes no expiry or original result. Partial/shrinking declarations, unresolved state, changed originals and live readers refuse; bounded digest/count receipts support exact reentry while retained. A new consumption activity uses a new ID.

Historical enrollment separately requires a policy-selected exact producer source/receipt, original owner lease, path/outcome binding, required roots and any exact consumer release evidence. It does not scan or delete. Compatible deployment must join/exclude old writers and readers, including legacy implementations that lack these capabilities. Older strict inventories/binaries must not be mixed without that cutover.

## Outstanding acceptance

The historical 512 records / 493 extant / 19 absent observation, OS39/OS54 originals, historical current-release and `ci-persistent-cache` two-endpoint migration mismatch remain unresolved. Their originals and current consumers stay protected under their actual owners; future policy selection does not settle them. L1 source and all retained commits remain intact. Fixed compatible input/native/PR207 handoffs, complete shared acceptance, exact-source native runs, independent review, coordinator deployment, actual landing/postconditions and full SPEC §10.1 acceptance remain caller/L3/L4 obligations. Local Rust regressions or generated upload YAML do not establish remote artifact availability or real native release.
