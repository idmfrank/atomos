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
