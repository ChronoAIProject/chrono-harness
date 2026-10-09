# Explicit mixed-change warnings

`chrono-judge-mixed` consumes registration's single interpreted endpoint views,
FILEMAP's bound v2 DELTA/impact and the cost judge's bound `chrono-costs/v1` output.
Declare all three as direct predecessors. Missing/conflicting impact, costs or
endpoint/tree/registry/context bindings are errors; this judge does not rebuild
the graph, discover dependencies, run tests or recompute costs.

Only changed Git paths are classified. Their base and candidate FILEMAP surfaces
are unioned: `product` and `test` are product changes; `judge-policy` and
`judge-implementation` are executable-rule changes. Reclassification or removal
does not erase an old surface. Other surfaces do not imply executable rules.
Product engine source can require integration through explicit workflow stability
registration without being classified as adopted host policy.

`config.semantic_fields` from both endpoints adds explicitly selected semantics.
Each declaration names a registered JSON file, pointer patterns and a change mode.
Only named changed files are read. Registry inputs use registration's interpreted
views; other JSON comes from the fixed Git objects and strict UTF-8 decoding.
Missing required bytes, invalid JSON or invalid pointer escapes fail explicitly.

Pointers use RFC 6901 `~0` / `~1` escaping; `*` selects exactly one object key or
array element. Path/id row arrays compare by `id` when present, otherwise `path`.
The registered workflow's `retirements` collection uses its declared `(kind, id)`
identity instead. Equal IDs across different retirement kinds are distinct;
duplicate identities, invalid identities and mixed identified/unidentified rows
are errors. Row order is neutral, including selected subtrees. Other arrays,
such as argv and operation sequences, retain their order. Physical source pointers
are preserved independently from stable comparison identities. Absence differs
from a present JSON null. `modify-delete-existing` compares only selections
present in the base; `add-modify-delete` also includes new selections. Schema
migrations remain registration/workflow responsibilities.

Ordinary source edits plus new FILEMAP members or dependency edges do not produce
rule changes under the adopted field declarations. Changing an existing action,
selected surface, execution plan or whole executable-rule file does. The judge
does not infer policy from filenames, documentation or agent-guide prose.

Every successful response publishes `outputs.rule_changes` with schema
`chrono-rule-changes/v1`, fixed `binding`, sorted `rule_paths`, `product_paths`,
`changes`, `product_changes`, `mixed`, `requires_acknowledgement: false` and the
original cost output. Semantic changes include pattern, mode, stable identity,
and before/after physical pointer and value. Costs retain both endpoint maps,
references, known/unknown coordinates, affected/retired tests and measurement
limitations; no new aggregate is inferred. The runner retains this output in the
mixed response, and the workflow consumer can request it as a direct predecessor.

When both groups are nonempty, the candidate's registered
`workflow.mixed_change.code` names the visible warning (adopted default:
`W_MIXED_JUDGE_PRODUCT`). Status is `warn`, exit 0, with both groups and cost
references. A rule-only DELTA still publishes its changes, with status `pass`.
The warning adds no approval or acknowledgement step and does not override other
failures. Workflow stability, migration, retirement and integration certification
remain separate unfinished obligations. Full host activation is not claimed.

The explicit classifier and fixed-object reader are shared with FILEMAP v2 so
workflow test requirements enter the existing plan before execution. Mixed has no
compile dependency on the FILEMAP or cost producer; its process consumes their
versioned wire outputs and bindings. Invalid named semantic inputs can therefore
fail in FILEMAP before mixed runs. Classification remains one implementation.
