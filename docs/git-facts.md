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
for event acquisition; see [the event contract](ci.md#registered-event-git). The
product repository uses independent-unit scoped v3 CI without opting into Git
binding, and retains proposed full registries. Version 3 binding does not establish full governance,
Git configuration or delegated dependency closure, deterministic-input parity,
or freedom from transient concurrent replacement. Hosts must explicitly register
Git configuration, executable interpreters/libraries, OS and other actual inputs
where required; the reader performs no dependency discovery.

Dedicated regressions exercise chosen Git paths with spaces and Unicode, ambient
shadowing, full/initial consumers, seven-judge integration and delivery, invalid
references, request/environment substitutions, wrong digest/version, persistent
byte drift, original binary output/exit, output bounds and timeout. These are
bounded implementation checks, not a Lean refinement proof or independent review.

## Explicit platform configurations

A scoped check or provider-v3 `facts_config` may name a separate, registered
selector instead of a direct full-v3 Git policy:

```json
{
  "schema": "chrono-git-configs/v1",
  "platforms": {
    "macos-aarch64": ".chrono-harness/git/macos.json",
    "linux-x86_64": ".chrono-harness/git/linux.json"
  }
}
```

Keys use the executing binary's Rust `OS-ARCH` constants. The host explicitly owns
every mapping and policy, including each platform's executable path, digest,
version, environment and guarded files. The reader does not discover installed
tools, infer host language or layout, generate declarations, or fall back for an
unregistered platform. Both selector and target paths must be literal relative
paths under `.chrono-harness/`; symlinks and nested selectors are rejected. Only
the selected policy is read on this machine; every map entry is syntax-checked.
Selected policies must use full-v3 Git binding, with no legacy fallback.

The reader binds both original files to the fixed candidate and rechecks their
bytes before and after every Git invocation, including the version probe. A
post-process mutation retains the original child output and exit. Reports add
`git_facts.selection`, containing the selector path/digest, actual platform and
selected policy path; `git_facts.config_path` and `config_sha256` continue to name
the selected full-v3 policy. Request consumers reject substituted or missing
selection observations. Direct policy files keep their prior report shape and
semantics. Register the selector, policies and their actual FILEMAP dependencies
explicitly; selection does not create dependency edges.

This source extension keeps the canonical command and facts-config entry path
identical across registered native platforms. Different selected policies are
different inputs, so it does not establish cross-platform verdict parity. It
does not select the full runner's root configuration or the initial profile's
host configuration, certify complete effective-input closure, or by itself adopt
provider v3 in a real host. The extension is not in public beta.16.

## Guarding declared Git inputs

Config v3 can opt into a versioned guard without changing old snapshots:

```json
"facts_git": {
  "tool": "git",
  "input": "git-executable",
  "guard": {
    "schema": "chrono-git-inputs/v1",
    "inputs": ["repository-config", "global-config", "absent-worktree-config"]
  }
}
```

Each nonempty, unique ID must name exactly one `environment.inputs` file. Present
files need a SHA-256; absent files must explicitly declare absence without a
digest. The host chooses configuration, includes, interpreters, libraries or
other files that must remain bound during Git acquisition. Declare their actual
locations and FILEMAP consumers. In a worktree, `.git` may be a pointer file;
this guard does not infer the administrative directory or expand include paths.

The reader checks these files before the first version probe and before and after
every subsequent Git process, including reads and provider fetches through the
same reader. A mismatch prevents the next process from starting. A mismatch after
execution fails the call while preserving the process's original bytes, exit and
bounds. Directories, symlinks and IO errors are not absence. The regular-file and
absence observer is shared with snapshot capture and registration validation.

`git_facts.inputs` retains the initial verified states under
`chrono-git-inputs-result/v1`. Downstream request consumers require the identical
observation. Omitting `guard` preserves the prior v3 behavior and report shape;
older binaries reject the unknown extension. Full/initial real consumers cover
adoption with unchanged historical config bytes and rejection before Git starts.
Runner tests additionally cover missing/duplicate references, absence, unsupported
file types, per-call drift and a mutating child whose failed receipt is retained.

These checks enforce the specified file boundary. They do not enumerate all Git
configuration, delegated programs, OS or network inputs, establish complete input
closure, or detect a concurrent change restored between observations. Reports
therefore retain `completeness_proven: false` and `input_closure_complete: false`.
Input snapshots and the unchanged canonical check command keep their existing
contracts. This extension is distributed in public beta.16.

## Public binary consumption

Public beta.16 was installed anonymously on macOS arm64 into a local clone of the
[Go host](https://github.com/ChronoAIProject/chrono-harness-examples-go) at base
`2d3bfb3d8cea0d9c0add957cca20ad4335d6601c`. The consumer explicitly adopted a separate
full-v3 facts configuration, the actual local Git executable digest/version,
`.git/config` content identity and `.git/config.worktree` absence. The existing
scoped-v3 profile selected that facts configuration; all declarations stayed under
`.chrono-harness/`, and the original historical configuration bytes were retained.

The installed public binaries ran the host's unchanged canonical command shape:

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/ci/check.json \
  --base BASE_OID --candidate CANDIDATE_OID --unit rates
```

The `rates` unit executed `build.rates` and `test.rates`; the `harness` unit
executed `ci.verify` and `test.bootstrap`, including the installed instruction
producer. Changing the registered Git config or creating the declared-absent
worktree config failed with no business operations executed. Restoring the inputs
restored acceptance. All six installed tools retained their public manifest
identities. This is local public-binary consumption with a locally declared Git;
it does not adopt that machine-specific configuration in the remote example or
establish native provider-v3 adoption, complete input closure or parity.

Git process observations can materially increase the transport size. This real
consumer's original 8,388,608-byte judge output limit failed with
`process output limit exceeded`. A direct `chrono-judge-ci` invocation using that
failed report's exact `request` exited 0 and produced a 9,735,075-byte response;
9,685,756 bytes of that response were Git observations from 25
processes, retaining 2,056,371 original stdout bytes plus the protocol's text and
byte representations. The host then explicitly registered a 16,777,216-byte
judge output limit and passed the canonical checks and drift/recovery cases.
These are measurements for that host and invocation, not a universal capacity
recommendation. Declare a suitable bound from actual report sizes; per-Git-process
bounds and complete-input obligations retain their existing meanings.

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
