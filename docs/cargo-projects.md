# Optional Cargo project judge

`judge-cargo` / `judge-cargo-tests` is an independent production/test pair. Generic
registration and projects have no dependency on it or on TOML. A host adopts the
adapter by registering its binary, arguments, policy input and FILEMAP edges. A
Go/TS/script host has no Cargo policy obligation merely because Cargo-like files
or directory names exist.

Public beta.16 distributes the v5 toolchain inventory described below. Its native
release executes this adapter's dedicated tests on macOS arm64 and Linux x86_64;
this does not certify complete compiler/SDK/delegated inputs or activate the product host.

The policy must be below the host `.chrono-harness/` and registered in FILEMAP:

```json
{
  "schema": "chrono-cargo-projects/v1",
  "projects": [
    {
      "project": "library",
      "manifest": "units/library/Cargo.toml",
      "lockfile": "units/library/Cargo.lock",
      "output": "outputs/library/",
      "ancestor_manifests": []
    }
  ]
}
```

Each row explicitly maps a registered project to owned manifest/lock inputs and
an already registered generated-output directory. Generic project fields are not
consulted for Cargo meaning. The output path is not inferred from a source root.
`ancestor_manifests` explicitly lists the registered Cargo ancestors that the
adapter must inspect. It verifies this inventory against registered ancestor
paths; a missing declaration is an error, never an automatically added input.

The bounded check rejects workspace aggregation/association, workspace dependency
inheritance, missing manifest/lock ownership, missing output ownership and path
dependencies without a unique declared Cargo project and explicit FILEMAP compile
edge. Normal, dev, build and target-conditioned TOML dependencies are checked.
Registry/git dependencies fail in this bounded structural check. Adopt the
separate guarded operation below to validate their retained inputs and actual
resolution before running Cargo. Neither mode certifies compiler/backend,
build-script, linker or SDK input completeness.

Full judge binding uses:

```text
chrono-judge-cargo --protocol chrono-judge/v1 --policy .chrono-harness/cargo/projects.json
```

Register it after `registration` and `filemap`, and before operations by making
`projects` depend on it. The full adapter validates request identity, registration
views, retained inputs and committed policy bytes, then checks only Cargo projects
reached by the supplied explicit FILEMAP impact. Other languages are unaffected;
a documentation DELTA does not rejudge historical Cargo defects. Policy changes
need explicit edges to their consumers, as do manifest/lock/ancestor changes.
Errors retain their actual policy path; passing output lists checked projects and
explicitly does not certify complete inputs.

A host that maintains full registration data can instead register a single
ordinary prerequisite with explicit selection:

```sh
chrono-judge-cargo check --host-root HOST --config .chrono-harness/config.json \
  --policy .chrono-harness/cargo/projects.json --project library --project library-tests
```

That command never selects projects itself; its caller's FILEMAP plan decides
when to run it. Full protocol and standalone mode use the same check function.
The surrounding canonical runner still owns clean snapshots, tool binding,
operation receipts and failed-prerequisite handling. No Cargo subprocess is
launched by this bounded adapter.

## Guarded Cargo operations

Register this ordinary operation as the project's action and select it through
the existing FILEMAP execution plan:

```sh
chrono-judge-cargo run --host-root HOST --config .chrono-harness/config.json \
  --policy .chrono-harness/cargo/inputs.json --operation test.library
```

The generic runner invokes the registered guard binary. The guard owns the Cargo
recipe: it validates the policy and retained inputs, observes the configured Cargo
binary and version, runs locked/offline metadata, compares resolution against the
declarations, and only then runs the declared Cargo consumer. It checks input
identities again after metadata and after the consumer. The ordinary full and
scoped execution paths require no Cargo-specific callback or schema field.

The guarded `run` entry accepts direct host configurations and the existing
`chrono-git-configs/v1` and `v2` selectors through the shared registry acquisition
owner. The registered action still names the caller's entry path exactly.
`registry_binding` reports that entry, its effective direct policy and any
selection. Both files' original bytes, together with the acquired registries,
are checked before child execution and after metadata and the consumer. A
selector does not qualify another host's toolchain or inputs; unknown identities
and selection/binding mismatches fail before Cargo runs.

The guarded policy entry can also be `chrono-cargo-input-policies/v1`, with only
`schema` and `configurations`. The latter maps explicit effective configuration
paths to direct `chrono-cargo-inputs/v2`–`v6` policy paths. Selection uses the
already acquired registry identity; it does not probe the platform or choose an
available toolchain. Both paths must be literal host `.chrono-harness/` paths,
and both policy files must be registered and connected to the actual consumer.
Missing mappings/files, nested selectors, aliases, traversal and a direct-target
argument bypass fail. The registered operation continues to name one entry.
Additive `policy_binding` reports both original hashes and the effective config;
both originals are retained and checked after every child boundary.

Policy and product change together in this host's diagnostics adoption: the
old native policy bytes move to `cargo/diagnostics-native.json`; the stable
`cargo/diagnostics.json` entry selects that file for pinned native configuration
or `cargo/diagnostics-local.json` for macOS 27. The local policy binds the actual
selected Rust/SDK roots and the original inventory root through `manifest_root`.
Local declared input paths were relocated only after checking their existing
digests or absences. The differently valued rustix index retains its original
owner's exact bytes, and is not represented as the selected Cargo home's index.
These proposed declarations do not establish full input closure or parity.
This adds one bounded policy read and identity rechecks; total validation cost
remains unmeasured. The existing deadlines and guarded operations still apply.

The registered development build of the guard uses optimization level 1, as do
the other inventory judges. Large selected sysroot/SDK inventories still receive
every original file, directory, alias and boundary check; this changes compiled
execution cost only. The isolated producer retains its 600-second guard deadline.
Its actual enclosing consumer must establish bound-compatible performance on
the selected host; a build profile or small regression alone is insufficient.

Directory validation shares a physical directory-prefix check only within one
pass, and revalidates those parents at the end of that pass. Every declared regular file is still hashed, every declared child
inventory is still read, and every alias is still checked. A new pass starts
with an empty prefix set, so checks after each child still detect persistent
parent replacement, file mutation and alias drift. This is no filesystem
snapshot guarantee: changes restored between observations remain outside the
existing contract. The optimization removes repeated metadata reads for shared
parents; it does not cache file identities across validation boundaries.

Each directory pass hashes its declared files in two bounded threads, with a
separate prefix set in each thread. Both threads finish before validation can
return or any guarded child can run; an error in either thread fails the pass.
Their merged physical parents receive the same final recheck. The selected host
declares 14,466,038,905 logical file bytes per complete pass, including overlapping
inventories. This concurrency bounds the hashing work without omitting bytes,
changing expected digests or retaining identities between boundaries. Its actual
enclosing performance still requires host measurement.

This host registers `test.judge-cargo-isolated-observation` as an explicit
experiment in the existing Rust test owner. It invokes the unchanged registered
`inputs.provision-isolated` action verbatim under the experiment's existing use
protection, through the existing retained process
engine, with a stricter 300-second observation window so the current terminal
can join it. A timeout of that observation is inconclusive for the production
600-second deadline. The portable suite leaves this host inventory experiment
ignored; its separate registered action selects it explicitly. Original streams,
the process result and elapsed time are retained under
unique `.chrono-harness/state/input-q1-repair/representative-retained-*/` directories for Q1 acceptance and
caller handoff, after which the caller owns their retention/cleanup. This adds
no worker, polling or transport implementation. Total validation/storage costs
remain unmeasured until execution.

The related `test.judge-cargo-guard-observation` diagnostic consumes the existing
`guard.check.runner` action verbatim under the experiment's use protection. It
uses the same selected input domain and a 300-second observation window, and
retains partial phase stderr through `run_process_observed` under
unique `.chrono-harness/state/input-q1-repair/representative-guard-*/` directories. This is guard
diagnosis only; it cannot replace the enclosing provisioning consumer's verdict.

The configuration policy has schema `chrono-cargo-inputs/v3` and these required fields.
The fixed v2 inventory contract remains supported with its original absolute
`CARGO_HOME` requirement and no ancestor-policy choice.

| Field | Contract |
| --- | --- |
| `root` | An explicit package ID whose `project` is the action owner |
| `projects` | Cargo project rows with the same shape as the structural policy above |
| `metadata` | `{tool, argv}` for the registered Cargo tool and metadata invocation |
| `operations` | Map from registered operation ID to its actual Cargo `{tool, argv}` |
| `target` | Explicit target triple shared by metadata and the consumer |
| `configuration_files` | Explicit `{path, input}` rows for root/Cargo-home lookup, argument and recursive include paths, plus ancestors when using `inventory`; `input` is a registered file-input ID, or explicit `null` for absence |
| `configuration_ancestors` | v3 requires `inventory` or `absent`; v2 uses only its original literal inventory |
| `packages` | Complete declared package resolution for this root and configuration |
| `timeout_seconds`, `output_limit_bytes` | Per-child limits; positive, output at most 64 MiB |

The guard invokes Cargo from the explicit host root. Declare both `.cargo/config`
and `.cargo/config.toml` at that root, plus `config` and `config.toml` inside the
explicitly configured `CARGO_HOME`. v3 accepts an absolute home or one resolved
relative to that same host root; it does not use the guard caller's cwd or infer
HOME. With `configuration_ancestors: "inventory"`, also declare both lookup
files at every strict ancestor of the root. Also declare
each ordered `--config` argument and each recursive `include` target. Paths may be
host-root-relative or absolute; their resolved locations must match the registered
inputs. An absent path still needs a row with `input: null`; omitting the field is
an error. Optional absent includes follow the same rule. These are host-owned
configuration declarations, never inferred FILEMAP edges or test selection.

With `configuration_ancestors: "absent"`, the host instead asserts that both
lookup files are absent at **every strict ancestor**. The guard checks that
explicit universal assertion and rejects any present file, symlink or overlapping
inventory row. There are no implicit exceptions for registered files; hosts that
need ancestor configuration select `inventory`. This choice avoids embedding the
checkout depth and ancestor names in an otherwise relative policy. It never
discovers or registers a present input. Root lookup, Cargo-home files, arguments
and includes still require their own explicit rows. The report's additive
`configuration.ancestor_absences` lists the locations actually checked, and those
absences are checked again after tool observation, metadata and execution. These
observations do not supply retained before/after endpoint snapshots, discover
unregistered semantic reads, or certify complete-input parity.

A present file requires a registered input with an expected SHA-256 and an
explicit dependency path to the consumer. The guard rejects missing, extra or
duplicate inventory rows, wrong presence or identity, undeclared includes and
include cycles. Symlink files or parent components (including a link before `..`)
are outside this adapter's supported path contract. Cargo's legacy `config`
takes precedence over `config.toml`; both presence/identities remain bound, but
the shadowed modern file is not parsed. The adapter validates include membership;
Cargo itself merges values and applies precedence. Present inputs and all declared
absences are rechecked after tool observation, metadata and consumer execution.
This detects persistent mutations across those boundaries, not writes restored
between observations.

For example, one inventory row is `{"path":".cargo/config.toml","input":null}`.
It does not stand for the other required paths. Migration from the earlier v1
contract replaces `configuration_inputs` with this complete explicit inventory,
retains existing input IDs/hashes/edges and updates the schema. v1 remains
rejected. To migrate v2 to v3, explicitly select the ancestor policy. `inventory`
preserves its rows; `absent` requires removing strict-ancestor rows and ensuring
the asserted files are absent. Relative home and input locations are explicit
host choices, not automatic rewrites. Existing v2 policies keep their meaning.
Full-host guarded adoption remains proposed; the finite scoped diagnostics
selection is described below.

Each package row contains `id`, `name`, `version`, `source`, `project`,
`manifest_input`, `inputs`, `checksum`, `features` and `dependencies`. Local
packages set `project`; their `source`, `manifest_input` and `checksum` are null
and `inputs` is empty. Their manifests, locks, source ownership and output
directories come from the explicit Cargo project rows and generic registration.
External packages set `project` to null and declare the exact Cargo registry or
Git source identity, including the Git revision. Their `manifest_input` names a
member of `inputs`, which lists every registered file in the actual package
directory. Registry packages require the lockfile checksum; Git packages use
null when Cargo.lock has no checksum. Input declarations retain explicit paths
and SHA-256 values in `config.environment.inputs`.

For example, a renamed, feature-enabled registry package can have:

```json
{
  "id": "dependency",
  "name": "fixture-dep",
  "version": "1.0.0",
  "source": "registry+https://github.com/rust-lang/crates.io-index",
  "project": null,
  "manifest_input": "dependency.manifest",
  "inputs": ["dependency.manifest", "dependency.lib", "dependency.checksums"],
  "checksum": "<exact Cargo.lock checksum>",
  "features": ["bonus"],
  "dependencies": [
    {"package": "leaf", "name": "fixture_leaf", "kinds": [{"kind": null, "target": null}]}
  ]
}
```

This is a shape example; the checksum and input IDs must be replaced with real
declarations. Dependency `package` refers to a declared package ID; `name` is
Cargo's resolved dependency alias. `kinds` preserves normal (`null`), `dev`,
`build` and any target condition. Feature sets and dependencies must match actual
metadata exactly. Package IDs are host-chosen; Cargo's location-dependent IDs
are mapped by exact name/version/source identity, never used to create edges.

Policy files, package inputs and configuration inputs need explicit compile,
build-input or runtime-input paths to the consumer in FILEMAP. Inventory checking
only compares real files with this whitelist: it never adds an input, dependency
or test. Unlisted files, missing files, unsupported filesystem entries, changed
digests, changed input locations, wrong lock checksums and disconnected inputs
fail. Actual metadata also checks package locations, registered target sources,
the independent workspace root/member and declared output directory.

Both commands must use the same registered tool, exact manifest and target,
locked/offline mode, feature selection and ordered `--config` file arguments.
`--config` order matters and is preserved. Accepted commands are `build`, `check`,
`test`, `run`, `bench`, `doc` and `clippy`; metadata requires format version 1 and
`--filter-platform`, while consumers require `--target`. The initial argument
grammar also accepts `--frozen`, `--features`, `--all-features`,
`--no-default-features`, `--release`, `--profile`, `--lib`, `--bins`, `--tests`,
`--all-targets`, `--examples` and `--benches` where applicable. Unlisted flags,
inline configuration assignments and arguments after `--` are rejected; supporting
them requires an explicit adapter extension. Cargo still determines whether a
particular accepted combination is legal and retains its real failure.

The registered outer action must invoke this guard with the exact root, config,
policy and operation arguments above. `chrono-cargo-run/v2` (or `v3` for a v5
toolchain policy) reports the observed
tool and actual metadata/consumer processes, including original stdout/stderr
bytes, hashes and exit status, plus the checked configuration inventory, input
IDs, digests, absent paths and parsed files. A rejected resolution retains successful metadata
and has no consumer receipt. A failing test retains its actual exit (including
101). Input changes after an otherwise successful test still fail the guard.
Process timeout/output failures are errors. Ordinary preflight/validation failure
returns 1; successful execution returns 0. The report is an operation result,
not a new judge-protocol success or a complete-input certificate.

Real fixtures cover renamed and transitive registry dependencies, feature and
dependency-kind mismatches, vendored fixed-revision Git dependencies, configuration
order, failed tests and post-execution mutation. Full and scoped runner fixtures
use the same registered action; documentation DELTAs leave unrelated historical
Cargo defects unexecuted. The dedicated `configuration`, `inputs` and
`input_consumer` tests cover Cargo-home/ancestor configuration, recursive includes,
legacy precedence, explicit absence, path aliases, mutation and full/scoped
rejection/nonexecution.

The product builds and distributes this optional binary. Its full host
registration remains proposed. Every guard report states
`input_closure_complete: false`: package metadata and retained declared files do
not close compiler/backend/linker/SDK, build-script reads or Cargo credential and
other service inputs. The configuration inventory covers Cargo 1.95 lookup and
include rules, not every input a Cargo child might consume. The remaining input
declarations, adoption and full native parity remain
required work. Existing core checks do not certify that missing scope.

## Explicit compiler binding

Hosts can opt into `chrono-cargo-inputs/v4` by retaining all v3 declarations and
adding `cargo_input` (an external input ID) plus `compiler: {tool, input}`.
`tool` names a distinct registered compiler tool; both input IDs must be present,
have expected SHA-256 values, and have explicit FILEMAP paths to the consumer.
The guard checks each resolved invocation path and its bytes against that input
before probing the version. Each tool requires its declared expected version.
The report retains the Cargo `tool` observation and the additional `compiler`
observation; v2/v3 retain their old contract and have no compiler-binding claim.
Those older policy schemas reject the v4 fields, including explicit nulls.

The effective host environment must explicitly set `RUSTC` to the compiler's
absolute input path and set both `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` to
empty strings. The guard passes that same environment to metadata and execution.
V4 rejects the three corresponding `CARGO_BUILD_*` aliases, and any parsed Cargo
configuration (including recursive includes) that declares those six environment
names or `build.rustc`, `build.rustc-wrapper`, or `build.rustc-workspace-wrapper`.
This contract gives compiler selection one declared source; it does not infer
which of several conflicting declarations a host intended. Configuration files
shadowed by Cargo's legacy-file rule remain unparsed, with their bytes retained.

V4 supports `build`, `check`, `test`, `run`, and `bench`; `doc` and `clippy` require
additional tool contracts and remain available through the original v2/v3 scope.
All retained inputs are checked after tool observation, metadata and the consumer.
An otherwise successful consumer that persistently changes a bound tool fails,
with the actual consumer receipt retained. Transient concurrent replacement is
outside the existing before/after observation guarantee.

This binds the executable supplied to Cargo. It does not close that executable's
own delegated processes, compiler libraries/sysroot/backend, linker, SDK,
build-script reads or other external inputs. `input_closure_complete` remains
false. The product host has not adopted v4. Dedicated direct and full/scoped tests
exercise actual Cargo compilation, selected-tool traces, before/after mismatch,
configuration rejection and documentation-only nonexecution.

## Explicit toolchain inventory

Hosts can opt into `chrono-cargo-inputs/v5` by retaining the v4 declarations and
adding a `toolchain` object. V5 does not discover a compiler environment. Every
toolchain item is an explicit input ID with a FILEMAP edge to the Cargo consumer.
The object contains:

| Field | Contract |
| --- | --- |
| `sysroot` | `{root, manifest}`; `root` is the exact selected directory, `manifest` is an input whose bytes use `chrono-input-directory/v1` or `v2`, and `RUSTFLAGS` must be exactly `--sysroot=<canonical root>` |
| `backend` | Nonempty or empty explicit input-ID list for backend files; IDs must be present and connected when listed |
| `linker` | `{tool, input, environment}` with a distinct registered tool, a present input ID, and an environment variable selecting the canonical linker path |
| `sdks` | Directory inventories with `{root, manifest, environment}`; each environment variable must select that exact canonical root |
| `build_script_inputs` | Explicit present input-ID list for build-script external reads |

Each directory manifest lists every retained regular file and its SHA-256 under
`chrono-input-directory/v1`; empty inventories, duplicate paths, symlinks and
identity mismatches fail before Cargo. `chrono-input-directory/v2` retains those
file entries and requires a `symlinks` array. Each link explicitly declares
`path`, the literal relative `target`, and `kind` (`file` or `directory`). Link
paths have physical parents. Targets may traverse other declared aliases and
relative parent components inside the inventory root. File targets must have a
registered digest; directory targets must contain registered files. Undeclared
aliases, root escapes, duplicate paths, wrong target kinds, cycles while resolving
a target and more than 64 followed links fail. A v1 manifest rejects the `symlinks` field, including
null. Directory inventories remain explicit subsets, not discovered dependencies
or proof that the SDK has no other inputs.

The guard rechecks the original root path's canonical destination, all directory
files and literal link targets, declared inputs and selected paths after the
consumer. Resolving to the same file does not make different literal link targets
equal. An otherwise successful consumer that changes a declared alias or redirects
the directory root fails with its original operation result preserved.
Recursive Cargo
configuration may not provide a second linker or sysroot selection source.
V5 reports `chrono-cargo-run/v3` with toolchain evidence and the observed linker
tool; v2/v3/v4 retain their original meanings and reject the v5 field.

This is a bounded, opt-in inventory. It does not prove that backend libraries,
linker delegates, SDK metadata, build scripts, Cargo credentials, OS state or
network responses are complete, so `input_closure_complete` remains `false`.
The full host remains proposed until the host registers and activates this
policy. Dedicated tests cover a real Rust sysroot, linker, SDK inventory,
build-script input, selection conflicts and directory/input drift through the
ordinary Cargo operation.


## Compatible v6 native target and v3 directory source

`chrono-cargo-inputs/v6` explicitly requires `target_mode: "native" | "explicit"`.
Native mode retains Cargo's original target-directory layout without injecting
`--target`, verifies the bound rustc `-vV` host against the declared target and
rejects competing Cargo build-target environment/configuration. V2–v5 retain their
previous semantics. Registered named test suites, filters and libtest tails remain
literal consuming argv. A declared `CARGO_NET_OFFLINE=true` may supply offline
selection; online or conflicting selection still rejects.

Directory inventory v3 adds exact child-name rows, including root `.` and empty
or alias-only directories. V1/v2 retain their original meanings. Optional
`manifest_root` explicitly relocates an unchanged original inventory to a different
selected live root; wrong origin, changed bytes, alias escape, cycles and child
drift still reject. The observer-produced original is consumed by the existing
guard, including its post-operation checks. Bounded phase observations retain
where a failure occurred without replacing any check.

The host registers one proposed isolated `guard.check.runner` consumer; its
current selected-home, input and participant bindings remain incomplete. Its
generic behavior cases stay in the dedicated Rust owner tests. Cargo guarding is
optional: the 95 current methods corresponding to PR163 retain 94 direct Cargo
recipes and one ordinary `build.diagnostics` action through
`.chrono-harness/cargo/diagnostics-config.json` and `diagnostics.json`.
Restoring the historical parallel guard aliases is not required for ordinary
scoped operation; underlying methods, assertions and input obligations remain.

Exact 51 native run `41021444210/1` qualifies 37 corresponding operations,
including diagnostics' 27-node locked/offline metadata and guarded offline build.
The other 58 methods remain registered without execution evidence in that cohort.
This native diagnostics domain does not qualify ordinary sibling package inputs,
the local diagnostics selection or the isolated runner home. Full-host bindings,
input closure and candidate acceptance remain separate. [Custody](input-source-custody.md)
records the retained source and each remaining consumer prerequisite.
