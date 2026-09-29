# Generated CI and the active CI slice

`runner`, `judge-registration`, `judge-filemap`, `judge-routes`, `judge-projects`, `judge-ci`, and `ci` are separate Rust production projects under
`crates/`, each beside its dedicated `-tests` project with separate Cargo.lock
and target directories. `instructions` and `instructions-tests` follow the same
layout. The container directory is not a Cargo workspace or a test-discovery rule;
each manifest, operation and dependency remains explicitly registered.
`runner` owns external process transport, request identity, strict protocol checks
and result publication. `judge-registration` owns full-format registration checks.
`judge-filemap` owns full typed declaration impact and shared union/closure mechanics.
`judge-routes` owns operation planning, tool binding and receipt comparison;
`judge-projects` owns affected pairs and actual execution results.
`judge-ci` owns the scoped CI registration, snapshot, seed and operation policy,
and consumes those shared mechanics through an explicit adapter. Its
`selection_explanation.extra_selections` explains legacy rules, with
`legacy_only_selections` identifying selections outside test-execution edges;
these do not become full-policy edges. See [the impact contract](filemap-impact.md).
`ci` owns the GitHub workflow projection and event input
preparation. It does not select tests or judge outcomes. `instructions` remains
independent and its catalogs, layouts and relative root alias are unchanged.

The five full-governance registries remain **proposed**. The bounded
`chrono-ci-check/v1` profile explicitly consumes the FILEMAP and projects data
below using `chrono-ci-judge/v1`. Separately, `check` supports configured
`chrono-judge/v1` transport and the registration/filemap/routes/projects judges through
`.chrono-harness/config.json` with `--context`. This proposed host remains
incomplete and returns nonzero; workflow has [bounded certification](workflow.md); automated lifecycle remains unimplemented. Mixed classification and warning behavior have a dedicated judge and real-chain tests; see [mixed](mixed.md). Cost reports have a dedicated judge and registered consumer tests. Routes/projects now supply the shared operation planner, executor and receipt comparison; see [execution](execution.md).

## This repository

Run from the repository root. This macOS host explicitly binds `/usr/bin/python3` with the exact `Python 3.9.6` contract for bootstrap and migration (also in the scoped tool map). Bootstrap requires that interpreter, Git, and rustup; it
installs Rust 1.95.0 if unavailable, independently ensures its rustfmt component
(including on a preinstalled minimal toolchain), and builds the explicitly listed bootstrap
operations using `.chrono-harness/projects.json`, then copies declared binaries.
It does not duplicate Cargo build recipes or discover projects.
Toolchain operations name the configured version explicitly and do not change
the global default toolchain. Bootstrap these tools before the dedicated CI tests;
their copied-host regressions consume the installed candidate binaries.

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py .
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
Event preparation and initial judges read original commit headers with Git replacement
overlays disabled. A same-tree child cannot become an initial root through a
`git replace` view or a shallow-history boundary. The explicit full-registry
initial profile is documented in [execution.md](execution.md#initial-inventory);
its inventory completion does not activate full governance.

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
| schema | `chrono-ci-check/v1` or explicit Git-binding `chrono-ci-check/v2` |
| judge | `{program, args, env?, timeout_seconds, output_limit_bytes}`; explicit executable/interpreter plus argv; positive bounds, at most 64 MiB per captured stream |
| report_path | Safe relative file beneath `.chrono-harness/state/`, also covered by a declared artifact directory |
| policy.filemap / projects | Explicit relative paths; FILEMAP v1 historical/v2 current, projects v1; registry path migration across an enforced range is unsupported |
| policy.tools | Tool ID → executable path or PATH command; legacy scoped resolution; current registration_config supplies actual version declarations; selected tools bind once per plan |
| policy.bindings | Historical FILEMAP v1 only; current v2 rejects duplicate bindings and consumes FILEMAP.execution_plans |
| policy.registration_config | Optional full config path; current host uses its candidate historical decoder and tool declarations |
| policy.facts_config | Required by scoped v2: literal path under `.chrono-harness/` selecting candidate full-v3 Git binding; v1 rejects the field, including null |
| policy.environment | Explicit environment overrides passed to operations; this host sets `RUSTUP_TOOLCHAIN=1.95.0` |
| policy.artifacts | Relative directory prefixes ending `/`; ignored execution products may exist there; tracked files may not overlap them |
| policy.required_inputs | Explicit regular tracked inputs required by this profile |
| policy.adoption_base | Null or exact full base OID whose missing prior CI profile is deliberately accepted |
| policy.operation_timeout_seconds / operation_output_limit_bytes | Positive serial operation process bounds |

Scoped v2 changes Git acquisition, preserving the selection, execution and
operation-environment contracts below. Its explicit `facts_config` binds the Git
executable, file digest, exact version, environment and per-process bounds using
the [full-v3 reader](git-facts.md). The candidate binding reads both endpoints;
the binding config and check profile must match their fixed candidate blobs.
There is no ambient fallback. Git binding alone does not require full registry
adoption: `registration_config` independently opts into the existing full
interpretation and observed canonical-invocation checks. When supplied under v2,
that interpreter reuses the same reader, including its recorded processes.

Successful and failed scoped v2 responses carry `evidence.git_facts` once the
binding is constructed, including failed version execution or version mismatch.
The record contains original stdout/stderr bytes and hashes, actual exits and
timeout/output-bound failures. Pre-binding validation cannot invent processes.
`previous_enforcement` names the actual base profile version, or `none` for
initial inventory/explicit adoption; reading a v1 base never executes its Git.
The canonical local/CI command and scoped judge protocol are unchanged. V1
retains its prior acquisition behavior; existing hosts are not auto-upgraded.
The product host still uses scoped v1. Scoped v2 is not included in beta.8 and
does not establish complete input closure, full governance or deterministic parity.

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

Full governance has a separate explicit dispatch schema,
[`chrono-github-full-ci/v1`](full-ci.md), preserving caller context bytes and
using the same full runner command locally and in CI. The event profiles below
retain their scoped semantics; adoption never silently upgrades them.

### Registered event Git

The source implementation accepts `chrono-github-ci/v3` with an explicit
`facts_config: ".chrono-harness/<registered-config>.json"`. That file must use
full config v3 and declare the [Git tool/input binding](git-facts.md). It must be
a policy input outside the artifact directory, distinct from generated initial
profiles and executable/context outputs. Enroll it and its actual consumers in
FILEMAP; the generator does not infer dependencies or create the Git declaration.
This capability is not part of public beta.8.

V3 retains the existing event and canonical check command construction. The
optional `initial_inventory` declaration keeps its existing projection semantics;
omission retains the single check profile. V1/v2 reject `facts_config`, including
null, and preserve their prior Git and initial-profile contracts. Adoption is
explicit; `init` preserves an existing source's customization and version.

Before event network observation, the selected Git checks the binding config's
original bytes against the fixed candidate and checks that HEAD is that candidate.
All v3 event reads, remote baseline observations, object probes, fetches and
initial parent reads use this same candidate binding with its declared environment
and per-process bounds. The event library still consumes the caller-supplied
`Config` and payload; it does not certify their provider provenance or that the CI
source itself matches the candidate. Generated workflow and event identity must
be verified by the actual delivery consumer.

The batch object probe sends one full OID on stdin. Only an exact successful
`OID commit` response establishes presence; only `OID missing` permits an explicit
fetch from `origin`. Nonzero exit, timeout, overflow, wrong type and malformed or
extra output are errors. Successful fetch is followed by another probe and commit
identity verification. This avoids interpreting diagnostic text or any arbitrary
probe failure as missing history. The old versions keep their legacy probe path.

Successful `chrono-ci-inputs/v1` contexts add `git_facts` observations. Once bound,
errors retain them in `E_CI_GIT` diagnostics; failures during binding keep the
reader's `E_GIT_FACTS` diagnostic where available. Each actual process retains
argv, stdin digest, original output bytes/digests, exit and bound failure. No
unstarted process or successful context is invented. Failure does not overwrite
an old context file; the caller must use the invocation's actual exit status.

The dedicated tests use actual local remotes and a selected Git entry, including
an actual CLI run from another cwd with ambient Git shadowed. They cover missing
history, failed/malformed probes, failed fetch, configured integration baselines,
initial/manual/PR inputs, source/checkout drift and old-version controls.
Native tests of these fixtures do not establish generated-v3 host event adoption.
The product host still uses v1 event preparation and its scoped judge. Network,
credentials, Git configuration, helpers, OS and delegated input closure remain
unverified; repository-configured implicit Git behavior is not disabled or made
complete by this binding. Full activation, host adoption of scoped v2 and deterministic
local/CI parity remain separate obligations.

`.chrono-harness/ci/github.json` uses `schema: chrono-github-ci/v1` and explicit
`workflow_path`, `name`, `runs_on`, `push_branches`, `pull_request_branches`,
`branch_creation_base_ref`, SHA-pinned `checkout_action` and `upload_artifact_action`,
`timeout_minutes`, `bootstrap` argv, `runner`, `generator`, `check_config`,
`context_path`, and `artifact_directory`. Paths are host-relative. The current
provider supports one job and GitHub Actions only. Scripts, tool bootstrap,
runner label, event branches and job timeout are data; selected test commands
never appear in YAML. Runner command rendering uses the same canonical argv
constructor as event evidence.

For a separate root inventory, opt in to `chrono-github-ci/v2` and add the
following explicit declaration to the same source (supply the actual selected
judge digest). The remaining provider fields retain their v1 meanings:

```json
"initial_inventory": {
  "path": ".chrono-harness/ci/initial.json",
  "profile": {
    "schema": "chrono-initial-check/v1",
    "host_config": ".chrono-harness/config.json",
    "timeout_seconds": 30,
    "stdout_limit_bytes": 1048576,
    "judges": [{
      "id": "initial-registration",
      "executable": ".chrono-harness/bin/chrono-judge-registration",
      "version": "0.1.0",
      "sha256": "<actual installed binary SHA-256>",
      "argv": ["--protocol", "chrono-initial-judge/v1"],
      "selector": "every-initial",
      "modes": ["inventory"],
      "after": []
    }]
  }
}
```

`init` and `generate` assemble this declaration into the registered JSON profile
and generate the workflow. `verify` checks both outputs without modifying either.
The source declaration owns its profile output: edit the source, then generate;
do not maintain an independent profile policy. Initial event preparation and the
workflow's initial branch select this exact profile through the same canonical
argv constructor. Ordinary DELTA checks continue to use `check_config`. V1 rejects
the extension and retains its original single-profile behavior; v2 requires it.
Neither version nor a null field silently selects a fallback.

The profile uses the runner's actual validator for schemas, bindings, digests and
DAGs. No judge IDs, binary identities, host language or layout are discovered.
The output is a separate path under `.chrono-harness/ci/`, outside artifact paths
and distinct from source, normal check, host config and executable paths. Source
and output must both be explicitly enrolled in the host FILEMAP. The generator
does not inspect or synthesize the five host registries or certify their content.

New adoption preflights both outputs and rejects a different existing profile;
an exact existing projection is reusable. Existing adopted source customization
is preserved. Once enrolled, generation replaces profile drift according to that
source; workflow marker and all symlink checks still apply. Changing/removing an
output declaration does not delete the old file: the AI explicitly retires its
old registration and artifact. Existing v1 source is never automatically upgraded.

Dedicated tests exercise the generated profile with actual runner/registration
processes on a committed parentless host, including a rejected unregistered file,
spaced paths, unrelated cwd and stored/stdout report identity. The public
[initial-host example](https://github.com/ChronoAIProject/chrono-harness-examples-initial)
also installs beta.5 and exercises a real parentless first push with generated v2
CI, followed by ordinary documentation DELTA checks on integration, PR and dev.
It explicitly registers macOS arm64 / `macos-14` and that platform's initial
judge digest. See the [example index](examples.md) for the actual evidence and
scope. This does not establish cross-platform initial profiles, complete
bootstrap provenance, full input closure, governance activation or deterministic
local/CI parity.

Optional `push_baselines` rows explicitly bind a literal branch-ref prefix to a
baseline branch on `origin`. This host and the copyable example adopt:

```json
"push_baselines": [
  {"ref_prefix": "refs/heads/integration/", "base_ref": "refs/heads/dev"}
]
```

A matching creation or subsequent push checks the complete candidate against the
observed baseline tip, including earlier commits on the same integration branch.
The prefix is a literal string ending in `/`, not a glob. Overlapping/duplicate
prefixes and a baseline inside its own matching prefix are errors. With configured
rows, push payloads must include a valid full branch ref. An absent/empty list
preserves existing push behavior; unmatched branches still use the ordinary event
mapping. No branch role is inferred from its name, host layout or language.

The preparer reads the exact named remote ref once and binds its full commit OID.
If needed it fetches that OID; it does not subsequently take identity from shared
`FETCH_HEAD`. Missing refs/objects or malformed events fail without falling back
to the payload's previous commit. Context source `push-configured-baseline-ref`
identifies this choice. Preparation binds inputs; freshness, stability and
integration certification remain the workflow judge's separate contract.

`chrono-ci init --host-root H --config INPUT` accepts explicit source JSON from
any directory, adopts it into `H/.chrono-harness/ci/github.json`, and produces the
workflow. Existing adopted configuration is preserved. Init preflights output
ownership before adopting new data. `generate` renders deterministically, does
not rewrite identical output, and refuses unowned/symlink collisions. `verify`
uses the same renderer and never repairs drift. V1 publishes the single owned regular workflow file; unrelated workflows are
preserved. No concurrent writer, multi-file crash transaction, or broad provider
framework is promised. An IO failure can leave some generated outputs published;
it remains an error and a later generation must reconcile the declared outputs.

Event mapping is preparation, not policy:

| Event | Fixed input behavior |
| --- | --- |
| Push matching a `push_baselines` prefix | Observed configured baseline ref / payload `after`, on creation and every subsequent push |
| Unmatched existing branch push | Payload `before` / `after` |
| Creation of the configured baseline branch (`created`, zero before, matching payload `ref`) | Parentless payload `after` uses the same check with `--initial`, no base, and `initial-inventory` mode; a parented candidate requires an explicit range |
| Unmatched creation of another branch (`created`, zero before, different payload `ref`) | Observe configured creation baseline ref once, obtain its fixed commit, use payload `after`; candidate equal to that existing baseline is an honest no-op |
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
`initial-inventory` for a parentless first push; other unmatched branch creations record
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

The initial registered-execution native run [36292159644](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36292159644)
failed on macos-14-arm64 at the canonical harness step after successful bootstrap.
Base was `4f08aef7ab40d7b0a3fb6ba42af2620600f18d98`; candidate and workflow source
were `556fdb73b6f93275d18bd210b9b01be1173daec9`. The failure was
`E_TOOL_BINDING: migration version mismatch`; no governed operations executed.
That report did not retain the interpreter's actual version output. The repaired
host uses the explicit system-interpreter path instead of PATH precedence, keeping
the exact version contract. `/usr/bin/python3 --version` returned `Python 3.9.6`
on the local Darwin arm64 verification host. The repaired integration, PR and dev
runs also observed that exact path and version. The verified [dev run 36295299904](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36295299904)
checked base `4f08aef7ab40d7b0a3fb6ba42af2620600f18d98` to landed candidate
`f8338fd9496878914216e2c070d8320ea50fbb6e`: 34 operations and 227 Rust tests
passed. Its tree equals the independently reviewed candidate tree; full parity
remains unestablished.
Failed migration binding now preserves expected and observed version diagnostics.

## Independent workflows

The source unit extension assigns complete plans explicitly and generates one
workflow per unit plus an offline report collection workflow. Local and native
unit execution use the same `check --unit` command. See [CI units](ci-units.md)
for the v3 profile, sharing, collection, provider contracts and adoption boundary.
