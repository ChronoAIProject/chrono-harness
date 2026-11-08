#!/bin/sh
# 启动入口的 shell 行为预期；外部工具接口由独立登记的 shell fixture 提供。
set -eu
task_repo=$(pwd -P)
task_entry="$task_repo/.chrono-harness/ci/bootstrap-native.sh"
task_fixture="$task_repo/.chrono-harness/ci/fixtures/bootstrap-launcher-tool.sh"
test -x "$task_fixture" || { echo 'missing registered launcher fixture' >&2; exit 2; }
task_temporary=$(mktemp -d "${TMPDIR:-/tmp}/chrono-native-launcher.XXXXXX")
task_temporary=$(CDPATH= cd -- "$task_temporary" && pwd -P)
trap 'task_status=$?; if [ "$task_status" -eq 0 ]; then rm -rf -- "$task_temporary"; else echo "launcher fixture retained: $task_temporary" >&2; fi' 0
mkdir "$task_temporary/tools"
cp "$task_fixture" "$task_temporary/tools/rustup"
export PATH="$task_temporary/tools:$PATH"
export CHRONO_LAUNCHER_CALLS="$task_temporary/calls"
unset CHRONO_WORKTREE_BOOTSTRAP RUSTUP_HOME
export CHRONO_LAUNCHER_RUSTUP_HOME="$task_temporary/selected-rustup"
task_main="$task_temporary/main host"
/usr/bin/git init --quiet "$task_main"
/usr/bin/git -C "$task_main" -c user.name=Fixture -c user.email=fixture@example.invalid commit --quiet --allow-empty -m fixture
task_linked="$task_temporary/linked host"
/usr/bin/git -C "$task_main" worktree add --quiet --detach "$task_linked" HEAD

fail() { echo "$*" >&2; exit 1; }
invoke() {
    task_status=0
    /bin/sh "$task_entry" "$@" > "$task_temporary/stdout" 2> "$task_temporary/stderr" || task_status=$?
}
reset_calls() { : > "$CHRONO_LAUNCHER_CALLS"; }

# 首次独立主检出可以初始化；编译器的原退出不能变成成功。
export CHRONO_LAUNCHER_COMPILER_EXIT=17
invoke "$task_main" .chrono-harness/ci/bootstrap-core.json
test "$task_status" -eq 17 || fail 'initial compiler exit was lost'
task_calls=$(cat "$CHRONO_LAUNCHER_CALLS")
case "$task_calls" in 'rustup
<run>
<--install>
<1.95.0>
'*) ;; *) fail 'initial compiler selection differs' ;; esac
unset CHRONO_LAUNCHER_COMPILER_EXIT

# 关联检出没有协调工具时，必须在编译前拒绝。
reset_calls
invoke "$task_linked" .chrono-harness/ci/bootstrap-core.json
test "$task_status" -ne 0 || fail 'missing linked coordinator was accepted'
test ! -s "$CHRONO_LAUNCHER_CALLS" || fail 'compiled before linked ownership admission'

# 已有协调者负责前端构建，随后运行保留的前端，路径含空格也须完整转交。
mkdir -p "$task_main/.chrono-harness/bin"
cp "$task_fixture" "$task_main/.chrono-harness/bin/chrono-worktree"
reset_calls
export CHRONO_LAUNCHER_NATIVE_EXIT=23
invoke "$task_linked" .chrono-harness/ci/bootstrap-shared.json install unit_runner
test "$task_status" -eq 23 || fail 'native frontend exit was lost'
task_expected=$(printf '%s\n' chrono-worktree '<bootstrap>' '<--host-root>' "<$task_linked>" '<--config>' '<.chrono-harness/worktree.json>' '<--bootstrap-config>' '<.chrono-harness/ci/bootstrap-frontend.json>' rustup '<run>' '<1.95.0>' "<$task_linked/.chrono-harness/bin/chrono-host-bootstrap>" "<$task_linked>" '<.chrono-harness/ci/bootstrap-shared.json>' '<install>' '<unit_runner>' chrono-host-bootstrap "<$task_linked>" '<.chrono-harness/ci/bootstrap-shared.json>' '<install>' '<unit_runner>')
test "$(cat "$CHRONO_LAUNCHER_CALLS")" = "$task_expected" || fail 'coordinator/frontend invocation differs'
unset CHRONO_LAUNCHER_NATIVE_EXIT

# 协调者拒绝时不运行旧前端，也不退回无保护编译。
reset_calls
export CHRONO_LAUNCHER_OWNER_EXIT=19
invoke "$task_linked" .chrono-harness/ci/bootstrap-core.json
test "$task_status" -eq 19 || fail 'coordinator refusal exit was lost'
task_calls=$(cat "$CHRONO_LAUNCHER_CALLS")
case "$task_calls" in *rustup*|*chrono-host-bootstrap*) fail 'coordinator refusal fell back to execution' ;; esac
unset CHRONO_LAUNCHER_OWNER_EXIT

# 原所有者的重入标记须与物理根和实际传递的使用凭据一起出现。
reset_calls
export CHRONO_WORKTREE_BOOTSTRAP="$task_main"
invoke "$task_linked" .chrono-harness/ci/bootstrap-core.json
test "$task_status" -ne 0 || fail 'mismatched bootstrap root was accepted'
test ! -s "$CHRONO_LAUNCHER_CALLS" || fail 'compiled with mismatched root'
export CHRONO_WORKTREE_BOOTSTRAP="$task_linked"
task_inherited_fds=${CHRONO_PROCESS_FDS-}
unset CHRONO_PROCESS_FDS
invoke "$task_linked" .chrono-harness/ci/bootstrap-core.json
test "$task_status" -ne 0 || fail 'bootstrap marker without capabilities was accepted'
test ! -s "$CHRONO_LAUNCHER_CALLS" || fail 'compiled without capabilities'
exec 9< "$CHRONO_LAUNCHER_CALLS"
export CHRONO_PROCESS_FDS=9
invoke "$task_linked" .chrono-harness/ci/bootstrap-owner-stage.json --owner-stage-validation report digest
test "$task_status" -eq 0 || fail 'protected compilation was refused'
task_calls=$(cat "$CHRONO_LAUNCHER_CALLS")
case "$task_calls" in 'rustup
'*'<--owner-stage-validation>
<report>
<digest>') ;; *) fail 'protected compiler arguments differ' ;; esac
exec 9<&-
if [ -n "$task_inherited_fds" ]; then export CHRONO_PROCESS_FDS="$task_inherited_fds"; else unset CHRONO_PROCESS_FDS; fi
unset CHRONO_WORKTREE_BOOTSTRAP
/usr/bin/git -C "$task_main" worktree remove --force "$task_linked"
echo 'launcher behavior: passed'
