# Registered CI caches

`chrono-cache` is an independent Rust product with its own `cache-tests` project,
CI unit and central release asset. It does not compile with the CI generator.

`chrono-cache plan --host-root H --config .chrono-harness/cache.json
--consumer ID` prepares keys for one explicitly registered consumer. This is an
internal provider entry; the daily check remains `chrono-harness check`.

The planner prepares identities; the GitHub provider projects pinned native
restore/save actions around the original bootstrap and check/release command.
The host explicitly adopts those actions in its check and release registrations.
A successful plan or generated workflow does not establish native restoration,
saving, compiler reuse or acceptance. SPEC §4.2 still requires backend evidence,
cold/warm/changed-source/failure/concurrent acceptance and current executable
verification. The transport currently requires a primary Git checkout; linked
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
and the selected judgments/tests remain obligations of the consuming pipeline.

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

This host's detector selects only the `ci` project target from its dedicated
bootstrap recipe. Its 25 check units and aggregate rebuild core bootstrap
projects on fresh runners. Their registrations select the existing bootstrap
outputs plus only the Cargo outputs claimed by each unit's FILEMAP plan. Existing
native Rust release builds and verification units select their own project
output. Python release verification and the package collector have no registered
compilation cache. Distinct grouped test producers have distinct cache entries.
The cache seed runs `host.bootstrap-cache`, which builds the candidate planner
using the existing bootstrap owner and `build.cache`, then installs that binary.
It is not a second Cargo build implementation.

The provider obtains output keys from `chrono-cache plan --github-output`, retains
the plan and original probe outputs, and restores before the original work.
By default it saves all selected caches after successful work. Optional
`save_caches` selects the caches this job may save; an empty list is restore-only.
Unknown or duplicate entries are rejected. This host assigns saving the four
shared startup caches to the existing aggregate job; unit jobs still restore and
rebuild them, then save their remaining selected outputs. This avoids competing
startup saves without adding a job or changing the original checks. If the chosen
writer does not complete its producer, that run supplies no new archive.
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
No extra job or API request is introduced by the transport report. Exact/compatible restoration is reported
only from a successful action with a matching key. Empty outputs are
`miss-or-unavailable`. Native save success is `unconfirmed` with
`W_CACHE_SAVE_UNCONFIRMED`: the pinned action can catch backend exceptions and
exit successfully. This report cannot claim a saved archive, compiler reuse or
verified executable identity from action outcomes. A cache excluded from this
job's save selection reports `not-requested` when no save was observed; an actual
save outside that selection remains visible with `W_CACHE_UNREGISTERED_SAVE`.
Original backend logs remain
necessary for those investigations. Failed report production is visible as its
own action outcome and does not replace the original check result.

Cache backend failures remain nonfatal to the following build. Confirmed backend
saves, explicit corrupt-cache recovery and preserving arbitrary project compilation
after a later test failure remain unfinished. Saving does not include `.chrono-harness/state`, release
handoffs, selected test results, installed `bin`, Git metadata or lifecycle locks.

The host registry binds actual compiler/Cargo files and version, OS/architecture,
registered linker/SDK observations, declared flag absence, manifests/locks and
build configuration. Source changes use FILEMAP's explicit compile closure.
Check, release build, release verification and platform declarations use separate
compatibility domains. This finite registration does not establish complete
compiler backend, SDK library, build-script or ambient configuration closure.
Current native adoption and missing evidence remain visible in the coverage map.
