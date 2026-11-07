#!/bin/sh
# 本宿主的共享入口；产物布局与安装器绑定由 bootstrap-shared.json 显式采用。
set -eu
test "$#" -ge 3 || { echo 'usage: bootstrap-shared.sh HOST CONFIG produce|install CONSUMER|verify PROFILE' >&2; exit 2; }
task_root=$(CDPATH= cd -- "$1" && pwd -P)
case "$3:$#" in
    produce:3) exec /bin/sh "$task_root/.chrono-harness/ci/bootstrap-native.sh" "$@" ;;
    install:4) task_relative=.chrono-harness/startup/tools/core/chrono-host-bootstrap ;;
    verify:4) task_relative=.chrono-harness/bin/chrono-host-bootstrap ;;
    *) echo 'invalid shared startup arguments' >&2; exit 2 ;;
esac
# 保持 Rust 物理路径合同；执行和 chmod 均不得经过符号链接。
task_path=$task_root
task_remaining=$task_relative
while :; do
    task_part=${task_remaining%%/*}
    task_path=$task_path/$task_part
    test ! -L "$task_path" || { echo 'symlink in shared startup executable path' >&2; exit 2; }
    case "$task_remaining" in */*) task_remaining=${task_remaining#*/} ;; *) break ;; esac
done
test -f "$task_path" || { echo 'missing shared startup executable' >&2; exit 2; }
if [ "$3" = install ]; then
    task_expected=${CHRONO_STARTUP_INSTALLER_SHA256-}
    test "${#task_expected}" -eq 64 || { echo 'missing or invalid startup installer binding' >&2; exit 2; }
    case "$task_expected" in *[!0-9a-f]*) echo 'invalid startup installer binding' >&2; exit 2 ;; esac
    task_observed=$(/usr/bin/shasum -a 256 "$task_path")
    test "${task_observed%% *}" = "$task_expected" || { echo 'startup installer bytes differ from producer binding' >&2; exit 2; }
    chmod 755 "$task_path"
fi
exec "$task_path" "$@"
