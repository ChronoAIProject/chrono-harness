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

Set context `retained_inputs` to the produced pair path. For ordinary local full
checks, the optional worktree `check_inputs.full_inputs.retained_inputs` binding
lets the existing producer retain and reference that output without editing the
birth origin; see [the worktree owner contract](worktree.md#current-full-check-input-references).
Registration accepts
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

## Product-host source declarations

The product host now has explicit local input IDs, consumer bindings and FILEMAP
paths for its 34 Cargo roots. Registration records the extracted package files
and the actual offline resolver archive/index bytes, compiler and formatter
executables, rustc driver/sysroot libraries, linker/delegates, SDK inventories,
configuration presence/absence and the selected environment. Shared package
inputs follow the existing explicit compile owners; redundant direct paths were
removed while retaining the same input reachability. No executor uses directory
or Cargo metadata discovery to select dependencies or tests.

These declarations are currently local macOS observations. Their absolute
locations do not bind a native runner, and directory manifests do not substitute
for original endpoint snapshots. Required native provisioning, runtime-platform,
retained-endpoint and non-Cargo/governance scopes remain explicit unresolved
work. Candidate artifact publication/binding and actual full plan/native
activation remain subsequent steps. The current fixed command still uses scoped
CI, and the full registrations remain proposed/incomplete.

P2 extends this whitelist to the existing two Python projects, six Python
scripts and seven full judges: 49 executable consumers in total, retaining the
34 Cargo roots. The local Python inventory lists the selected Xcode Python
3.9.6 framework/executable and 783 standard-library files. Those files are also
individual declared inputs with explicit consumer edges; the directory document
does not discover dependencies at execution time. Enforcement binary inputs
retain P1 beta.21 byte expectations separately from current source outputs.
Local `SystemVersion.plist` and `dyld` bytes bind only the `runtime-identity`
domain. They leave `runtime-platform` unresolved because applicable shared-cache
and delegated runtime inputs are not complete. Script delegated tools, native
provisioning and original endpoint evidence remain unresolved, independently of
these new bindings. No status is promoted by this source step.

Before changing these declarations, the inputs owner captured the clean fixed
`e21ded0dfa914db65d09479456205c3a8428bdad` local endpoint at
`.chrono-harness/state/p2/e21-local-original.json` (SHA-256
`375123c974e8f1952382945c5e5010194116c93553a98f10da7f27e54238c895`).
All 3,181 original file observations matched that endpoint's declarations;
655,422,063 present bytes were retained through content-addressed blobs.
This original predates the new declarations. It cannot supply unobserved native
identities or new candidate inputs. Caller must preserve the snapshot and its
blobs, capture a clean committed candidate, and provide original native base
observations through the existing composition sources. Capture success is not
judge-chain acceptance or past business execution.

The selector owner now supports separate `local`/`ci` declarations on the same
OS/ARCH via `chrono-git-configs/v2`; snapshot v3 retains the source binding.
The canonical host has not adopted that selector or a full route. Provisioning
an actual native policy must precede capturing its fixed base: a later selector
cannot rewrite e21's direct, local policy or manufacture a historical native
observation. Missing native originals require a coherent committed provisioning
endpoint before the later activation candidate.

This source flight changes product adapters and host policy together. Its actual
validation includes the Cargo pair and main native check, all three complete
group inventories, formatting and host bootstrap/projection checks. A clean
committed canonical run and native unit/collection acceptance remain caller
obligations. None of these declarations asserts universal deterministic parity.
