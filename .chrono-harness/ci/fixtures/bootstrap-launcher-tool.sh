#!/bin/sh
# 启动入口测试的外部命令替身；不编译工具，也不取得真实租约。
set -eu
task_name=${0##*/}
{
    printf '%s\n' "$task_name"
    for task_argument do printf '<%s>\n' "$task_argument"; done
} >> "$CHRONO_LAUNCHER_CALLS"
case "$task_name" in
    chrono-worktree) task_exit=${CHRONO_LAUNCHER_OWNER_EXIT:-0} ;;
    rustup)
        if [ "$#" -ge 3 ] && [ "$1" = run ] && [ "$2" = 1.95.0 ]; then
            if [ "$3" = cargo ]; then
                if [ "$#" -eq 4 ] && [ "$4" = --version ]; then
                    task_exit=${CHRONO_LAUNCHER_PROBE_EXIT:-0}
                else
                    task_exit=${CHRONO_LAUNCHER_COMPILER_EXIT:-0}
                fi
            else
                shift 2
                export RUSTUP_HOME="$CHRONO_LAUNCHER_RUSTUP_HOME"
                exec "$@"
            fi
        elif [ "$#" -ge 2 ] && [ "$1" = toolchain ] && [ "$2" = install ]; then
            task_exit=${CHRONO_LAUNCHER_INSTALL_EXIT:-0}
        else
            task_exit=${CHRONO_LAUNCHER_COMPILER_EXIT:-0}
        fi
        ;;
    chrono-host-bootstrap)
        test "${RUSTUP_HOME-}" = "$CHRONO_LAUNCHER_RUSTUP_HOME" || {
            echo 'frontend did not inherit the selected Rustup home' >&2; exit 2;
        }
        task_exit=${CHRONO_LAUNCHER_NATIVE_EXIT:-0}
        ;;
    *) echo 'unknown launcher fixture tool' >&2; exit 2 ;;
esac
if [ "$task_name" = chrono-worktree ] && [ "$task_exit" -eq 0 ]; then
    task_root=
    while [ "$#" -gt 1 ]; do
        if [ "$1" = --host-root ]; then task_root=$2; break; fi
        shift
    done
    : "${task_root:?missing host root}"
    mkdir -p "$task_root/.chrono-harness/bin"
    cp "$0" "$task_root/.chrono-harness/bin/chrono-host-bootstrap"
fi
exit "$task_exit"
