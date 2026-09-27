# Declared workflow test selection

FILEMAP impact v2 makes declared stability tests part of the normal execution
plan. It calls mixed's shared explicit classifier, lowers applicable workflow
requirements to `test-execution` edges, and computes the same endpoint union and
closure used by ordinary dependencies. Routes merges the resulting registered
plans; projects executes each shared operation once. No second executor or
all-tests fallback is introduced.

For each endpoint, a stability row applies when its explicit `paths` intersect
the DELTA, or its normalized definition/registry location changed. Path matches
use literal registered paths. Definition changes retain old and new test lists;
source pointers retain each endpoint's actual row index. Reordering sets within
the same stability definition does not change that definition.

Integration is required by an applicable stability row, or by an explicit rule
change when either endpoint enables `semantic_changes_require_integration`.
The rule classification is exactly the implementation used by the mixed judge:
whole executable-rule surfaces and explicitly selected semantic fields. The
base and candidate `integration.tests` lists then apply. Removing a requirement
or disabling the candidate flag cannot erase the base obligation in that DELTA.
Ordinary membership/edge registration and disconnected docs do not activate this
suite. The suite contains only declared test IDs, never discovered tests.

`impact.workflow` uses `chrono-workflow-selection/v1` and contains:

| Field | Meaning |
| --- | --- |
| `integration_required` | Whether this DELTA requires integration; not proof it occurred |
| `rule_changes` | Shared mixed classification before cost packaging |
| `trigger_paths` | Paths that activated stability or semantic requirements |
| `requirements` | Endpoint, registry, physical pointer, reason, trigger paths, declared test IDs and lowered edges |
| `certification` | Explicit selection-only boundary |

Every lowered edge goes from a trigger file to a declared test. Its origin remains
base/candidate/both in the normal union, and changes carry references to the
requirement in `impact.workflow`. New/deleted trigger paths are resolved in the
union of file identities: a newly added rule can trigger a suite declared in the
base, and a deleted rule can trigger candidate tests. Targets must exist in the
declaring endpoint. Ordinary FILEMAP edges retain endpoint-local reference checks;
a workflow edge does not hide a dangling FILEMAP edge.

The usual typed-edge errors, ambiguity checks, required/retired tests and old/new
cost references apply. No edge means no selected test. Missing selected targets,
plans, operations or semantic inputs fail explicitly. An unrelated historical
bad requirement does not become a new finding solely because it was read.

Full judge execution reads named JSON only from fixed Git objects and consumes
registration's single interpreted view. The `produce` and `produce_with_inputs`
library APIs provide the five known registry documents; callers using other
named semantic JSON must supply it through `produce_with_documents` or
`produce_with_inputs_and_documents`. Missing required documents are errors, not
an empty classification. Mixed consumes FILEMAP v2 as a wire value, so its shared
classifier has no compile dependency on the downstream impact or cost producers.

Dedicated tests cover declaration removal, old costs, semantic versus ordinary
edits, row ordering, new/deleted paths, relevant reference failures and docs
locality. Real runner → registration → filemap → routes → projects consumers
verify additional suite execution, shared prerequisites running once and failures
before operations. The existing mixed consumers verify the shared named-JSON
decoder, which now rejects invalid semantic input before downstream judges run.

This is test selection and execution evidence. Branch freshness, integration
evidence production/consumption, retirement and migration certification remain
workflow judge obligations. The scoped CI adapter still has its labelled legacy
selection boundary; full native activation, complete inputs and parity remain
unfinished. No successful integration certificate is produced by this increment.
