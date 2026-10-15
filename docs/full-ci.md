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

The finite Q3 increment keeps the deployed scoped check, input acquisition and
truthful refusal diagnostics. Full native activation is future work. The persistent
full-SPEC development goal was cleared; full activation is not a prerequisite to
this bounded source extraction, dev integration or auxiliary-checkout cleanup.

The stable `.chrono-harness/config.json` now uses the existing
`chrono-git-configs/v2` source/platform selector. Its local Mac target is
`.chrono-harness/local-config.json`; scoped CI targets are
`.chrono-harness/ci/scoped-macos.json` and `scoped-linux.json`. The current generated
scoped workflow selects macos-26. Linux product/release consumers retain their
own platform inputs; registering a selector target does not establish a deployed
Linux scoped workflow. Unsupported platforms/sources still fail without fallback.
All targets select the existing `ci/check.json` profile, 26 units and the same
short check. The scoped targets retain the explicit local input graph needed by
root FILEMAP; those declarations do not assert native input completeness or
positive native guard admission. Their effective execution environment uses the
platform's existing source-bootstrap selection. Git expectations come from the
committed native identities in `migrations/fixtures/*-host.json`; Cargo/rustc
bootstrap bytes, versions and official component provenance remain in
`ci/bootstrap-toolchains.json`. No failing executable is measured to redefine its
expected bytes.

The local guard continues to name the direct local policy. Its private Cargo and
Rustup homes now belong to the surviving primary
`/Users/auricstudio/chrono-harness/.chrono-harness/state/isolated-rust/`.
The original 92-file Rust manifest is unchanged, including its original
`/Users/auricstudio/.rustup/toolchains/1.95.0-aarch64-apple-darwin` root.
`manifest_root` admits that unchanged inventory at the primary's selected root.
Package/features/dependency-kind, configuration and ancestor absence, SDK/full
Developer, alias, sysroot/backend/linker and post-consumer checks remain required.
The local/native expectation split in `cargo/observe.json` remains exact; a
native rejection of local bindings is diagnostic coverage.

The existing worktree owner now reuses the selected host's local input-producer
policy for default lifecycle calls when that producer and `worktree.check` name
the same owner. Explicit `--config` keeps its existing meaning. The owner still
checks committed policy bytes, coordinator compatibility, registered inputs,
leases and original results. This closes the stable `check` entry's fixed-policy
selection gap without another check entry or a native lifecycle bypass.
Core/cache/detector/local bootstrap participation delegates its omitted config to
that same default. Acquisition keeps its explicit policies. Real startup carries
the caller's existing source selector through the protected consumer; independent
custody fixtures register their own direct host and rebound policy.

The caller must deploy compatible candidate tools and committed policies through
the surviving coordinator, then run the registered `inputs.provision-isolated`
action **at the primary root**. Its existing `use` protects Rustup production,
all 34 fetch actions and the immediate `guard.check.runner` consumer. The caller
must keep the existing protection alive across any retained prerequisite outputs
and subsequent real-startup/guard consumption, including primary custody when a
candidate consumes the primary's private tools. Separate completed uses do not
protect a later consumer. No produced state, installed identity or successful
result can be filled in by hand.

### Retired declaration closure

Unique original bytes remain reachable at
`87a77182c5da33627deb3b837e7515748eb6fd58`, tree
`adf79eba9137f7c3822c11b138534c953229e94c`; caller owns ref preservation and final
PR disposition. This source extraction retires the 33 sibling local guard policy
files and unselected guarded actions, including the unconsumed runner build
variant. Their exclusive package-file bindings, inputs and edges retire with
them. Every actual metadata/fetch operation and all nonexclusive source,
configuration, compiler/SDK, Python and runtime declarations remain.

The unselected native selector, full-native units, native bootstrap/cache/shared
sources and full-policy/cache/projection declarations retire. Acquisition
bootstrap profiles directly consume the native worktree/cleanup policies, so
those four files remain. The original Mac config/cargo-runner pair remains
byte-identical, explicitly owned test data for
`inputs-tests::main_host_observation_reuses_all_declared_cargo_acquisition_operations`.
Its original paths inside the data are opaque fixture values. No action selects
that pair; the test retains its SDK/full Developer, manifest path and 200000-entry
assertions against the actual acquisition declarations.

No committed bootstrap profile consumed the optional host-script distribution
branch. That branch and its exclusive fixture tests retire. The distribution
installer, independent release inventory, source bootstrap, shared startup,
custody and real-consumer assertions remain. Preservation is not implementation
or PR settlement; caller must establish useful retained delivery before closing
or merging the original mixed proposal.

Known diagnostic gaps remain truthful: the Mac `Developer/usr/bin/altool` alias
escapes to sibling `Contents/SharedFrameworks`, and the Linux proposal had 140
unobserved inherited configuration facts. Neither is converted to success,
collected again, narrowed away or waived by this extraction. Positive native
guard/full activation, fixed full-input endpoint pairs, lineage/composition,
governance/integration adoption and stronger parity remain unestablished.

**Policy and product change together.** Validation and total runtime/CI/storage
costs are unknown. Caller must regenerate registered instructions after the host
context change, commit a clean candidate, run registered source bootstrap and the
FILEMAP-selected canonical/native checks, and complete independent review.
`registered_full_filemap_capacity_preserves_exact_original_bytes` and all ordinary
metadata/hash/length/oversize/control assertions remain unchanged; its existing
authenticated original fixture hook remains available. Exact FILEMAP capacity is
2097152 bytes; ordinary diagnostics stay 1048576 bytes. No source-stage result
establishes canonical/native acceptance, dev landing, PR settlement, cleanup or
full SPEC completion. Caller owns actual landing SHA/tree, post-landing primary
consumption and authorized lifecycle cleanup.
