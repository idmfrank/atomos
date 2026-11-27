#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
samples="${OUTCOME_SAMPLES:-300}"
[[ "$samples" =~ ^[1-9][0-9]*$ ]] && ((samples<=1000))
filesystem="$(findmnt -n -o FSTYPE -T "$tmp")"
[[ "$filesystem" != tmpfs ]] || { echo 'Persistent TMPDIR required; tmpfs is not storage durability evidence.' >&2; exit 1; }
reference=c563599f2e2c8b5abb7e03c75bb85e67e831e75d
git cat-file -e "$reference^{commit}"
echo "OUTCOME_BENCH samples=$samples filesystem=$filesystem reference=$reference candidate=$(git rev-parse HEAD)"
uname -smr
rustc --version
mkdir -m700 "$tmp/reference-source"
git archive "$reference" | tar -x -C "$tmp/reference-source"
# Identical harness, separately compiled against frozen runtime. The extended
# branch is cfg-excluded in the reference. No runtime file is replaced.
cp crates/atomos-core/examples/outcome_bench.rs "$tmp/reference-source/crates/atomos-core/examples/admission_bench.rs"
(cd "$tmp/reference-source" && CARGO_TARGET_DIR="$tmp/reference-target" cargo build --release --locked --offline -p atomos-core --features atomic-admission --example admission_bench)
cp "$tmp/reference-target/release/examples/admission_bench" "$tmp/frozen"
CARGO_TARGET_DIR="$tmp/candidate-target" RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg atomos_extended_candidate" cargo build --release --locked --offline -p atomos-core --features atomic-admission --example outcome_bench
cp "$tmp/candidate-target/release/examples/outcome_bench" "$tmp/extended"
wc -c "$tmp/frozen" "$tmp/extended"
mkdir -m700 "$tmp/frozen-state"
/usr/bin/time -f 'mode=frozen case=strict process_peak_rss_kib=%M elapsed_s=%e' "$tmp/frozen" strict "$tmp/frozen-state" "$samples"
for case in strict auxiliary reasoning refusal; do
  mkdir -m700 "$tmp/extended-$case"
  /usr/bin/time -f "mode=extended case=$case process_peak_rss_kib=%M elapsed_s=%e" "$tmp/extended" "$case" "$tmp/extended-$case" "$samples"
done
if command -v strace >/dev/null; then
  for mode in frozen extended; do
    mkdir -m700 "$tmp/$mode-trace"
    strace -yy -o "$tmp/$mode.strace" -e trace=write,fsync "$tmp/$mode" strict "$tmp/$mode-trace" 3 trace >/dev/null
    python3 scripts/count-outcome-io.py "$mode-strict" "$tmp/$mode.strace"
  done
  mkdir -m700 "$tmp/refusal-trace"
  strace -yy -o "$tmp/refusal.strace" -e trace=write,fsync "$tmp/extended" refusal "$tmp/refusal-trace" 3 trace >/dev/null
  python3 scripts/count-outcome-io.py extended-refusal "$tmp/refusal.strace"
else
  echo 'syscall_measurement=pending strace_unavailable=1'
fi
echo OUTCOME_BENCH_complete
