#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --locked --offline -p atomos-tasks --all-targets
cargo build --locked --offline -p atomos-tasks --bin atomos-taskd
target="$(realpath -m "${CARGO_TARGET_DIR:-target}")"
TASKD_BIN="$target/debug/atomos-taskd" timeout --kill-after=5s 90s python3 -u scripts/test-task-service.py
