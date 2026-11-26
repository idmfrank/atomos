#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo build --release --locked --offline -p atomos-cli
binary=target/release/atomos
wc -c "$binary"
if [[ ! -x /usr/bin/time ]]; then
  echo "GNU time required for process-level RSS and elapsed time" >&2
  exit 2
fi
# Text-only simulation, NOT a conversational latency or power benchmark.
{ for ((i=0;i<1000;i++)); do echo 'what is your name?'; done; echo '/quit'; } |
  /usr/bin/time -f 'elapsed_seconds=%e peak_rss_kib=%M' "$binary" >/dev/null
echo 'Physical ASR/TTS, latency, and wall power remain unmeasured.'
