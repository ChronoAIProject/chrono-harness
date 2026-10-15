# Explicit retained input snapshots

`chrono-inputs` captures only the file IDs in a fixed config's
`environment.inputs` and the variable names in `environment.inherit`. Optional
`capture --unit ID` uses registration/input's shared prerequisite projection for
that registered full unit, including shared operations and governance consumers.
It does not
discover dependencies, rewrite expected hashes, add tests or declare input closure
complete. It is an independent Rust production/test pair. Git must be available
to read the explicit fixed commit. Config v3 uses the explicit
[Git facts binding](git-facts.md); v1/v2 retain their prior reader. This host still has incomplete Cargo/SDK input
registration; installing this transport does not activate full governance.

## Explicit consumer closure bindings

Config schema v3 may carry an optional `input_closure.bindings` whitelist. Each
binding has a unique `id`, a `consumer`, a non-empty caller-defined `kind`, and a
non-empty `inputs` list:

```json
{
  "id": "registration-git-facts",
  "consumer": "judge:registration",
  "kind": "git-facts",
  "inputs": ["tool:facts-git", "input:facts-git-bytes", "environment:PATH"]
}
```

The consumer must already be a registered `project`, `script`, `test`, or
`judge` node. Each input must already be a registered `tool`, `input`, or
`environment` node. The FILEMAP `project_edges` table must also contain an
explicit edge from every listed input node to that consumer. The edge `kind`
is checked by the FILEMAP schema independently and need not equal the binding's
caller-defined `kind`. Registration reports `E_REFERENCE` for an unknown node,
an invalid node kind, or a missing input-to-consumer edge. Config v1 and v2
reject the field, preserving their historical schemas.

`declared-complete` is the trusted AI's accountable engineering declaration of
the host-defined governance and input scope. Judges check the declared references,
identities, endpoint snapshots and observed operations. Known missing inputs,
unresolved required dependencies, missing bindings or snapshots, and drift still
fail; the declaration cannot clear them or replace a producer's result. Real
toolchain, SDK and configuration obligations within that scope remain.

When schema v3 declares the closure `declared-complete`, the `bindings` field
is mandatory and every registered `tool`, `input`, and `environment` node
must occur in at least one binding. This is a fail-closed inventory consistency
check; it still does not prove that the host has no unregistered external
input. Current registration reports retain `completeness_proven: false`.
Ordinary full check can pass with this value once its registration requirements
and selected obligations pass: that DELTA satisfies the active registered
contract. Ordinary acceptance requires neither a universal proof of hidden-input
absence nor a VM. This host's declarations remain proposed/incomplete.

The separately owned proposed Mac policy at
`.chrono-harness/native/macos/config.json` now binds supported native inputs from
run 38697657077/job 116297942471 retained originals. Its original input IDs and
consumer scopes are preserved, with independent native executable, interpreter,
toolchain, package and environment bindings. Two exact directory inventories,
19 external configuration inputs and three tool-version observations remain
unavailable; required nulls still fail their consumers. Native lineage, endpoint
composition, positive guard and activation are not produced by initial
observation. See the [exact source handoff](full-ci.md#mac-source-bindings-from-initial-acquisition)
for provenance, current consumer boundaries, caller acceptance and costs. The
custom local policy and unresolved Linux producer remain separate.

Local and CI must use the same registered check command. Equal verdicts across
environments additionally require equal complete effective inputs and deterministic
evaluation. The separate runner parity comparator requires
`completeness_proven: true` and complete verdict-bearing observations; registration
does not produce that stronger evidence. Without it, parity remains unestablished
without becoming an ordinary full-check gate. A matching pair of observations
alone does not prove universal determinism; do not handwrite the stronger value.

Config v3 may explicitly adopt a versioned `input_closure.coverage` contract:

```json
{
  "schema": "chrono-input-coverage/v1",
  "required": [{"consumer": "judge:registration", "domains": ["git-facts", "remote-response"]}],
  "rows": [
    {"consumer": "judge:registration", "domain": "git-facts", "state": "bound",
     "bindings": ["registration-git-facts"], "reason": "bound Git bytes and environment"},
    {"consumer": "judge:registration", "domain": "remote-response", "state": "not-applicable",
     "bindings": [], "reason": "this consumer reads local objects only"}
  ]
}
```

`required` lists a nonempty scope of executable consumers and their nonempty,
unique domains. Every consumer/domain pair must have exactly one row; missing,
extra or duplicate rows fail. The domain names are host-owned, such as compiler
libraries, SDKs, delegated tools, configuration, network, clock, randomness or
concurrency. They are never inferred from a language, directory or command.
Every row needs a nonempty reason. `bound` and `absent` require nonempty binding
references to the same consumer, with nonempty input lists. Bindings may serve
more than one domain, but cannot repeat within a row. `absent` additionally
requires that every referenced node be an explicitly absent file input; the
existing retained/current input checks verify its actual absence. A credential
file appearing at an explicitly absent path therefore fails even when the domain
row has not changed. `not-applicable` and `unresolved` cannot list bindings.
Not-applicable is an AI declaration of applicability, not an observed absence.

When coverage is adopted and the closure is `declared-complete`, all declared
bindings must occur in at least one row and no row may be unresolved. Incomplete
configurations may retain unresolved rows but still cannot pass activation.
A successful registration response emits `input_coverage` with schema
`chrono-input-coverage-result/v1`, the original declaration, scope
`declared-domains`, and `completeness_proven: false`. This is inventory consistency
and declared-file state validation; it is not proof of undisclosed reads or
complete OS/network/toolchain inputs.

Omitting coverage preserves the existing v3 contract and report shape. V1/v2
reject the extension. New consumers read historical v3 snapshots without adding
coverage to them; older binaries reject the unknown field. AI adoption upgrades
the selected binary and adds its explicitly chosen scopes and rows together.

This is an explicit relationship table, not a dependency discoverer. The
registration judge never scans directories, manifests, language files, commands,
or toolchains to add bindings. A valid binding records what the AI registered;
it does not prove that the real host has no omitted input. `input_closure.status`
and `unresolved` describe the accountable declaration and its outstanding work;
Cargo/compiler/SDK inputs still require their applicable registrations and tests.

V3/v4 may factor repeated literal binding vectors into `input_closure.input_sets`,
an object from a nonempty name to a nonempty, duplicate-free input-node array.
A binding then carries `input_set: "name"` instead of `inputs`. Exactly one is
required; missing names, nulls, recursive aliases and mixed forms fail. These
aliases stay in the original endpoint configuration bytes. Registration resolves
them at its existing binding, coverage and required-input consumers, with the
same explicit FILEMAP edge and absence requirements as inline vectors. No edge,
dependency, test or completeness claim is generated. V1/v2 and older binaries
reject the extension. This host uses it for repeated local declaration vectors;
the five full registries remain proposed.

## Actual input observations

`chrono-inputs observe --host-root H --manifest P --output P` consumes an explicit
`chrono-input-observation/v1` manifest. It runs bounded `operations` and optional
`registered_operations` from the exact project/script action owner, retaining the
original manifest, projects bytes, launch records and stdout/stderr. Missing or
ambiguous actions fail before execution. Literal file/directory locations or one
successful operation's single-line path output select observations; no operation
or dependency is inferred. Directory traversal has an explicit entry bound and
does not select tests. Directory results use `chrono-input-directory-observation/v1`
and are not directly usable as guarded directory declarations. An observation is
data for adoption, not an expected-input
update, historical endpoint capture or governance verdict.

`registered_operations.operation_bounds` optionally maps selected operation IDs
to their own `{timeout_seconds, output_limit_bytes}`. Omitted IDs use the existing
registered defaults; unknown IDs and invalid bounds fail before any child runs.
The optional manifest `stop_on_failure: true` stops the ordered acquisition at
its first failed operation or observation. It retains that failure and every
executed original, without launching subsequent consumers. Omitting it preserves
the existing independent observation behavior. This is ordered acquisition, not
test selection or a scheduler.

Actual failure, timeout, output overflow and inventory errors retain a failed
report and raw available streams. A second invocation cannot overwrite an
original output or process directory. Current observation results use
`chrono-input-observation-result/v2`: process streams reference their already
retained stdout/stderr/launch files through the existing original contract.
`scoped_report::restore_process` verifies these references, including the launch
record; a retention error preserves the inline receipt and fails observation.
The CLI prints a bounded `chrono-input-observation-reference/v1` containing
`status` and `original: {path, sha256}`. Consumers read and verify that original;
they do not treat an isolated status prefix as acceptance. Historical v1 report
bytes remain original. No compression or stream budget change is involved.

An optional `platforms` list admits only the listed native OS-architecture pairs.
`program: {stdout: OPERATION_ID}` selects an executable from a preceding successful
single-path output. `registered_operations.tools` accepts the same selection;
its optional `environment` maps variable names to preceding path outputs, allowing
the actual selected `RUSTC` to reach Cargo. Suffixes, missing or later producers,
failed path observations and reserved ownership variables reject. This is an
explicit acquisition chain, not input inference or expected digest generation.

### Native producer repair (source only)

**Product and acquisition policy change together.** File declarations may now
explicitly select `resolution: "canonical-target"`. Only that acquisition row
resolves an existing target; its result retains the original `location`, the
canonical `resolution.location`, and traversed symlink paths, literal targets and
actual file/directory kinds. Broken links and resolution failures remain failures,
not absence. Omission preserves strict regular-file/absence validation. The Linux
OS-release and compiler-reported libc rows select this mode; shared snapshot,
registration and Cargo file validators retain their rejection of symlinks and
non-normal paths. A logical alias observation does not itself authorize a later
strict input binding at the alias location.

An explicit directory row may additionally declare
`manifest: {"path": ".chrono-harness/state/guard/sdk.json", "root": "/declared/sdk"}`.
The existing inventory traversal produces `chrono-input-directory/v3` at that
state path and references its original bytes from the directory observation.
The declared manifest root must resolve to the selected inventory scope. The
manifest contains all physical file hashes, each physical directory's exact
direct child names (including empty directories and the root row `"."`), and
actual relative symlink targets and file/directory kinds. Traversal does not
follow aliases to add files. Escaping, absolute, broken or unsupported aliases
fail this production; no manifest is reconstructed from an old flat observation.
Legacy observation rows retain their existing schema and alias shape, sharing
the same traversal. Entry bounds apply during enumeration, before collecting
an oversized directory; publication reuses the existing atomic, no-clobber
state-output owner. Existing manifest paths reject before child execution.

Rejected manifest aliases now retain a failed directory row in the original
observation report. Its `location` preserves the selected root spelling and
`root` records the canonical inventory scope. `error` preserves the existing
rejection; `alias_failure` records the root-relative alias `path`, literal
`target`, `literal_absolute`, `canonical_destination` and `within_root`. Failed
canonical resolution leaves the last two fields null and retains the exact
`resolution_error` message and raw OS error code. `target_kind` records an
`observed` file, directory or other kind from metadata at the canonical
destination, or an honest `unavailable` result with its metadata error/code
(or the reason that canonical resolution failed). Resolution and metadata are
sequential observations, not an atomic filesystem snapshot. The failed row
has no manifest reference or partial file inventory. Successful observation
and v3 manifest shapes, rejection predicates, entry bounds and guard acceptance
are unchanged; no new schema, platform or acquisition policy is introduced.
Existing report/source/process originals and no-clobber publication remain the
evidence owners. The extra failure-only metadata read and retained facts have
unmeasured cost.
It cannot recover the unobserved target of the old native `usr/bin/altool`
failure; fresh native acquisition is still required.

The Mac declarations produce both guard originals at
`.chrono-harness/state/native/macos/guard-inputs/{sdk,developer}.json`. The SDK
root and full `/Applications/Xcode_26.6.app/Contents/Developer` root match the
two existing Cargo guard scopes; the older toolchain-only observation is also
retained. Both native declarations select Python's configured `stdlib`,
`platstdlib`, extension directory and shared runtime image separately from its
interpreter. The `python.root` operation now means `sysconfig`'s stdlib, rather
than `sys.base_prefix` (which is `/usr` on the failed Linux acquisition). Framework
and static interpreter builds retain their explicit runtime-image ownership;
missing configuration fails without falling back to capturing a prefix tree.

Both manifests explicitly acquire the 18 previously missing parent/home
configuration and credential-file presence/digest observations, the private
Rustup `settings.toml`, and selected linker, Rustup and cargo-fmt version outputs.
HOME, its parent and the private Cargo/Rustup homes are selected by the existing
Python probe; root paths and configuration suffixes are declared literals.
Configuration acquisition records absence or hash/length only: credential
contents are neither printed nor copied into retained transport artifacts.
These are observation rows, never handwritten expected hashes or successes.

Rust regressions live in the existing inputs-tests inventory; the existing
Cargo guard tests consume the exact observer-produced original and exercise
file, child-list and alias drift. Their only added dependency is the existing
local inputs production project, with its lock and FILEMAP compile edge paired. Python runtime selection has Python behavior
coverage in the existing host-bootstrap-tests script. Existing project/script
actions, test inventories, execution plans, scheduling caps and native bounds
are retained. Costs are unmeasured: full Developer traversal and the retained
legacy subtree overlap; separate Python components may overlap as well. Each
root still has its original 200000-entry bound; failure is preserved without
raising it or narrowing the declared guard scope.

This source repair has not been executed here. Native topology admission,
positive guarded consumption and full activation remain future work. The finite
extraction retains the Mac policy pair as identical, explicitly owned observation
contract data; unselected native deployment declarations retire as described in
[the host boundary](full-ci.md#main-host-adoption-inputs). Original native refusal,
missing Linux facts and earlier evidence retain their scope. No full-native input
success is manufactured from these declarations.

Main-host acquisition uses existing registered operations:

| Operation | Manifest | Actual consumer |
| --- | --- | --- |
| `inputs.acquire-macos` | `.chrono-harness/inputs/acquire-macos.json` | Ordered delegation to `inputs.provision-native`, then `inputs.observe-macos` |
| `inputs.acquire-linux` | `.chrono-harness/inputs/acquire-linux.json` | Ordered delegation to `inputs.provision-native`, then `inputs.observe-linux` |
| `inputs.provision-native` | `.chrono-harness/inputs/native-provisioning.json` | Existing `inputs.fetch.*` actions, with network acquisition explicitly allowed |
| `inputs.observe-macos` | `.chrono-harness/inputs/native-observation.json` | macOS arm64 compiler, SDK, Python and offline Cargo metadata/fetch observations |
| `inputs.observe-linux` | `.chrono-harness/inputs/linux-observation.json` | Linux x86_64 compiler, header/runtime, Python and offline Cargo observations |

The first production entry is `host.acquire-native-macos` on macos-26 arm64 or
`host.acquire-native-linux` on Linux x86_64. The existing `host-bootstrap` Python
owner executes the action in projects.json, selecting
`.chrono-harness/ci/bootstrap-acquire-PLATFORM.json`. For example, the macOS action
is `/usr/bin/python3 -B .chrono-harness/ci/bootstrap.py .
.chrono-harness/ci/bootstrap-acquire-macos.json`; Linux selects the `linux` file.
These are initial acquisition profiles, separate from the full-unit shared
startup profiles. Both retain the original 14 source build operations and 14
install destinations. Their final operation, before installation, is the existing
`inputs.acquire-PLATFORM` action. It executes the just-built
`crates/inputs/target/debug/chrono-inputs`; its nested observer delegates the same
built binary. No already-installed observer or direct full policy is required
by this first producer. Relative tool paths resolve against the declared host
root; shared validation binds them to the recorded producer root.

The genuine first caller inputs are a clean committed candidate source in an
actual native Git **main** checkout, the registered source/profile/projects and
official toolchain-registry bytes, `/usr/bin/python3`, `/usr/bin/git`, a working
`rustup` in PATH, and the platform's native compiler/linker, SDK or system
headers/libraries and runtime. PATH/HOME/TMPDIR, applicable SDK selections and
network access to the official Rust distribution and locked Cargo dependencies
must be supplied by that actual job. The original Cargo build actions use their
default native target/output locations; conflicting target-directory or build
target overrides cannot supply the declared built observer. The bootstrap's
existing compiler/wrapper checks still reject conflicting selections.
Rustup itself and the native OS/SDK are
external prerequisites, not products of these Cargo builds. The bootstrap
delegates `rustup toolchain install 1.95.0 --profile default --no-self-update`
using disjoint checkout-relative homes under
`.chrono-harness/state/native/PLATFORM/{rustup,cargo}`. The existing official
Cargo/rustc byte/version bindings still apply; no global Rustup selection or
expected digest is changed.

`bootstrap.participate` already permits a Git main whose worktree-owner binary
is absent to seed its tools. In that branch it does not open the referenced
native lifecycle policy. Acquisition now runs inside that same invocation,
before installing the owner, without consuming proposed native full/Cargo/cache
policies. The retained native worktree/cleanup policies serve existing-owner
participation. This seed branch creates no birth, enrollment or
full admission. An existing owner or linked checkout still goes through the
original participation path and needs genuine compatible committed policies and
enrollment/leases; a refusal never falls back to seeding. Do not remove an owner
or set an ownership marker to manufacture this initial condition. This source
staging checkout is not a caller native acquisition host or a linked birth.

The existing observer executes provisioning then platform observation and stops
on failure. The delegated actions remain their sole implementation. A failure
prevents the bootstrap install phase and retains its failed original report and
operation streams. These operations have fixed manifest/output paths in
projects.json; a preexisting observation is preserved and refuses reentry.
Retain and hand off the original job and any existing lifecycle consumer before
the caller selects a genuinely new production location. Inner acquisition limits
remain 120 seconds and 1 MiB. Each
delegation of a complete observation uses the existing observer's 900-second
maximum and 1 MiB console bound; elapsed acquisition cost is unmeasured. The
observer selects actual Cargo/rustc through `rustup which --toolchain 1.95.0`
within those private homes, retains those selections and supplies that compiler
path to Cargo. Linux metadata actions explicitly use the Linux target.

These manifests collect evidence; they do not activate native policies. Native
Git, compiler/backend/linker, SDK/system libraries, interpreter, Cargo package and
configuration/ancestor bindings must still be reviewed against the actual
originals. A sysroot/path probe is not a complete runtime inventory. Missing
packages remain failures; observations do not replace fixed endpoint snapshots.
The Cargo observation declaration contains test expectations, not recorded
successes. Local tool/SDK declarations do not establish native full-input expectations;
scoped CI retains their explicit root graph as declaration data only. The
remaining adoption bindings are specified in [full CI](full-ci.md#main-host-adoption-inputs).
The new acquisition manifests retain their outer originals under
`.chrono-harness/state/native/{macos,linux}/acquisition.json` and its
`.processes` directory. The inner original provisioning/observation reports and
their streams remain under `.chrono-harness/state/inputs/`; retain both declared
closures. Also retain `acquisition-bootstrap.json` and its sibling
`bootstrap-*/report.json`/operation streams under the platform state directory.
The caller must retain the initial job's original stdout/stderr and exit even
when a missing interpreter, Git, Rustup, SDK or early provisioning failure occurs
before a bootstrap report can be published. The compact observer stdout is a
reference: verify its addressed original and stream digests, not merely its
status. Observed file/directory digests are adoption data; endpoint capture later
retains the actual declared file blobs at each genuine checkout. Reentry
preserves originals and fails. No acquisition or test was executed for this
source handoff.

The existing native acquisition composite also transports the exact 14 installed
executables declared by both acquisition profiles, alongside these state roots.
Bootstrap remains their producer and records actual installed hashes; caller
acceptance requires the original bytes for each claimed installation. The upload
uses explicit install paths, excludes private Rustup/Cargo homes and build targets,
and retains available failure evidence even when no installation occurred. Restore
the artifact at `.chrono-harness/` to preserve both `bin/` and `state/`. Its
seven-day retention, caller custody obligations, failure limitations and unknown
storage/execution costs are owned by the [native CI transport](ci.md#candidate-native-initial-acquisition-entry).
This source correction does not supply native observations or endpoint originals.

```sh
chrono-inputs capture --host-root /path/to/host \
  --config .chrono-harness/config.json --commit FULL_COMMIT_OID \
  --output .chrono-harness/state/input-candidate.json

chrono-inputs pair --host-root /path/to/host \
  --base-snapshot .chrono-harness/state/input-base.json \
  --candidate-snapshot .chrono-harness/state/input-candidate.json \
  --output .chrono-harness/state/inputs.json
```

Capture requires that exact commit to be checked out and the config's disk bytes
to match its fixed blob. It reads the original registry values at that commit;
it is not a dirty-check or a passed governance report. Every observed regular file is
copied with a bounded buffer into `.chrono-harness/state/inputs/blobs/<sha256>`.
Equal contents share one retained blob. Existing content addresses are verified
before reuse. V1 missing/nonregular inputs and invalid paths fail without publishing
a snapshot. V2 can retain an explicitly observed absence; nonregular files still fail. Source bytes are never reconstructed from today's files during pair.

A config v1 snapshot has schema `chrono-input-snapshot/v1`, `commit`, `config_path`, the
canonical config-value `config_digest`, `environment` and `files`. Environment
values preserve strings, empty strings and absent/null; configured overrides remain
in the config, and undeclared ambient variables are not captured. Each file ID maps
to `{blob, sha256, length}`. These are observed values: a changed file can be
captured while its expected registry hash stays unchanged, and the judge then
rejects that mismatch. Snapshot success only means capture completed.

Pair preserves both explicitly supplied snapshots. Optional `--base-root` and
`--candidate-root` select their source hosts; both default to `--host-root`. It
copies the referenced retained blobs into the destination's state and verifies
their identities. It never reads historical external input locations or infers
their new locations. The snapshots and blobs can also be transported together as
CI artifacts. Original base bytes must remain available; a missing base blob is an
error even when current candidate bytes exist.

Snapshot and pair output paths must lie under `.chrono-harness/state/`. Repeating
identical output is a no-op; different output at an existing path fails with
`E_SNAPSHOT_EXISTS`, preserving the old evidence. Choose a new explicit path for
new observations. Blob and output references reject symlink components and path
traversal. Ordinary IO uses temporary files and publication without clobbering;
power-loss recovery and concurrent mutation of source files remain outside this
contract.

Set context `retained_inputs` to the produced pair path. Registration accepts
either the existing inline `{bytes: [...]}` representation or exactly one
`{blob, sha256, length}` reference. It checks the snapshot's endpoint/config binding,
declared file IDs, original blob digest/length and current candidate disk identity.
Full unit capture and live validation use the same explicit unit input projection;
unrelated retained entries may be omitted, while supplied entries still undergo
validation. Unscoped checks and collection retain global input obligations.
Collection validates original blobs and compares original per-unit evidence with
the corresponding projection, without reading live business inputs or launching
business/version tools. Structural and reference checks remain global.
Unknown file IDs, conflicting representations, missing/corrupt blobs and changed
candidate inputs fail. Blob bytes are streamed and stay outside judge JSON; the
effective-input result is the same for matching inline and blob representations.
Reports still state `completeness_proven: false`: hashes bind the declared inputs,
and do not prove no actual input was omitted.

Dedicated tests cover exact binary/environment preservation, immutable outputs,
deduplication, transport after the original input disappears, corrupt addresses,
invalid paths and a real seven-judge consumer. Large-file and rejection cases
exercise the actual check entry rather than constructing success reports.

Config **v2** requires each input to declare either
`{id, location, presence: "present", sha256}` or
`{id, location, presence: "absent"}`. Absence has no hash field. A null hash still
means unbound, never absent. Capturing v2 produces `chrono-input-snapshot/v2`:
regular files keep the blob format; a missing input becomes `{absent: true}`.
An empty file has its own blob and length zero. Capture observes actual state even
when it contradicts the declaration; registration rejects that contradiction.
V2 requires the versioned snapshot, exact config binding and explicit state for
every ID. V1 snapshots and unversioned legacy observations cannot carry absence.

When the capture entry is a native platform selector, it publishes
`chrono-input-snapshot/v3`. The existing `config_path` remains the caller's entry
identity; `effective_config_path` and `selection` bind the selected direct policy.
The structural selection binding is `chrono-registry-selection/v1` and carries the
entry path, selected target path and host platform; the selector digest remains in
the fixed registry bytes. Registration checks this binding against the fixed endpoint
snapshot and keeps the selector and target registry bytes under their real paths.
Direct v1/v2/v3 configs
retain their previous snapshot schemas.

For a new or renamed candidate unit, capture both endpoints with optional
`--base BASE_OID --candidate CANDIDATE_OID` alongside `--commit` and `--unit`.
The commit must be one of those exact endpoints. This uses registration's
existing two-endpoint projection, including historical requirements when the
candidate unit ID was absent at the base. It still observes only the checked-out
endpoint; it never substitutes current input bytes for historical observations.

`chrono-inputs compose --host-root H --config P --base BASE_OID --candidate
CANDIDATE_OID --manifest M --output O --receipt R` joins explicitly addressed
original pairs offline. Its bounded manifest has schema
`chrono-input-composition/v1` and `sources`, each with a `pair: {path, sha256}`,
optional `transport: {source_directory, directory}`, and optional original
report `artifacts` map. Sources are never discovered. At most 4096 sources are
accepted; JSON inputs retain the 64 MiB bound. Blob copying and identity checks
stream bytes through the original descriptor/transport contract.

Composition checks each snapshot's exact endpoint, config, selector and
environment binding. Compatible overlapping observations merge; contradictory
presence, digest, environment or config rejects. Both endpoints must cover every
globally required declared input, including governance requirements outside the
selected business tests. Missing coverage names the endpoint and input. No live
business input, version probe or SDK install is used. A source's `pair` address may
also name one genuine endpoint snapshot, captured before another endpoint exists.
Its commit chooses the fixed endpoint; both endpoints still require complete
coverage from the explicitly listed sources. Original snapshot/pair bytes remain
addressed in `.chrono-harness/state/inputs/originals/`; a separate
`chrono-input-composition-result/v1` receipt names the manifest, constituents
and output. Snapshot v2/v3 shape stays unchanged and the receipt reports
`completeness_proven: false`.

`chrono-inputs capture-governance --host-root H --config P --base OID --candidate OID
--manifest P --output P` consumes the same original-source manifest for the base
governance projection and separately observes the checked-out candidate. It never
captures candidate bytes as historical base data. Required governance inputs come
from registration's existing empty-business-plan projection. The resulting pair,
streamed blobs and `OUTPUT.receipt.json` retain source identities and raw originals;
missing base observation/coverage or endpoint/configuration/presence/blob mismatch
rejects. Native publication passes the provider's explicit `composition_sources`
through this command before publishing its seed. Hosts provision those originals
through their registered bootstrap; daily check commands remain unchanged.

Pair transports absence unchanged without opening its historical location.
Registration, routes and projects consume the same validation, including a final
candidate check after operations. Effective reports use `chrono-effective-inputs/v2`
when either endpoint uses config v2; v2 file facts carry `presence`. Present facts
also carry digest/length, absent facts carry neither. State changes require matching
registration changes and follow only explicit `input:ID` edges, including old edges.
Stable absences and disconnected changes do not select unrelated tests.

V2 locations reject symlink components, including dangling links, and `..`.
Use canonical absolute locations when a platform exposes temporary directories
through a symlink. A missing parent directory is a valid absence; a directory at
the file location, a non-directory ancestor, permissions and other IO errors are
not. Observations before/after execution detect persistent changes, not transient
concurrent changes restored between observations.

The Cargo guard retains connected declared absences and rechecks them after the
actual operation. An absent ID cannot supply a package manifest, configuration
file, package bytes or a tool. Configuration lookup inventories and their
relationships are still explicit declarations; this does not infer them.

Both endpoints may already use v2. Workflow v3 can select a host-registered v1→v2
decoder by the exact declared endpoint schema versions; see [the decoder contract](execution.md).
Its dedicated compatibility test must execute and pass. Historical config and v1
snapshots remain original; unbound digests cannot acquire absence semantics.
The runtime does not generate a host's config conversion algorithm. This implementation does
not activate the repository's proposed full configuration or establish complete
compiler/backend/linker/SDK/build-script inputs.
