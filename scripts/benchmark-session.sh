#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
samples="${SESSION_SAMPLES:-300}"
[[ "$samples" =~ ^[1-9][0-9]*$ ]] && ((samples<=1000))
filesystem="$(findmnt -n -o FSTYPE -T "$tmp")"
[[ "$filesystem" != tmpfs ]] || { echo 'Persistent TMPDIR required' >&2; exit 1; }
echo "SESSION_BENCH samples=$samples filesystem=$filesystem commit=$(git rev-parse HEAD)"
rustc --version
target="$(realpath -m "${CARGO_TARGET_DIR:-target}")"
cargo build --release --locked --offline -p atomos-core --features atomic-admission --example session_bench
for mode in direct session; do
  for kind in text refusal failed-known failed-unknown cancelled; do
    mkdir -m700 "$tmp/$mode-$kind"
    /usr/bin/time -f "mode=$mode kind=$kind peak_rss_kib=%M elapsed_s=%e" "$target/release/examples/session_bench" "$mode" "$kind" "$tmp/$mode-$kind" "$samples"
  done
  if command -v strace >/dev/null; then
    mkdir -m700 "$tmp/$mode-trace"
    strace -yy -o "$tmp/$mode.strace" -e trace=write,fsync "$target/release/examples/session_bench" "$mode" text "$tmp/$mode-trace" 3 trace >/dev/null
    python3 scripts/count-outcome-io.py "$mode-session-comparison" "$tmp/$mode.strace"
  else
    echo 'syscall_measurement=pending strace_unavailable=1'
  fi
done
cmp "$tmp/direct-text/state/synthetic-atomic-admission-v1/admission.log" "$tmp/session-text/state/synthetic-atomic-admission-v1/admission.log"
echo 'same_wire_journal_parity_mismatches=0'
cargo build --release --locked --offline -p atomos-cli
cp "$target/release/atomos" "$tmp/default"
cargo build --release --locked --offline -p atomos-cli --features session-replay
cp "$target/release/atomos" "$tmp/replay"
wc -c "$tmp/default" "$tmp/replay" "$target/release/examples/session_bench"
/usr/bin/time -f 'mode=cli-pipe process_peak_rss_kib=%M elapsed_s=%e includes_python_harness=1' timeout --kill-after=5s 30s python3 scripts/benchmark-session-pipe.py "$tmp/replay" "$tmp/pipe" "$samples"
python3 -c 'import sys;open(sys.argv[1],"w").write("question\n"*int(sys.argv[2])+"/status\n/quit\n")' "$tmp/input" "$samples"
/usr/bin/time -f 'mode=local-mock cli_peak_rss_kib=%M elapsed_s=%e' "$tmp/default" --session < "$tmp/input" > "$tmp/mock-output"
mkdir -m700 "$tmp/cli-rss"
/usr/bin/time -f 'mode=synthetic-replay cli_peak_rss_kib=%M elapsed_s=%e' "$tmp/replay" --session --replay text --ledger "$tmp/cli-rss" --init-ledger < "$tmp/input" > "$tmp/replay-output"
grep -Fq "charged=$((samples*47)) background=0 pending=0" "$tmp/replay-output"
echo SESSION_BENCH_complete
