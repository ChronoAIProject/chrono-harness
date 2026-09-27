# Declared impact costs

The independent `chrono-judge-cost` executable consumes registration's interpreted
endpoint views and FILEMAP's existing `chrono-filemap-impact/v2` output. It does
not discover dependencies, select tests, execute operations or measure resources.
Its dedicated test project is `judge-cost-tests`.

The judge publishes `outputs.costs` with schema `chrono-costs/v1`. The runner carries
that original value into the report's `costs` field and records its source pointer.
The output binds the exact base, candidate, candidate tree, registry and context.
Missing or conflicting FILEMAP output, unsupported impact schema or mismatched
DELTA produces an error; no empty success is substituted.

`declared_before` and `declared_after` map affected node IDs to cost entries. They
include changed/affected files, projects and scripts reached by the explicit graph,
and required tests from both endpoints. A missing endpoint is JSON null, preserving
additions and removals. Each entry retains its member identity, cost-model reference,
four coordinate values, basis and JSON pointers to the model and declaration in the
corresponding endpoint's FILEMAP. Projects/scripts retain the costs of their
explicitly owned files; no project multiplier or inferred member is added.

`affected_tests`, `retired_tests` and `removed_nodes` preserve distinct identities.
Multiple paths reaching one test do not duplicate it. Equal model references on
different members remain separate references. No total is calculated: memory peaks
and parallel wall time are not additive, and no scalar penalty is invented.

`unknown` identifies the endpoint, node, member, reference, unknown coordinates and
declared basis. Missing member cost references are explicitly unknown. A null
coordinate stays null while known coordinates remain intact. Any unknown produces
`W_COST_UNKNOWN`, status `warn`, exit 0; a dangling model or test-cost reference is
an error. Warnings also work for retained-environment impact with no Git-path delta.

`measured` is null with an explicit reason. These are declared estimates, not
measurements of the invoked build or test. No timing, memory or IO claim is derived
from process receipts. A future measurement producer must retain its tool, sample
and observation time separately from declarations.

The repository's full configuration remains proposed. Current scoped CI builds
the binary and runs its registered test plan; the dedicated consumer tests invoke
actual runner/registration/filemap/cost processes on fixed Git commits. This is
cost-report behavior evidence, not full host activation, mixed-change enforcement,
migration/retirement certification or complete Cargo/SDK input closure.
