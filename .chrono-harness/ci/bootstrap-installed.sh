#!/bin/sh
# 共享安装已核验前端身份；单元只转发调用，避免重新安装改变原产物。
set -eu
test "$#" -eq 2 || { echo 'usage: bootstrap-installed.sh HOST CONFIG' >&2; exit 2; }
task_root=$(CDPATH= cd -- "$1" && pwd -P)
exec "$task_root/.chrono-harness/bin/chrono-host-bootstrap" "$@"
