#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
command -v cargo >/dev/null || { echo "Rust/Cargo required (>=1.75)." >&2; exit 127; }
cargo test --workspace --locked --offline
cargo build --workspace --locked --offline
output="$(printf '%s\n' 'what is your name?' 'explain entropy' '/quit' | cargo run --quiet --locked --offline -p atomos-cli --bin atomos)"
printf '%s\n' "$output"
grep -Fxq 'I am AtomOS.' <<< "$output"
grep -Fxq 'Mock response: explain entropy' <<< "$output"
grep -Fxq 'Simulated inference calls: 1' <<< "$output"
echo 'PASS: workspace tests and offline CLI smoke test'
echo 'Running AMG process-level persistence tests'
timeout --kill-after=5s 30s bash scripts/test-memory.sh

echo 'Running conversational AMG integration tests'
timeout --kill-after=5s 40s python3 -u scripts/test-conversation-memory.py -v

echo 'Running AMG crash recovery test'
timeout --kill-after=5s 30s python3 -u scripts/test-recovery.py -v


echo 'Running synthetic inference broker smoke test'
(
  umask 077
  broker_tmp="$(mktemp -d)"
  trap 'rm -rf -- "$broker_tmp"' EXIT
  output="$(timeout --kill-after=5s 20s cargo run --quiet --locked --offline -p atomos-core --example broker_demo -- "$broker_tmp")"
  printf '%s\n' "$output"
  grep -Fxq 'I am AtomOS.' <<< "$output"
  grep -Fxq 'Synthetic replay: entropy example.' <<< "$output"
  grep -Fxq 'Synthetic dispatches: 1' <<< "$output"
  grep -Fxq 'Synthetic charged units: 47' <<< "$output"
  grep -Fxq 'Real provider calls: 0' <<< "$output"
)
