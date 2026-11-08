#!/bin/sh
# 原生 job 只恢复下载工具的 mode，再由 Rust 核验原件并执行业务。
set -eu
test "$#" -ge 3 || { echo 'usage: consume.sh ROOT FRONTEND_DIRECTORY OUTPUT [FLAGS]' >&2; exit 2; }
task_root=$(CDPATH= cd -- "$1" && pwd -P)
task_frontend=$(CDPATH= cd -- "$task_root" && cd -- "$2" && pwd -P)
tar -xpf "$task_frontend/executable.tar" -C "$task_frontend" executable
mv "$task_frontend/executable" "$task_frontend/chrono-host-release"
shift 2
exec "$task_frontend/chrono-host-release" "$task_root" "$@"
