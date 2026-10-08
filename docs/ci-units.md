# Independent CI units

`chrono-ci-check/v3` explicitly assigns every registered test plan to one unit.
`chrono-github-units/v1` (scoped) and `/v2` (full) accept an explicit optional
`job_gating` registration. The current product host adopts it: one detector,
independent conditional unit jobs, and an always-run aggregate in one parent
workflow. Each unit retains its own checkout, name, runner, bootstrap, timeout,
context, upload and job rerun. Host languages and layouts remain opaque.

The product host registers an explicit inventory of conditional units. Its existing
workflow test project has three explicitly registered test groups, assigned to
the units `judge-workflow` (core),
`judge-workflow-units` (direct full-unit tests), and `judge-workflow-short`
(full short-command tests). Registered actions select each group's members;
the inventory guard verifies their coverage without inferring dependencies.
Together, the groups cover the two original test binaries and preserve their
fixtures, profiles and business limits. `actions.execute` and its original
`test.judge-workflow-tests` operation remain unfiltered for release. The legacy
test identity binds a distinct filtered core action through `test_groups`.

Each group retains the registered build prerequisites and runs the host-owned
`.chrono-harness/ci/workflow-inventory.py` guard. Its explicit config binds the
original action, group map, Cargo tool and binary lists. Real unfiltered, ignored
and filtered libtest listings must form nonempty disjoint exact coverage;
failed listings retain their original exit and output. The guard compares current
sets rather than fixing the test count forever. The migration's historical name
baseline remains a separate implementation observation. Shared prerequisites and inventory
operations are explicitly assigned to all three units. Each has a distinct context
and check report; the existing detector and always-run aggregate remain the native
entry and sole required branch-protection check. The grouping has no native timing
or activation acceptance until the caller runs the committed candidate there.

The worktree test project has separate `worktree` and `worktree-adoption` units.
Its adoption action explicitly names six host-entry integration tests for the
participating check, detached Python consumer, short check, registered Python
consumers, native Cargo child and standalone bootstrap. The core action excludes
those same names. Both retain their original test IDs, assertions, deadlines and
default libtest threads. The unfiltered action remains the release verifier.
`worktree.inventory` uses the same independent Python inventory owner with
`.chrono-harness/ci/worktree-inventory.json` to require nonempty, disjoint,
complete coverage of the current unfiltered test list. Both groups retain the
same build prerequisites and declare their shared target and inventory resource;
their local operations cannot write that target concurrently. Independent native
jobs retain separate checkouts, reports and reruns. This grouping does not certify
that every host can satisfy a timing bound under arbitrary external load.

The projects test project has separate `judge-projects` and
`judge-projects-migration` units. The core action runs the `consumer`, `execution`
and `languages` binaries; the migration action runs the `migration` binary,
including the real historical-host repair and its nested CI suite. Both retain
the existing assertions, fixture deadlines and default libtest concurrency.
`projects.inventory` uses the same Python owner and
`.chrono-harness/ci/projects-inventory.json` to compare their union with the
current unfiltered inventory. New or omitted binaries fail inventory validation;
the registration does not freeze a historical test count.

Each group has its own 900-second operation bound, checkout, check report and
rerun; this replaces the single combined operation in ordinary CI. It does not
claim the same aggregate time budget or a reduction in total work. The original
unfiltered `execute` action remains available for release, and nested tests are
not omitted or replaced by stored results. The groups share their declared
target and inventory resource in a local checkout, so local execution remains
exclusive. Native timing and complete candidate acceptance require the actual
registered checks.

Providers without `job_gating` retain their original separate-workflow behavior
and evidence contracts. Installing a new binary or running init does not opt
an existing host in. This source correction has no published release or native
Actions verification claim; earlier beta.12–19 evidence keeps its original scope.

The repository's schema4 contract uses fixed `check`, `check --unit ID`, and
value-less `check --collect`. Bare check keeps global DELTA execution. The local
worktree producer freezes registered remote target/HEAD endpoints; the existing
CI owner creates the collection manifest from explicit unit report paths and
independently observed runner/judge pins. Local collection asks the adjudicator's
shared inventory/assignment owner for DELTA-required units and reads only those
registered paths. An absent or stale unrelated report is not supplied; required
missing, stale or failed reports still fail. Empty DELTA requires no reports.
Missing units are never executed by
collection. Prior producer observations remain retained while declared current
outputs are replaced; no crash/concurrent transaction guarantee is established.
Provider v4 puts event preparation and native gathering inside the same short
command. Older hosts/examples below retain their registered explicit v3 forms.
Full independent short scopes use registered full-v3/v4 execution units and the same seven-judge DAG.

Short preparation binds the selected native provider source bytes and its
`artifact_directory`. Each unit retains context, payload, producer evidence,
acquisition receipts and immutable check reports beneath its own selected root;
collection also retains original gather/manifest evidence in its root. Originals
are addressed by path and SHA-256 and reused, rather than copied inline into every
receipt. Mutable current outputs remain projections. Provider/upload mismatches,
missing originals and changed original bytes fail. The upload step keeps its
selected directory; it need not upload unrelated units.

Native gather adds an `artifacts` object to each short report manifest input:
`source_directory` is the producing unit's selected upload root and `directory`
is its explicit downloaded root. The collector resolves original relative paths
through that binding, preserving the observed producing checkout/executable
identities. No directory discovery or recursive evidence inference is used.
Historical explicit reports retain their existing manifest contract.

Full-v3/v4 hosts use the same explicit unit map and selector suffix through the
native seven-judge entry. Their unit report manifests may use
`chrono-full-collection/v1` (or the compatible `chrono-ci-collection/v1` shape)
with `unit`, `path`, and `sha256`, optional explicit executable pins and native `artifacts` mappings; runner and seven-judge pins are derived
from the fixed candidate registry and bound invocation. The full collector
retains a reconstructible sealed request template, exact process/response bytes,
and addressed original context/input/evidence bytes. It validates every imported
judge and receipt against the current declaration contract, without launching
business tools or their version commands. Original unit roots and business
executables may be unavailable. The completed collector report retains the
original evidence closure for subsequent integration certificate consumption.
Both integration and delivery contexts support contribution-only units without
requiring a prior completed certificate; only collection resolves global
completion. Full-v3 report bounds apply to report JSON and the manifest;
external retained blobs stream under their declared upload roots, including the
full nested collector closure. Legacy inline originals remain supported.
Missing, replaced, failed, stale or contradictory evidence rejects.
Ordinary unscoped full check retains local blob references and validates their
identities by streaming; the portable unit/report bounds do not limit those input
files. Provider generation and artifact gathering remain owned by `chrono-ci`;
actual native full host activation remains pending.

Full short preparation preserves the exact local context through collection.
Local manifest production asks the shared FILEMAP/routes obligation owners for
required units and binds all seven judge executables. Registration consumes the
explicit original report inputs for any historical conversion before the final
required-unit manifest is published; it does not rerun the decoder. Original short acquisition
receipts and addressed bytes are retained in full portable evidence.

Local scoped preparation reuses an existing context only when its endpoints,
origin role, birth association and retained-input reference still match the
current producer facts and a fresh producer observation still satisfies the
workflow-owned age predicate. While valid, its original `observed_at` stays pinned
so independent reports bind one exact context. After expiry the same commands
retain the original and publish a new immutable observation; old contributions
cannot complete the new context and workflow rejects the stale branch. A bare
check produces a fresh observation. This refresh does not reconstruct the branch.

Registration/input owns the file-input projection used by live unit validation,
`chrono-inputs capture --unit ID` and collection comparison. It uses the unit's
explicit registered plans and operation owners, FILEMAP prerequisites and closure
bindings, including shared operations and governance consumers. Structural and
reference checks remain global. A missing unrelated SDK need not be retained or
read live for that unit; selected/shared/governance omissions, drift, malformed
snapshots and unknown inputs fail. Collection validates original per-unit blobs
and compares each unit's effective projection against current obligations without
launching business operations or their version tools. Original complete snapshots
remain supported. Native Go/TS execution and full host activation remain unverified.

`chrono-github-units/v2` connects full scopes to the existing automatic push/PR
provider. V1 remains scoped and rejects the new field. V2 keeps the existing
collection, units and gather contracts, requires collection schema
`chrono-github-ci/v4`, and adds one explicit mapping:

```json
"full_contexts": {
  "collection": ".chrono-harness/state/collection/full.json",
  "units": {
    "service": ".chrono-harness/state/service/full.json",
    "client": ".chrono-harness/state/client/full.json"
  }
}
```

Unit keys must equal the full profile and workflow keys exactly. Each mapped file
is a separate input beneath its selected upload directory. The existing bootstrap
supplies exact schema2 context and retained inputs. Preparation compares its
base/candidate with actual event endpoints. Gathering compares full context
digests and all seven judge pins, retains workflow/attempt evidence, and writes
`chrono-full-collection/v1` with explicit source/download upload mappings. Failed
runs remain failures with downloaded originals retained. Each workflow builds and
retries independently; collection runs no business or business version commands.
The single registered CI producer `--config` selects this map for all short
invocations. No role, birth time, certificate or layout is inferred.

The product host uses scoped V1 with explicit job gating. Clean-candidate admission,
actual native full adoption, public binary publication and full SPEC acceptance
remain separate obligations.

## Optional native provisioning extension

Full units/v2 using the conditional parent can explicitly select `native_adoption` with schema
`chrono-native-adoption/v1`. Omitting it preserves existing generation and
projection behavior. A compatible released generator is required before adoption.
The product owns the embedded `assets/ci/native.py` template and owned output
updates. The host owns these extension values and its registered operations:

```json
"native_adoption": {
  "schema": "chrono-native-adoption/v1",
  "lineage": {"path": ".chrono-harness/ci/lineage.json", "sha256": "ORIGINAL_BIRTH_SHA256"},
  "adapter_path": ".chrono-harness/ci/native.py",
  "interpreter": "/usr/bin/python3",
  "inputs_program": ".chrono-harness/bin/chrono-inputs",
  "seed_directory": ".chrono-harness/state/shared-seed/",
  "seed_artifact": "chrono-context",
  "retained_inputs": ".chrono-harness/state/inputs.json",
  "composition_sources": [{"path": ".chrono-harness/state/inputs/base-capture.json", "sha256": "ORIGINAL_BASE_CAPTURE_SHA256"}],
  "push_roles": {"refs/heads/integration/": "integration", "refs/heads/dev": "delivery"},
  "pull_request_role": "delivery",
  "integration_evidence": null
}
```

Before the candidate commit the caller tracks and registers unchanged original
successful `start`/`reconstruct` report bytes as `lineage`. The native consumer
checks their digest, original branch/fork/time and event association; it does not
read destination origin or manufacture historical birth. `prepare-endpoints`
exposes the existing full-policy event preparation before full-context loading.
Detection delegates it, observes one actual UTC time, retains the
authentic event/payload/revision/repository records and publishes schema2 context.
The detector imports original base governance observations from the explicitly
addressed `composition_sources` snapshots or endpoint pairs, then captures current
candidate governance inputs through `chrono-inputs capture-governance`. The inputs
owner validates fixed endpoint/configuration, presence and streamed blob identities,
and retains source bytes and a provenance receipt in the seed closure. A base
snapshot can be captured before the candidate exists; its digest can then be bound
in the provider without a circular candidate identity. This supports an H0-to-H1
change at the same input location. Missing, corrupt or misbound originals and
drifted current governance inputs fail.
This does not claim a past business execution. Generation uploads the seed before
dependent jobs start. Units and the later aggregate acquire the unique original
detector run/production-attempt/digest-bound seed without polling, retaining its
original bytes and producer closure. Missing, ambiguous or misbound acquisition fails. Transport checks observation
ordering. The CI producer records a genuine current observation separately from
the original context; the workflow judge alone applies the unchanged registered
age bound for both consumer and certificate producer. Local preparation uses the
same separate current-observation contract when it reuses a retained context.
Original context digests and successful reports remain unchanged. Acquisition jobs
have `actions: read`; the generated acquisition steps receive the actual
`CHRONO_WORKFLOW_REVISION`. Bootstrap receives the host's registered argv. A hook
that needs that revision must explicitly include the argv prefix
`["env", "CHRONO_WORKFLOW_REVISION=${{ github.workflow_sha }}", ...]` in its bootstrap
registration; generation preserves this host choice.

Register the adapter as the sole `canonical_check.inputs.ci` action using an
existing interpreter tool, with argv `[adapter_path, "forward", "--config",
PROVIDER_PATH]`. Exactly one provider `--config` pair binds the selected upload
contract. Remove acquisition-only `GITHUB_EVENT_NAME`, `GITHUB_EVENT_PATH`,
`CHRONO_WORKFLOW_REVISION` and `GITHUB_REPOSITORY` from business inherit; retain
`CHRONO_CHECK_SOURCE`, actual business variables and the existing credential
contract. Forwarding reads authentic retained acquisition records and passes their
actual named-env values to `chrono-ci check-inputs`. An internal CI bridge delegates
bounded child execution to the existing Rust transport and retains child stdin,
stdout, stderr, executable identity and process result. It is needed because
acquisition event fields and credentials are outside business inheritance; the
adapter does not contain a second subprocess runner. The outer producer
receipt continues to describe its actual environment. Collection forwards gather
then invokes the inputs owner's composition before returning to full execution.
The original detector governance pair and selected unit contributions supply
collection inputs. `composition_sources` supplies original base snapshots or pairs
to seed preparation. Collection consumes their validated governance projection
from the transported seed, whose provenance retains the source bytes; the detector's
original source paths need not remain live in other checkouts.
FILEMAP/routes remains the DELTA selection owner; registration projects its
selected units through both endpoint assignments to check collection coverage.
Unselected units need neither SDK snapshots nor synthetic reports. Collection
performs no SDK installation or business probe.

Optional `integration_evidence_path` selects original transported certificate
bytes for delivery publication; their actual SHA-256 becomes the context binding.
Missing bytes leave the original null value for the workflow's impact decision.
An optional declared `integration_evidence` digest must match supplied bytes.
Optional `integration_transport: {source_directory, directory}` binds the
producer's original upload root to its delivery download root, including the
finalized report and nested blobs. The host bootstrap acquires that explicit
completed certificate/report closure; this extension does not add a scheduler.
Workflow retains all nine certificate bindings and requires the matching
finalized producer when impact selects integration. Ordinary feature preparation
can carry null evidence; only the existing workflow decides whether it suffices.

Register lineage, adapter, interpreter, inputs binary, bootstrap inputs, provider
and composition originals in the host FILEMAP/input/dependency declarations, and
use disjoint seed/unit/collector/download roots. Existing owned output preflight
preserves customized host source and refuses unowned overwrites. Main and examples
are not activated by these source contracts; public release and real native
push/PR/dev adoption remain caller-owned.

## Conditional jobs and the required aggregate

The parent workflow replaces superseded runs of the same pull request through
native workflow concurrency. Its group binds the workflow path, event and PR
number; events without a PR use their unique run ID. Push runs therefore retain
their separate complete before/after obligations instead of losing queued
DELTAs. Unit jobs remain independently scheduled inside each parent.

This host runs development-branch checks through pull requests to `dev`, and
checks landed changes on pushes to `dev`. An integration branch is verified by
its PR run; it does not also launch the same unit suite on every branch push.
Hosts continue to own their explicit trigger branches.

The smallest projection is one parent at `collection.workflow_path`. Require only
its aggregate job (`collection.name`, currently **chrono / collection** in this
host) in branch protection. The generator never mutates branch protection.
`detect` publishes one Boolean output per registered unit. Each independent unit
has `needs: detect` and a job-level `if` on its Boolean; an irrelevant job allocates
no runner and executes no bootstrap or SDK setup. `aggregate` has `if: always()`
and needs the detector and every unit. No workflow-level `paths` or `paths-ignore`
is emitted. No matrix fail-fast couples unit failures.

The host adds just this optional provider block, with its own tool acquisition:

```json
"job_gating": {
  "schema": "chrono-job-gating/v1",
  "detector": {
    "runs_on": "macos-26",
    "timeout_minutes": 45,
    "bootstrap": ["/usr/bin/python3", ".chrono-harness/ci/bootstrap.py", ".", ".chrono-harness/ci/bootstrap-detector.json"],
    "sparse_checkout": [".chrono-harness/", "crates/", "assets/", ".github/workflows/chrono-ci.yml", ".gitignore"]
  }
}
```

This form requires collection provider v4. The generated internal `chrono-ci
detect --host-root . --config SOURCE --github-output "$GITHUB_OUTPUT"` command
reads the event and native bindings itself; it adds no daily flags or manual
preparation. All actual checks remain `chrono-harness check`, `check --unit ID`,
and `check --collect`. Short collection owns both native transport/status rejection
and the existing final judge admission. A successful GitHub status alone cannot
supply a successful original report. No synthetic reports are made for skipped
units. Empty DELTA still reaches structural/global checks with an empty manifest
and zero business operations.

Scoped detection and local collection share judge-ci's inventory, two-endpoint
impact and `units::required` owner. Scheduling skips checkout-materialization
admission, while actual checks still enforce their clean candidate and canonical
entry contracts. Registration's scheduling entry shares historical profile
matching and rejects decoder-dependent history before acquiring an interpreter
or executing conversion; actual check/collection conversion admission is preserved.
Detection reads complete Git trees and registered registry blobs;
it does not execute business tests or business version probes. Only declared
product-source inputs need materialization for a source bootstrap. The sparse
paths above are this Rust product host's choices; a released-binary consumer can
materialize just its host registration/installer/workflow paths. The detector
never discovers host SDKs, imports, directories or language boundaries.

This host's detector produces the registered core startup tools for the unit and
collection jobs. Its cache preparation builds `chrono-cache`; its bootstrap calls
the existing core profile once and publishes all five installed tools. Consumer
jobs validate and install the selected profiles, then verify them at the original
cache/core preparation points. Their canonical checks and business build plans
remain unchanged. Local and release bootstraps keep their registered source builds.

Optional `job_gating.startup` uses schema `chrono-shared-startup/v1`. It declares a
literal host `directory`, artifact name, pinned `download_action`, and `consumers`
keyed by explicit job IDs. Each consumer supplies nonempty `need`, `output_use` and
literal `install` argv. The existing detector bootstrap is the sole producer and
writes an opaque `binding` to `GITHUB_OUTPUT`; no additional job is generated.
The provider uploads the declared directory before detection, and carries the
producer binding and actual upload artifact ID as detector outputs. Consumers
download that ID, including when only failed jobs are rerun in a later attempt.
Empty artifact IDs cannot trigger download-all. The installer must reject a missing
binding or missing transfer; it has no success exemption. Transfer directories must
not overlap evidence roots, preventing binaries from being copied into every
ordinary evidence archive. Unlisted consumers and configurations without startup
keep their existing behavior. Hosts own compatibility and install checks; the
provider infers no language, layout or platform compatibility.

The host Python implementation binds a clean primary source commit/tree, platform,
configuration/profile bytes, original bootstrap reports, and file digests, sizes
and executable modes. It validates the complete transfer before replacing tools,
restores executable modes lost by artifact transport, and retains exact producer
reports beside explicit import provenance. Installation is not a local build.
Point-of-use verification rejects installed drift. Missing/changed bindings,
source/config/platform mismatches, symlinks, damaged payloads and failed publication
stop the job. This covers registered startup outputs, not complete compiler/SDK
input closure, adversarial attestation, linked-worktree ownership or power-loss
atomicity across all installed files. Native transfer and rerun acceptance remain
separate from fixture validation. Startup upload/download/install steps have
explicit resource categories; their observed costs count toward evaluating reuse.

PR endpoints are event base/head. Default push endpoints are the complete event
before/after, including multiple commits. Explicit `push_baselines` and branch
creation rules keep their existing meanings; the detector observes a configured
remote baseline once and its fixed output supplies the dependent jobs. Acquisition
starts with blobless depth-two checkout/fetch where sufficient and explicitly
fetches missing endpoint OIDs using `--depth=2 --filter=blob:none`. Depth two is
not a general history guarantee. Full unit/aggregate jobs retain their existing
complete history acquisition, with blob filtering, for the branch/fork judge;
their detector still reads exact endpoint trees from shallow acquisition.
Missing/unobtainable endpoints, parented initial
baseline creation, deletion without a candidate, and malformed registrations fail;
there is no HEAD^, merge-base or select-all fallback. Rename is the deleted and
added registered endpoint paths, preserving both dependency closures.

The changed-file count comes from complete Git trees before selection. There is
no file-count gate or truncation.
GitHub documents that native workflow path filtering
has a [3000-file limit](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#git-diff-comparisons),
the [PR files API](https://docs.github.com/en/rest/pulls/pulls#list-pull-requests-files)
returns at most 3,000, and the
[compare API](https://docs.github.com/en/rest/commits/commits#compare-two-commits)
returns at most 300 changed files for the comparison. None supplies this detector's
file list or defines a universal runtime cap.

The Rust provider rejects failed/cancelled/missing detection and every selected
unit whose result is failure, cancelled, missing or unexpectedly skipped. A skip
is accepted only for a detector-declared nonrequired unit. Gather validates the
exact parent run/repository/current attempt/event/workflow source, then reads its
paginated job history after dependencies have completed. It never polls separate
workflows or waits for its own parent to finish. Independent retries retain the
original detector and successful prerequisite attempts. Nonrequired skips are
bound by `needs` and need neither job execution metadata nor artifacts. Each selected unit must have a successful latest API status in the same parent
run. Its generated `production` output addresses the actual original artifact,
context digest, report digest and producing attempt. API job identities and
`run_attempt` can describe carried work and never manufacture artifact names.
Gather checks the original context/run/attempt/job and both digests against that
output. A later failed/skipped execution cannot be replaced
by an older success. Parent-attempt drift and mismatched/missing artifacts fail.
Original process bytes, source SHA/revision, event/endpoints, report digests,
executable pins and upload/source-directory mappings remain bound and judged.

Scoped providers register `CHRONO_CI_DETECTION`, `CHRONO_CI_NEEDS`,
`GITHUB_RUN_ID`, `GITHUB_RUN_ATTEMPT` and `GITHUB_JOB` in the producer environment.
The optional full native adapter instead records their actual acquisition values
and forwards them outside business inheritance, preserving differences between
independent jobs. The generated aggregate supplies its exact `needs` object; the
provider owns the acceptance predicate in Rust. YAML and bootstrap Python contain
no second whitelist or status/selection algorithm.

New v4 unit initialization chooses the conditional parent when no existing
provider has adopted a topology. Existing explicit legacy provider meanings and
customization remain preserved.

Full-v2 scheduling reuses pure FILEMAP declaration impact, fixed semantic-document
reads, routes `global_selection`/assignment validation and `units::required` before
SDK snapshots exist. It does not provide a schema2 full context, original birth
receipt, input snapshots, streamed artifacts or historical conversion observations.
Final full collection still validates actual effective inputs and seven-judge
originals. An external-input change that introduces additional final obligations
rejects missing evidence; declaration scheduling does not claim those snapshots
are known. Historical registries needing a decoder fail with a named scheduling
boundary instead of executing historical code or guessing. Actual full native
context/snapshot provisioning and activation remain a separate caller handoff.

On adoption, generate/init preflight all outputs and retire only exact known
legacy unit projections. A modified obsolete file or unowned parent collision
fails before writes; use explicit `migrate` with the exact prior provider snapshot
when settings/addresses also change. Regeneration preserves explicit runners,
bootstrap argv, timeouts and all host source data. In this opt-in form,
`units.*.workflow_path` retains the declared legacy addresses for migration; all
current preparation and source checks bind the one `collection.workflow_path`.
The existing gather wait/poll settings keep their legacy-workflow meaning and are
unused by this nonpolling parent transport. Unlisted or edited files are never
retired by scanning. Verification detects both
parent drift and still-present known obsolete projections without repairing them.

The [copyable conditional host](../examples/ci-host-job-gating/README.md) uses
installed tools and the fixed short commands. The old ci-host bundle and released
Go/TS/mix providers retain their historical forms until explicitly migrated by
the caller after public binaries are available.

## Assignment and execution

Keep the existing scoped check fields and use `schema: chrono-ci-check/v3`.
Add the following explicit policy fields:

```json
{
  "units": {
    "service": {
      "tests": ["test:service-tests"],
      "report_path": ".chrono-harness/state/service/check.json"
    },
    "client": {
      "tests": ["test:client-tests"],
      "report_path": ".chrono-harness/state/client/check.json"
    }
  },
  "shared_operations": {
    "build.shared-input": ["client", "service"]
  }
}
```

Assignments must cover the complete candidate plan registry exactly once.
Unknown tests, missing assignments, duplicate assignments, report collisions
and undeclared shared operations fail before business execution. The example
shared operation is valid only when both registered complete plans contain it.
Use `{}` when no operation is shared across units. Sharing means executing the
declared prerequisite in each isolated checkout; it does not transfer artifacts
or create an implicit dependency between workflows. Cross-workflow artifact
dependencies are not implemented by this version.

Both endpoint registries still determine DELTA impact, including removed edges,
operations and test replacements. Changes to unit assignment select the affected
plans. The judge validates the complete selected operation graph before filtering
to a requested unit, so a contradictory global operation order cannot be hidden
by that filter. Each unit executes its selected complete plans through the
existing routes/projects engine. Replacement success belongs to the unit owning
the replacement plan; collection requires that unit's successful evidence.

Input projection follows the candidate unit's declared plans even when that
unit ID is new or renamed. At the base endpoint it also retains any historical
plans assigned to that ID, using the same declared input closure for both
assignments. An unrelated unit's business inputs are not a fallback requirement.

Use the same command locally and in the generated unit workflow:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID --unit service
```

The optional `--unit` and `--collect` selectors require v3 and are mutually
exclusive. Their transport is `chrono-ci-judge/v2`; old calls retain v1. A v3
call without either selector remains the global scoped check. Where the host
uses `registration_config`, a scoped invocation must match its registered
canonical command with exactly the chosen selector appended. Initial mode uses
`--initial` without `--base` and still requires a parentless candidate.

Unit evidence separates `selected`, `global_selected`, `assigned_elsewhere`,
`required_units` and `not_required`. A successful unit never certifies global
completion, and obligations assigned elsewhere are never labeled not-required.
Run concurrent units in separate checkouts: this version does not provide shared
checkout artifact locking.

## Offline collection

Supply a manifest beneath registered `.chrono-harness/state/`:

```json
{
  "schema": "chrono-ci-collection/v1",
  "reports": [
    {
      "unit": "service",
      "path": ".chrono-harness/state/imported/service.json",
      "sha256": "REPORT_SHA256",
      "runner_sha256": "RUNNER_SHA256",
      "judge_sha256": "JUDGE_SHA256"
    }
  ]
}
```

Digest fields contain full lowercase SHA-256 values. Include every required
unit exactly once. Reports from registered units without DELTA are optional;
when supplied they are validated too. Use:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --collect .chrono-harness/state/collection/manifest.json
```

The collector recomputes the current global obligations and verifies report
digests, endpoint/profile/selector bindings, expected executable pins, original
judge output, plan identities and exact candidate operations/order/bounds. It
checks original process bytes and receipts using the shared receipt verifier.
Missing or duplicate evidence, stale bindings, edited output and unsuccessful
units cannot pass. It executes no business test operations. An empty DELTA can
be collected with an explicitly empty report list.

Scoped v3 reports use compact JSON so retained byte arrays do not acquire one
indented line per byte. The parsed report, original process bytes and their hashes
are unchanged. Short calls (`check`, `check --unit ID`, `check --collect`) print
the original status and exit code, bounded identifying diagnostics and the exact
immutable original report path. Omitted diagnostic rows or text are explicitly
marked; consumers read the complete report at its existing published path.
Legacy explicit `--config` calls keep stdout identical to the stored JSON.
Historical reports keep their original bytes and hashes, including pretty-printed
reports. Projection runs after publication and executes no business operations.
Producer failures and version mismatches refer to retained original acquisition
or probe evidence. Execution errors before completed publication refer to a
retained original error, not a completed report; retention failures preserve the
original failure and explain the write failure without claiming an artifact.
Early configuration/selection errors keep their existing `E_CHECK` diagnostics.

Scoped v3 hosts can adopt `policy.report_publication: "retained-reference/v1"`
to keep one complete immutable check report. The registered fixed report path
then contains only `{"schema":"chrono-check-reference/v1","original":{"path":...,"sha256":...}}`.
The short console still points directly to the complete original; the reference
contains no verdict. Local and native manifests bind the reference's exact bytes.
Collection reads the original through the existing declared artifact transport,
verifies its digest and retained path, and applies every existing report, process,
plan and executable check. It does not follow reference chains. Missing, changed,
symlinked, undeclared or oversized originals fail; neither a small reference nor
a successful download replaces the original evidence. Both the reference and
original are subject to the registered report read limit separately.

Adoption requires matching runner and collector binaries and the prepared short
check entry; explicit endpoint calls fail before business execution. Old profiles,
including scoped v3 hosts omitting this field, keep complete fixed-path reports.
Unknown values, explicit null and adoption in scoped v1/v2 are rejected. Consumers
that read a fixed path must understand the reference before a host adopts it.
Upload the registered artifact directory containing both reference and original;
do not upload only the small reference. This reduces duplicate check-report
publication; it does not deduplicate other acquisition, process or gather evidence.

With explicit `retained-reference/v2`, the fixed reference format stays the same,
but its original uses `chrono-check-report/v2`. The original preserves request,
runner identity, process metadata, exit status, failure and retained path. The
judge's exact stdout and stderr bytes are retained separately through the existing
original mechanism; `judge.stdout_original` and `judge.stderr_original` each hold
`{path, sha256}`. The report has no separate `response` or inline judge `stdout`,
`stderr`, `stdout_bytes`, `stderr_bytes`. Collection verifies both stream hashes
against their references and process metadata, restores transient process views,
and decodes the response from original stdout. Existing protocol, executable,
plan, receipt and verdict checks still apply. Business and transport failures keep
their original status and bytes; the bounded console points to the report.

For v2, `report_bytes` covers the combined bytes of original report metadata,
stdout and stderr for each unit. The fixed reference has its own bound of the
same size. Collection resolves every original through the declared transport;
missing or damaged streams cannot fall back to producer-local copies. Upload the
whole registered artifact directory and upgrade all actual readers before
adoption. Inline stream views are rejected; consumers must use verified original bytes. This storage contract does not deduplicate business receipts
inside judge output or other acquisition evidence. V1 and omitted policy keep
their existing formats.

Hosts may declare independent collection read limits in their check policy:

```json
"collection_limits": {
  "manifest_bytes": 1048576,
  "report_bytes": 134217728
}
```

Both values are positive byte counts below the platform's `usize::MAX`;
an omitted object preserves the existing
64 MiB limit (per file, or the v2 original-and-stream total described above).
A supplied object requires both fields and accepts no
unknown fields or explicit `null`. It is a scoped-v3 extension; older binaries reject the new field,
so install a release containing it before adopting the policy. Each original unit
report is read and verified separately. The limit applies to its actual serialized
bytes, not its compressed download or parsed payload; it is distinct from the
judge process stdout limit. Equality is accepted; overflow fails with the path,
limit name and observed byte count before parsing that input. A bounded reader
also stops a file growing beyond its configured limit. Limits are host resource
policy, not an assertion that the host has enough memory or that inputs are
complete. Adjust them from measured original reports; do not rewrite old evidence
to make its digest or size fit. Collection still verifies every required report
and never reruns business operations.

The reports must already match the candidate and configuration being collected.
Changing limits changes that configuration's identity; it does not authorize
reuse of reports bound to the previous configuration. Produce matching reports
for the new candidate while retaining the original results separately.

Executable pins are explicit caller inputs. Collection binds reports to those
pins; it does not independently prove how the executables were built or certify
their external input closure. Original tool observations can come from different
roots and platforms. That fact does not establish environmental equivalence.

## GitHub projection and transport

Put a `chrono-github-units/v1` source under `.chrono-harness/ci/`. It contains:

- `collection`: an existing `chrono-github-ci/v1` or v3 provider configuration
  describing the common events, actions, runner, generator and check profile,
  and the collection workflow's path/name/bootstrap/runner/artifacts.
- `units`: a map with exactly the same IDs as the check profile. Each value
  declares `workflow_path`, `name`, `runs_on`, `timeout_minutes`, `bootstrap`,
  `context_path` and `artifact_directory`.
- `gather`: the GitHub CLI `program`, `inherit_environment`,
  `credential_environment`, literal `environment`, process `timeout_seconds`,
  `output_limit_bytes`, overall `wait_seconds`, `poll_seconds`, and explicit
  `manifest_path`, `download_directory`, `report_path`.

Optional `gather.download_retry` explicitly declares `max_attempts` (1–10),
`delay_seconds` (1–60, less than `timeout_seconds`), and a nonempty, duplicate-free
`http_statuses` list of HTTP error codes (400–599). Omission preserves one download
attempt and the existing destination layout; null and unknown fields fail.
For example, a host can select three attempts, a two-second delay and statuses
`[429, 500, 502, 503, 504]`. This policy applies to unit artifacts and shared native
seeds, without changing API query handling or rerunning any business unit.

With `job_gating`, optional `gather.resource_observation` declares an exact
`step_categories` map from GitHub step names to host category names. Empty names,
control characters and the reserved category `unclassified` are rejected.
Unknown step names remain `unclassified`; the provider does not infer language,
dependencies, necessity or categories from command text. Omission preserves the
existing gather output. The host can update this map without changing Rust.

The existing paginated jobs response supplies `resources` in the original gather
report (`chrono-ci-resources/v1`); no extra request, runner or business execution
is added. It binds the response's process index/digest, candidate, run and observed
attempt, and counts each identical job ID once across pages. Conflicting copies
and invalid identities are excluded with explicit issues. Totals and comparisons
use only the current attempt's API job view. `excluded_prior_attempts` lists older
job IDs and attempts; the original source response retains their full rows,
including failures. GitHub may copy successful jobs into a new attempt with new
IDs and unchanged execution timestamps. The current view may therefore include
carried results; it is neither distinct execution history nor the cost of rerunning
the current attempt. The observer does not infer execution identity from matching
names or timestamps and does not add historical rows to the current view.

`known_step_seconds` sums only completed, non-skipped steps with valid ordered
timestamps. It is null when nothing was measured. Each step retains its category,
timestamps and measured/unknown/skipped state. Categories and jobs have separate
totals; unclassified steps, missing steps, conflicting rows and unfinished jobs
remain visible. `observed` means the returned rows have usable classified data,
not that the entire workflow or its resource use is known. `partial` reports
incomplete observations. A gather attempt that never acquired jobs reports
`unavailable`; failures before gather have no fresh gather report.

These seconds are neither billing minutes nor parallel workflow wall time or
CPU time. Queue time, peak memory, disk bytes, work after the snapshot and the
compile/test split inside a canonical check remain unmeasured. Collection is
still running when it observes itself. Native failure, transport acceptance and
the final judge keep their existing rules; resource observations cannot turn a
failed check green or prove that a successful dependency is necessary.

Optional `comparisons` registers warning rules keyed by a host-owned ID. Each
rule names distinct registered `category` and `reference_category`, a positive
finite `factor`, and nonnegative integer `minimum_seconds`. For each completed
observed job, a warning is produced when the category's measured seconds are at
least the minimum and strictly greater than `factor * reference_seconds`.
Equal values stay within the threshold; a measured zero reference is usable.
The host adopts `bootstrap-vs-check` with factor 1 and a 60-second minimum to
expose startup work that exceeds the same job's canonical check.

Only the two explicitly named categories participate. Missing categories,
unknown timestamps and unfinished jobs produce `unavailable` with a reason;
skipped jobs are `not-applicable`. Other categories cannot silently substitute
for a missing reference. Completed failed jobs retain their measured cost.
Deduplicated job identity and attempt remain attached to every comparison, and
the Actions summary lists `W_CI_RESOURCE_COMPARISON` findings with their measured
values. These are host policy warnings, not proof that the work is redundant or
that an optimization saved resources. They neither change admission nor add
requests, jobs or executions. Omission preserves the existing report format.

Optional `summary_environment` names an explicitly inherited variable containing
an existing appendable summary file. The host adopts `GITHUB_STEP_SUMMARY` in its
input environment to show the same category totals and unknowns on the Actions
page. Missing or unwritable summary sinks are reported as unavailable in the
original report and do not replace the admission verdict. This presentation
uses the same check invocation and adds no workflow step.

Retries require a nonzero process exit, no process-bound failure, and one
unambiguous `HTTP nnn:` status in the registered CLI's stderr that appears in
the whitelist. All attempts and delays share the original download deadline;
each output stream shares its original byte limit. An exhausted budget,
unlisted or ambiguous status, timeout or output overflow stops acquisition.
Each configured attempt writes a separate `attempt-N` directory. Original
process evidence, partial files and the actual retry/completed/failed action
remain available, including after successful recovery. Only the successful
directory supplies report inputs; fixed artifact identity, digests, workflow
source, run/attempt and final judge checks still apply. Exhausted recovery
produces no accepted manifest and cannot make a failed unit pass.

The bootstrap list is a literal argv. Register only the tools each unit needs;
the generator never infers packages, language setup or build dependencies. Each
unit's report and context must be inside its uploaded artifact directory.
Workflow paths and names must be unique. The entire output set is preflighted
before generation; unowned files are not overwritten. Filesystem failures are
reported without claiming a multi-file transaction. Removed workflow paths must
be explicitly retired by the host alongside their registrations.

```sh
chrono-ci init --host-root . --config /path/to/units.json
chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

`init` uses `.chrono-harness/ci/units.json` and preserves an existing host source.
It needs an already registered v3 check profile; it does not invent assignments.
Existing single-workflow, full-context and release provider schemas keep their
semantics and projections.

On push and pull_request events, preparation binds the generated provider,
workflow and profile to the candidate and records the exact canonical argv and
pre-execution runner/judge identities. The collection workflow has `actions:
read`; its separate gathering step alone receives `GH_TOKEN`. Declare
credential environment keys explicitly so transport reports omit their values.
The report retains original nonsecret process bytes, exit/failure and an identity
of the actual environment. Never pass credentials as literal transport values.

Gathering waits within declared bounds for each workflow's run at the candidate
and event, pins its actual attempt, downloads the attempt-specific artifact,
checks matching fixed contexts and the actual workflow source bytes, and checks
the attempt again. Missing, ambiguous, changed or failed runs fail collection.
Original downloaded reports remain available. The resulting manifest is
`gathered-unjudged` until the same canonical `check --collect` succeeds. Unit
failure does not cancel unrelated unit workflows; retrying a unit does not
rerun them. Retry collection afterward to gather the new pinned attempt.

Automatic collection currently covers push and pull_request events. Individual
unit workflows also accept explicit manual inputs; manual combinations use
explicit manifests and the local collection command. The collection workflow
does not expose an unsupported manual trigger. Existing collection outputs are
not overwritten by a retry; native retries use fresh checkouts. Workflow-source
and endpoint mismatches remain errors, including when a moving integration base
changes while separate workflows prepare their inputs.

Source behavior tests include actual candidate binaries in independently cloned
hosts, concurrent execution, offline collection, and synthetic GitHub transport
responses. The public Go, TypeScript and mixed [example hosts](examples.md) use
independent native workflows. Their local/integration/PR/dev reports were checked,
as were actual cancellation with selective retry, business failure with another
unit passing, and restoration with zero selected business operations. Initial
inventory remains a separate contract. The product host registers sixteen units,
one per complete test plan, and a collector in `.chrono-harness/ci/units.json`.
Its core bootstrap explicitly builds runner, judge-ci, ci and worktree; each selected plan
keeps its complete existing build/check/test operations. Shared operations are
listed in the check profile and repeat only in isolated checkouts. The default
full bootstrap remains available when its existing configuration is selected.

The CI generator consumer also executes the generated unit workflow's actual
`chrono-ci prepare` CLI against a committed push event, checks the
candidate-bound workflow source and context, then invokes the recorded local
`chrono-harness check` argv and reads the retained unit report. It checks the
literal generated command shape and the selected operation in
`crates/ci-tests/tests/units.rs`, so a projection change cannot silently diverge
from the local command. Live push and pull-request dispatches for the public
Go, TypeScript and mixed hosts, including their collection and every declared
unit, are recorded in [native adoption evidence](native-ci-adoption.md).
That closes generated-unit adoption for this bounded profile. Complete
effective-input closure, `chrono-github-ci/v3` host adoption, deterministic
parity and full host activation remain separate obligations.

## Host customization and updates

Use the host's registered source paths. The table identifies which declaration
owns each customization; the generator requires no business directory layout
or language convention.

| Change | Host-owned source | Apply |
| --- | --- | --- |
| Commands, tests or dependencies | Project/script actions and FILEMAP edges/plans | Update declarations and run the canonical DELTA check |
| Unit boundaries or shared operations | Check profile `policy.units` / `policy.shared_operations`, plus provider `units` | Keep each complete test plan assigned once; regenerate owned workflows |
| SDKs, runner image, timeout or startup | Provider `bootstrap`, `runs_on`, `timeout_minutes` and explicitly registered bootstrap data | Generate, verify, then check the changed native event |
| Workflow paths or removed units | Previous and next provider sources | Use `migrate` below to verify ownership and retire obsolete outputs |
| Harness version | Public distribution pin and selected tool list | Adopt the new pinned release; keep the host's CI and SDK sources |

The [Go, TS and mixed examples](examples.md) demonstrate the last row by changing
only their distribution lock and README. None requires editing Rust or adding a
language-specific case to the generator. Schema/behavior changes still need an
explicit migration; a version bump does not certify compatibility on its own.

The host owns the provider JSON, check profile, unit assignments and bootstrap
commands. Customize these explicit sources; generated workflows are projections.
A unit can call any registered script or plugin through its literal bootstrap
argv and complete test plan. Unrelated host workflows can coexist. The generator
never selects a language, SDK, directory layout or dependency on the host's behalf.

`init` preserves an existing host source byte for byte, including custom runner,
timeout, bootstrap arguments, context and artifact settings. Supplied defaults do
not replace it. After installing a pinned newer binary, use the same `generate`
and `verify` commands on that source. Verification of projections is separate from
the actual canonical checks and native event validation.

Public beta.13 provides `chrono-ci migrate` for an explicit projection transition.
Prepare the new check profile, assignments and provider source, then migrate the
verified old owned workflow:

```sh
chrono-ci migrate --host-root . \
  --from .chrono-harness/ci/github.json \
  --config .chrono-harness/ci/units.json
```

This accepts a scoped v1 or units provider as the previous source and an explicitly
configured units provider as the destination. It verifies every old generated file
and every new output collision before writing. It preserves the destination JSON
exactly, writes the new projections, removes only obsolete previous workflows and
retires the distinct previous source only when its bytes match the explicit input.
A host-edited old projection or colliding new file fails before any write. Unrelated
files are preserved. Initial-inventory, full-governance and release-provider
transitions are rejected by this command; it never silently weakens those contracts.

For an update at the same source path, retain the previous source bytes before
editing (a run-local file belongs beneath `.chrono-harness/state/`). Supply its
original source address explicitly so embedded workflow commands remain bound to
the correct source:

```sh
chrono-ci migrate --host-root . \
  --from .chrono-harness/state/previous-units.json \
  --previous-config .chrono-harness/ci/units.json \
  --config .chrono-harness/ci/units.json
```

This also retires explicitly removed or renamed workflow paths. It leaves the
new source and retained prior-input file untouched. The JSON result lists written
and retired paths and input hashes. Update FILEMAP and any explicit consumers for
those paths, then run the canonical DELTA checks. Migration does not infer or edit
the host's registration policy, and its success does not certify governance or CI.
Filesystem failure reports completed changes; there is no cross-file transaction
or concurrent-writer isolation. Preserve the original result and reconcile the
reported paths before retrying; once migrated, ordinary generation is idempotent.

For adopted v4 gather, the acquisition owner reserves and records its directory
before download attempts. Failed retry directories remain originals. The current
immutable gather report references that acquisition, and pointer replacement
settles its previous root only after publication. External download bytes are
accounted during sealing; controlled directory creation checks the publisher
node bound. Protected overflow is preserved and reported separately from the
transport result. Detector seed acquisition and complete native interruption,
retry, and delivery acceptance remain separate unfinished boundaries.
