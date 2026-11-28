#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
cargo test --locked --offline --workspace --features session-replay
cargo build --locked --offline --workspace
target="$(realpath -m "${CARGO_TARGET_DIR:-target}")"
cp "$target/debug/atomos" "$tmp/default"
export SESSION_DEFAULT_BIN="$tmp/default" SESSION_ADMIN_BIN="$target/debug/atomos-memory"
cargo build --locked --offline -p atomos-cli --features session-replay
cp "$target/debug/atomos" "$tmp/replay"
export SESSION_REPLAY_BIN="$tmp/replay"
reference=1b07356a6ceaaaa724e3ce3329500d206cb88cf9
mkdir "$tmp/reference-source"
git archive "$reference" | tar -x -C "$tmp/reference-source"
(cd "$tmp/reference-source" && CARGO_TARGET_DIR="$tmp/reference-target" cargo build --locked --offline -p atomos-cli)
export SESSION_REFERENCE_BIN="$tmp/reference-target/debug/atomos"
timeout --kill-after=5s 60s python3 -u scripts/test-session.py
