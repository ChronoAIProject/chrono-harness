# Installed-tools host with conditional CI jobs

This scoped host registers two independent units (`example` and `harness`), one
Git detector and one aggregate. Require only **Example / aggregate** in branch
protection. Each selected unit has its own runner, bootstrap, timeout, upload and
job rerun; unselected jobs skip before allocating a runner. The aggregate runs
with `always()` and judges original selected reports through `check --collect`.

Copy this directory into a fresh repository. Install a compatible public release of the
`chrono-harness`, `chrono-judge-ci`, `chrono-ci` and `chrono-worktree` binaries in
an explicit tools directory. The sample self-hosted runner reads `/opt/chrono-tools`.
Edit every selected bootstrap argv and runner in `.chrono-harness/ci/units.json`
to match the host. The detector only installs those harness tools and reads its
explicit sparse source paths; it does not import `message.py` or run a business
check. Business SDK acquisition belongs to a selected unit's bootstrap.

The sample explicitly chooses macOS Git `/usr/bin/git` and Python
`/usr/bin/python3`. Before the first commit, register the actual Git executable
SHA-256/version in config.json and worktree.json, and the actual Python version
in config.json. These are host choices, not portable product defaults. No Rust
source checkout is required. The installed release must contain the conditional
parent and original `production` output contracts; older releases are not compatible.
Public release publication and
actual native Actions validation remain caller-owned.

Initialize and verify the projection after configuring those choices:

```sh
/usr/bin/python3 .chrono-harness/ci/install.py . /absolute/path/to/installed-tools
.chrono-harness/bin/chrono-ci init --host-root . --config .chrono-harness/ci/units.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

Commit the complete registered inventory, create the `dev` remote baseline, and
use the fixed daily commands from the host root:

```sh
.chrono-harness/bin/chrono-harness check
.chrono-harness/bin/chrono-harness check --unit example
.chrono-harness/bin/chrono-harness check --unit harness
.chrono-harness/bin/chrono-harness check --collect
```

Local collection needs the successful original reports for the selected DELTA;
it does not run missing units. Automatic CI prepares the same short commands
internally. A parentless first `dev` push uses the registered initial scoped
inventory; a parented baseline creation fails with a named endpoint error. PRs
use event base/head, ordinary pushes use complete before/after, and this sample
explicitly retains integration pushes against the detector's observed `origin/dev`.
The detector freezes that observation for the dependent jobs.

A change to `message.txt` selects `example`; a README-only DELTA selects no business
units. An incorrect greeting fails the real Python assertion. Provider/workflow
changes select `harness`, whose real check invokes generator verification. Both
endpoint registrations govern dependency, assignment, deletion and rename changes.
Unregistered inputs and missing evidence fail; no directory/language inference or
select-all fallback is used.

The detector reads the complete Git DELTA, including more than 3,000 paths.
No harness file-count gate is applied.

This example demonstrates scoped checks. It does not activate full governance,
supply full birth/input certificates, or establish local/native parity.
The [legacy example](../ci-host/README.md) keeps its original explicit command and
workflow contract for historical consumers.
