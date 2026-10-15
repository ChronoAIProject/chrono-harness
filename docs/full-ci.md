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
| First native input acquisition | In an actual supported Git main with no installed worktree owner, use `host.acquire-native-macos` on macos-26 arm64 or `host.acquire-native-linux` on Linux x86_64. The existing bootstrap builds the original 14 tools in private Rustup/Cargo homes, then runs `inputs.acquire-PLATFORM` through the just-built observer before installing them. Its existing missing-main-tool seed branch does not require the later full policies. Genuine initial tools/environment/network and original-retention inputs are listed in [inputs](inputs.md#actual-input-observations). Existing owners and linked checkouts retain the full participation requirements. Retain source/install/tool identities, bootstrap originals, every observer manifest/launch/stream/exit and the inner `.chrono-harness/state/inputs/` closure. Resolve observed absences and known undeclared inputs before adoption. |
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
| `.chrono-harness/native/macos/config.json`, `.chrono-harness/native/linux/config.json` | The Mac direct v4 source is independently bound as described below; its required unavailable inputs still fail. The Linux direct source is independently bound below; both platforms retain unavailable requirements as rejecting gaps. Preserve declared inputs, Cargo target/configuration/alias checks, runner/judge pins and cap2. Register actual sources before generation; Mac values never supply the Linux row. |
| Each direct policy's `execution_units` | Preserve the current check source's exact unit-to-test assignments and shared operations; declare all 26 units. For platform `P`, use report paths under `.chrono-harness/state/native/P/units/ID/`, collection report under `.../collection/`, and the current collection limits. These paths must be distinct from the provider's context/full-context/manifest/gather outputs. Registration and the existing generator own admission. |
| Each direct policy's `canonical_check` | Bind its full profile to its own direct policy path. Bind the CI action to the native interpreter tool and argv `[".chrono-harness/ci/native-P.py", "forward", "--config", ".chrono-harness/ci/units-native-P.json"]`. Keep `CHRONO_CHECK_SOURCE`; move acquisition-only event/payload/revision/repository names out of business inheritance as required by the adapter contract. Bind the local producer to the genuine platform lifecycle policy. |
| `.chrono-harness/native/P/worktree.json` | Actual compatible native Git byte/version/environment binding, host selector/policy, cleanup/configuration and origin paths. Managed native bootstrap/cache and later acquisition through an existing owner require this committed policy. The first missing-owner Git-main seed uses the existing exception, without claiming enrollment. Deploy compatible candidate tools and both committed lifecycle policies through the surviving coordinator; source staging is not enrollment or birth. |
| `.chrono-harness/native/P/cargo-runner.json` | Authentic platform Cargo guarded policy, preserving package, target, configuration, sysroot/SDK/runtime and alias assertions. Its `guard.check.native.P.runner` operation must preserve the original guard's exact Cargo check argv. The registered guard entry names that same operation ID and the actual direct native config; the guard's existing direct-policy API does not resolve the selector. Require an actual positive result on both platforms. The prior local/native negative expectation in `cargo/observe.json` remains separate evidence. |
| `.chrono-harness/native/P/cache.json` | The Mac source reuses the existing cache registration with native profile, FILEMAP, lifecycle, private homes and bootstrap inputs. Cache IDs, producer/consumer assignments, source inputs and restore/backend assertions remain. Detector preparation still names the native cache bootstrap whose report shared startup consumes. Actual cache production/transport is unverified. The Linux source below binds its independent acquisition record and private homes; it does not certify current cache production. |
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

### Mac source bindings from initial acquisition

The nine sources under `.chrono-harness/native/macos/` supply the direct policy,
judge registry, FILEMAP, guarded runner, lifecycle, cleanup, cache and selected
Rust/Python inventory consumers above. They remain proposed; the main custom
local policy is unchanged. The
existing native core/cache bootstrap profiles select the initial acquisition's
private default Rustup/Cargo homes and native developer/SDK environment, preserving
their build/install inventories and separate shared-startup roles. No source
enrollment, installed state, workflow/adapter projection or full activation is
produced by editing these declarations.

The source is macos-26 arm64 job `116297942471`, run `38697657077`, attempt 1,
commit `5333a5b8feb25c627a9bef88d817cbc62a0dc2db`, tree
`a49ca1f2e11aba94cdd82ad53c016d914713c8b9`. Its independently retained artifact
ZIP has SHA-256 `7cdbc3872ee1e4789d1d84541a93343c89f7194354ce24164ef7a3e1bb091573`
and restore root `.chrono-harness/`. Source inspection rehashed the 438 addressed
originals, including all 14 installed tools, plus observation stdout/stderr/launch
references. No downloaded binary was executed. Runner and seven judge pins use
installed bytes; installer, Cargo/rustc, Python launcher/interpreter and native
compiler/delegates retain separate identities. External hashes are observer
records, not independent rereading of excluded private homes, SDK or runtime bytes.

Existing input IDs and consumer bindings are preserved. Supported native locations
and hashes come from `observe-macos.json`; unchanged source declarations retain
their source meaning. Selection uses Rust 1.95.0, Xcode 26.6, the resolved
MacOSX26.5 SDK, private toolchain and recorded runner PATH. The guard retains the
original native target, configuration, sysroot, full Developer-root and SDK alias
assertions. Four observed Cargo configuration absences remain absences;
unobserved external configuration remains required with null hash. The existing
`cargo/observe.json` negative regression expectation is unchanged; acquisition
executed no guard.

The Rust v2 inventory preserves its exact 92 selected file paths and empty link
list, with all native digests read from the retained directory observation. The
Python inventory preserves its exact 785 selected files with native hashes and
root. This reuses the existing contracts without enrolling the observer's extra
documentation, cache or runtime inventory as new mandatory scope.

The native FILEMAP is an explicit policy snapshot because the local table names
acquisition environment nodes excluded from native business inheritance. It keeps
the same files, 26 execution plans, costs, cap2, claims, inventories and operations;
native input edges and acquisition-variable edges differ. Standard compact JSON
keeps this table within the unchanged 2 MiB immutable-input bound, with no
compression dependency. Root FILEMAP registers every new source and actual
consumer. Both native immutable-input capacities remain 2097152 bytes. Full units
copy the current check's exact 26 assignments/shared operations, separate native
report paths and omitted-policy collection defaults of 67108864 bytes per limit.

**Policy and routing change together; runtime behavior remains unverified.** The
finite remaining Mac producer/consumer handoff is:

1. Produce exact native `chrono-input-directory/v3` inputs at
   `.chrono-harness/state/native/macos/guard-inputs/{sdk,developer}.json`
   and bind their genuine digests. The retained file/link observations lack exact
   directory-child records; Developer scope covers only
   `Toolchains/XcodeDefault.xctoolchain`, while the preserved guard requires the
   full Developer root. Guard consumption already supports v3, but the current
   `chrono-inputs observe` inventory API emits neither directory-child records
   nor link target kinds. No registered producer of those v3 records exists in
   the audited input/guard owners. The remaining producer work belongs in that
   acquisition owner, including the full Developer selection, before these guard
   inputs can be bound. Do not downgrade inventories, infer empty directories or
   fabricate missing scope.
2. Resolve the 19 other null-hash inputs at their declared locations: ancestor/HOME
   Cargo, rustfmt and rust-toolchain lookups, private Cargo credentials and private
   Rustup settings. Produce actual byte/absence originals and update declarations
   and coverage. Obtain clang, Rustup and cargo-fmt version outputs; their native
   `expected_version` is null. The guard requires a bound linker version before
   positive admission. No alternative probing entry is introduced here.
3. Complete genuine lineage, fixed base composition and addressed blobs, endpoint
   pair, governance projections, applicable integration evidence and original
   reports. Provider lineage/composition digests remain null; its decoder must
   continue rejecting them.
4. Reconcile existing stable-entry/lifecycle consumers before activation:
   `prepared::participate` and short check load `.chrono-harness/config.json`;
   participation defaults to `.chrono-harness/worktree.json`, and
   `automatic::check_command` requires literal `worktree.check` / `["check"]`.
   A separate native policy cannot switch those consumers with alternative argv.
   The native local-input producer/lifecycle sources await compatible caller
   selector/policy cutover. Preserve the custom local policy; no second short
   entry or lifecycle workaround is introduced.
5. Following those bindings and a clean commit, caller runs registered bootstrap,
   committed-parent real bootstrap consumption, FILEMAP DELTA-selected canonical,
   `guard.check.native.macos.runner`, native generate/verify, original unit
   acquisition/capture, adapter, cache/shared startup and collection/full adoption.
   Linux requires independent current-candidate native/full production and acceptance.
   Whole-Q3 review/landing/release and original R4 cause/consumer work remain open.

Cost accounting is UTF-8 source bytes against the fixed source above; the worker
result lists each changed path and before/after bytes. Native snapshots have
explicit FILEMAP ownership and consumers, and add no active workflow pipeline.
Runtime/CI/peak storage/retention costs remain unmeasured; no saving is claimed.
This assignment executed zero build/test/generate/bootstrap/probe/provision/guard/
observer/reproduction operations. Retain original acquisition inputs through
native/full acceptance and whole-Q3 review, then use existing caller retention
and lifecycle policy. Source staging is neither acceptance nor partial release.

### Linux direct-policy source bindings

The Linux row now has the same nine natural-owner sources as the Mac row under
`.chrono-harness/native/linux/`: `config.json`, `judges.json`, `cargo-runner.json`,
`worktree.json`, `cleanup.json`, `cache.json`, `FILEMAP.json`, `rust.json` and
`python.json`. They are proposed source declarations and do not activate a new
workflow or replace the stable local profile. The root FILEMAP records each source
with its existing runner, CI, inputs, registration, Cargo, worktree and cache
consumers and leaves their costs `unmeasured`.

The bindings use the authentic Linux x86_64 acquisition `run 38716787272`,
`job 116361317528`, source `dc254c556072d535f3399ce2ccccd49e6c69d858`, tree
`c887f8fd06439b7be8e8a1d5f2fbee4d8fa7c996`, and retained ZIP SHA-256
`41d007ae1ec28ef734e0238252573e300841aa06e9ba7bb73414f712bea2396b`. Its
`state/inputs/observe-linux.json` member is 10,496,919 bytes with SHA-256
`804b0a023701bc4b4107c14a3eb95c28970fe3cafb4a1c654b1ee1ead404a189`. The
source preserves the observed Cargo/rustc 1.95.0 host
`x86_64-unknown-linux-gnu`, Rustup/Cargo private-home locations, Git 2.55.0
(`/usr/bin/git`), Python 3.12.3 (`/usr/bin/python3.12`), GCC 13
(`/usr/bin/x86_64-linux-gnu-gcc-13`), archive tool, libc, os-release and Python
shared-library identities. Rust and Python declarations retain every observed
file digest. Python retains the original inline v1 observation and its three
literal aliases; it is not a v2/v3 guard manifest. Python `root` and `platstdlib`
share `/usr/lib/python3.12`; the extension role reuses the same physical file
inputs, as the producer reported. The guarded runner preserves the original
Linux metadata resolution, including `linux-raw-sys`, exact target predicates,
features, checksums and package file identities. The other consumers retain
their inherited explicit package subsets as incomplete declarations.

The producer observed 22 external configuration absences and private Rustup
settings. The 140 inherited configuration requirements without Linux observations
remain present/null (unbound, with no assertion of observed presence or absence).
Its inline `/usr/include` and `/usr/lib/gcc` inventories contain aliases outside
their roots; compatible guard manifests, delegate closure and alias target
admission remain unavailable. The required Linux
SDK-equivalent, Python runtime and other package coverage rows remain unresolved
and reject activation; the guard's empty `sdks` list is not full input acceptance.
Mac SDK/Developer assertions remain in the Mac sources. The producer did not
supply v3 directory-child manifests, a birth or endpoint pair, a fixed base or
composition, governance projection, positive Cargo guard, native adapter, full
adoption or delivery evidence. These pins identify the retained acquisition
bytes; Linux expectations require Linux originals, and fresh candidate-byte
evidence is still required. The caller must
rebind excluded private/system bytes on the real Linux endpoint and run the
registered bootstrap, positive Cargo guard, native generation/verification and
full consumer acceptance before changing `proposed` or claiming Q3 completion.
The Linux FILEMAP retains both platforms' source registrations and the existing
26 plans, and connects every declared Linux input to its actual consumer. Its
2,345,120 source bytes exceed the existing 2,097,152-byte immutable FILEMAP bound;
that bound is unchanged, so native lifecycle admission remains rejecting until
the caller resolves this owner/capacity gap. The direct config remains within its
existing bound. Source-byte accounting is exact for this delta; native execution,
CI, peak storage, retention and complete input-closure costs remain unknown.

**Host policy and consumer routing change together.** Previously only the local
bootstrap/environment and scoped provider were selected; the new sources declare
native tool production, independent native policy routes, full-context ownership
and positive guard entries. The Mac SDK/Developer guard assertions remain
required; the Linux equivalent coverage gaps above reject full activation. Source
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
zero business executions. The first-producer continuation also fixes relative
bootstrap tool paths and their shared producer validation. Its Python owner tests
define success from another cwd, acquisition before installation with absent
full policy, private-home propagation, original references and failure streams,
existing-owner refusal, linked-checkout refusal and changed-executable rejection.
They use the explicitly registered Python fixtures. All remain unexecuted here.
All checks, native source projection, bootstrap,
committed-parent real consumer, canonical/native full runs and acceptance remain
unexecuted for this handoff. Later full consumers must keep rejecting null or
missing bindings; initial acquisition grants no waiver for those consumers.

**First-producer policy/product impact:** the two new acquisition profiles append
one existing observer operation to the preserved 14-build inventory. The proposed
full-unit/shared/cache profiles retain their prior sources, so collection does
not unconditionally acquire every SDK and Cargo input. The small Python change
resolves explicit relative executables at HOST_ROOT and checks the corresponding
recorded producer location; it changes neither participation nor observer/full
admission. Source/schema/runtime compatibility is unverified. Cost method is
source byte accounting for this fixed-source delta; build/acquisition/test/CI
seconds, peak bytes and retention allocation over a complete native job are
unknown until the caller executes and measures it. Keep initial originals through
native policy binding, endpoint/full acceptance and whole-Q3 review, then apply
the existing caller-owned evidence retention and lifecycle. An initial observation
cannot supply a historical original or a positive Cargo guard result.

The [native acquisition transport](ci.md#candidate-native-initial-acquisition-entry)
now includes the 14 registered installed executable originals, enabling the
caller's existing installed-byte verification alongside source/report/stream
verification. Its restore root is `.chrono-harness/`, with seven-day Actions
retention; private toolchain homes and build targets remain excluded. This closes
the source-declared omission of installed bytes, not native runtime acceptance.
Exact pushed source, original exits/failures, both actual native artifact round
trips and hashes, independent native policies, full input/adoption closure,
R4/whole-Q3 review, formal delivery, all original 28 PR dispositions and full SPEC
acceptance remain caller obligations. Native storage and execution costs are
unknown until measured.
