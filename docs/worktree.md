# Registered worktree creation

`chrono-worktree` is an independent Rust producer with its own `worktree-tests`
project. It creates a new feature or integration worktree from a freshly fetched,
explicit remote target. It does not select projects from directory names, run a
language restore, reset existing branches, open PRs or claim a governance verdict.

The host registers `.chrono-harness/worktree.json` in FILEMAP. Its schema is
`chrono-worktree-config/v1`; the adopted file supplies the complete shape:

```json
{
  "schema": "chrono-worktree-config/v1",
  "host_config": ".chrono-harness/config.json",
  "remote": "origin",
  "git": {"program": "git", "expected_version": null, "sha256": null},
  "environment": {
    "inherit": ["PATH", "HOME", "SSH_AUTH_SOCK"],
    "values": {"GIT_TERMINAL_PROMPT": "0"}
  },
  "timeout_seconds": 120,
  "output_limit_bytes": 1048576,
  "report_directory": ".chrono-harness/state/worktrees/"
}
```

The five host registries remain the authority for target branch, feature and
integration prefixes, FILEMAP membership and artifact ownership. The producer
reuses their existing strict loader. A successful load or creation is not full
reference admission, complete input certification, freshness certification or
activation. Workflow's normal judge still evaluates delivery context and DELTA.

Install `chrono-worktree` from the pinned public beta.8 release through the host’s
registered installer (the product repository can also bootstrap its candidate):

```sh
.chrono-harness/bin/chrono-worktree start \
  --host-root . --config .chrono-harness/worktree.json \
  --kind integration --name task --path ../new-task-checkout
```

All five arguments are required; unknown, repeated or empty arguments fail.
The destination is explicit, its parent must exist, and it must not exist or be
nested within an existing registered worktree. Paths are relative to the supplied
host root, not the caller's working directory. Spaces and UTF-8 names are literal
arguments. This version supports the tested Unix Git interface.

The producer resolves the configured Git executable once for observation and
binds its actual bytes for every Git operation. Optional expected digest and
version values reject mismatches; a null value makes no expected-identity claim.
Only the declared environment is inherited, and explicit values override it.
For untracked inventories, the producer explicitly disables Git global literal,
glob, noglob and case-folding pathspec modes in that child environment. The
exclusion arguments themselves are literal and case-sensitive; each process
records its actual environment, including these scoped overrides.
Git repository-redirection variables are rejected. Git configuration, credential
helpers and hooks are not certified as a complete input closure. No global
configuration is edited, and new branches explicitly disable automatic tracking.

The local policy must match the source commit. The producer reads the declared
workflow target, fetches that exact remote branch into a unique temporary ref,
records its commit and tree, then deletes only that temporary ref using its
observed OID. It reads the fetched registries and requires the policy bytes and
target branch to remain compatible before creating anything. Remote movement
after the fetch is not claimed absent; the receipt identifies the observed tip.
An advanced source checkout or unrelated dirty source files are not reset.

Creation uses a new branch and a worktree lock tied to this invocation. Actual
Git inventory, HEAD, branch, root and checkout cleanliness are checked before
unlocking it. The producer reuses registration's artifact classifier: declared
untracked artifact paths are allowed and preserved. Tracked changes are always
checked separately, including beneath artifact roots. Separate ignored and
nonignored file queries exclude only explicitly registered artifact directories
before returning paths; large build outputs therefore do not consume the path
observation bound. Remaining paths use the original registration classifier.
Unknown neighbors, literal or case lookalikes, and a file or symlink in place of a
registered directory remain errors. The output bound is unchanged. Existing paths and branches
are preserved. A failing checkout hook,
timeout, changed identity or dirty new checkout fails creation and preserves the
new branch/worktree for recovery. There is no automatic deletion or success
inference from a partially completed Git command. A failed fetch may retain its
named temporary ref; inspect the report before recovery.

Each observed attempt writes a unique `chrono-worktree-report/v1` JSON file under
the configured state directory and returns the same JSON on stdout. It contains
source identity, observed base/tree, branch/destination, start time, configuration
digest, inherited/effective environment, Git version observation and actual
subprocess argv, bytes, hashes, exits and bound failures. Failures before tool
observation/configuration acceptance return a nonzero diagnostic without inventing
a process receipt. After observation, `status` is `created` or `failed` and the
CLI exits 0 or 2 respectively. `governance` remains `not-evaluated` and `parity`
remains `unestablished`. The partial `context` is an input for later work, not a
complete check context or integration certificate.

The dedicated tests use real bare remotes, advanced remote commits, arbitrary
host layouts, spaced/Unicode paths, existing dirty and locked worktrees, local and
remote policy drift, missing remotes, tool mismatch and real checkout hooks. The
adopted policy has an actual subprocess consumer. The host registers both the
binary and dedicated test plan. Public beta.8 includes this tool for macOS arm64
and Linux x86_64; its native release recipe passed all 49 dedicated worktree tests
on each platform. The four [example hosts](examples.md) explicitly adopt the tool
and policy; no host-language or directory inference supplies their registrations.

Registered maintenance below adds recovery of reconciled checkouts and explicit
cleanup. AI semantic reconciliation, interrupted-rebind continuation, PR provider
operations, merge and actual landing orchestration remain unfinished. Subsequent governance
checks continue to use the same registered `chrono-harness check` command locally
and in CI. This increment changes product and adopted policy together; validation
scope expands by the new project/test pair and bootstrap/release registrations.
Declared costs remain unknown, and no acknowledgement or human approval is added.


## Explicit reconstruction

Use the same host policy and an explicit run-local plan under the host's registered
untracked `.chrono-harness/state/` artifacts:

```sh
.chrono-harness/bin/chrono-worktree reconstruct \
  --host-root . --config .chrono-harness/worktree.json \
  --kind integration --name task-r2 --path ../task-r2 \
  --plan .chrono-harness/state/reconstruction.json
```

The strict `chrono-worktree-reconstruction/v1` plan names fixed full commit IDs
and the complete original net DELTA. Renames are deletion plus addition. Example
shape (replace endpoint placeholders with actual full commit IDs):

```json
{
  "schema": "chrono-worktree-reconstruction/v1",
  "base": "OLD_FULL_BASE_OID",
  "candidate": "OLD_FULL_CANDIDATE_OID",
  "changes": [
    {"path": "arbitrary/source.ext", "action": "carry"},
    {"path": "obsolete-change.data", "action": "retire", "reason": "Superseded requirement"}
  ]
}
```

Every changed path must appear exactly once, with no unrelated entries. Retirement
requires a nonempty reason. The AI chooses continued necessity; the tool checks
coverage and applies the chosen changes without inferring language or test scope.
The old base must be a Git ancestor of the source candidate; the latter must equal
source HEAD. Source work must be committed and clean except declared artifacts.
The plan itself must have a registered artifact owner. Existing policy-adoption
requirements still apply before creating the new worktree.

The producer extracts the original binary/full-index patch with literal paths,
no rename heuristic, external diff or text conversion. It reuses `start` to fetch
the target and create a new branch; it never merges the old branch. Git applies
the selected patch with `--3way --index`. Raw diff and apply process results are
retained, including the patch input hash. Output bounds fail before creation if
the patch cannot be retained. Empty or retired-only DELTAs do not run apply.

A successful result has `status: reconstructed`, exit 0, the new target parent,
actual staged path set and `reconstruction.index_tree`; the partial context's
`candidate` is null because the staged changes are not a new commit. No source
worktree or branch is deleted. Failure is exit 2; conflicts, original process
failure and the invocation lock are preserved for AI reconciliation. Source
mutation or a changed destination identity also fails without deleting work.

After success, the AI reviews the staged result, reconciles semantic changes and
registrations, commits it, and runs the same registered canonical check with new
evidence. A reconstruction result is neither a freshness verdict nor a check or
integration certificate. The workflow judge continues to own freshness policy;
Git/config/hook closure, automatic conflict resolution and provider orchestration
remain outside this producer's current guarantee. The separate maintenance
operations below validate recovery and explicit saved-work cleanup.

## Registered recovery and cleanup

The same committed `.chrono-harness/worktree.json` supplies the Git binary,
process limits and report location for maintenance. These operations consume
explicit plans under a registered `.chrono-harness/state/` artifact directory;
no branch age, directory name or host language selects work. The commands are:

```sh
.chrono-harness/bin/chrono-worktree recover \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/recover.json
.chrono-harness/bin/chrono-worktree cleanup \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/cleanup.json
```

Recovery completes a caller's reconciliation of an existing failed `start`,
`reconstruct` or `cleanup` checkout. Its strict `chrono-worktree-maintenance/v1` plan is:

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "recover",
  "receipt": {
    "path": ".chrono-harness/state/worktrees/FAILED_REPORT.json",
    "sha256": "SHA256_OF_ORIGINAL_REPORT_BYTES"
  },
  "head": "EXPECTED_FULL_HEAD_OID",
  "index_tree": "EXPECTED_RESOLVED_INDEX_TREE_OID"
}
```

The original report must identify this source, the current committed policy, a
failed creation/reconstruction/cleanup, the exact destination/branch and its owned lock.
The caller explicitly supplies the reconciled HEAD and index tree. An owned lock
left by failed cleanup can be recovered before retrying disposal. Recovery
requires the original fetched base to remain an ancestor, a resolved index,
no unstaged changes or unregistered files, and the same physical Git repository.
It rechecks identity before releasing that lock. Committed reconciliation is
allowed; staged work remains staged. The new report retains the original bytes,
digest and failure, and reports `recovered`; it does not rerun hooks, turn the
original failure into success, or issue a check/integration certificate. The
caller commits any remaining staged work and runs the canonical check.

Cleanup names one exact worktree and a separate local branch that preserves its
committed work. All fields below are required; substitute actual paths and OIDs:

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup",
  "path": "../finished-checkout",
  "branch": "integration/finished-task",
  "head": "EXPECTED_FULL_HEAD_OID",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "EXPECTED_FULL_RETAINED_COMMIT_OID",
  "retention": "same-tree",
  "discard_artifacts": [".chrono-harness/bin/", ".chrono-harness/state/"],
  "remove_branch": true,
  "allow_absent_worktree": false
}
```

`retention: ancestor` proves the expected HEAD is an ancestor of the fixed
retained commit. `same-tree` supports squash delivery by requiring exact tree
equality. Both retain a named local branch at the declared commit; neither
certifies a remote merge, tests, or semantic equivalence of different trees.
The work branch must match an explicitly registered feature/integration prefix.
Main/source, locked, nested, foreign, mismatched, dirty or unknown worktrees fail.
Every disposable directory must be an untracked artifact declared in both the
source and target HEAD registries. Unlisted artifacts also prevent deletion;
registration alone is not permission to discard them. Tracked changes under an
artifact path remain errors. Cleanup uses the existing bounded literal inventory
queries, so registered build output does not require listing every cache file.

The producer checks the exact checkout, HEAD, branch and preservation reference,
acquires an invocation lock, rechecks, releases only that lock, rechecks saved work and contents, then removes
the worktree. It verifies path/inventory absence before any optional branch deletion.
Branch removal uses the expected OID and checks that no registered worktree still
uses it. Failures retain actual process results and explicit partial effects;
there is no rollback claim. `worktree_removal` and `branch_removal` distinguish
`not-attempted`, `attempted-unverified`, `verified-absent` and `already-absent`;
a branch kept by the plan is `not-requested`. The `*_removed` booleans are true
only after this invocation's removal and absence checks succeed. False does not
prove that a failed Git operation had no effect; inspect the explicit state and
original process evidence. `allow_absent_worktree: true` explicitly permits a
retry after partial removal, including a remaining branch. Already absent work
is observed as absent and is not reported as a new removal. The retained branch
must still match. Configuration, hooks and concurrent filesystem writers are not
an atomic transaction; these checks do not claim exclusion of all concurrent
mutations.

Maintenance operations return the ordinary stored/stdout report with actual process
bytes, `governance: not-evaluated` and `parity: unestablished`; success exits 0,
failure exits 2. They preserve source work. Interrupted owned checkouts with a
retained intent use the separate contract below. Damaged/missing Git metadata and lost old receipts use the separate explicit
metadata-rebind contract below; no automatic metadata deletion or
semantic reconciliation is provided. PR/merge/landing producers remain pending.
These commands are included in the public beta.8 release and installed through
the same pinned host distribution entry.


### Temporary fetch-ref cleanup

A failed fetch or temporary-ref deletion can retain a ref without creating a
worktree. With the original failed report available, use the same maintenance
entry configuration and a `cleanup-fetch` plan:

```sh
.chrono-harness/bin/chrono-worktree cleanup-fetch \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/cleanup-fetch.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup-fetch",
  "receipt": {
    "path": ".chrono-harness/state/worktrees/FAILED_REPORT.json",
    "sha256": "FULL_ORIGINAL_REPORT_SHA256"
  },
  "head": "FULL_EXPECTED_FETCH_COMMIT",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "FULL_EXPECTED_RETAINED_COMMIT",
  "allow_absent_ref": false
}
```

The original report must bind this source/configuration and a failed `start` or
`reconstruct` invocation whose temporary ref was not reported removed. Its
`fetch_ref` must equal `refs/chrono-harness/fetch/` plus its invocation token.
If it recorded a base, that base must match the plan's `head`. The exact expected
commit must be an ancestor of the explicitly named local retention branch at
`retained_commit`. A symbolic temporary ref, changed OID or moved retention
branch fails. No worktree, other ref or remote branch is removed.

The producer rechecks retention and original report bytes, deletes with the
expected OID, and observes ref absence. Original failure bytes remain intact.
`fetch_ref_removal` uses `not-attempted`, `attempted-unverified`, `verified-absent`
and `already-absent`; `fetch_ref_removed` is true only after this invocation's
removal and absence check. A failed command can have changed the ref. Set
`allow_absent_ref: true` only for an explicit absence-tolerant retry; retention
must still hold. This does not reconstruct a missing/interrupted original report
or make concurrent ref/configuration writers atomic. It exits and reports through
the same maintenance contract above and is included in public beta.8.

## Interrupted checkout recovery

Public beta.8 also provides `recover-interrupted`, installed through the host’s
pinned distribution lock. Before `start`/`reconstruct`
creates a checkout, and before `cleanup` acquires its checkout lock, the producer
publishes a `chrono-worktree-recovery-intent/v1` file next to the result path,
with the suffix `.intent.json`. It writes and syncs a temporary file, then
publishes without replacing an existing path. Publication failure prevents that
subsequent checkout/lock operation. Earlier preparation, including fetching,
can already have occurred.

The intent retains source/configuration/registry identity, selected base,
destination, branch, invocation lock reason and final-result path. It contains
no outcome or fabricated process exit. A completed result references its path
and digest. If the process stops before final publication, the caller can select
the corresponding explicit intent using the observed invocation/lock identity;
the tool does not scan for or guess an owner. Keep this file and any incomplete
result until recovery is complete. Its presence does not prove that the original
invocation stopped: the AI caller must establish that before starting recovery.

Use the same registered policy and an explicit state plan:

```sh
.chrono-harness/bin/chrono-worktree recover-interrupted \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/interrupted-recovery.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "recover-interrupted",
  "intent": {
    "path": ".chrono-harness/state/worktrees/start-INVOCATION.json.intent.json",
    "sha256": "ACTUAL_INTENT_SHA256"
  },
  "result": {"presence": "absent"},
  "head": "CURRENT_RECONCILED_HEAD_OID",
  "index_tree": "CURRENT_RECONCILED_INDEX_TREE_OID"
}
```

When an empty or truncated result file exists, use
`"result": {"presence": "present", "sha256": "ACTUAL_RESULT_SHA256"}`.
Missing and empty are different observations. The `absent` variant accepts no
payload fields; `present` requires its string `sha256`. Unknown fields, missing or
unknown presence tags and invalid payload types fail plan parsing before Git
observation or report creation. The reader rejects changed
presence/bytes, nonregular or linked inputs, mismatched intent/configuration,
and an original JSON result that declares a terminal worktree outcome. Use the
ordinary maintenance contract for such a retained result; do not remove it to
make this path eligible.

Recovery reuses the original `recover` checks for actual checkout, common
repository, branch, owned lock, base ancestry, reconciled HEAD/index, unstaged
changes and declared artifacts. It rechecks the selected evidence before and
after releasing the lock. The new report retains the original intent and any
result bytes, sets `original_outcome: unknown`, and reports only the current
recovery outcome. It neither completes the interrupted command nor restores a
lost original index. Staged work still needs a candidate commit and the canonical
check. If cleanup already removed the checkout, use its explicit absence-tolerant
cleanup plan instead of claiming a recovered checkout.

Real Git tests terminate the CLI after checkout creation, patch application and
cleanup locking. They also cover absent/empty/truncated results, terminal-result
refusal, identity drift and occupied intent paths. These are bounded process
interruption checks, without a power-loss or concurrent-writer transaction
guarantee. Interrupted fetches use the separate intent below. Missing intent,
metadata rebind and PR/merge orchestration remain outside
this contract. No governance or local/CI parity guarantee is added.

## Interrupted fetch cleanup

Public beta.8 also provides `cleanup-fetch-interrupted`. Before `start` or `reconstruct` runs Git fetch, it validates the source
policy/report artifact registrations and publishes an immutable
`chrono-worktree-fetch-intent/v1` at `REPORT_PATH.fetch-intent.json`. It uses the
same synced, no-clobber publication as checkout recovery. Publication failure
prevents fetch. The completed result references this intent separately from any
later checkout intent.

The fetch intent binds the source commit and registry digest, source/configuration
identity, remote, target ref, invocation token, exact temporary fetch ref and final
result path. It has no fetched base or outcome: neither exists as an observed fact
at publication time. A stopped invocation can leave a ref or already have removed
it while its final report is still absent, empty or incomplete. The caller must
establish that the original invocation has stopped before cleanup.

```sh
.chrono-harness/bin/chrono-worktree cleanup-fetch-interrupted \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/interrupted-fetch-cleanup.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup-fetch-interrupted",
  "intent": {
    "path": ".chrono-harness/state/worktrees/start-INVOCATION.json.fetch-intent.json",
    "sha256": "ACTUAL_FETCH_INTENT_SHA256"
  },
  "result": {"presence": "present", "sha256": "ACTUAL_PARTIAL_RESULT_SHA256"},
  "head": "CURRENT_EXPECTED_FETCH_COMMIT_OID",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "EXPECTED_RETAINED_COMMIT_OID",
  "allow_absent_ref": false
}
```

The strict `result` variants are the same as checkout interruption recovery;
use `{"presence":"absent"}` only when the original result is absent. A retained
terminal result is refused and must use ordinary receipt maintenance. No intent
or owner is discovered from a directory or ref prefix.

The entry reuses ordinary `cleanup-fetch`: it requires a direct temporary ref,
the explicitly expected current OID, and a separate local branch retaining that
commit; deletion uses the expected OID and verifies absence. Only an explicit
`allow_absent_ref: true` accepts an already absent ref. It retains original intent
and result bytes, rechecks them before disposal and after final retention checks,
and reports `original_outcome: unknown`. A post-effect failure retains the actual
removal state and process results; it does not roll back or fabricate the original
fetch outcome. It neither unlocks nor removes any checkout that the original
invocation might have created later.

Real CLI tests cover process termination after fetch and after ref removal,
start/reconstruct consumers, absent/empty/truncated results, explicit retry,
identity/retention drift, symbolic refs, terminal reports, occupied publication
paths and evidence changed during deletion. Missing both usable receipt and
intent and PR/merge orchestration remain
outside this contract. These observations do not make concurrent writers atomic,
provide power-loss durability, or certify governance and local/CI parity.


## Remote branch retirement

The beta.10 public macOS binary retired the product release branch and each of
the four example adoption branches after verified landing. Original process
bytes, expected branch/target identities, leased deletion and observed absence
were checked. Dedicated worktree tests also ran in both native release jobs;
see [release evidence](distribution.md) and [example adoption](examples.md).

Public beta.10 provides `cleanup-remote`. It uses
an explicit plan and the same registered worktree configuration and process
reporting as local maintenance:

```sh
.chrono-harness/bin/chrono-worktree cleanup-remote \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/remote-cleanup.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup-remote",
  "branch": "integration/finished-task",
  "head": "EXPECTED_FULL_REMOTE_BRANCH_OID",
  "expected_url": "https://example.org/owner/project.git",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "EXPECTED_FULL_TARGET_OID",
  "retention": "same-tree",
  "allow_absent_ref": false
}
```

The source registries must declare the work branch prefix and target branch;
`retained_ref` must be that distinct target. The configured remote must resolve
to exactly one push URL matching the plan. Both remote refs are observed at that
URL, without consulting remote-tracking refs. The target must match the fixed
retained commit locally and remotely. Existing saved-work checks require either
`ancestor` or `same-tree` retention. Objects must already exist locally; this entry
does not infer or fetch missing history, a landing, PR status or tests.

The push names only that URL and full branch ref, disables following tags and
submodule pushes, and requires `--force-with-lease=REF:EXPECTED_OID`. A changed
branch is preserved by Git's lease. Local worktrees, local branches, other remote
refs and Git configuration are not disposal targets. The plan and committed
policy bytes, endpoint and saved work are checked around the operation. Symbolic,
unexpected, duplicate or malformed remote ref advertisements fail.

Success requires a successful push followed by observed branch absence and
matching retention/plan/endpoint checks. The report uses `remote_branch_removal`
with `not-attempted`, `attempted-unverified`, `verified-absent` or `already-absent`.
`remote_branch_removed` is true only for this invocation's verified removal. A
failed command may already have deleted the branch; its original bytes and exit
remain in the failed report. An explicit `allow_absent_ref: true` retry validates
retention and absence without rewriting the original result. Reports keep
`governance: not-evaluated` and `parity: unestablished`.

The lease protects the disposal ref's expected OID. Target/URL observations and
local checks are not an atomic transaction with the remote server, do not prevent
all concurrent writers or transient changes, and do not close Git configuration,
hook, transport, credential or OS inputs. If the target changes during a push,
the final observation can fail after deletion. An interrupted attempt has unknown
outcome and can use an explicit absence-tolerant plan after the caller establishes
it has stopped. No remote recovery intent, server lock, PR authorization or merge
certificate is inferred. Credentials may still be required by the configured
transport; do not put secret-bearing URLs in a retained plan/report.

Real bare-remote consumers cover unchanged local dirty work, explicit endpoint
selection despite configured push refspecs, squash retention and absence retries,
endpoint/branch/retention rejection, symbolic refs, concurrent branch updates,
original failure after deletion, and target/plan changes after deletion.

## Explicit metadata rebind

`inspect-rebind` and `rebind` are source additions, not part of public beta.10.
They recover a linked checkout's Git attachment when its metadata is missing or
unusable, including loss of all old operation receipts. The AI supplies the
existing branch, exact current commit, desired index tree and each path. The old
index and original operation outcome are never inferred. A healthy attachment
uses ordinary maintenance.

Both commands use the registered worktree policy and the maintenance plan:

```sh
chrono-worktree inspect-rebind --host-root ROOT --config .chrono-harness/worktree.json --plan .chrono-harness/state/rebind-inspection.json
chrono-worktree rebind --host-root ROOT --config .chrono-harness/worktree.json --plan .chrono-harness/state/rebind-plan.json
```

The inspection plan is explicit data; OIDs below must be replaced with actual
commit/tree identities. `index_tree` declares the index to install, which can
represent chosen staged work and may differ from HEAD.

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "inspect-rebind",
  "head": "<existing branch commit OID>",
  "binding": {
    "path": "../orphan checkout",
    "branch": "feature/recovered-work",
    "index_tree": "<chosen Git tree OID>",
    "metadata_id": "orphan-checkout",
    "backup": ".chrono-harness/state/original-metadata",
    "donor": "../fresh-donor",
    "expected": null
  }
}
```

Inspection reports `observed` and a `proposed_plan` with `operation: rebind` and
observed `expected` identities. Save that exact proposed plan in registered state
before executing it. `expected` contains nullable gitfile identity
(`sha256`, `size`, permission `mode`), nullable metadata digest and the visible
subtree digest. The report retains the scope entries with raw path bytes, regular
file content hashes/sizes/modes and literal symlink targets. Only the target root
`.git` is excluded from visible work. This filesystem observation verifies an
explicit preservation scope; it never enrolls files or derives test dependencies.
Inspection writes its state report but does not change the selected checkout.

The target must be a physical directory outside the source/common Git directory
and other checkouts. The branch must use a registered work prefix, exist at the
declared HEAD and not belong to another checkout. The original metadata member is
explicitly selected under the same common repository's `worktrees` directory;
that parent may itself be absent. A usable old gitfile pointer and metadata
backlink must agree with the selected target/member. Relative pointers are
resolved lexically from the containing directory; symlink aliases are not treated
as equivalent physical ownership. If metadata exists, at least one usable link
must identify it. Missing old receipts do not authorize another checkout's
metadata. Root `.git` directories/symlinks and special files in the preservation
scope are rejected.

Backup must be an absent path under registered `.chrono-harness/state/` with
existing physical parents. Donor must be an absent directory with an existing
parent and must not overlap protected paths. Choose a donor basename that is not
a prefix of `metadata_id`, so Git cannot reuse the old allocation. Before effects,
execution rechecks source/configuration/branch/plan and observed identities and
publishes immutable `chrono-worktree-rebind-intent/v1` containing the exact plan
hash and expected-input digest. It copies the old gitfile bytes/mode, renames
remaining metadata into `backup/metadata`, then creates a locked detached donor
with `--no-checkout`. It sets the declared branch and index, publishes the donor
pointer, runs Git repair, verifies attachment, removes the empty donor and
releases the lock. No checkout/reset writes the original working files. Final
success also rechecks the visible subtree and preserved backup.

`rebound` means the declared attachment/index are verified and the observed work
was preserved. It reports `index_origin: explicit-plan`,
`original_outcome: unknown`, `governance: not-evaluated` and
`parity: unestablished`; staged or unstaged work remains the caller's responsibility
before canonical checks. It does not certify historical-index recovery, full
input closure or a checked candidate.

An ordinary failure retains phase, original process bytes/exit, immutable intent,
backup and partial effects. Never rerun blindly over a nonempty backup/donor.
Once Git repair has attached the destination and its original ownership lock is
still present, existing receipt-bound `recover` can release it after explicit AI
reconciliation; the original failed report, backup and any remaining donor stay
untouched. Before attachment, or if the rebind result itself is missing, automated
continuation is not yet implemented. The AI must first establish that the old
process stopped and reconcile the retained evidence. Backup uses same-filesystem
rename; cross-device failure is reported without deleting old metadata. There is
no concurrent-writer isolation or crash/power-loss atomicity guarantee, and the
observed identity does not preserve timestamps, ACLs, xattrs or hardlink topology.

### Exact physical snapshot before release or deletion

The source checkout guard now reuses the runner's literal file/index comparison
([contract](git-facts.md#literal-checkout-identity)). Creation, reconstruction,
reconciled recovery and cleanup reject bytes, link representation and owner
execute changes even when Git configuration reports a clean working tree.
Cleanup preserves such unsaved work; recovery keeps its lock until the actual
files match the explicitly supplied index tree. The existing artifact exclusion
and saved-commit checks still apply. This source change is not in beta.10 and
does not claim complete Git configuration closure or concurrent deletion safety.

## Declared artifact disposal before checkout removal

Source `cleanup` now separates explicitly selected generated output directories
from the bounded Git checkout-removal process. Before deleting any selected
directory, it verifies every entry is an untracked artifact in both endpoint
registries, contains no tracked candidate path, and is absent or a physical
directory without symlink ancestors. Existing saved-work, branch, checkout,
unknown-file and nested-worktree checks still apply. Disposal runs under the
owned worktree lock; literal checkout and retention are checked again before
unlock and Git removal. The plan never discovers disposable directories.

`artifact_disposals` records each attempted path with `already-absent`,
`attempted-unverified`, or `verified-absent`. The filesystem phase is within the
caller’s host job lifecycle and has no Git subprocess deadline. A filesystem
failure may partially delete the authorized artifacts while retaining the lock;
resolve the cause and use existing receipt/intent recovery before explicit retry.
A later Git failure retains the original process bytes and the completed artifact
effects. Neither failure is rewritten as successful checkout removal.

This addresses generated build outputs consuming the Git-removal deadline and
leaving a partially deleted checkout. It does not make concurrent changes or
power loss atomic, recover a checkout already partially deleted by an older
Git-removal attempt, or guarantee a deadline for arbitrary tracked source volume.
The change is newer than beta.11. Dedicated real-Git consumers verify removal
ordering independently of elapsed time, tracked-content rejection before any
disposal, internal-symlink target preservation and a later Git failure/retry.
