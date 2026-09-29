# Registered Git facts

Public beta.11 includes the previously released full-v3/scoped-v2/provider-v3 Git binding. The literal
checkout identity change below is included in beta.11.

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
Git's `-C` argument uses the bound canonical checkout path. Caller-relative host
roots are checked against that binding before invocation, avoiding a second
relative-path interpretation after the process changes its working directory.

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

Scoped `chrono-ci-check/v2` explicitly selects this reader through
`policy.facts_config`, independently of optional full `registration_config`.
Both endpoint snapshots, original parents, index and dirty checks, registry
interpretation and post-execution checks share that reader. Structured
`evidence.git_facts` retains successful and failed process observations, including
version-binding failures. The scoped v1 contract remains unchanged and rejects
the new field even when null. Selection and operation environments keep their
scoped semantics; the binding does not silently activate full governance.
See [the scoped contract](ci.md#configuration-and-selection-contract).

CI provider v3 explicitly selects this reader through `facts_config`
for event acquisition; see [the event contract](ci.md#registered-event-git). The product repository still uses its registered scoped v1 CI and
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

## Literal checkout identity

A configured Git diff can hide raw differences: `core.filemode=false` ignores
executable changes, text attributes normalize line endings, `core.symlinks=false`
accepts a link represented by a regular file, and clean filters can turn changed
physical bytes into an unchanged blob. Running a diff can also execute a filter.
The checkout reader now compares literal files against the fixed Git tree, and
separately compares every index stage/mode/OID with that tree. Opposite index and
working-file changes cannot cancel. It does not run working-tree diff, clean or
textconv filters for this comparison.

The shared runner implementation consumes the existing explicit tree/index
inventories. It streams regular bytes through RustCrypto SHA-1 or SHA-256 with the
Git blob header, checks regular-file versus literal-symlink type and Git's owner
execute bit, and hashes the symlink target bytes without following it. Missing or
mismatched files are changed inputs. A directory alias cannot make an observed
file match a different physical parent. Files are opened without following the
final symlink or blocking on a substituted FIFO; IO and malformed inventory
errors fail. This is a Git object identity check, not a new cryptographic trust
claim. It does not inspect dependencies, register policy or select tests.

Full and initial consumers use the shared checkout reader; scoped CI uses its
tracked comparison before and after selected operations. Worktree creation and
cleanup compare against HEAD; reconstruction and reconciled recovery compare
against the explicitly chosen staged tree. They use the existing registered Git
process runner for inventories and preserve the original operation reports and
partial work. Rebind remains a separate preservation contract and does not
require its intentionally dirty work to match the new index.

Configuration that produces CRLF/filtered files or regular-file symlink emulation
must be changed by the host to materialize the registered exact snapshot. The
checker does not silently normalize those files. Permissions outside Git's
executable class, timestamps, ACLs, xattrs and hardlink topology are not Git tree
identity; no concurrent-writer or crash-atomic snapshot is claimed. Supported
entries remain regular files and literal symlinks with UTF-8 registered paths;
Gitlinks and special files are not accepted as matching regular source files.
Complete Git configuration, delegated executable, OS, compiler/SDK input closure
and full host activation remain separate unfinished obligations. The source
change is distributed in beta.11.

The beta.11 public macOS runner/scoped judge and worktree binary were consumed
in an actual upgraded mixed-host clone. With local `core.filemode=false`, Git
reported no changed path for an executable-bit change; the canonical check and
registered cleanup both rejected it, preserving the checkout. Restoring the
mode restored acceptance. This is a bounded public-binary consumer, not input
closure or deterministic-parity certification.
