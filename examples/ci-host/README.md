# Explicit installed-tools host (legacy provider)

This bundle retains the original v1 behavior and historical initial-inventory
tests. New short-command hosts can adopt the [conditional-job example](../ci-host-job-gating/README.md); migrate explicitly rather than reinterpreting prior reports.

Copy this directory to a fresh Git repository. Install or copy `chrono-harness`,
`chrono-judge-ci`, and `chrono-ci` together into an explicit tools directory.
This example has no dependency on the harness source checkout or Rust at runtime.

The example GitHub configuration uses a `self-hosted` runner with the tools at
`/opt/chrono-tools`. Set `bootstrap` in `.chrono-harness/ci/github.json` to the
actual installed tools directory before initialization; configure the runner
label there as appropriate. A hosted runner needs its own explicitly configured
pinned download/install bootstrap. No published binary service is assumed.

```sh
python3 .chrono-harness/ci/install.py . /absolute/path/to/installed-tools
.chrono-harness/bin/chrono-ci init --host-root . --config .chrono-harness/ci/github.json
```

Commit the complete initialized inventory. For a parentless first commit:

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/ci/check.json --candidate FULL_INITIAL_OID --initial
```

For later commits use the same command with `--base FULL_BASE_OID --candidate
FULL_CANDIDATE_OID`, omitting `--initial`. The initial commit's default branch is
`dev`; the normal push/PR routes are configured explicitly. When first pushing
this parentless commit to `dev`, the ordinary branch-creation event automatically
runs that same `check --candidate FULL_INITIAL_OID --initial` command. It executes
`example.check` and workflow verification; setting `message.txt` to anything other
than `hello` plus a newline fails locally and in that first-push check.

The example explicitly registers `refs/heads/integration/` in `push_baselines`
with `refs/heads/dev` as its baseline on `origin`. Every matching push compares
the whole candidate with that observed dev tip, including later repair pushes.
Creating an integration branch at the existing `dev` tip uses a real empty DELTA
and reports no selected operations. Other existing branches retain payload
before/after behavior. Creating `dev` from a commit with parents
has no prior baseline and fails preparation: supply an explicit full base/candidate
range through `workflow_dispatch` or the canonical local command. Do not use
`--initial` for parented history. An existing host's
first adoption must set `policy.adoption_base` to its exact previous full OID and
retain real base FILEMAP/projects registries. It cannot adopt unregistered history
by pretending it was an empty tree.

Add a script and action to projects.json, a `test:...` binding to check.json and
explicit FILEMAP files/edges to extend checks. Run `chrono-ci generate --host-root
. --config .chrono-harness/ci/github.json` after changing provider/bootstrap data.
Do not edit workflow YAML. `message.txt` changes invoke the real Python assertion;
README changes do not select it. Unknown inputs fail.
