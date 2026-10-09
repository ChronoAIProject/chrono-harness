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
