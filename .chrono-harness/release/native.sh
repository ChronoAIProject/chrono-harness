#!/bin/sh
# 构建并运行本宿主登记的 Rust 发布工具。
set -eu
test "$#" -ge 3 || { echo 'usage: native.sh ROOT BOOTSTRAP_PROFILE OUTPUT [FLAGS]' >&2; exit 2; }
task_root=$(CDPATH= cd -- "$1" && pwd -P)
/bin/bash "$task_root/.chrono-harness/ci/bootstrap-native.sh" "$task_root" "$2"
shift 2
exec "$task_root/.chrono-harness/bin/chrono-host-release" "$task_root" "$@"
