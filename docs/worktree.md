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

Install `chrono-worktree` from the pinned public beta.6 release through the host’s
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
binary and dedicated test plan. Public beta.6 includes this tool for macOS arm64
and Linux x86_64; its native release recipe passed all 23 dedicated worktree tests
on each platform. The four [example hosts](examples.md) explicitly adopt the tool
and policy; no host-language or directory inference supplies their registrations.

Complete AI reconciliation, retry/cleanup recovery, PR provider operations,
merge and actual landing orchestration remain unfinished. Subsequent governance
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
Git/config/hook closure, automatic conflict resolution, cleanup and provider
orchestration remain outside this producer's current guarantee.
