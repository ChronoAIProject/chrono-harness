#!/bin/sh
# 本宿主的薄启动入口；已部署所有者保护前端编译，首次主检出可初始化。
set -eu
test "$#" -ge 2 || { echo 'usage: bootstrap-native.sh HOST CONFIG [ARGUMENTS]' >&2; exit 2; }
task_root=$(CDPATH= cd -- "$1" && pwd -P)
if [ "${CHRONO_WORKTREE_BOOTSTRAP+x}" = x ]; then
    test "$CHRONO_WORKTREE_BOOTSTRAP" = "$task_root" && test -n "${CHRONO_PROCESS_FDS-}" || {
        echo 'bootstrap reentry requires the original root and ownership capabilities' >&2; exit 2;
    }
else
    task_common=$(/usr/bin/git -C "$task_root" rev-parse --path-format=absolute --git-common-dir)
    case "$task_common" in */.git) task_anchor=${task_common%/.git} ;; *) echo 'unsupported bootstrap Git owner' >&2; exit 2 ;; esac
    task_owner="$task_anchor/.chrono-harness/bin/chrono-worktree"
    if [ -e "$task_owner" ] || [ -L "$task_owner" ]; then
        "$task_owner" bootstrap --host-root "$task_root" --config .chrono-harness/worktree.json --bootstrap-config .chrono-harness/ci/bootstrap-frontend.json
        exec "$task_root/.chrono-harness/bin/chrono-host-bootstrap" "$@"
    fi
    test "$task_anchor" = "$task_root" || { echo 'linked bootstrap requires its deployed coordinator' >&2; exit 2; }
fi
exec rustup run --install 1.95.0 cargo run --locked --manifest-path "$task_root/.chrono-harness/ci/native/Cargo.toml" --target-dir "$task_root/.chrono-harness/cache/host-bootstrap-native/target" --bin chrono-host-bootstrap -- "$@"
