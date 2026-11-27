#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --locked --offline -p atomos-core --features atomic-admission
bash scripts/benchmark-admission.sh
