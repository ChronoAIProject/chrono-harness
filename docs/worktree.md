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

`host_config` accepts the same stable registered `chrono-git-configs/v1` platform
entry as other full consumers, as well as a direct configuration. The shared
registry snapshot reader selects the direct full-v3 target separately from each
fixed source and fetched commit. Their effective configuration and workflow paths
may differ. Missing platform entries, missing targets and unsupported selected
policies fail without a fallback. All acquisition still uses the worktree policy's
bound Git runner, declared environment and process limits; host configurations
remain data and never select historical executable judges.

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
The source registrations are acquired once before fetch and reused for policy
checks and their digest. Both registry digests cover the complete parsed snapshot,
including the stable selector entry and its effective target when selected;
original bytes and failures remain in the fixed Git process observations.
Receipt and intent `config_path`/`config_sha256` continue to identify the worktree
policy itself. Reconstruction and maintenance, including recovery, rebind and
cleanup, use the same snapshot acquisition helper.

Resolved registry lists use the runner's shared bounded Git batch acquisition.
Metadata fixes each blob identity and size; content requests are partitioned under
the existing output limit and validated against that metadata. Small lists or
insufficient framing space retain direct reads. Original process bytes and exits
remain in the report on malformed or failed batches, before checkout effects.
Only fully validated immutable bytes enter the operation-local cache; live HEAD,
checkout, policy, attachment and ownership observations remain fresh.

For an explicitly named immutable input larger than the ordinary diagnostic
bound, v1/v2 policies may additionally declare `immutable_input_limits`, a map
from exact repository-relative paths to positive byte limits of at most
67,108,864 bytes. An omitted map preserves the original transport contract.
This host declares `.chrono-harness/config.json` and
`.chrono-harness/FILEMAP.json` at 2,097,152 bytes while keeping
`output_limit_bytes` at 1,048,576. These declarations grant acquisition
capacity; they do not register files, outputs, consumers or cleanup eligibility.

The existing blob owner first acquires fixed-commit object metadata under the
ordinary output/time limits. Oversized inputs are refused before acquiring their
content. For an admitted blob, only stdout uses its exact observed length
(one byte for an empty blob); stderr and other Git commands retain the ordinary
limit. The data command addresses the observed object OID, and Rust validates
its complete Git blob hash and length. Both original processes, bytes, digests,
exits and any partial failure remain in the report; the data process also records
its stdout and stderr transport bounds. Failed or mismatched bytes never enter
the immutable cache. The process owner and lease handoff remain unchanged.

This is a mixed product/policy adoption. Install the compatible binary before
committing this field in host policies; older strict decoders reject it. A larger
input allowance does not waive committed policy equality, live checkout checks,
artifact registration or migration. Complete input transport does not establish
full input closure or deterministic parity.

The Rust acquisition regression accepts an optional read-only
`CHRONO_IMMUTABLE_INPUT_FIXTURE` to verify the original large host input, checking
its fixed length and SHA-256 before use. Without that explicitly registered test
input it exercises the same capacity boundary with synthetic bytes. This does
not change the production input or its expected identity.

Coordinator maintenance may acquire an existing enrollment with this data
contract while preserving that target's committed configuration and HEAD. Its
unchanged target policy inputs must still match the enrollment's current binding,
and its cleanup policy hash must match the coordinator's policy. Changing only
the coordinator's transport declaration does not authorize changing an enrollment
or rewriting a previous attempt. If a pending cache attempt binds the old
coordinator configuration, its existing reconciliation checks still apply.

Successfully parsed trees at fixed OIDs share their original acquisition within
one operation and root. Cleanup reuses that immutable tree across its guards;
the index, physical source, policies, locks and usage checks retain their live
reads. Failed or malformed trees never enter the cache, and a later operation
acquires its own evidence.

Live checkout identity reads its root, common repository, metadata directory,
HEAD and full branch name in one bounded Git process. Maintenance consumes that
same observation for branch and attachment checks, alongside fresh inventory,
physical identities, policy bytes and coordinator checks. Every disposal guard
acquires a new observation; none is reused across deletion effects. Malformed
fields, mismatched identities and failed Git processes still preserve pending
work and the original failure evidence.

Local check preparation compares the index and physical files with its fixed
candidate both before and after fetching the base. Its final live HEAD check
does not select a different snapshot for the following file comparison. A clean
replacement commit cannot validate the original candidate; untracked-file
checks remain live. These observations do not form an atomic Git transaction.

Creation uses a new branch and a worktree lock tied to this invocation. Actual
Git inventory, HEAD, branch, root and checkout cleanliness are checked before
unlocking it. The producer reuses registration's artifact classifier: declared
untracked artifact paths are allowed and preserved. Tracked changes are always
checked separately, including beneath artifact roots. One untracked-file inventory
includes ignored and nonignored files, independent of changes to ignore rules.
It excludes only explicitly registered artifact directories before returning paths;
large build outputs therefore do not consume the path observation bound.
Remaining paths use the original registration classifier.
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
cleanup. AI semantic reconciliation, PR provider
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

Public beta.11 includes `inspect-rebind` and `rebind`.
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
untouched. `resume-rebind` below also handles pre-attachment failures and missing
results using the original intent and plan. The AI must first establish that the
old process stopped and reconcile conflicts in the retained evidence. Backup uses same-filesystem
rename; cross-device failure is reported without deleting old metadata. There is
no concurrent-writer isolation or crash/power-loss atomicity guarantee, and the
observed identity does not preserve timestamps, ACLs, xattrs or hardlink topology.

### Continue an interrupted or failed rebind

The source adds `resume-rebind`; public beta.15 includes it. It consumes
an existing `chrono-worktree-rebind-intent/v1` and the exact original rebind plan
at the intent's `plan_path`. The continuation plan must use its own registered
state path. No new index, backup, donor or ownership choice is inferred.

```sh
chrono-worktree resume-rebind --host-root ROOT --config .chrono-harness/worktree.json --plan .chrono-harness/state/resume-rebind.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "resume-rebind",
  "head": "<original branch HEAD>",
  "intent": {
    "path": ".chrono-harness/state/worktrees/rebind-INVOCATION.json.rebind-intent.json",
    "sha256": "<observed original intent digest>"
  },
  "result": {"presence": "present", "sha256": "<observed original result digest>"}
}
```

Use `result: {"presence": "absent"}` only for an observed missing result. Empty
and partial result bytes use the present form. A complete original failed rebind
must match its intent; it remains `original_outcome: failed`. Missing/partial
results remain `original_outcome: unknown`. A completed successful result is
refused. The original intent, plan, result and preserved metadata remain unchanged;
the new report retains their input identities and original process evidence.

Initial and resumed rebind share the same preservation/attachment code. It checks
source commit, registry, policy, plan bytes, branch HEAD and explicit index tree;
visible file bytes/modes/literal links must still match the original observation.
It completes missing pointer preservation or metadata relocation, reuses only the
original locked donor, installs an absent index from the declared tree, attaches
the target, removes the empty donor and releases its own lock. Existing indices
must already match; no index or visible work is overwritten to force agreement.
An already attached, unlocked destination is accepted only with the exact branch,
index and original backup and an absent donor. Continuation may itself be retried
against the unchanged original intent and result after its prior process stops.

Conflicting backups, changed visible work, donor files, another lock/branch,
metadata belonging elsewhere, stale plan/result/configuration and ambiguous
allocation are preserved and rejected before continuation effects. Interrupted
Git `index.lock`/`HEAD.lock`, incomplete donor metadata, partial temporary files
or a partially written old backup need explicit AI reconciliation; the tool does
not discard those bytes or guess their meaning. Saved pointer publication now
uses an atomic file replacement. Backup relocation still requires the same
filesystem. Neither the intent nor this command proves the prior process stopped,
concurrent-writer exclusion, power-loss atomicity or historical-index recovery.

Real CLI tests terminate the producer before preservation and around donor
creation, branch/index preparation, pointer attachment, repair and unlock. They
also cover absent/partial/failed results, repeated continuation interruption,
changed identities and completed-result refusal. `rebound` verifies the current
attachment and preservation only; it is not a candidate check or governance pass.

### Exact physical snapshot before release or deletion

The source checkout guard now reuses the runner's literal file/index comparison
([contract](git-facts.md#literal-checkout-identity)). Creation, reconstruction,
reconciled recovery and cleanup reject bytes, link representation and owner
execute changes even when Git configuration reports a clean working tree.
Cleanup preserves such unsaved work; recovery keeps its lock until the actual
files match the explicitly supplied index tree. The existing artifact exclusion
and saved-commit checks still apply. This change is distributed in beta.11 and
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

The beta.11 anonymous macOS binaries were installed in a clone of the upgraded
public mixed host and its owned checkout. With the old receipts removed and
the index damaged, explicit inspect/rebind preserved staged and unstaged work,
raw binary bytes, executable mode, literal symlink, selected index and original
metadata. The original outcome stayed unknown. Retained work was committed
before registered cleanup verified checkout/branch removal. This does not cover
interrupted rebind continuation; see [example adoption](examples.md).

## Opt-in automatic lifecycle cleanup

The source adds `automatic_cleanup` to the existing v1/v2 worktree configuration.
Its value is one tracked, FILEMAP-registered policy path under `.chrono-harness/`.
Omitting it retains the existing explicit maintenance behavior. There is no daemon,
periodic idle trigger, age expiry, sibling-directory inventory or language inference.
The main host invokes its registered `.chrono-harness/bin/chrono-worktree`; deploy
the candidate binary before using its newly adopted configuration.

The adopted main-host configuration selects `.chrono-harness/cleanup.json`. Its
v2 policy retains these v1 fields:

| Field | Contract |
| --- | --- |
| `coordinator_root` | An exact absolute physical Git checkout root, or the explicit `git-main-worktree` selector delegated to the existing Git owner inventory. It must survive disposal and belong to the same common repository. |
| `state_directory` | A registered untracked directory under the coordinator's `.chrono-harness/state/`. |
| `retained_ref`, `retention` | One direct local branch and existing `ancestor` or exact `same-tree` semantics. |
| `remove_branch` | Explicit choice; main host keeps branches. |
| `allow_evidence_disposal` | Whether the terminal owner may additionally select evidence disposal. Main host disables it. |
| `artifacts` | Exact `config.artifacts` directory paths, each with `dispose`, `retain`, or `evidence-retain`. No kind-name or layout interpretation. Omitted paths remain retained. |

Policy/configuration bytes must match committed inputs at the invoking host and
coordinator. Selected artifacts must be untracked declarations at both disposal
endpoints. Main-host adoption individually selects the current registered build
outputs and cache. It retains `.chrono-harness/bin/` and evidence in
`.chrono-harness/state/`; its ordinary finish therefore reclaims caches and retains
the checkout. Settling that evidence for whole-checkout removal requires an
explicit policy change by its owner, followed by enrollment migration; changing a
policy does not silently reinterpret existing enrollments.

Successful `start` and `reconstruct` automatically enroll their exact linked
checkout attachment and retain the producer's birth observation at the surviving
coordinator. Failed creation, missing state and unknown ownership remain protected.
For v2, persisted enrollment with unsealed birth evidence is reconciled at a normal
entry after admission/attachment exclusion and original policy validation. The
original missing or partial outcome stays unknown; recovery does not claim an
original successful birth or terminal handoff. Before creation, both entries call the shared
drain after validating policy/identity, excluding the invoking source and intended
destination. Unrelated drain failures retain their original reports and do not block valid
admission; target identity and coordinator publication failures still block.

At the beginning of that shared drain, the bound Git runner reads one
`worktree list --porcelain -z` inventory. The parent report's
`worktree_inventory` rows contain the exact registered `path`, `head`, `branch`,
`locked` and `prunable` flags, the original `git` fields (including lock/prune
reasons), and available `checkout_identity` and physical `attachment` fields.
`enrollment_match` requires the exact path and attachment of a non-disposed ledger
entry in this common repository. Current HEAD/branch observations are recorded;
birth HEAD/branch are not universal physical identities. Main and active consumers
can legitimately advance or switch them.

Rows without a current enrollment are `preserved` with an explicit-import reason;
unreadable or changed attachments are `blocked` with `attachment_error` and a
preservation reason. The coordinator and explicit source/destination exclusions
are reported as protected. This observation never imports, acquires a consumer
lease, marks terminal or reclaims an unknown row. Explicit `import` remains the
ownership transition for an existing linked checkout; old disposed generations
grant no ownership over a reused path. A blocked inventory row is a diagnostic,
not a new deletion or admission policy. Inventory command and report-publication
failures are fatal. Existing enrolled cleanup still takes leases and rechecks
identity, retention and effects; unrelated cleanup failures retain their existing
admission behavior, while explicit maintain/finish remains fail-closed.

The inventory describes the beginning of drain, not the post-cleanup tree. It
covers only Git-registered rows, not orphan directories or arbitrary external
writers. Dirty work, branches, unretained commits and evidence in an unowned row
remain untouched. It cannot take custody of an active external validation checkout
or reconstruct its missing history. Local and generated CI retain the existing
canonical check entry; local fixture evidence does not establish native parity,
complete input closure or full host activation.

The fixed lifecycle commands default to `--host-root .` and
`--config .chrono-harness/worktree.json`:

```sh
chrono-worktree use --path /exact/worktree --operation test.example
chrono-worktree finish --path /exact/worktree
chrono-worktree maintain
chrono-worktree import --path /exact/existing-worktree
chrono-worktree migrate --path /exact/enrolled-worktree
```

`use` consumes exactly one registered project/script operation and its tool,
argv, environment and process bounds. It records managed use under the same
admission gate used by finish/disposal, invokes the existing bounded process
owner, preserves original process bytes and exit, and releases use after the child
has joined. A nonzero child exit is reported as failure even when use was safely
released. Lifecycle results publish at the surviving coordinator even when the
wrapper was invoked from the disposable checkout, so release leaves no result
writer there. Admission/release wait within the registered process bound when
another owned gate is busy; an interrupted gate is preserved, never expired.
Active use blocks finish. Terminated wrappers, missing results and
unknown ownership retain the use record; they are never expired by age. The
existing process owner terminates lingering descendants within its owned process
group; detached/background writers escaping that ownership are unsupported and
must be joined by the caller. Git locks provide additional checks, not this use
protocol.

`finish` is the caller's explicit terminal handoff after joining all owned jobs
and establishing completion/landing. The runtime observes the enrolled physical
attachment, current HEAD/branch and allowed retained ref, publishes an immutable
terminal receipt, and immediately invokes the same drain. It does not create a
source-HEAD retention ref or determine PR/merge success. `--retained-commit OID`
optionally verifies a caller's actual squash-landing pin before the reference can
move; the observed OID is always fixed in the receipt. A later changed ref is a
visible failure, never a replacement landing guess.

`--artifacts-only` requests cache disposal with checkout/refs retained. Default
unsettled `evidence-retain`, unretained commits, dirty source or other retained
artifacts also produce an explicit preserved reason and permit disposal only of
selected quiescent outputs. These successful cache-only entries become `retained`
and are not rescanned on every later start. A subsequent explicit finish may settle
evidence using `--dispose-evidence` only when policy allows it; the prior terminal
receipt stays in history. New managed use is refused after terminal handoff.
When finish runs from the enrolled checkout itself, the drain excludes that source;
a later coordinator `maintain` or start performs disposal.

Whole-checkout removal delegates to existing `Cleanup`, including literal
cleanliness, two-endpoint artifact whitelist, exact retention, physical identity,
worktree/Git locks, nested checkout checks and optional expected-OID branch removal.
The shared artifact phase also protects staged source inside artifact prefixes,
rejects symlink roots/ancestors and unlinks internal symlinks without following
external targets. Automatic reports measure literal entry lengths before disposal
and verified zero afterward; these are logical bytes, not allocated-block savings.
Legacy explicit maintenance keeps its report shape.

The optional v2 `main_cache_only: true` enrolls the physical Git main checkout
through the same owner on ordinary `use`, canonical `check`, and adopted
`bootstrap`. Its enrollment, use intents, shared kernel lease, original process
result, release, and cache attempts use the existing lifecycle state. Main finish
releases caches and stays active; it never authorizes checkout, branch, index,
source, bin, evidence, or commit deletion. Evidence-disposal and landing-pin flags
are refused for main. Pre-existing registered outputs without adopted consumer
release remain explicitly protected historical objects. Enrollment owns the
absent, individually registered outputs before managed production starts; this
is not a migration of the current host's historical targets or producers.

For historical outputs, the same owner accepts an explicit current-custody
transition through `migrate --path /exact/main --adopt-cache STATE_PATH`.
The strict `chrono-main-cache-custody/v1` declaration under registered host state
contains `path` (the physical main checkout), its full current `head`, exact
`artifacts` paths, `current_consumers_released: true`, and a nonempty `reason`.
The current operator must actually join or exclude prior unmanaged consumers and
state its present custody decision. This is an engineering transition, not an
assertion that a past producer finished, succeeded, sealed, or released anything.
Only the active adopted main enrollment and registered `dispose` outputs qualify;
source/index paths, bin/evidence, unknown paths and live adopted consumers refuse
adoption. Unselected historical objects stay protected without vetoing selected
eligible siblings.

Admission and the original exclusive enrollment lease protect this transition.
Its separate immutable receipt preserves the raw declaration and original
attachment, records the ownership delta, and keeps historical producer outcome
`unknown`. Historical producer and disposal outcomes are preserved. The existing
`reconcile_uses` may append interrupted-use history and remove stale use tokens;
mutable enrollment/uses are not guaranteed byte-identical. A
published receipt without a ledger update can be retried with the same binding;
repeat adoption of managed paths adds no history and performs no disposal. The
existing pending cache generation and ordinary drain reclaim the newly managed
outputs after exclusion, without setting finish or adding a cleaner. Subsequent
bootstrap/check/use share the same leases and interrupted-use recovery. Deploy
the compatible owner before adopting; old readers reject the new nonempty
`main_cache_adoptions` ledger field.

Main re-entry drains released pending cache generations before starting a
new consumer. A linked checkout being entered stays excluded until its new
consumer has acquired protection; another ordinary entry can recover that
checkout after release. Installation, tests, cache saving, and other registered operations
must run under that same use protection. Actual shared descriptors protect live
descendants and overlapping consumers. No post-check Cargo clean is added.
The disposal owner rechecks HEAD, physical attachment, policy, and the live index
before each removal. Logical lengths and allocated blocks (`st_blocks * 512`,
including directory blocks) are reported separately. Physical release is unknown;
allocation differences do not establish filesystem savings. Failed removal
retains the original error and observed remaining objects, and retries preserve
the original attempt.

The optional v2 `retained_producers` lists exact producer IDs, evidence directories,
and generated output paths relative to each explicitly registered receipt.
The adopted `ci-command-result` family belongs to
`ci-tests::behavior::retain_command_result`. The Rust `RetainedArtifact` interface
admits the exact preserved object and publishes its intent. The adopted wrapper
first copies original stdout, stderr, binding, configuration and state, then calls
`begin` and seals generated directory identities and original stream/binding
digests. Interruption or admission failure before `begin` leaves an unregistered
copy protected as unknown evidence; an admitted but unsealed receipt also remains
protected. The interface records consumers
with independent explicit acquire/release receipts. Dropping a handle or losing
a process never releases a receipt. The existing drain reads only that family's
`objects.json`; it does not discover directories or infer references.

Publication resolves the selected producer through `RetainedArtifact::registered_store`,
using the same committed worktree configuration, cleanup policy, registration,
and physical Git main coordinator checks as drain. The default wrapper supplies
its producer checkout separately from the command's fixture root, so a linked
producer publishes below the main coordinator's declared evidence directory.
`CHRONO_CI_TEST_RECEIPTS` may select only that exact physical store; undeclared,
overlapping, mismatched, or symlink destinations refuse publication and remain
protected. Store resolution does not release consumers or grant main checkout
or branch deletion authority. Earlier linked stores have no adopted forwarding
route and remain protected unknown evidence.

This host admits only the two copied independent Cargo outputs at
`original-state/cargo/producer/target/` and `original-state/cargo/tests/target/`,
and the producer additionally requires their exact source artifact declarations.
The selected expected-exit assertion publishes release after consuming the
original result. An unexpected exit, incomplete publication, or active diagnostic
reference retains the copies. After all references explicitly release, the owner
may reclaim those generated copies; stdout, stderr, binding, configuration,
source, recovery originals, and other state remain evidence. Existing callers
without an expected-exit contract retain an unreleased reference. Unknown legacy
receipts and every unregistered object stay protected; `.chrono-harness/state/`
does not become disposable. Compatible committed policy and binary deployment,
real-host migration, native CI cache-save participation, and full SPEC adoption
remain outside this implementation's verification.

The coordinator ledger records original identity, terminal receipts and attempted
report paths before effects. Failed results retain their original bytes/digests and
partial effects. A normal retry can reconcile only its recorded owned lock and
exact unchanged checkout, then use existing absence-tolerant cleanup. Missing or
interrupted results, changed attachment/HEAD/policy/ref, another lock or damaged
source are preserved for explicit reconciliation. Retrying never overwrites an
original failure. Storage failure, unregistered concurrent writers and power loss
are not transactional guarantees; stale admission/use records currently require
caller reconciliation after joining their original jobs.

`import` enrolls one explicitly named existing linked worktree, validates its
current registration/attachment and binds its existing policy inputs (including
explicit absence). It can import a legacy checkout without editing that checkout
or pretending it adopted the new automatic policy. Disposal paths must already
be declared at both endpoints. Its observation is `import`, with no historical
birth claim. It does not enumerate old hosts. Caller-owned Git/GitHub,
publication, immediate existing-host reclamation and evidence settlement remain
outside this producer. This repository's worktree fixtures select their Git
executable from `.chrono-harness/tests/worktree-tools.json` for each explicitly
registered OS/architecture. The fixture then records its real executable identity
and version through the existing binding checks; the product's host Git policy
remains independent. Missing test-platform registration fails without a PATH
fallback. Local real-Git fixtures exercise full checkout removal;
actual main-host finish/landing is the caller's acceptance event after adoption.

### Explicit policy migration

`migrate --path /exact/enrolled-worktree` adopts the coordinator's current,
committed cleanup policy and worktree configuration for one existing v2 kernel
enrollment. Run it from the surviving coordinator after committing matching
inputs in the target. It requires the original admission and exclusive enrollment
leases, the same physical Git attachment, validated birth evidence, no foreign
Git locks, and artifact registration at both endpoints. Policy input addresses,
coordinator, state directory and ownership protocol stay fixed; relocating those
identities is outside this transition. Legacy or missing kernel ownership and
pending terminal disposal remain protected. Policy changes do not migrate entries
implicitly.

When two endpoints have different host configurations, preserve each host's
registered sources and inputs. The migration comparison covers the explicitly
bound cleanup policy and worktree configuration; it does not require copying the
coordinator's complete host configuration. Every artifact selected by the adopted
cleanup policy must already be registered at both endpoints. If one endpoint has
no real producer for a selected output, do not invent that registration: choose an
explicit policy limited to supported outputs, or leave that enrollment on its
protected binding until its owning adoption is complete. This command has no
per-enrollment policy override and never changes Git itself. Preserve original
commits and outcomes when the caller creates compatible adoption commits.

The original enrollment, birth policy hash, birth receipt, uses and prior cleanup
attempts retain their identities. A separate immutable transition records old and
new input bindings, retained original bytes and observed commits. The ledger
appends its receipt only after checking the new bindings again. Normal use,
cleanup and finish validate this chain and use its final binding. A missing or
changed receipt or retained input refuses subsequent use/disposal. Deploy a
compatible lifecycle binary before adopting a transition; older readers reject
the new nonempty migration field instead of interpreting it as an old binding.

Migration performs no disposal, Git update or terminal handoff. It preserves
source, outputs and task status. Active entries begin a new cache generation;
retained entries remain retained and need a new explicit finish before further
terminal disposal. Quiescent interrupted uses retain their reconciliation records.
A repeated request with unchanged policy bindings adds no history. An immutable
transition published before interrupted ledger adoption is reused on retry; its
existence alone does not claim successful adoption. Unknown or unsealed birth
outcomes must be reconciled under their original policy before migration.

### Kernel ownership and unfinished cache recovery (v2)

The v2 candidate uses `chrono-worktree-automatic-cleanup/v2` with the existing
policy fields and a new registered state directory. The host source selects
`.chrono-harness/state/automatic-cleanup-v2/`. Caller cutover must seed the
compatible coordinator binary and exclude incompatible producers; v1 gates,
tokens and unknown records are never retrospectively treated as kernel leases.

The caller owns staged validation and coordinator cutover. First validate the
exact committed candidate in an independent, clean Git main clone using the
registered bootstrap and short check commands. The current scoped local producer
has `full_context: None`; this route needs no synthetic linked origin receipt.
Main-only bootstrap may seed a missing owner binary. This is candidate validation,
not acceptance of the surviving coordinator or of full/native obligations.

For actual cutover, join/exclude incompatible producers, retain the v1 state,
and install the compatible binary in the surviving main coordinator. Commit and
deploy the matching v2 worktree policy, cleanup policy, selected host configuration
and participation declarations there before starting v2 linked work. Only then
use ordinary `start`/`reconstruct` for automatic v2 enrollment and the unchanged
public bootstrap/check commands in a fresh linked checkout. Working and committed
coordinator policy/config bytes must agree with the invoking checkout; upgrading
only the binary or only a linked policy intentionally refuses before build or
cleanup. Never convert v1 tokens to leases. The real coordinator transition and
its retained evidence remain caller-owned acceptance.

Admission is a stable identity-bound flock inode. Each physical attachment
generation has one stable SH/EX lease. Normal use acquires SH before intent/spawn
and holds it across result publication; admission is released before business
execution. Git mutation children inherit admission. Cleanup holds admission and
nonblocking EX, preserving live holders. Last close releases ownership; no
`LOCK_UN` is issued on a shared inherited open description. Missing/replaced
identities, policy drift and foreign Git locks preserve work. Birth holds its
attachment lease through enrollment and receipt sealing. If that producer is
interrupted, normal use/maintain/finish can publish an immutable identity-bound
reconciliation under EX. Repeated interruption after reconciliation publication
reuses its original bytes before attaching the pointer to the ledger. Recovery
retains the original report and any unreferenced sealed receipt as input evidence;
it leaves `sealed_receipt` absent and records `original_outcome: not-established`.

Cleanup refusals are scoped to their enrollment. Normal start/reconstruct/use/check
and bootstrap retain unrelated failures in `drain`, immutable failed receipts and
`cleanup_failures`, then admit a valid target. Target admission/ownership failures
and coordinator/state publication errors still block. Explicit `maintain` returns
failure while any attempted cleanup fails; it never discards an earlier receipt.

Automatic lifecycle reports preserve prior inputs through
`chrono-worktree-retained-input/v1` references. Each reference carries the physical
coordinator `source_root`, relative state `path`, `sha256`, `byte_length`, and
`format` (`receipt` or opaque `bytes`). Completed immutable receipts are referenced
in place. Partial or unsealed bytes are published once under the coordinator's
registered state directory; their missing/unknown outcome is not upgraded to
success. Existing originals are never rewritten. Retrying an unchanged operation
does not embed the previous report's bytes or parsed history.

The reference-bearing fields are `terminal_input`, `prior_report.input`,
`prior_cache_attempt.intent_input`, `prior_cache_attempt.result.input`,
`original_report.result.input`, `unreferenced_original_receipt.input`, and
`original_result.input`, and `drain[].input`. Current drain rows retain the exact
child report through that descriptor and the existing `receipt`, with the child's
`status` and `error` available in the parent. They do not embed the child's parsed
report. Consumers needing cleanup effects or original process observations read
the addressed child and verify its length and digest; canonical check and bootstrap
continue to consume the unchanged parent `managed_process` streams and status.
Receipt consumption checks these explicit references transitively,
deduplicating reads within the check. Missing, changed, incorrectly bound or
symlinked inputs refuse cleanup; an opaque partial result is preserved without
requiring valid JSON. Historical inline evidence remains readable. Explicit
maintenance outside automatic lifecycle retains its existing report shape.
This bounds historical embedding, not the number or total size of necessary
original observations; retention and disposal remain governed by host policy.
Historical `drain[].report` remains supported by the current reference verifier.
The stdout protocol bound is unchanged. Parent publication still fails on its own
IO or transport errors, and a retained child failure remains a cleanup failure.

Later start/reconstruct/maintain and managed admission can dispose unfinished
quiescent caches, including a successful use with no orphan token. EX permits
orphan-use reconciliation as result-unavailable while retaining original result
bytes. Cache disposal keeps `status: active`, `terminal: null`, checkout, refs,
dirty/staged source, unretained commits, bin and evidence. It selects only
`dispose` paths registered at both endpoints, rejects tracked/index paths and
symlink ancestors, and creates no persistent owned Git worktree lock. A successful
pass clears its pending generation; later use rearms it. Admitted consumption
after any published cache attempt also opens a new generation, even if the prior
attempt was interrupted or failed. It retains the old intent/result unchanged and
permits current HEAD/registry bindings for rebuilding and later reclamation. Cache-specific immutable
intent binds attachment, lease, HEAD, policies, registries, paths and generation
before effects. Missing/failed results retry idempotently under reacquired
exclusion with unchanged bindings and a new real receipt; originals stay intact.

The optional canonical config declaration is:
`"participation":{"operation":"worktree.check","tool":"chrono-worktree","argv":["check"]}`.
The public spellings remain `check`, `check --unit ID`, and `check --collect`.
Participation forwards original stdout, stderr and exit, including successful
warnings and nonzero diagnostics. The owner separately marks a completed command
failure; transport, ownership and result-publication failures remain lifecycle
errors with their original report reference. The CLI's string result preserves
UTF-8 console bytes; owner process reports retain the original byte arrays.
The worktree owner admits the existing runner before acquisition and forwards its
original console/exit. Bootstrap separately declares `entrypoint` tool/script and
`participation` coordinator/program/Git/config. Standalone bootstrap delegates to
the registered deployed coordinator binary before build effects, including fresh
linked checkouts with no local bin. Missing ownership fails explicitly.

The existing engine transfers explicit opaque descriptors using CLOEXEC owner
copies and inheritable child copies in pre-exec. `CHRONO_PROCESS_FDS` survives
nested-runner cleared environments. The three registered Python consumers use
`process_fds.py` and explicit `pass_fds`. These are tested routes, not universal
inheritance. The real Cargo test regression verifies distinct launcher and native
child identities, kills the Cargo and managed wrappers, and checks that the live
child retains the lease until it completes. Cargo run may exec-replace its own
process; killing that PID does not demonstrate surviving-child behavior. Do not
infer child exit from a PID or parent disappearance. Committed composition and
supported native platforms remain required before complete adoption.

Whole-checkout finish/retention and legacy owned-lock recovery retain their
contracts. There is no daemon, global process/filesystem inventory, age expiry,
idle scheduler or power-loss transaction claim. Caller-owned committed canonical
checks, supported native platforms, accepted process-engine composition and main
host deployment/cutover remain acceptance work.

A Git main checkout with no deployed owner may seed its own bootstrap tools: it
cannot enroll as a disposable linked attachment. A linked checkout without the
registered coordinator owner fails before build effects. This keeps clean native
CI bootstrap possible without changing workflow topology or authorizing unsafe
linked-checkout fallback.

### Current custody of exact evidence aliases

`chrono-worktree retire-evidence --host-root ROOT --config POLICY --plan STATE_PATH`
is the maintenance owner's finite file transition. It consumes a
`chrono-worktree-maintenance/v1` plan with `operation: "retire-evidence"`, exact
`head`, and `custody` using `chrono-evidence-custody/v1`. Custody declares the
physical `root`, full `branch` ref, `current_consumers_released: true`, engineering
`reason`, an `authority` original (`path`, SHA256, `length`), and exact `objects`.
Each object has an `original` identity, a distinct retained `survivor` path,
producer `owner`, and release `reason`. Equal bytes verify preservation; the
current custodian's explicit release supplies disposition authority. The caller
joins or excludes prior unregistered consumers and ends copied-path assertion or
manifest replay before declaring release. Historical manifests remain original
evidence; this transition does not claim that retired paths remain replayable.

The `exclusion` declares the existing `coordinator_root`, tracked `config_path`
and adopted `state_directory`. The command reads their current committed kernel
policy as data and holds the existing admission and enrollment leases. It checks
one current active enrollment, its physical attachment, outstanding use tokens,
foreign Git locks and clean fixed checkout. It does not drain other entries,
migrate old source policy, update Git, release the checkout or fabricate a past
producer seal. A historical host can invoke the candidate binary with its own
unchanged committed worktree policy and artifact registrations.

Only explicitly listed untracked regular members under registered evidence
state are eligible. Source/index paths, selected survivors, the authority and
attempt inputs/outputs refuse retirement. Each alias and survivor is streamed
against its declared identity, including stable physical file identity, just
before unlinking. Independent identity/preservation failures leave that alias
and continue other eligible objects. The attempt's separate `effects.jsonl`
journal syncs its declaration reference and each pre-effect intent/result;
reports contain counts and references, never preceding report bodies. Repeating
the same finite plan rechecks survivors and records absent aliases as observed
absence, without inventing which interrupted attempt removed them. Original
failures, enrollment, source and refs remain unchanged. Exit 0 means this exact
finite disposition completed; unresolved members or binding failures exit 2.
The `removed` count includes only fully verified completions. A post-unlink
verification failure can increment `failed` after an effect recorded by its
pre-effect intent; zero `removed` does not prove no unlink occurred. The exact
referenced custody declaration and authority remain required retained inputs;
reports do not guarantee a self-contained or independently immutable declaration
copy. Logical removal counts do not establish physical shared-volume disk release.

Worktree test fixture retention remains explicitly requested by
`CHRONO_WORKTREE_TEST_RECEIPTS` or panic. Successful default fixtures do not copy
all state. Retained fixture assertions and failure diagnosis remain consumers
until their current custodian explicitly ends those copied-path obligations.

### Local initial input preparation

The registered `check-inputs` action supports config schema4's optional separate
`canonical_check.initial_profile`. It binds both profile digests to committed
HEAD and records physical commit parents through the existing observed Git owner.
A genuine parentless candidate prepares standalone inventory with no base, fetch,
origin or full context. Nonroots retain the registered target fetch and ordinary
DELTA/context contract. Missing targets and shallow histories do not become roots.
The bare runner entry validates the original preparation before the existing
initial judge runs; the producer itself never publishes inventory success.

CI copied-host 断言现在复用专属测试的独立 receipt owner 和显式平台 Git 输入；不把源宿主已部署的 Git 摘要／版本当作原生 fixture 的政策，也不改源宿主绑定。原错误与 passing control 均由实际命令产生。worktree 的未保留 commit-tree fixture 显式提供作者姓名／邮箱。

真实 Cargo retained-rebuild handshake 位于冷构建之后；该 fixture 等待现役 managed process 的实际结果，沿用其原登记的 30 秒进程上限，不用另一个 10 秒 readiness 时钟杀死编译过程。其它 readiness 等待仍为 10 秒。缺 marker 时消费并保留真实退出与过程原件，退出前未完成则仍不产生成功；专属负例用缺失 manifest 检查失败和加入子进程。原生旧失败只留下空 outer 流与 use intent，不能据此断定具体编译阶段或停滞原因；支持平台的修复后原生验证仍待调用方执行。

### Prospective temporary-host custody

**Policy and product change warning (R1B):** `fixture_bodies:true` changes the
adopted lifetime of individually enrolled copied fixture inputs. Previous policy
kept every copied executable indefinitely. The copy producer now retains original
bytes once per SHA256 in its existing store, then publishes an immutable member
intent before copying. The original allocation intent and producer outcome remain
unchanged. Drain removes only the enrolled matching file after kernel exclusion
and current diagnostic-reference release. It retains source, Git objects/commits,
raw captures, partial or changed members, missing-original failures and unknown
neighbors. This requires matching candidate binaries and committed policy; an
older binary rejects the added registration. Retained-original storage cost is
one file per distinct input; classification hashes real bytes, and recovery checks
original/member bytes and explicit Git roots. Physical APFS savings are unknown.
Targeted Rust tests cover no-finish recovery, overlapping holders and diagnostic
references, interruption/live native descendants, partial copies, missing
originals, source guards, finite migration and repeated recovery. R2 still owns
the broad canonical/coordinator receipts and external/native delivery boundaries.

`TemporaryHost::register_copy` receives the exact declared store, producer, real
source file, destination, possible Git root and producer recipe. It preserves the
real input before publishing membership; the existing joined copy route continues
to own writable executable descriptors. Native and engine consumers retain their
existing allocation scopes. Enrolled bodies do not acquire an outcome from a
command exit. A partial copy is not identical and stays protected; an interrupted
copy whose complete bytes match its retained input may be recovered after the
last supported holder without Drop, finish or a fabricated seal.

Current custodians enroll exact legacy members through the same lifecycle entry:
`chrono-worktree migrate --path <coordinator> --adopt-fixtures <state-plan>`.
The `chrono-fixture-body-custody/v1` plan binds current HEAD, actual current
consumer disposition and its reason; each selection binds producer, allocation,
physical identity and original intent, and each member binds path, SHA256, length,
explicit possible Git root and committed producer recipe file. Migration retains
the actual member bytes and immutable custody/provenance receipts. It deletes
nothing, releases no old diagnostic reference and imports no unknown object.
Ordinary adopted recovery performs the effects. Source/index guards run again
before removal. Cargo `disposed` state does not settle subsequently enrolled
bodies. Concrete unavailable members fail independently of checked siblings.

The behavior oracles for temporary allocations are declared before their tests:
production must follow an immutable allocation intent; ordinary maintenance must
preserve a distinct live native descendant and an overlapping holder, then reclaim
only the selected rebuildable directories after final actual release. Normal exit,
SIGTERM and SIGKILL before Drop/finish must preserve source, partial streams and
missing-result truth. Unknown objects and unreleased diagnostic references must
remain protected. Literal absolute allocation bindings must survive nested native
commands, cleared command environments and Cargo working-directory changes,
including spaces and Unicode; no ambient caller export is the custody authority.
An intentionally blocked source check for one excluded allocation must still
allow an unrelated allocation to publish. A diagnostic claim on the excluded
allocation must fail without publishing a reference. After the check resumes,
only that eligible output is removed. Native captures must open retained raw
streams and intent before spawn: killing the producer while it waits must leave
the child's exact partial bytes and no joined result, seal or completion. A
normally joined command must return those same bytes and its actual exit code.
Each actual native route must bind its command exactly once. Binding an allocation
must forward the already held scope, without adding another copy of that same
lease. Captured and asynchronous routes must carry the same descriptor count as
one native forwarding operation in the same thread; their children must still
hold the allocation through interruption. This count is a local descriptor-cost
oracle, not an attribution of a historical system file-table failure.
An excluded allocation whose entire declared output set is absent requires no
disposal publication or registry rewrite. Its producer intent, missing outcome,
source and registration must remain unchanged, and the observation must list
only the declared absent paths. A present output still follows ordinary checked
disposal. This avoids durable writes for outputs that were never produced.
The interruption fixture places its independent native consumer in a distinct
process group and checks its actual PID after joining the killed wrapper and
after maintenance. A readiness marker cannot establish survival after managed
group cancellation; an actually terminated child supplies no live-holder claim.
An unavailable registered allocation must retain its original registration and
report a concrete failure. Exclusion and validation of an eligible sibling must
still proceed, including within the same producer. Explicit maintenance remains
nonzero for the missing allocation; sibling disposal does not repair that failure.
The regression moves the unavailable fixture to a retained unknown neighbor,
checks those source bytes and the unchanged failed row, and independently checks
the eligible sibling's actual removal. Real-coordinator tests retain maintenance
streams and validate their own rows while preserving unrelated failure outcomes.

Offline fixture consumers may make their Git path unavailable after joining and
transporting its originals, but their allocation and declared TMPDIR must remain available. The
shared projects fixture separates the Git root (`fixture/`) from its retained
allocation. Its external input is retained beside the movable checkout, keeping
the declared absolute input location available for delivery. Workflow tools and cloned hosts adopt that same adapter before
production. The checkout moves to `retained-offline-checkout/`, preserving source,
commits and raw captures. Both literal roots and the four actual `p`, `t`, `p2`, `t2`
target directories at each root are declared before production. Collection must still work with the old
Git root and business executable absent; it must launch no business processes.
The existing native capture implementation also serves already bound short/native
fixture routes at their explicit retained host. It does not add a capability or
establish custody; their declared command adapter forwards held capabilities once.

`retained_producers[].temporary: true` opts a declared store into prospective
cache recovery using the existing producer registry and kernel Lease. A
`TemporaryHost` resolves the committed host policy, publishes an exact child and
its output whitelist before returning to the producer, and holds a shared kernel
lease. The existing drain obtains exclusive exclusion; it does not require a
success seal or publish diagnostic release, task completion or producer success.
Drop closes ownership only. Source, originals and every unselected path remain
in place, so evidence custody survives a hard kill. Native Command adapters call
`bind_command`, which forwards owned descriptor copies through the existing
process engine adapter and binds the absolute TMPDIR; engine calls consume the
same capabilities. Host `temporary_environment` optionally selects one declared
producer for bootstrap/check/use command temporaries. Host paths and disposable
outputs remain in `.chrono-harness/cleanup.json`, independent of host language.

This repository explicitly adopts worktree test allocations, projects execution
and migration fixtures, and CI test hosts. Smaller nested temporary allocations
use their bound retained container; no system temporary directory is scanned or
imported. Declared Cargo outputs and individually enrolled copied inputs are
eligible for automatic disposal; other fixture contents retain their existing
source/evidence custody. Explicit fixture moves forward the registered copy
members before rename, including their possible Git roots; missing or changed
members acquire no new eligibility. Test allocation retention is intentional
and is not a completion claim. Unclaimed neighbors and earlier system-temporary
objects have no retrospective custody. Platform/native acceptance and external
coordinator delivery remain separate from local behavior tests.

The current producer correspondence is explicit: `creation::temporary_host`
owns worktree fixtures, `support/temporary::temporary_host` owns projects
execution and shared Host fixtures, `migration::temporary_host` owns migration
checkouts, and CI `support/tools::temporary_host` owns CI fixtures. Their native
command adapters forward the held scopes; managed engine calls use those scopes
and the declared absolute TMPDIR values. The CI receipt Cargo fixture additionally
binds both actual Cargo Commands, with outputs below `.chrono-harness/state/cargo/`.
Anonymous tempfile handles used solely as kernel test leases have no named output
to recover. Explicit retained failure-file publishers keep their existing evidence
directories and are not disposable temporary allocations.
The CI judge fixture also consumes the existing CI temporary-host adapter. Its
source and pre-evaluation request survive interruption; a returned response is
retained before assertions. Compact fixture JSON keeps the actual selected
Git output fault inside its original 4096-byte bound. This is fixture adoption,
not a claim that arbitrary native process streams have been captured.

`source_roots` declares allocation-relative Git roots (`.` means the allocation).
Before each removal the same owner checks the actual declared root's HEAD and
index; absent roots cannot silently resolve to the coordinator or a parent Git
repository. Worktree fixtures declare their main and linked roots, migration
fixtures declare their checkout root. The CI Cargo receipt fixture has no Git
repository and retains its source outside the selected target paths. No nested
repository discovery supplies custody. An entirely absent selected output set
requires no Git tree acquisition or disposal publication: the read-only
`already-absent` observation leaves intent, outcome and registration unchanged.

The live Cargo-output regression joins its bounded build before observing a
separate native installation/test/cache consumer. Its original failure had a
timed-out build and a stale readiness marker; the marker alone is not liveness.
This repairs that observation without increasing the command bounds. Other
historical ownership and nested CI timeout causes require their own controlled
evidence; a later passing run cannot change those original outcomes.
