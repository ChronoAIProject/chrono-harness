# Explicit installed-tools host

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
`dev`; the normal push/PR routes are configured explicitly. An existing host's
first adoption must set `policy.adoption_base` to its exact previous full OID and
retain real base FILEMAP/projects registries. It cannot adopt unregistered history
by pretending it was an empty tree.

Add a script and action to projects.json, a `test:...` binding to check.json and
explicit FILEMAP files/edges to extend checks. Run `chrono-ci generate --host-root
. --config .chrono-harness/ci/github.json` after changing provider/bootstrap data.
Do not edit workflow YAML. `message.txt` changes invoke the real Python assertion;
README changes do not select it. Unknown inputs fail.
