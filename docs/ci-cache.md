# Registered CI caches

`chrono-cache` is an independent Rust product with its own `cache-tests` project,
CI unit and central release asset. It does not compile with the CI generator.

`chrono-cache plan --host-root H --config .chrono-harness/cache.json
--consumer ID` prepares keys for one explicitly registered consumer. This is an
internal provider entry; the daily check remains `chrono-harness check`.

## Bounded native acceptance

The host has a real GitHub Actions consumer for the registered `cache` unit. The
first run of [workflow 37627480033](https://github.com/ChronoAIProject/chrono-harness/actions/runs/37627480033)
was a cold run: both `check.cache` and `check.cache-tests` reported misses,
continued through the original bootstrap and canonical check, and completed their
native saves. A later rerun observed the platform's cache visibility delay and
saved again; this is retained as an observation rather than generalized cache
behavior. On [attempt 3](https://github.com/ChronoAIProject/chrono-harness/actions/runs/37627480033/attempts/3)
of the same immutable commit, both entries restored with `exact` status. The
transport report recorded `current_executable_verification: matched`,
`recovery.status: not-required`, no warnings, a successful original work result,
and `verdict: not-a-judgment`. The canonical bootstrap and check still ran after
the hits, and no save was attempted for the exact-hit attempt.

This historical observation proves the registered consumer's bounded cold miss/save and exact-hit
restore path, including current cache executable identity and recovery evidence.
It does not prove changed-source reuse, incompatible-input partitioning, corrupt
or unavailable archive handling, concurrent producers/reruns, complete backend
lifecycle protection, or cache completeness for other consumers.

The current `producer-manifest/v1` domain has additional bounded native evidence
on macos-26, Rust/Cargo 1.95.0, under PR #207's `refs/pull/207/merge` scope:

| Scenario, both `check.cache` and `check.cache-tests` | Fixed native observation |
| --- | --- |
| Cold production domain | [39688684172/1](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39688684172/attempts/1), seed `98615238`: original Cargo work, seal/save and collection passed |
| Compatible source change | [39690377913/1](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39690377913/attempts/1), production `72b1d08`: equal compatibility prefixes, distinct source suffixes, seed-key restores and matched producer inventories, retained incremental data and current registered rebuild/tests passed |
| Healthy exact hit | [39690377913/2](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39690377913/attempts/2): both primary keys matched, inventories verified and original canonical work/collection passed; detector/startup retain their original attempt 1 identities |
| Incompatible producer environment | [39695762671/1](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39695762671/attempts/1), scenario `c515cd05`: both domains changed, neither restore matched a key, and original optimized Cargo builds/tests, seals and collection passed |
| Unusable contents, recovery, persistent repair and repaired exact hit | Initial observation: Variables GET/list HTTP 403 prevented activation. Later C1 partially observed unusable payload recovery/rebuild and replacement caches; repaired original-domain healthy exact hit remains unmet (see retirement below). |

In the incompatible scenario the only changed selected observation was
`check.environment`; all other selected runtime/probe/absence observations and
producer argv matched production. Compatibility digests changed from
`d24c19748a988bdb5732af429ed07932e5e4ac25774ef4d4613b5be87ce34c6f` to
`b9653959eb2bc10a7a166822f4e59c4374d31105acf865a8b980c372c4af7987`
for `check.cache`, and from
`070f000f3499ba497fc6d1c4eac347720586017d61c114954293f69c21964228` to
`17b22e6e2b9ea72b6eb2afdc98a63075b62fe778baba507511337f259b317cbc`
for `check.cache-tests`. Each restore exposed only its new domain prefix;
production fallback was impossible through that declared key set. Both actual
Cargo producers received `CARGO_INCREMENTAL=1` and the two optimization values,
and reported optimized dev/test output. The transport retained
`miss-or-unavailable`, successful original work and matched current planner
identity; backend entries `9310328424` / `9310330849` were observed, with creator
unestablished. This is partition/rebuild evidence, not an exact-restore test of
those scenario archives.

Original scenario artifacts are cache `12391316467`, detector `12391201225`,
startup `12391076536`, runner `12390726564` and collection `12391598633`, all from
attempt 1. Collection verified 26 unit originals and passed all 27 result rows.
The cache and collection jobs measured 113 and 106 seconds respectively from
native start/completion timestamps; these windows are not billing, speedup or
complete lifecycle costs. Transfer/storage and total incremental costs remain
unknown. An ordinary revert `c72c141` restores the exact production `72b1d08`
tree and removes both temporary optimization settings while preserving the
scenario commit. Unusable-content acceptance remains required; the access gap
does not waive it. The optional remote activation continuation has since been retired as documented below;
its partial historical observations do not establish a repaired exact hit. Other consumers,
concurrency, linked ownership and complete compiler/SDK/input closure remain open.

The planner prepares identities; the GitHub provider projects pinned native
restore/save actions around the original bootstrap and check/release command.
The host explicitly adopts those actions in its check and release registrations.
A successful plan or generated workflow does not establish native restoration,
saving, compiler reuse or acceptance. SPEC §4.2 still requires backend evidence,
cold/warm/changed-source/failure/concurrent acceptance and current executable
verification. Each plan also records the exact `chrono-cache` executable digest
and version that prepared it. The report step compares that identity with the
currently running executable before publishing transport evidence; a mismatch
fails closed. The transport currently requires a primary Git checkout; linked
worktree restore/save needs a lease spanning the entire transport and remains
unsupported.

This host explicitly sets `CARGO_INCREMENTAL=1` for its canonical check actions.
The core, full and cache seed bootstrap registrations and the v5 release recipe set
`rust_incremental: true`; their existing build invocations receive the matching
environment value, including release builds and Rust verification units. The
release recipe's explicit `rust_toolchain` selection keeps that setting scoped
to Rust consumers. Non-Rust units retain their own environment.

Bootstrap and release receipts record the value passed to children. Release
import and collection compare it with the registered choice and include it in
the producer toolchain fingerprint. Omitted settings preserve the previous
environment behavior; malformed settings fail before launching build tools.
These observations do not establish cache transport, actual compiler reuse,
complete build inputs, or native acceptance. Rust profile, flags, compiler and
SDK identities still belong in the persistent cache compatibility contract.

The host adopts `chrono-cache/v2`; the registration belongs to its `.chrono-harness/`
directory. It explicitly declares:

- A namespace and an `artifact_registry` path. Host cache artifacts reference
  exact paths in that registry's `artifacts` array; each must have the same owner,
  `tracked: false`, and must not be execution evidence. An explicitly external
  dependency cache may instead declare an absolute path. Other artifacts remain
  relative to the host. No directories or languages are discovered.
- Named inputs: `file` has a literal path and expected `presence` (`present` or
  `absent`); `environment` has a variable name and expected presence; `literal`
  carries a JSON declaration. Present files are hashed from their actual bytes;
  environment observations retain a digest and byte length without the value.
  `json-value` has a literal `path` and an explicit RFC 6901 `pointer` into a
  present JSON document. Its key binds the selected value, including explicit
  null, independent of object ordering, formatting and other fields. Empty
  pointer selects the whole document. Invalid JSON/pointers and missing values
  fail; the planner does not infer a selection. The plan also retains the whole
  file's digest and length in `source_file` as provenance outside the key.
  The same path and input/output exclusion rules apply as for file inputs.
  Literal declarations are not observations of actual toolchains or platforms.
  A `command` explicitly registers a bounded runner command, inherited variable
  names and `result: stdout` or `file-path`. Successful observations bind the
  executable bytes, actual argv, hashed environment and stdout/stderr. The latter
  mode also hashes the actual file named by one absolute UTF-8 output path.
  Failed probes retain their original process report before returning nonzero.
  Raw reports belong to the host's private execution evidence and may contain
  inherited environment values; the key uses observations, not an evidence path.
  A `registered-files` input names a FILEMAP v2 `registry`, `node` and explicit
  `edges` kinds. It hashes the registered transitive source closure and selected
  edges; it does not scan imports, directories, languages or undeclared files.
  FILEMAP validation remains the existing judge's responsibility.
- Named artifacts with `owner`, `path`, `kind` (`dependencies`, `compilation`, or
  `executable-candidate`) and explicit `external` boolean.
- Named caches with `owner`, registered `producer` operation ID, `consumers`,
  artifact IDs, disjoint `compatibility_inputs` and `source_inputs`, and `restore`
  (`exact` or `compatible`). Each list is nonempty and has unique members.
- `consumer_operations` binds every consumer to its existing method registrations.
  Each reference has a literal host `registry` path and a JSON `pointer` selecting
  one operation ID or a nonempty array of unique IDs. Missing references, malformed
  values and subscriptions whose producer is absent from that consumer's selected
  operations fail before input probes and native restoration. The plan retains
  source digests and selected IDs. It does not infer methods from language, paths
  or runtime activity, and a valid declaration does not prove actual execution.

A consumer selects only caches naming it. Missing consumers fail instead of
selecting all caches. Only the selected input closure is read; an unavailable
input belonging solely to another consumer does not affect this plan. Registration
shape and input/artifact references are checked without probing unrelated inputs.
Only the selected consumer's operation sources are read; unselected consumers'
sources are not probed. Source registries cannot themselves be cached. The host
references its bootstrap recipes and FILEMAP plans for check jobs, and its release
recipe's build unit IDs or verification operations for release jobs. Native
execution and completed producer evidence remain separate obligations.

The compatibility digest binds the namespace, cache identity, owner, producer,
restore policy, artifact declarations and their selected registry records, plus observations of
the declared compatibility inputs. The source digest binds the declared source
input observations. Keys are `chrono-v2-<compatibility digest>-<source digest>`.
Consumer routing is checked separately; adding a consumer does not invalidate
unchanged producer outputs. Any consumer-specific build inputs must still be
declared in the selected cache's compatibility or source inputs.
Reordering input IDs does not change the digest. Changes to unrelated cache
declarations or unrelated files do not invalidate the selected cache.

The earlier v1 draft remains readable under its original contract: consumer names
participate in its `chrono-v1-` keys and it has no operation-reference enforcement.
V1 cannot adopt `consumer_operations`. V2 uses a separate compatibility domain;
it does not restore v1 archives through a broad fallback.

`compatible` supplies exactly one restore prefix ending after the compatibility
digest. Source changes can therefore find older compilation data in the same
compatibility domain. Compiler, target, profile, flags, SDK, build-script,
configuration and environment identities belong in that domain whenever
applicable; the host must register them explicitly. The planner does not infer
these dependencies or prove that this list is complete. Lock files and manifests
must likewise be assigned according to their actual compatibility role.

`executable-candidate` artifacts permit exact restoration only. Every plan entry
still requires its registered build producer; this interface never authorizes
direct execution of a restored historical judge. Current executable verification
also verifies the cache planner/report executable identity. The selected
judgments/tests remain obligations of the consuming pipeline; this identity
check does not prove that a restored project executable was rebuilt.

Literal paths reject traversal, globs and symlink components. Selected outputs
cannot overlap one another, contain Git metadata or harness execution state, or
contain one of their declared file inputs. This is a finite path/registration
check, not a claim that arbitrary contents of a cache have been proved safe or
that all rebuild inputs are known. Restoring and saving must additionally hold
the adopted lifecycle protection until joined producers and saving are complete.

The returned `chrono-cache-plan/v2` includes selected entries and input
observations. Its status is `prepared-unrestored`, execution is `not-started`, and
`input_completeness_proven` is false. These are plan facts, not passing judgments,
cache hit states, successful builds or parity evidence. Cache backend results,
failure chains and original reports must be supplied by their actual producers.

## Restore recovery

V2 can explicitly enable `recover_failed_restores: true` for isolated primary
checkouts. Selected external artifacts remain unsupported because their separate
ownership is not established. This choice is retained in the plan outside build
cache keys. The planner publishes the adopted method to the native provider;
older registrations and planners do not execute recovery.

The final transport report checks each planned restore step against the original
observations consumed by recovery. Later bootstrap, build, test and save steps
may be added, but an existing restore observation cannot disappear or change.
On a mismatch the report retains both snapshots in the upload directory and
returns `E_CACHE_REPORT` with the cache ID and both original paths; it does not
publish a normal transport report or query the backend. This checks consistency
between retained observations, not the truth of the native action's claim.

The provider invokes `chrono-cache recover` after native restores and before the
original bootstrap/build, using the existing plan, registration and original step
observations. The owner revalidates the selected artifact/consumer contract with
the original declared observations, without executing input probes again. Changed
registration, host, selected paths or plan bindings fail before removal. Only
caches whose native restore **outcome** is `failure` have their registered host
artifacts removed. Success or skipped outcomes preserve outputs; missing,
cancelled or unknown outcomes fail. Nested symlinks are removed without following
their targets, while symlinked artifact roots or ancestors are refused.

An intent precedes removal. Results retain the original restore observations,
each attempted path and any failure; partial cleanup or publication failure is
nonzero and stops the original build. The final transport report retains the
recovery result and its original step bytes alongside the unchanged restore and
business outcomes. Missing or damaged recovery evidence makes reporting fail.
Repeating an already completed attempt returns its original result without
discarding outputs that a later producer rebuilt. An unfinished intent is retained
and refused; a previous failure is not automatically retried.

The host adopts this method in its existing jobs and classifies its step time as
`cache-recovery`. No job, build recipe or business-test retry is added. The failure-only policy covers
reported native restore failures and partial extraction. The optional producer
content verification below also covers reported valid hits for selected caches.
Backend archive replacement, linked-worktree leases and full native
failure-scenario acceptance remain obligations.

The host additionally adopts `recover_unconfirmed_restores: true`. This optional
v2 policy requires `recover_failed_restores` and its primary-checkout boundary;
omission preserves the earlier failure-only behavior. The same recovery entry
also discards registered outputs after a successful restore step whose matched
key is absent, empty or outside the declared exact/compatible domain. A step can
succeed after the native cache implementation suppresses download or extraction
errors. Missing keys remain `miss-or-unavailable`; cleanup does not diagnose which
case occurred or manufacture a restore failure. Recovery and transport use the
same key classification. Exact/compatible hits and skipped restores preserve
outputs; missing/cancelled outcomes and malformed outputs still fail before any
removal. The policy enters the retained recovery plan, not compilation cache keys.

Original outcomes, reasons and per-artifact results remain available to the
transport reporter. Completed reentry preserves subsequent builds. An ordinary
cold miss with all selected artifacts already absent adds no recovery warning;
removed leftovers and explicit failures remain visible. The original build and
selected tests still run, and their failures remain failures. This extension alone does
not verify the contents of a restore that reports a valid matched key, preserve
external ownership, or establish a native corruption experiment.

## Opted-in producer content verification

This host opts only `check.cache` and `check.cache-tests`, consumed by the existing
`check.unit_cache` job, into `verify_contents: true`. This v2 option requires the
existing primary-checkout failed/unconfirmed recovery policies; external artifacts
remain unsupported. Other consumers retain their previous contract. The method
`producer-manifest/v1` enters the compatibility identity, so earlier unsealed
archives cannot enter this domain through a broad prefix.

Those two compatibility domains additionally select their actual producer action
from `projects.json` (`build.cache` or `test.cache-tests`, including tool and argv)
and the Cargo tool record in the adopted direct Git policy. Existing inputs bind
check environment, Cargo/compiler observations, manifests, locks, bootstrap and
platform declarations. Unrelated project metadata remains outside these selected
values. These declarations do not prove complete compiler/SDK input closure.

The provider's existing save phase invokes `chrono-cache seal` after the original
work has ended and before saving. It revalidates the plan/registration and current
planner identity, requires the original producer step's successful **outcome**,
and inventories the explicitly selected artifact directories. Each artifact gets
`.chrono-cache-content-v1.json`, binding cache ID, producer operation, actual source
key, artifact declaration and its contents. The inventory records regular-file
SHA-256/length/execute bits, directories (including empty ones), and literal
symlink targets without following them; special files and non-UTF-8 names fail.
Only this method's own marker is excluded. The marker contains output identities,
not test results, judge responses or a cached passing status. Original native
steps, manifests and sealing observations remain in host execution evidence.
Saving an opted-in entry requires its successful seal output in addition to the
existing producer/save conditions. No build recipe, job or test retry is added.

Before any original build consumes a claimed exact/compatible hit, `recover`
compares its producer manifest and a fresh content inventory. Compatible restores
bind the manifest to the **matched** source key, while the candidate retains its
new source identity. Changed/missing contents, a missing/malformed manifest or an
incompatible producer binding are recorded as unusable. Original manifest bytes,
observed content identities/errors and native restore observations are retained
before the existing recovery contract discards only selected registered outputs.
The original build and check still run; replaying completed recovery preserves
subsequently rebuilt outputs. Reporting retains the original key classification
and exposes `unusable-content` separately from an accepted hit, copying referenced
verification evidence into the existing upload directory.

Hashing cost is additional: sealing and restore verification record wall-clock
seconds per selected cache, with the measured window in each observation; the
native step is classified `cache-verification`. Total transfer/storage/runner
cost remains unknown; bounded native acceptance is recorded above. Local Rust regressions include an
actual Cargo invocation read from the unchanged registered producer action on a
small fixture: compatible source changes produce the candidate binary and retain
incremental data, while changed producer settings partition the compatibility
domain. This fixture is compiler evidence, not a native archive acceptance run.
Content manifests are consistency evidence from trusted but fallible producers,
not archive authentication or proof of complete build inputs. The native results
above cover cold/save, exact, compatible changed-source and incompatible-input
scenarios for these two entries. Unusable-content recovery remains unverified.

## Actual provider consumers

The optional `persistent_cache` field on check/release providers adopts
`chrono-github-cache/v1`. It contains the registered planner `program`, registry
`config`, pinned `restore_action` and `save_action`, and an explicit `jobs` map.
Each existing job names `need`, `output_use`, its literal `prepare` argv,
`consumer` ID and exact `caches` set. The two purpose fields declare why that
existing job needs the outputs; they are not measured usage evidence. Unknown
jobs and missing, empty, extra or mismatched cache references fail before output
writes. The host cache registry explicitly sets `require_primary_checkout: true`.
The planner rejects a linked worktree before restoration; this filesystem guard
is not a general proof of Git ownership or lifecycle exclusion.

This host's detector builds the shared startup profiles and selects their four
registered compilation caches. Its 26 check units and aggregate install the
source-bound startup artifact. Unit cache registrations select only the actual
Cargo producers remaining in each unit's FILEMAP plan; the aggregate and units
without compilation operations have no compilation-cache subscriptions. Existing
native Rust release builds and verification units select their own project
output. Python release verification and the package collector have no registered
compilation cache. Distinct grouped test producers have distinct cache entries.
The cache seed runs `host.bootstrap-cache`, which builds the candidate planner
using the existing bootstrap owner and `build.cache`, then installs that binary.
Its bootstrap recipe explicitly selects `report_path` as
`.chrono-harness/state/cache-bootstrap.json`; the later core bootstrap retains
the default `.chrono-harness/state/bootstrap.json`. The host bootstrap accepts
an optional literal report file under `.chrono-harness/state/`, rejects escaped
or symlinked locations, and propagates report publication errors. Each recipe
owns its selected report slot; repeated use of the same slot replaces its latest
result. The existing artifact upload retains both independent records.

Cache transport reports retain the reporting process's executable path, observed
file digest and version. Consumers can match that digest to the installed cache
binary in the separate bootstrap record and its clean source identity. A digest
observation does not prove complete compiler inputs or loaded-memory identity;
missing or dirty-source bootstrap evidence cannot establish a clean source binding.

The detector cache binds its own bootstrap recipe and has no business-check
policy input. Check compilation caches explicitly select `/policy/environment`
and `/policy/tools/cargo` from the host check configuration. Unit membership and
report locations do not change those keys; environment or Cargo-tool changes do.
Dedicated Rust host-adoption regressions exercise these distinctions, including
an absent business-check policy for the detector and a changed detector build
parameter. They cover these declared dependencies, not complete build-input
closure or a general proof that all registrations are necessary.

The provider obtains output keys from `chrono-cache plan --github-output`, retains
the plan and original probe outputs, and restores before the original work.
By default it saves all selected caches after successful work. Optional
`save_caches` selects the caches this job may save; an empty list is restore-only.
Unknown or duplicate entries are rejected. Across one provider configuration,
each cache ID has at most one saving job. Generation and verification reject
conflicting writers with `E_CACHE_SAVE_OWNERSHIP`, identifying the cache and both
jobs. Other consumers explicitly select restore-only for that cache; a pipeline
may intentionally have no writer. Separate cache identities keep independently
registered production domains separate, including different release platforms.
This host assigns the four shared startup caches to the existing detector and
each remaining cache to its registered production unit. Consumers still restore,
rebuild and check without extra jobs or prerequisite waits. If the chosen writer
is skipped or does not complete its producer, that run supplies no new archive.
An explicit `save_after_bootstrap`
subset may instead use the separate bootstrap step's success after the original
work has ended, preserving reusable startup compilation after later test failure.
Unknown, duplicate or non-saving subset entries fail projection; cancellation prevents saves.
This subset is a host declaration, not proof that each later build succeeded.
Restored compilation candidates still require the original build entry.
The planner and provider sort the consumer's exact cache set lexically and use
`cache_0_key`, `cache_0_paths`, `cache_0_restore` (then `cache_1_*`, etc.) as
job-local output names. These compact references do not rename cache identities
or keys. Planner and provider must implement the same output contract; registry
validation rejects mismatched selected sets before projection.
It does not skip the canonical check or replace build/test results with cache
status. A final `chrono-cache report` step records the native action outputs,
requested and matched keys, original work/bootstrap outcomes and immutable source
reports. `evidence_directory` may place originals within the existing release
upload directory; ordinary checks retain them under the uploaded host state.
The detector's optional `evidence_directory` retains originals through the same
pinned upload action in its existing job, including after failure. The directory
must be a literal host state path, with hidden-file upload explicitly enabled;
missing files fail that upload instead of claiming preservation. This host chooses
`.chrono-harness/state/` to include bootstrap, plans and probe originals. Without
this adoption, detector files remain temporary runner state.
The transport report runs in the existing job. Without a registered backend it makes no API request. Exact/compatible restoration is reported
only from a successful action with a matching key. Empty outputs are
`miss-or-unavailable`. Native save success is `unconfirmed` with
`W_CACHE_SAVE_UNCONFIRMED`: the pinned action can catch backend exceptions and
exit successfully. This report cannot claim a saved archive, compiler reuse or
verified executable identity from action outcomes. A cache excluded from this
job's save selection reports `not-requested` when no save was observed; an actual
save outside that selection remains visible with `W_CACHE_UNREGISTERED_SAVE`.
Backend and action logs remain necessary to attribute a particular save or diagnose archive contents. Failed report production or evidence retention fails its workflow step and job;
the original business result is retained independently. The report step has no
`continue-on-error` exemption. A backend query failure that is successfully
recorded remains a nonfatal availability warning.

Cache backend failures remain nonfatal to the following build. Native acceptance of backend observations, explicit corrupt-cache recovery and preserving arbitrary project compilation
after a later test failure remain unfinished. Saving does not include `.chrono-harness/state`, release
handoffs, selected test results, installed `bin`, Git metadata or lifecycle locks.

`chrono-cache/v2` optionally registers `backend` with `kind: github`, a concrete
`need` and `output_use`, a bounded `command`, explicit `inherit` and
`credential_environment` lists, and `repository_environment` / `ref_environment`
names. Exactly one complete command argument is `{endpoint}`. The host uses
`gh api --paginate --slurp {endpoint}` with a 30-second, 4-MiB bound. The command
receives `repos/{repository}/actions/caches?ref={encoded-ref}&per_page=100` and
returns an array of GitHub response pages. The policy is retained in the plan
but is outside build cache identity; planning never queries the backend.

The existing report step queries once per batch only when a native save succeeded
and is still unconfirmed. Otherwise it records `not-required` without launching a
query. The generated report step supplies `GH_TOKEN`; cache workflows permit
`actions: read`. The subprocess inherits only declared variables. Retained process
evidence includes the executable digest, command, raw stdout/stderr, original
exit/failure and environment digest; declared credential values are omitted from
the retained environment. Commands must not print credentials into raw streams.

An exact key and ref with one observed backend entry changes save status to
`backend-entry-observed`, with availability `confirmed: true` and
`creator: unestablished`. This confirms an entry observed at query time, not which
action created it, archive integrity, compiler reuse or current executable identity.
Multiple archive identities are `ambiguous`; missing matches are `not-observed`,
not proof of absence, because pagination is not an atomic inventory. Query failure,
timeout, truncated/malformed responses and conflicting identities retain the original
failure, emit `W_CACHE_BACKEND_UNAVAILABLE` and leave saves unconfirmed. Failed or
skipped native saves and the original business verdict are never replaced by backend
presence. Query cost remains in the registered `cache-report` step category.

The host registry binds actual compiler/Cargo files and version, OS/architecture,
registered linker/SDK observations, declared flag absence, manifests/locks and
build configuration. Source changes use FILEMAP's explicit compile closure.
Check, release build, release verification and platform declarations use separate
compatibility domains. This finite registration does not establish complete
compiler backend, SDK library, build-script or ambient configuration closure.
Current native adoption and missing evidence remain visible in the coverage map.

With explicit shared startup adoption, the detector builds the host core tools
once and publishes them to registered consumers (see [startup transfer](ci-units.md)).
Only actual remaining build/test producers retain target cache subscriptions;
aggregate and script-only consumers do not restore startup compilation targets.
The detector is the sole saver for its separate `detect.*` startup caches; actual
unit producers save their `check.*` caches. Detector compatibility excludes check
policy inputs that its bootstrap does not consume. Imported startup verification is not a compilation
producer and does not grant `save_after_bootstrap`. Original producer reports and
explicit imported provenance remain available in each consumer's evidence; startup
binaries stay outside evidence uploads. Transfer costs are measured separately,
and a source-bound artifact is not proof of complete build input provenance.

发行验证的 `chrono-cache` 资产在 Cargo test destination 被消费；bootstrap 安装的运输 executable 保持归 bootstrap。发行配方不再将资产 staging 到同一个 bin 路径。Python release integration 回归覆盖真实 CLI 的 plan／recover／staging／report 与 inode、摘要保留；这个本地证据不证明原生缓存后端或外围 workflow 成功。详见[发布所有者](release-ci.md)。

## Retired remote acceptance controls

**Product and host policy change:** the optional native manifest-removal fixture,
its CLI/configuration and provider registration, repository Variables activation,
and live PR-body reader have been removed. The provider no longer adds
`pull-requests: read` for activation or reads PR metadata. No replacement
activation transport, variable management, remote cache deletion or additional
permission is required. Retired configuration fields fail normal strict decoding;
they are not silently accepted as successful activation.

The existing `check.unit_cache` consumers still plan and restore their two
registered compilation archives, validate `producer-manifest/v1` contents, discard
unusable host outputs through ordinary recovery, execute the original candidate
build/tests, seal successful producer outputs and save through the existing
native cache actions. Required producer/seal outcomes still gate saving. Ordinary
backend observation retains its existing necessary scope and original failures.
Other registered build/release cache consumers retain their existing behavior.

Historical native evidence remains attached to its exact source/run/attempt.
Variables GET/list HTTP 403 was a real historical failure. Later bounded C1
observations recorded unusable payload save/restore/rejection, rebuild and
replacement caches for both entries, but a healthy exact hit in the original
compatibility domain was not achieved: the selected rustup identity changed;
the latest collection returned HTTP 502. Those observations are partial, and
this deletion neither reruns nor completes them. Original records, streams,
commits and retained evidence remain under their existing custodian. The former
fixture and its finite caller sequence remain reproducible from retained source
`f52442addfa3079f644cc7be59809a4ca6e756d5`; they are not current instructions.

Costs: ordinary execution needs no activation process/API request. Deleting the
optional layer removes its reader, fixture, response/intent evidence IO and
exclusive tests; native time, transfer, storage and total lifecycle savings are
unmeasured. Existing same-language content/recovery, invalid-input and provider
regressions and the registered DELTA-selected canonical check remain required.
Native acceptance, concurrency, other-consumer adoption, full input closure,
parity and actual delivery remain separate uncompleted obligations.
