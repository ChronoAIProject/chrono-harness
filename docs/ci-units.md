# Independent CI units

`chrono-ci-check/v3` explicitly assigns every registered test plan to one unit.
`chrono-github-units/v1` generates one GitHub Actions workflow for each unit and
one collection workflow. A unit has its own checkout, job name, runner,
bootstrap command, timeout, context, artifact upload and retry. Host languages
and directory layouts are opaque to both contracts.

These contracts extend the scoped CI adapter. They do not activate the full
seven-judge profile, prove complete input closure or establish deterministic
local/CI parity. Public beta.11 predates these source changes.

## Assignment and execution

Keep the existing scoped check fields and use `schema: chrono-ci-check/v3`.
Add the following explicit policy fields:

```json
{
  "units": {
    "service": {
      "tests": ["test:service-tests"],
      "report_path": ".chrono-harness/state/service/check.json"
    },
    "client": {
      "tests": ["test:client-tests"],
      "report_path": ".chrono-harness/state/client/check.json"
    }
  },
  "shared_operations": {
    "build.shared-input": ["client", "service"]
  }
}
```

Assignments must cover the complete candidate plan registry exactly once.
Unknown tests, missing assignments, duplicate assignments, report collisions
and undeclared shared operations fail before business execution. The example
shared operation is valid only when both registered complete plans contain it.
Use `{}` when no operation is shared across units. Sharing means executing the
declared prerequisite in each isolated checkout; it does not transfer artifacts
or create an implicit dependency between workflows. Cross-workflow artifact
dependencies are not implemented by this version.

Both endpoint registries still determine DELTA impact, including removed edges,
operations and test replacements. Changes to unit assignment select the affected
plans. The judge validates the complete selected operation graph before filtering
to a requested unit, so a contradictory global operation order cannot be hidden
by that filter. Each unit executes its selected complete plans through the
existing routes/projects engine. Replacement success belongs to the unit owning
the replacement plan; collection requires that unit's successful evidence.

Use the same command locally and in the generated unit workflow:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID --unit service
```

The optional `--unit` and `--collect` selectors require v3 and are mutually
exclusive. Their transport is `chrono-ci-judge/v2`; old calls retain v1. A v3
call without either selector remains the global scoped check. Where the host
uses `registration_config`, a scoped invocation must match its registered
canonical command with exactly the chosen selector appended. Initial mode uses
`--initial` without `--base` and still requires a parentless candidate.

Unit evidence separates `selected`, `global_selected`, `assigned_elsewhere`,
`required_units` and `not_required`. A successful unit never certifies global
completion, and obligations assigned elsewhere are never labeled not-required.
Run concurrent units in separate checkouts: this version does not provide shared
checkout artifact locking.

## Offline collection

Supply a manifest beneath registered `.chrono-harness/state/`:

```json
{
  "schema": "chrono-ci-collection/v1",
  "reports": [
    {
      "unit": "service",
      "path": ".chrono-harness/state/imported/service.json",
      "sha256": "REPORT_SHA256",
      "runner_sha256": "RUNNER_SHA256",
      "judge_sha256": "JUDGE_SHA256"
    }
  ]
}
```

Digest fields contain full lowercase SHA-256 values. Include every required
unit exactly once. Reports from registered units without DELTA are optional;
when supplied they are validated too. Use:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --collect .chrono-harness/state/collection/manifest.json
```

The collector recomputes the current global obligations and verifies report
digests, endpoint/profile/selector bindings, expected executable pins, original
judge output, plan identities and exact candidate operations/order/bounds. It
checks original process bytes and receipts using the shared receipt verifier.
Missing or duplicate evidence, stale bindings, edited output and unsuccessful
units cannot pass. It executes no business test operations. An empty DELTA can
be collected with an explicitly empty report list.

Executable pins are explicit caller inputs. Collection binds reports to those
pins; it does not independently prove how the executables were built or certify
their external input closure. Original tool observations can come from different
roots and platforms. That fact does not establish environmental equivalence.

## GitHub projection and transport

Put a `chrono-github-units/v1` source under `.chrono-harness/ci/`. It contains:

- `collection`: an existing `chrono-github-ci/v1` or v3 provider configuration
  describing the common events, actions, runner, generator and check profile,
  and the collection workflow's path/name/bootstrap/runner/artifacts.
- `units`: a map with exactly the same IDs as the check profile. Each value
  declares `workflow_path`, `name`, `runs_on`, `timeout_minutes`, `bootstrap`,
  `context_path` and `artifact_directory`.
- `gather`: the GitHub CLI `program`, `inherit_environment`,
  `credential_environment`, literal `environment`, process `timeout_seconds`,
  `output_limit_bytes`, overall `wait_seconds`, `poll_seconds`, and explicit
  `manifest_path`, `download_directory`, `report_path`.

The bootstrap list is a literal argv. Register only the tools each unit needs;
the generator never infers packages, language setup or build dependencies. Each
unit's report and context must be inside its uploaded artifact directory.
Workflow paths and names must be unique. The entire output set is preflighted
before generation; unowned files are not overwritten. Filesystem failures are
reported without claiming a multi-file transaction. Removed workflow paths must
be explicitly retired by the host alongside their registrations.

```sh
chrono-ci init --host-root . --config /path/to/units.json
chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

`init` uses `.chrono-harness/ci/units.json` and preserves an existing host source.
It needs an already registered v3 check profile; it does not invent assignments.
Existing single-workflow, full-context and release provider schemas keep their
semantics and projections.

On push and pull_request events, preparation binds the generated provider,
workflow and profile to the candidate and records the exact canonical argv and
pre-execution runner/judge identities. The collection workflow has `actions:
read`; its separate gathering step alone receives `GH_TOKEN`. Declare
credential environment keys explicitly so transport reports omit their values.
The report retains original nonsecret process bytes, exit/failure and an identity
of the actual environment. Never pass credentials as literal transport values.

Gathering waits within declared bounds for each workflow's run at the candidate
and event, pins its actual attempt, downloads the attempt-specific artifact,
checks matching fixed contexts and the actual workflow source bytes, and checks
the attempt again. Missing, ambiguous, changed or failed runs fail collection.
Original downloaded reports remain available. The resulting manifest is
`gathered-unjudged` until the same canonical `check --collect` succeeds. Unit
failure does not cancel unrelated unit workflows; retrying a unit does not
rerun them. Retry collection afterward to gather the new pinned attempt.

Automatic collection currently covers push and pull_request events. Individual
unit workflows also accept explicit manual inputs; manual combinations use
explicit manifests and the local collection command. The collection workflow
does not expose an unsupported manual trigger. Existing collection outputs are
not overwritten by a retry; native retries use fresh checkouts. Workflow-source
and endpoint mismatches remain errors, including when a moving integration base
changes while separate workflows prepare their inputs.

Source behavior tests include actual candidate binaries in independently cloned
hosts, concurrent execution, offline collection, and synthetic GitHub transport
responses. Native generated workflow adoption and public release/example
upgrades must be verified separately before claiming those delivery stages.
