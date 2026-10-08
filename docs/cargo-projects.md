# Optional Cargo project judge

The main-host metadata consumer registers `inputs.fetch.*` actions as package
acquisition prerequisites for its 34 original `inputs.metadata.*` actions.
Acquisition uses the same locked manifest and declared target; metadata remains
locked and offline. Both processes retain launch records and original streams
under `.chrono-harness/state/inputs/`, including failures and enclosing
termination. The release recipe carries that directory for failed
`test.judge-cargo-tests` operations. An empty package inventory is a controlled
reproduction of a missing prerequisite. Diagnosing a native metadata failure
requires its original inner process streams; a local reproduction alone does
not identify that failure's cause.

**Mixed-change warning:** These host prerequisites and transport bindings change
alongside their Rust consumer tests and release evidence registration. Validation
includes the original affected Rust suites, Python release/bootstrap tests and
workflow projections. Local source checks do not establish native acceptance,
full activation or publication.

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

The guard emits bounded `CHRONO_CARGO_PHASE` JSON lines on stderr at the start
and terminal return of input preparation, each declared directory bind/recheck,
tool probes, native-host observation, metadata, validation and consumer execution.
Terminal observations include elapsed milliseconds and whether the phase returned
or raised an error. A missing terminal line remains an incomplete observation;
silence is not evidence of a hang. The original process report continues to own
child exits, failures and stream identities. Timing does not skip any input check.
The product-host native metadata test observes the guard through the existing
bounded process engine, retains its stdout/stderr and terminal process report,
and reserves time within its existing registered test budget to publish a timeout.

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
The product host has not yet activated the guarded operation.

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
`--all-targets`, `--examples` and `--benches` where applicable. Named `--test`, `--bin`, `--example` and `--bench` selectors are supported,
including repeated named suites. `test` and `bench` accept one positional filter.
Registered `test`, `bench` and `run` tails after `--` are forwarded unchanged to
the child; they do not alter Cargo feature/configuration selection. This retains
libtest listing, exact filters, skips, threads, original assertion failures and
application arguments. Other unlisted Cargo flags and inline configuration
assignments are rejected. Cargo still determines whether a
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

## Native consumers and offline environment (v6)

`chrono-cargo-inputs/v6` retains every v5 binding and requires `target_mode`,
either `explicit` or `native`. Explicit mode retains `--target`. Native mode
requires its absence, rejects `CARGO_BUILD_TARGET` and parsed Cargo
`build.target`/`env.CARGO_BUILD_TARGET`, and retains an additional real compiler
`-vV` process at `native_target`. Its single observed `host:` must equal the
declared target before metadata or consumer execution. Metadata continues to
use the declared `--filter-platform`. Native mode keeps the original native
Cargo output layout, including `target/debug`; it does not rewrite output paths.

V6 accepts the original locked command without an offline flag when its explicit
effective environment sets `CARGO_NET_OFFLINE` to exactly `true`. Otherwise the
existing locked/offline flags remain required. V2–v5 reject `target_mode`, even
null, and keep their original target/offline requirements. Formatting retains
its Cargo/rustfmt contract and is not a supported guarded verb.

`chrono-input-directory/v3` retains the v2 file and literal-alias inventories
and requires `directories`, an array of `{path, entries}` declarations. Paths
are physical directories and entries are their exact immediate child names,
including an empty list. Such a directory may supply a directory-alias target
even when it contains no registered regular file. The owner checks physical
parents and exact child membership before and after the real consumer. Added
children, missing/replaced directories, duplicate/invalid names and alias drift
fail while preserving the original consumer result. Older inventory schemas
reject the new field. This closes the real macOS SDK's empty and alias-only
directory targets; it is not a discovery or hidden-input completeness proof.

The product host stages v6 policies for its 34 existing Cargo roots under
[`.chrono-harness/cargo/`](../.chrono-harness/cargo/observe.json). Each original
source build/check/test/run and unfiltered release action remains available;
`guarded_*` actions declare the same consumer argv, with offline selection in
the environment. Existing group inventory still uses its original complete and
disjoint listing operations. Switching the full execution plans and inventory
to compatible guarded actions remains activation work.

The explicit local compiler, resolver archive/index, extracted package, SDK,
linker/delegate and configuration declarations are source-backed observations,
not native CI identities or endpoint snapshots. Compiler/SDK directory manifests
remain separately identified under `.chrono-harness/inputs/local/`. The dedicated
Rust consumer observes each explicitly named main metadata operation, retains
its original streams under host state, and exercises the registered main native
check on its declared platform. This is local source validation. Native
provisioning, runtime-platform scope, genuine historical/current snapshots and
fixed compatible artifacts remain prerequisites; full host enforcement stays
proposed/incomplete. Product code contains no host layout or platform recipe.

The native consumer explicitly asks the existing process engine to retain raw
stdout/stderr as they are read, within the original output bound. Its atomic
launch record identifies the actual executable, argv, environment, stdin and
spawned child. Enclosing termination can leave these streams as partial originals.
Returned process and terminal JSON are published atomically and preserve the
engine's actual observations; when both are absent, the child terminal remains
unobserved by those receipts. Complete runs still compare retained bytes
with the original process result and preserve the existing metadata and native
assertions. The registered Cargo operation remains bounded at 900 seconds;
the per-test timer cannot account for preceding test binaries. Rust regressions
exercise enclosing termination of the real registered native guard and a stalled
fixture, including interruption before terminal publication. These bounded source
checks do not diagnose a past timeout or replace clean canonical/native acceptance.
