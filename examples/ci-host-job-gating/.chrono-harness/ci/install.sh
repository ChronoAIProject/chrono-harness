#!/bin/sh
set -eu
[ "$#" -eq 2 ] || { echo 'usage: install.sh HOST_ROOT TOOL_DIRECTORY' >&2; exit 2; }
destination=$1/.chrono-harness/bin
mkdir -p "$destination"
exec cp -p "$2/chrono-harness" "$2/chrono-judge-ci" "$2/chrono-ci" "$2/chrono-worktree" "$destination/"
