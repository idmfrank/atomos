#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
samples="${ADMISSION_SAMPLES:-300}"
[[ "$samples" =~ ^[1-9][0-9]*$ ]] && ((samples<=1000))
echo "ATOMIC_ADMISSION version=1 samples=$samples filesystem=$(stat -f -c %T "$tmp")"
uname -smr
cargo build --release --locked --offline -p atomos-core --features responses-replay --example admission_bench
cp target/release/examples/admission_bench "$tmp/reference"
cargo build --release --locked --offline -p atomos-core --features atomic-admission --example admission_bench
cp target/release/examples/admission_bench "$tmp/candidate"
wc -c "$tmp/reference" "$tmp/candidate"
for mode in reference candidate; do
  mkdir -m700 "$tmp/$mode-state"
  /usr/bin/time -f "mode=$mode process_peak_rss_kib=%M elapsed_s=%e" "$tmp/$mode" "$mode" "$tmp/$mode-state" "$samples"
done
# Trace separately so syscall instrumentation never contaminates reported latency/RSS.
if command -v strace >/dev/null; then
  for mode in reference candidate; do
    mkdir -m700 "$tmp/$mode-trace"
    strace -yy -o "$tmp/$mode.strace" -e trace=write,fsync "$tmp/$mode" "$mode" "$tmp/$mode-trace" 3 trace >/dev/null
    python3 scripts/count-admission-io.py "$mode" "$tmp/$mode.strace"
  done
else
  echo 'syscall_measurement=pending strace_unavailable=1'
fi
