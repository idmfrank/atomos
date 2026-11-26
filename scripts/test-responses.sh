#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --locked --offline -p atomos-core --features responses-replay --test responses
cargo build --release --locked --offline -p atomos-core --features responses-replay --example responses_bench
wc -c target/release/examples/responses_bench
/usr/bin/time -f 'responses_decoder_process elapsed_s=%e peak_rss_kib=%M' target/release/examples/responses_bench
