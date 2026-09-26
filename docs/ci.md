# Generated CI and the active CI slice

`runner`, `judge-registration`, `judge-ci`, and `ci` are separate Rust production projects under
`crates/`, each beside its dedicated `-tests` project with separate Cargo.lock
and target directories. `instructions` and `instructions-tests` follow the same
layout. The container directory is not a Cargo workspace or a test-discovery rule;
each manifest, operation and dependency remains explicitly registered.
`runner` owns external process transport, request identity, strict protocol checks
and result publication. `judge-registration` owns full-format registration checks.
`judge-ci` owns the scoped CI registration, snapshot, DELTA and operation policy.
`ci` owns the GitHub workflow projection and event input
preparation. It does not select tests or judge outcomes. `instructions` remains
independent and its catalogs, layouts and relative root alias are unchanged.

The five full-governance registries remain **proposed**. The bounded
`chrono-ci-check/v1` profile explicitly consumes the FILEMAP and projects data
below using `chrono-ci-judge/v1`. Separately, `check` supports configured
`chrono-judge/v1` transport and the registration judge through
`.chrono-harness/config.json` with `--context`. This proposed host remains
incomplete and returns nonzero; the other six judges and automated lifecycle
are not implemented.

## This repository

Run from the repository root. Bootstrap requires Python 3, Git, and rustup; it
installs Rust 1.95.0 if unavailable, independently ensures its rustfmt component
(including on a preinstalled minimal toolchain), and builds the explicitly listed bootstrap
operations using `.chrono-harness/projects.json`, then copies declared binaries.
It does not duplicate Cargo build recipes or discover projects.
Toolchain operations name the configured version explicitly and do not change
the global default toolchain. Bootstrap these tools before the dedicated CI tests;
their copied-host regressions consume the installed candidate binaries.

```sh
python3 .chrono-harness/ci/bootstrap.py .
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/github.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/github.json
```

After changes have been committed and the candidate checkout is clean, the
canonical judgment command is **identical locally and in generated CI**:

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/ci/check.json --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID
```

Always bootstrap from the candidate checkout before checking it. The runner
reports the actual runner/judge/operation executable hashes; bootstrap records
Cargo/rustc versions and installed binary hashes. These observations do not
constitute reproducible-build or complete binary-provenance certification.
A parentless commit can instead use `--candidate FULL_OID --initial` without
`--base`. The report explicitly says `initial-inventory`; it does not fabricate
a historical DELTA.

The check exits 0 only after a valid passing response; judge failure exits 1;
usage, unsupported configuration, transport, malformed protocol or report I/O
failure exits 2. Information commands (`--help`, `--version`, `spec status`) do
not perform checks. A valid no-project-check result still validates snapshots and
registered inventory and reports each unselected binding as `not-required`.

Reports reside in the ignored `.chrono-harness/state/` directory. `check.json`
contains the request, runner and judge identities, actual selected operations,
executions with exit/output/executable hashes, blocked and not-required entries,
causes and scope. `context.json` records fixed event inputs and the workflow source
revision separately from candidate/base. `bootstrap.json` records actual tools.
Costs remain unmeasured. The same command alone does not establish local/CI
parity: ambient compiler backend, SDK, configuration, environment, external input
and deterministic evaluation are not completely modeled. No cache is enabled.

## Configuration and selection contract

The JSON decoder rejects duplicate members. The profile, provider, command and
protocol types also reject unknown fields. FILEMAP/projects are existing broad
registries: this slice validates the fields it consumes and does not pretend to
validate every proposed field's semantics.

`.chrono-harness/ci/check.json` uses these fields:

| Field | Contract |
| --- | --- |
| schema | Exactly `chrono-ci-check/v1` |
| judge | `{program, args, env?, timeout_seconds, output_limit_bytes}`; explicit executable/interpreter plus argv; positive bounds, at most 64 MiB per captured stream |
| report_path | Safe relative file beneath `.chrono-harness/state/`, also covered by a declared artifact directory |
| policy.filemap / projects | Explicit relative paths; schema_version 1; registry path migration across an enforced range is unsupported |
| policy.tools | Tool ID → executable path or PATH command; resolved once for each execution and hashed using the command's PATH override, otherwise inherited PATH; relative PATH entries use the child cwd |
| policy.bindings | `test:ID` → ordered, nonempty registered operation IDs; no test discovery |
| policy.environment | Explicit environment overrides passed to operations; this host sets `RUSTUP_TOOLCHAIN=1.95.0` |
| policy.artifacts | Relative directory prefixes ending `/`; ignored execution products may exist there; tracked files may not overlap them |
| policy.required_inputs | Explicit regular tracked inputs required by this profile |
| policy.adoption_base | Null or exact full base OID whose missing prior CI profile is deliberately accepted |
| policy.operation_timeout_seconds / operation_output_limit_bytes | Positive serial operation process bounds |

Both endpoint trees are read as NUL-delimited `git ls-tree -rz` records, preserving
path, blob identity and mode. Comparing these inventories implements A/M/D and
rename-as-delete-plus-add without rename heuristics. All tracked paths at both
endpoints must have FILEMAP entries; every registered file must exist. Relative
file symlinks require exact declared targets to regular registered files. Directory
symlinks, submodules, unsupported modes and non-UTF-8 paths are rejected.

The candidate must be HEAD at a complete 40- or 64-character lowercase commit
OID. Both objects must exist. Staged/unstaged changes and all untracked nonartifact
files, including otherwise ignored files, fail. Index entries marked
`assume-unchanged` or `skip-worktree` are unsupported and fail explicitly, even if
Git's diff hides changed bytes. The judge reads NUL-delimited index facts before
and after execution and never clears these flags or repairs the index. Use a
complete checkout without these flags for the supported clean-snapshot boundary.
The profile bytes must match
both request digest and candidate tree. The snapshot is checked again after
operations; a check that mutates tracked input fails. Arbitrary commit pairs are
explicit comparisons; freshness, ancestry and integration provenance are not
implemented by this slice.

Selection seeds changed files, changed FILEMAP records, project/script records,
operations, bindings, tool/environment/bounds settings and both endpoints of
changed graph edges. Propagation uses the union of old and new graphs. Only
explicit `test-execution` edges select a test binding; compile/build-input/runtime-
input edges propagate to declared nodes. `judge-trigger` metadata remains outside
this slice. Missing nodes, bindings, duplicate operations, invalid actions and
unknown changed paths fail. Removing an edge still selects its old surviving
consumer. Removing an affected binding or bound operation fails; no implicit
retirement or full-suite fallback exists. Candidate operations run serially,
keeping binding order and deduplicating shared operations. Historical commands are
never executed. Each command's actual failure remains in the result. This host
binds each affected production/test pair to formatting, the production binary
build and the dedicated test execution. The production build covers executable
entry points that dependency-only library compilation in a separate test project
does not cover; no redundant Cargo check or test-project build is scheduled.
Bootstrap builds the four execution tools before they exist; canonical build
results remain explicit checks of selected candidate executable sources.

For first adoption, the exact `adoption_base` permits only the absence of the base
CI profile, retaining and validating the base's real FILEMAP/projects registries.
Candidate bindings are projected onto existing base registrations; newly introduced
bindings do not invent base nodes. The report says `previous_enforcement: none`.
This repository explicitly adopts c4536e5c35d06302cdec655b3f11f9c257919155. An
unregistered older repository first needs honest registries; it cannot substitute
an empty tree or the candidate for the base. A new parentless host uses `--initial`.

## External judges and scripts

The runner sends one JSON `chrono-ci-judge/v1` request on stdin:
`{protocol, request_id, host_root, config_path, config_sha256, base, candidate,
initial}`. The response is one JSON object:
`{protocol, request_id, status, results, evidence}`. `status` is `passed`, `failed`,
`blocked` or `not-required`. `results` is nonempty, with unique nonempty IDs and
`{id, status, cause, exit_code}`; cause is nonempty and exit_code can be null for
nonprocess checks. A `not-required` result always has null `exit_code`; an executed
exit (including zero) cannot be labeled unexecuted. Passed process results require
zero and failed process results require nonzero. The aggregate is `not-required`
exactly when all results are `not-required`. Evidence must be a nonempty object. IDs/protocol, aggregate and
individual statuses and actual child exit must agree. Empty/extra/malformed output,
missing results, timeout, output overflow or crashes fail. Processes use bounded
capture and Unix process groups; the implementation is validated on macOS. Reports
preserve actual child stderr/stdout; a script can participate through an explicitly
registered interpreter. No plugin registry or alternate platform is implied.

## Owned GitHub projection

`.chrono-harness/ci/github.json` uses `schema: chrono-github-ci/v1` and explicit
`workflow_path`, `name`, `runs_on`, `push_branches`, `pull_request_branches`,
`branch_creation_base_ref`, SHA-pinned `checkout_action` and `upload_artifact_action`,
`timeout_minutes`, `bootstrap` argv, `runner`, `generator`, `check_config`,
`context_path`, and `artifact_directory`. Paths are host-relative. The current
provider supports one job and GitHub Actions only. Scripts, tool bootstrap,
runner label, event branches and job timeout are data; selected test commands
never appear in YAML. Runner command rendering uses the same canonical argv
constructor as event evidence.

`chrono-ci init --host-root H --config INPUT` accepts explicit source JSON from
any directory, adopts it into `H/.chrono-harness/ci/github.json`, and produces the
workflow. Existing adopted configuration is preserved. Init preflights output
ownership before adopting new data. `generate` renders deterministically, does
not rewrite identical output, and refuses unowned/symlink collisions. `verify`
uses the same renderer and never repairs drift. Only the single owned regular
workflow file is published; unrelated workflows are preserved. No concurrent
writer, multi-file crash transaction, or broad provider framework is promised.

Event mapping is preparation, not policy:

| Event | Fixed input behavior |
| --- | --- |
| Existing branch push | Payload `before` / `after` |
| Creation of the configured baseline branch (`created`, zero before, matching payload `ref`) | Parentless payload `after` uses the same check with `--initial`, no base, and `initial-inventory` mode; a parented candidate requires an explicit range |
| Creation of another branch (`created`, zero before, different payload `ref`) | Fetch configured baseline ref once, record its full base OID, use payload `after`; candidate equal to that existing baseline is an honest no-op |
| PR to dev | Event `pull_request.base.sha` / `head.sha`; check out head, never the default merge ref |
| workflow_dispatch | Explicit full base/candidate, or explicit initial plus parentless candidate |
| Branch deletion | Job has no candidate check and cannot produce a passing candidate report |

The generated workflow checks dev and integration/** pushes, PRs targeting dev,
and manual dispatch. It checks out the event candidate, bootstraps candidate
registered tools, calls `chrono-ci prepare`, invokes the canonical check, and
uploads evidence even on failure. Checkout's temporary read credential remains
available for fetching required objects; post-job checkout cleanup removes it.
Missing objects are fetched explicitly or fail. Manual dispatch availability can
depend on workflow registration on the default branch. Workflow source revision
is recorded independently with `github.workflow_sha`.
Creation events require a valid `refs/heads/...` payload ref. Creating the baseline
itself never treats the newly pushed candidate as its own prior baseline. The
context records source `baseline-creation-initial-inventory`, null base, and mode
`initial-inventory` for a parentless first push; other branch creations record
`branch-creation-baseline-ref` and `delta`. For a parented first baseline push,
supply real full base/candidate OIDs via manual dispatch or the canonical local
command; the ordinary creation route fails rather than fabricating a range.

The `ci.verify` registered operation checks drift, with explicit FILEMAP dependencies
on generator/config/workflow/bootstrap sources. Regeneration does not happen in CI
before this comparison. Workflow green is evidence of its actual selected checks;
it is not a branch-protection/ruleset guarantee. No human gate is introduced.

## Another host

[`examples/ci-host`](../examples/ci-host/README.md) is a complete small host bundle:
explicit FILEMAP, projects, check profile, provider config, install bootstrap,
Python check and input. It runs with installed/copied binaries and Python, without
a harness source checkout or inferred language/project paths. Its GitHub bootstrap
expects an explicitly configured installed-tools directory on a self-hosted runner;
a hosted runner needs an explicitly registered pinned install/download bootstrap.
No nonexistent prebuilt distribution or implicit network installation is assumed.

Copy the bundle, install the three binaries, configure the bootstrap source, run
init, then commit the initialized inventory. A copied `chrono-ci` binary works from
another cwd and a host path containing spaces. To add checks, register their real
operations, files/edges and profile bindings. To change runner/bootstrap/event data,
edit provider data and run generate. Workflow YAML remains a projection.

Behavioral coverage in the dedicated test projects includes real passing/failing
children, malformed protocol and timeouts, DELTA inside/outside closures, edge and
operation changes/deletions, unknown registration, exact snapshots, adoption,
initial inventory, no-op generation, drift and ownership collisions, preservation,
extension and event mapping. Existing instruction behavior remains covered by its
61 tests. Local tests do not certify native GitHub event routing; delivery must
verify actual candidate runs, selections, failure/success and PR head behavior
before claiming those external results.
