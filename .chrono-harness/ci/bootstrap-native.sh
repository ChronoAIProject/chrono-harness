#!/bin/sh
# 本宿主的 Rust 构建入口；调用方持有既有所有者的 bootstrap 使用保护。
set -eu
task_root=$(CDPATH= cd -- "$1" && pwd -P)
exec rustup run 1.95.0 cargo run --locked --manifest-path "$task_root/.chrono-harness/ci/native/Cargo.toml" --target-dir "$task_root/.chrono-harness/cache/host-bootstrap-native/target" --bin chrono-host-bootstrap -- "$@"
