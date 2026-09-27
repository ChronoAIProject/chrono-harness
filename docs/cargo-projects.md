# Optional Cargo project judge

`judge-cargo` / `judge-cargo-tests` is an independent production/test pair. Generic
registration and projects have no dependency on it or on TOML. A host adopts the
adapter by registering its binary, arguments, policy input and FILEMAP edges. A
Go/TS/script host has no Cargo policy obligation merely because Cargo-like files
or directory names exist.

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
`--all-targets`, `--examples` and `--benches` where applicable. Unlisted flags,
inline configuration assignments and arguments after `--` are rejected; supporting
them requires an explicit adapter extension. Cargo still determines whether a
particular accepted combination is legal and retains its real failure.

The registered outer action must invoke this guard with the exact root, config,
policy and operation arguments above. `chrono-cargo-run/v2` reports the observed
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
