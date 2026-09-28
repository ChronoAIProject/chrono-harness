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

The full check config must use schema v3 with a declared
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
`--context`. Upload runs with `always()` and preserves the declared directory,
including failure evidence; an early bootstrap failure may leave only job logs.

## Publication and retry boundary

Preparation preflights both context/report outputs before writing either.
Identical existing bytes are reusable; differing bytes are a collision and are
not overwritten. Git acquisition changes subsequent observations, so rerunning
after a fetch can collide with the retained first preparation report. Use a
fresh explicit run location/checkout and retain the earlier artifacts. Do not
discard old evidence to pretend that the first invocation never happened.

Publication uses the existing per-file writer. It is not a multi-file crash or
concurrent-writer transaction: an IO failure can leave one output published.
Successful preparation is neither full governance nor a parity certificate.
Native consumers, generated workflow execution, host activation, complete input
closure and local/CI comparison are separate evidence obligations. This manual
dispatch projection does not orchestrate PR creation, merging, required-check
publication or landing.
