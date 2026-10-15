# Full context CI transport

`chrono-github-full-ci/v1` is an explicit GitHub dispatch projection. It carries
the caller's full context to the same runner command used locally:

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/config.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --context .chrono-harness/state/full/context.json
```

This extension is distributed in public beta.10. It does not activate the
product host's proposed full registries or replace its current scoped CI.

The fixed-source beta.10 release ran the registered CI and seven-judge consumer
tests on macOS arm64 and Linux x86_64. An anonymously downloaded macOS CI binary
also verified release/full generation, drift rejection/regeneration and exact
context/argv preparation on a real Git fixture. These are native product and local
public-binary consumers; generated full-dispatch execution on GitHub and full
host activation remain outstanding. See [release evidence](distribution.md).

`chrono-github-full-ci/v2` uses the schema4 short entry. Its generated and prepared
argv retain the configured scope: `check`, `check --unit ID`, or `check --collect`.
The registered CI action has one provider path, so this dispatch source must
match the requested selection exactly. Multiple automatic full units use the
explicit `chrono-github-units/v2` map in [CI units](ci-units.md), through the
existing event preparation and native gather owners. The full dispatch contract
continues to require supplied exact context and provisioned manifest evidence;
it does not perform automatic gathering.

## Explicit source and adoption

Supply a JSON source with these fields; replace the bootstrap and platform with
the host's registered choices:

```json
{
  "schema": "chrono-github-full-ci/v1",
  "workflow_path": ".github/workflows/full.yml",
  "name": "Full harness check",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["python3", ".chrono-harness/bootstrap.py", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "generator": ".chrono-harness/bin/chrono-ci",
  "check_config": ".chrono-harness/config.json",
  "context_path": ".chrono-harness/state/full/context.json",
  "preparation_path": ".chrono-harness/state/full/preparation.json",
  "artifact_directory": ".chrono-harness/state/full/"
}
```

`chrono-ci init --host-root H --config INPUT` adopts this schema into
`.chrono-harness/ci/full.json`. Existing adopted customization is preserved.
`generate` and `verify` use the same source and renderer; verify never repairs
drift. The workflow has a distinct ownership marker. Unowned outputs, symlinks,
overlapping paths, unknown fields, unpinned actions and expression-bearing
settings fail. Settings are literal single-line strings; shell metacharacters
in allowed literal arguments retain their bytes. Register source, workflow,
bootstrap, policy and their actual consumers in FILEMAP explicitly.

## Separate unit and collection sources

The optional `scope` field is a strict `chrono_harness::units::Scope`: either
`{"kind":"unit","unit":"compile"}` or
`{"kind":"collect","manifest":".chrono-harness/state/evidence/manifest.json"}`.
Omit it for the existing unscoped v1 projection and preparation; it is omitted
when an unscoped source is serialized. Null, unknown kinds/fields, invalid unit
IDs or manifest paths, and overlapping input/output ownership fail. Older
provider decoders reject the unknown `scope` field. One source selects one scope
once; dispatch accepts the caller context without a scope selection menu.

For example, explicitly register these two separate full provider sources in
the host FILEMAP. Each has its own workflow, name, bootstrap and artifact paths;
the shared runner and check policy are explicit host choices. These examples
are configuration inputs, not adopted product-host registrations.

`.chrono-harness/ci/full-compile.json`:

```json
{
  "schema": "chrono-github-full-ci/v1",
  "workflow_path": ".github/workflows/full-compile.yml",
  "name": "Full compile unit",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["/bin/sh", ".chrono-harness/provision-compile.sh", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "generator": ".chrono-harness/bin/chrono-ci",
  "check_config": ".chrono-harness/config.json",
  "scope": {"kind": "unit", "unit": "compile"},
  "context_path": ".chrono-harness/state/full-compile/context.json",
  "preparation_path": ".chrono-harness/state/full-compile/preparation.json",
  "artifact_directory": ".chrono-harness/state/full-compile/"
}
```

`.chrono-harness/ci/full-collect.json`:

```json
{
  "schema": "chrono-github-full-ci/v1",
  "workflow_path": ".github/workflows/full-collect.yml",
  "name": "Full collection",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["/bin/sh", ".chrono-harness/provision-collection.sh", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "generator": ".chrono-harness/bin/chrono-ci",
  "check_config": ".chrono-harness/config.json",
  "scope": {"kind": "collect", "manifest": ".chrono-harness/state/full-collect/manifest.json"},
  "context_path": ".chrono-harness/state/full-collect/context.json",
  "preparation_path": ".chrono-harness/state/full-collect/preparation.json",
  "artifact_directory": ".chrono-harness/state/full-collect/"
}
```

Generate and verify each explicitly selected source through the existing entry:

```sh
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/full-compile.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/full-compile.json
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/full-collect.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/full-collect.json
```

Changing and regenerating one source updates its owned projection and preserves
the other workflow and both host sources. Register each source, output,
bootstrap, manifest/evidence location and consumer explicitly; generation does
not register units or infer dependency linkage. Collection bootstrap/caller
work must provision the manifest and its original reports. This provider never
fabricates a manifest, gathers reports or orchestrates workflow dispatches.

Both workflow rendering and preparation append the same `Scope::argv()` suffix
after runner/config/base/candidate/context. In the example, the local command
above uses the compile context path followed by `--unit compile`; the collection
command uses the collection context path followed by
`--collect .chrono-harness/state/full-collect/manifest.json`. There is still one
canonical local/CI check. The provider validates transport syntax and ownership;
the full core judges determine registered unit and collection admission. Scoped
full checking requires the matching full-v3/v4 `execution_units` and sealed scope
request implementation. Provider preparation and a recording command consumer
establish neither full scoped execution nor admission. Native generated scoped
workflow execution and host activation remain separate, outstanding evidence.

The full check config must use schema v3 or v4 with a declared
[Git binding](git-facts.md). CI does not synthesize tools, judges, input closure,
project layout, language or evidence acquisition. The bootstrap argv must
provision the declared executables and every context-referenced retained input,
blob, integration certificate and completed report. Missing evidence remains a
judge failure. Use the declared artifact directory to retain all necessary
outputs; upload of that directory does not discover evidence elsewhere.

## Context and execution

The workflow accepts one `workflow_dispatch` string input, `context`, containing
the full context JSON. Supply schema 2, explicit `integration` or `delivery` role,
full base/candidate OIDs and the fields required by the full judges. In particular,
branch start/observation times, fork point, dev tip, operation, retained input
reference and integration evidence are caller inputs. No current-clock fallback,
branch-name role inference or automatic certificate selection occurs.

Checkout selects the supplied candidate. Preparation uses the candidate-bound
Git to check HEAD, full config bytes, provider source bytes and the generated
workflow blob at the supplied `github.workflow_sha`. It acquires a missing base
or workflow revision from `origin` only after an exact missing-object probe.
Failed probes/fetches retain original process bytes, digests and exit status.
This checks supplied Git identities; it does not independently authenticate
GitHub provenance, credentials, Git configuration or delegated input closure.

Preparation writes the UTF-8 bytes of the decoded context string exactly,
including whitespace, separately from its `chrono-full-ci-inputs/v1` report.
That report includes the raw context bytes/hash, source identities, observed Git
processes and canonical argv. Its status is `prepared`, governance is
`not-evaluated` and parity is `unestablished`. The preparer validates transport
fields; the actual judges own the remaining context and admission semantics.
An invalid freshness claim can therefore prepare successfully and fail judgment.

The generated Bash step saves original preparation stdout/stderr as
`prepare.stdout.json` and `prepare.stderr` within the artifact directory, and
propagates the actual process exit. Shell noclobber refuses existing stream files
before invocation. These names are reserved and cannot overlap
context/report paths. The full check invokes the shared canonical argv with
`--context` and the configured scope suffix, when present. Upload runs with
`always()` and preserves the declared directory,
including failure evidence; an early bootstrap failure may leave only job logs.

## Publication and retry boundary

Preparation preflights both context/report outputs before writing either.
Identical existing bytes are reusable. For v1, when context bytes and every
semantic preparation observation agree, differences only in invocation
`launcher_pid`, `child_pid`, or `ownership_fds` reuse the original prepared report;
the new attempt's complete original report is retained separately. Other differing
bytes collide and are not overwritten. V2 retains immutable originals and replaces
the declared current projections through the existing writer. Git acquisition
changes subsequent observations, so rerunning
after a fetch can collide with the retained first preparation report. Use a
fresh explicit run location/checkout and retain the earlier artifacts. Do not
discard old evidence to pretend that the first invocation never happened.

Publication uses the existing per-file writer. It is not a multi-file crash or
concurrent-writer transaction: an IO failure can leave one output published.
Successful preparation is neither full governance nor a parity certificate.
`check_config` may use the same strict one-level native selector as the full local
entry. Preparation retains the entry path and records the selected target binding;
the generated workflow invokes the stable entry argv unchanged.

Native consumers, generated workflow execution, host activation, complete input
closure and local/CI comparison are separate evidence obligations. This manual
dispatch projection does not orchestrate PR creation, merging, required-check
publication or landing.

## Main-host adoption inputs

The registered native acquisition chain is documented in [inputs](inputs.md).
Its observation outputs are inputs to engineering adoption, not endpoint
snapshots or full activation. The main `config.json` still selects local absolute
paths and scoped CI. Do not dispatch that local policy as a native full policy.
The source/CI selector and native adapter already implement the required routing;
the following values require authentic caller production before they can be bound:

| Owner / destination | Required authentic input and remaining action |
| --- | --- |
| Supported native bootstrap and input acquisition | On macos-26 arm64 use `host.bootstrap-native-macos`, then `inputs.acquire-macos`; on Linux x86_64 use `host.bootstrap-native-linux`, then `inputs.acquire-linux`. Acquisition delegates the existing provisioning and platform observation operations. Supply the declared native lifecycle policy first; retain bootstrap source/install/tool identities, original manifests, every launch/stream/exit and observation report, including the inner `.chrono-harness/state/inputs/` closure. Provide actual Git, Cargo/rustc/backend/linker, SDK/runtime/Python paths and bytes, Cargo target/configuration lookup and directory membership/aliases. Resolve observed absences and required undeclared inputs before adoption. |
| Main config selector | Retain the local direct policy at its own registered path; supply independently bound CI direct policies for each native platform. Use existing `chrono-git-configs/v2` `platforms[OS-ARCH].local/ci` at the stable entry, registering every target and its own immutable-input capacity. The selected policy must bind candidate runner/judges and actual tool/input/environment selections; unsupported/missing sources fail. Do not copy local Mac observations into CI expectations. |
| Fixed endpoints | At each actual fixed base/candidate checkout, run `chrono-inputs capture --host-root HOST --config ENTRY --commit OID --output SNAPSHOT`, with applicable `--unit ID --base BASE --candidate CANDIDATE` projections. Retain snapshot source/configuration identities and all addressed blobs. Use `pair` for the real base/candidate snapshots; unavailable historical originals remain unavailable. |
| Local full-input producer | `worktree.json.check_inputs.full_inputs.retained_inputs` now names `.chrono-harness/state/inputs/full.json`. Produce the exact endpoint pair there. `check-inputs` copies that current reference independently of immutable birth; missing bytes fail when a full profile is selected. Supply the existing successful original birth report/origin association; do not rewrite birth to insert current inputs. The independent source-staging checkout has no such origin claim. |
| Native unit/provider owner | Adopt units/v2 and genuine `native_adoption` values per [CI units](ci-units.md): original tracked lineage path/digest, actual base `composition_sources`, native adapter/interpreter/inputs binary, seed/artifact paths, roles and applicable integration certificate/transport. Register every source, binary, original and generated output. Use the generated adapter as `canonical_check.inputs.ci`; it retains authentic event/payload/revision/run/attempt records and delegates existing preparation/gather. |
| Governance and collection | Produce candidate governance projection with `capture-governance` using original base sources; produce selected unit snapshots and completed original reports. The existing native seed/acquire/gather/compose chain verifies fixed identities and imports their blobs. Missing, corrupt, nonzero or truncated originals remain failures. Five registry statuses may become active only after the applicable accepted producers and consumers exist. |

The caller must commit a clean candidate, deploy compatible candidate binaries
and both committed lifecycle policies through the existing surviving coordinator,
then run the registered bootstrap, committed-parent real bootstrap consumer,
canonical check and both native/full adoption chains. Ordinary full check retains
its documented declared-input contract; universal hidden-input proof and VM
isolation belong to neither this handoff nor a new acceptance gate. Whole-increment
review, landing/release identity and public consumption remain separate caller
obligations after complete Q3 engineering/acceptance, not initial review of a
partial source increment.

## Registered native source handoff

This source adds proposed `units-native-macos.json` and `units-native-linux.json`
under `.chrono-harness/ci/`. Both use the existing units/v2 generator, conditional
parent, native adapter, seed/acquire/gather/compose chain and all 26 existing unit
IDs. Native source/install inventory, shared startup, pinned actions, caches,
business operation owners, 45-minute job deadlines and gather limits are reused.
Per-platform collection, unit, seed, detector and startup roots are separate.
This is source registration, with no generated workflow or adapter published here.
The current scoped provider remains the deployed source until complete caller
cutover; the proposed native sources are not an additional active check pipeline.

The exact registered projection actions are `ci.native.macos.generate` /
`ci.native.macos.verify` and `ci.native.linux.generate` /
`ci.native.linux.verify`. Each invokes `chrono-ci generate` or `verify` with
`--host-root . --config .chrono-harness/ci/units-native-PLATFORM.json`.
Set `CHRONO_CHECK_SOURCE=ci` in the admitted native environment for these actions
and endpoint capture. The native selector is
`.chrono-harness/native-config.json`; it routes CI only to the required native
direct policy, with no fallback to the local Mac policy. Its Mac `local` route
keeps the existing direct policy; the Linux routes require their own direct
policy. This preparatory entry does not switch the main canonical command.

The remaining bindings below are inputs, not generated expectations:

| Source / destination | Exact caller binding |
| --- | --- |
| `.chrono-harness/native/macos/config.json`, `.chrono-harness/native/linux/config.json` | Independently authored and byte-bound direct v4 policies from actual native acquisition. Keep all applicable declared inputs, Cargo target/configuration/alias checks, runner/judge pins and cap2. Bind native interpreter/environment/Git selections, FILEMAP dependencies, artifact roots and selected policy input capacities; do not copy the local policy's paths or expected bytes. Register these actual source files before generation. |
| Each direct policy's `execution_units` | Preserve the current check source's exact unit-to-test assignments and shared operations; declare all 26 units. For platform `P`, use report paths under `.chrono-harness/state/native/P/units/ID/`, collection report under `.../collection/`, and the current collection limits. These paths must be distinct from the provider's context/full-context/manifest/gather outputs. Registration and the existing generator own admission. |
| Each direct policy's `canonical_check` | Bind its full profile to its own direct policy path. Bind the CI action to the native interpreter tool and argv `[".chrono-harness/ci/native-P.py", "forward", "--config", ".chrono-harness/ci/units-native-P.json"]`. Keep `CHRONO_CHECK_SOURCE`; move acquisition-only event/payload/revision/repository names out of business inheritance as required by the adapter contract. Bind the local producer to the genuine platform lifecycle policy. |
| `.chrono-harness/native/P/worktree.json` | Actual compatible native Git byte/version/environment binding, host selector/policy, cleanup/configuration and origin paths. Native bootstrap and cache profiles explicitly require this policy. Deploy compatible candidate tools and both committed lifecycle policies through the surviving coordinator; source staging is not enrollment or birth. |
| `.chrono-harness/native/P/cargo-runner.json` | Authentic platform Cargo guarded policy, preserving package, target, configuration, sysroot/SDK/runtime and alias assertions. Its `guard.check.native.P.runner` operation must preserve the original guard's exact Cargo check argv. The registered guard entry names that same operation ID and the actual direct native config; the guard's existing direct-policy API does not resolve the selector. Require an actual positive result on both platforms. The prior local/native negative expectation in `cargo/observe.json` remains separate evidence. |
| `.chrono-harness/native/P/cache.json` | Adopt the existing cache registration for the actual native provider, preserving all declared cache IDs, consumer operations, source inputs, capacity and transport assertions. Bind the native bootstrap/profile sources and the native artifact registry; do not dispatch the local cache policy or its macOS-only probes on Linux. Detector preparation names the native cache bootstrap, so its original report is the one native shared startup consumes. This authentic host-policy binding remains absent. |
| `units-native-P.json.native_adoption.lineage` | Preserve successful original start/reconstruct bytes at the declared tracked `.chrono-harness/native/P/lineage.json` path and supply their actual digest. The source deliberately has `sha256:null`, which the existing decoder rejects. Supply genuine event/branch/fork/time association; do not manufacture birth for this source-staging checkout. |
| `units-native-P.json.native_adoption.composition_sources` | Supply original base snapshot/pair bytes at the declared `.../native/P/originals/base-capture.json` path and their actual digest, plus all addressed blobs. Its null digest deliberately prevents generation. Capture at the actual fixed base using its committed policy; unavailable originals remain unavailable. Add other genuine sources explicitly if required. |
| Native roles and delivery evidence | The proposed feature/integration push and PR role is integration; dev push is delivery. Reconcile these host choices with the actual tracked lineage and workflow policy. Supply the existing integration certificate/path/transport when impact requires it; null is not a waiver. |
| Generated sources and retained outputs | After binding originals, generate and register each owned workflow, native adapter and exact bootstrap/selector/native-policy/original dependency. Add the two `startup-native-P/` artifact roots to the admitted native policies. Retain the detector seed and each selected unit's capture, original reports and streams through collection and caller acceptance; lifecycle retention remains caller-owned. |

The native shared-startup sources select the complete native bootstrap as `core`
and the existing cache bootstrap as `cache`, with disjoint installed binaries and
native lifecycle paths. They do not acquire SDK/business inputs for collection.
Caller bootstrap hooks must supply the actual selected-unit native input closure
before the existing adapter captures it; the registered acquisition operations
provide the producer path, not a claim that those bytes are installed. Do not run
all SDK/business acquisition in an unselected aggregate.

**Host policy and consumer routing change together.** Previously only the local
bootstrap/environment and scoped provider were selected; the new sources declare
native tool production, independent native policy routes, full-context ownership
and positive guard entries. No product API or guard assertion is weakened. Source
and CI projection compatibility, generation and runtime behavior are unverified.
Validation costs and native closure/storage costs are unknown; the source adds
two platform configurations and their explicitly owned outputs, not more business
capacity. All five full registries and ordinary full enforcement remain incomplete.
Stronger parity is separately unestablished.

Caller validation must use FILEMAP's two-end DELTA closure and existing registered
plans, including `test:ci-tests`, `test:inputs-tests`, `test:runner-tests`,
`test:judge-cargo-tests`, `test:worktree-tests` and `test:host-bootstrap-tests` when
selected. Their existing behavioral expectations include source/CI selection with
no fallback; original endpoint preservation; wrong/missing/corrupt lineage or
composition failure; ordered acquisition stopping after failure; immutable
publication; current retained inputs independent of birth; and collection with
zero business executions. No implementation logic was added, so no new bearing
test fixture is introduced. All checks, native source projection, bootstrap,
committed-parent real consumer, canonical/native full runs and acceptance remain
unexecuted for this handoff. The null/missing inputs must first produce the
existing failures; success requires the actual accepted producer/consumer chain.
