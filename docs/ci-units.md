# Independent CI units

`chrono-ci-check/v3` explicitly assigns every registered test plan to one unit.
`chrono-github-units/v1` generates one GitHub Actions workflow for each unit and
one collection workflow. A unit has its own checkout, job name, runner,
bootstrap command, timeout, context, artifact upload and retry. Host languages
and directory layouts are opaque to both contracts.

These contracts extend the scoped CI adapter. They do not activate the full
seven-judge profile, prove complete input closure or establish deterministic
local/CI parity. Public beta.12 introduced these contracts; beta.13 adds the
explicit customization-preserving migration below.

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
responses. The public Go, TypeScript and mixed [example hosts](examples.md) use
independent native workflows. Their local/integration/PR/dev reports were checked,
as were actual cancellation with selective retry, business failure with another
unit passing, and restoration with zero selected business operations. Initial
inventory remains a separate contract. The product host registers sixteen units,
one per complete test plan, and a collector in `.chrono-harness/ci/units.json`.
Its core bootstrap explicitly builds runner, judge-ci and ci; each selected plan
keeps its complete existing build/check/test operations. Shared operations are
listed in the check profile and repeat only in isolated checkouts. The default
full bootstrap remains available when its existing configuration is selected.

The CI generator consumer also executes the generated unit workflow's actual
`chrono-ci prepare` CLI against a committed push event, checks the
candidate-bound workflow source and context, then invokes the recorded local
`chrono-harness check` argv and reads the retained unit report. It checks the
literal generated command shape and the selected operation in
`crates/ci-tests/tests/units.rs`, so a projection change cannot silently diverge
from the local command. This is bounded host-side adoption evidence for the
units projection; it does not claim a live GitHub dispatch, complete external
input closure, or generated-v3 provider adoption.

## Host customization and updates

Use the host's registered source paths. The table identifies which declaration
owns each customization; the generator requires no business directory layout
or language convention.

| Change | Host-owned source | Apply |
| --- | --- | --- |
| Commands, tests or dependencies | Project/script actions and FILEMAP edges/plans | Update declarations and run the canonical DELTA check |
| Unit boundaries or shared operations | Check profile `policy.units` / `policy.shared_operations`, plus provider `units` | Keep each complete test plan assigned once; regenerate owned workflows |
| SDKs, runner image, timeout or startup | Provider `bootstrap`, `runs_on`, `timeout_minutes` and explicitly registered bootstrap data | Generate, verify, then check the changed native event |
| Workflow paths or removed units | Previous and next provider sources | Use `migrate` below to verify ownership and retire obsolete outputs |
| Harness version | Public distribution pin and selected tool list | Adopt the new pinned release; keep the host's CI and SDK sources |

The [Go, TS and mixed examples](examples.md) demonstrate the last row by changing
only their distribution lock and README. None requires editing Rust or adding a
language-specific case to the generator. Schema/behavior changes still need an
explicit migration; a version bump does not certify compatibility on its own.

The host owns the provider JSON, check profile, unit assignments and bootstrap
commands. Customize these explicit sources; generated workflows are projections.
A unit can call any registered script or plugin through its literal bootstrap
argv and complete test plan. Unrelated host workflows can coexist. The generator
never selects a language, SDK, directory layout or dependency on the host's behalf.

`init` preserves an existing host source byte for byte, including custom runner,
timeout, bootstrap arguments, context and artifact settings. Supplied defaults do
not replace it. After installing a pinned newer binary, use the same `generate`
and `verify` commands on that source. Verification of projections is separate from
the actual canonical checks and native event validation.

Public beta.13 provides `chrono-ci migrate` for an explicit projection transition.
Prepare the new check profile, assignments and provider source, then migrate the
verified old owned workflow:

```sh
chrono-ci migrate --host-root . \
  --from .chrono-harness/ci/github.json \
  --config .chrono-harness/ci/units.json
```

This accepts a scoped v1 or units provider as the previous source and an explicitly
configured units provider as the destination. It verifies every old generated file
and every new output collision before writing. It preserves the destination JSON
exactly, writes the new projections, removes only obsolete previous workflows and
retires the distinct previous source only when its bytes match the explicit input.
A host-edited old projection or colliding new file fails before any write. Unrelated
files are preserved. Initial-inventory, full-governance and release-provider
transitions are rejected by this command; it never silently weakens those contracts.

For an update at the same source path, retain the previous source bytes before
editing (a run-local file belongs beneath `.chrono-harness/state/`). Supply its
original source address explicitly so embedded workflow commands remain bound to
the correct source:

```sh
chrono-ci migrate --host-root . \
  --from .chrono-harness/state/previous-units.json \
  --previous-config .chrono-harness/ci/units.json \
  --config .chrono-harness/ci/units.json
```

This also retires explicitly removed or renamed workflow paths. It leaves the
new source and retained prior-input file untouched. The JSON result lists written
and retired paths and input hashes. Update FILEMAP and any explicit consumers for
those paths, then run the canonical DELTA checks. Migration does not infer or edit
the host's registration policy, and its success does not certify governance or CI.
Filesystem failure reports completed changes; there is no cross-file transaction
or concurrent-writer isolation. Preserve the original result and reconcile the
reported paths before retrying; once migrated, ordinary generation is idempotent.
