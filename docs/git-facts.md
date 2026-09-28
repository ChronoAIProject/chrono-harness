# Registered Git facts

This source increment is not included in public beta.8.

Full config v3 requires `facts_git: {"tool": "git", "input": "git-executable"}`.
Both IDs are explicit: `tools` supplies `program`, `resolution: "PATH-once"`,
`version_argv` and a nonempty `expected_version`; `environment.inputs` supplies
`location`, `presence: "present"` and the expected SHA-256. The two paths must
resolve to the same executable. Register the input's actual consumers with
FILEMAP edges; this link does not infer test dependencies.

The runner reads the selected candidate config before acquiring Git facts, checks
the declared digest before running even the version command, then checks the
config's original bytes against the fixed candidate blob using that Git. The
candidate binding reads both endpoints, including historical schema inputs. Full
registration, FILEMAP, routes, projects, cost, mixed, workflow and the optional
Cargo judge use the request-bound reader. Initial inventory and `chrono-inputs
capture` also use the candidate config binding. No baseline binary executes.

The process environment contains only the declared inherited and configured
values. An absent inherited variable remains absent; configured values override
inheritance. Bare tool names require a declared PATH, including an explicitly
empty PATH. An unrelated ambient Git cannot supply facts. GIT_DIR, GIT_WORK_TREE
and GIT_INDEX_FILE are rejected when explicitly supplied because they redirect
the checkout. Existing no-replace-objects/no-optional-locks arguments remain.

Each Git invocation uses the configured protocol timeout and output bound. The
reader checks the config bytes, selected path and executable/input digest before
and after the invocation. `git_facts` in full/initial reports and full judge
outputs records the binding, environment and actual processes. Raw stdout/stderr
bytes, their digests, exit codes and truncation/timeout failures remain available.
Early failures use `E_GIT_FACTS` with compact JSON observations when a binding was
constructed; they do not invent a completed report or an unstarted process.
Consumers reject missing/mismatched binding observations before invoking Git.
Process bounds apply per invocation; accumulated report storage is not a total
run memory/disk quota.

Config v3 preserves v2 external-input presence and snapshot semantics:
`chrono-input-snapshot/v2` and the existing effective-input schema stay unchanged.
Capture publishes an input snapshot, not a Git-process report or a governance
verdict. Config v1/v2 reject `facts_git`, including null, and keep their prior Git
behavior. Cross-version full governance still needs the explicit workflow
migration and compatibility evidence required by SPEC; accepting the schema is
not evidence that a host completed that migration.

The unchanged full local/CI command is:

```sh
chrono-harness check --config .chrono-harness/config.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --context .chrono-harness/state/context.json
```

This does not change the scoped `chrono-ci-check/v1` contract or CI event input
preparation. The product repository still uses its registered scoped CI and
proposed full registries. Version 3 binding does not establish full governance,
Git configuration or delegated dependency closure, deterministic-input parity,
or freedom from transient concurrent replacement. Hosts must explicitly register
Git configuration, executable interpreters/libraries, OS and other actual inputs
where required; the reader performs no dependency discovery.

Dedicated regressions exercise chosen Git paths with spaces and Unicode, ambient
shadowing, full/initial consumers, seven-judge integration and delivery, invalid
references, request/environment substitutions, wrong digest/version, persistent
byte drift, original binary output/exit, output bounds and timeout. These are
bounded implementation checks, not a Lean refinement proof or independent review.
