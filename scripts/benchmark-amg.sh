#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
[[ -x /usr/bin/time ]] || { echo 'GNU time required' >&2; exit 2; }
samples="$(printenv AMG_SAMPLES || echo 100)"
[[ "$samples" =~ ^[0-9]+$ ]] || exit 2
stage_seconds="$(printenv AMG_STAGE_SECONDS || echo 180)"
[[ "$stage_seconds" =~ ^[1-9][0-9]*$ ]] || exit 2
if [[ "$#" == 0 ]]; then set -- 1000; fi
for n in "$@"; do [[ "$n" =~ ^[0-9]+$ ]] || exit 2; done
cargo build --release --locked --offline -p atomos-memory --example amg_bench
cargo build --release --locked --offline -p atomos-cli
bench=target/release/examples/amg_bench
cli=target/release/atomos
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
echo "AMG_BENCH version=2 samples=$samples stage_timeout_seconds=$stage_seconds"
uname -smr
rustc --version
pkg-config --modversion sqlite3
grep -m1 'model name' /proc/cpuinfo || true
grep -m1 'MemTotal' /proc/meminfo || true
wc -c "$bench" "$cli"
failed=0
{ for ((i=0;i<1000;i++)); do echo 'what is your name?'; done; echo '/quit'; } > "$tmp/baseline.in"
{ for ((i=0;i<1000;i++)); do echo 'recall node000000 status'; done; echo '/quit'; } > "$tmp/active.in"
/usr/bin/time -f 'metric=cli_no_store elapsed_s=%e peak_rss_kib=%M' "$cli" < "$tmp/baseline.in" > "$tmp/baseline.out"
grep -Fxq 'Simulated inference calls: 0' "$tmp/baseline.out"
for n in "$@"; do
  dir="$tmp/store-$n"
  echo "BEGIN_AMG_DATASET entities_excluding_owner=$n"
  echo "BEGIN_AMG_STAGE dataset=$n stage=seed"
  if timeout --kill-after=5s "$stage_seconds" /usr/bin/time -f 'metric=seed_process elapsed_s=%e peak_rss_kib=%M' "$bench" seed "$dir" "$n" "$samples"; then
    echo "END_AMG_STAGE dataset=$n stage=seed"
  else
    rc=$?; echo "DATASET_FAILED dataset=$n stage=seed exit=$rc"; failed=1; continue
  fi
  stat -c 'db_bytes_after_seed=%s' "$dir/memory.db"
  du -sb "$dir" | awk '{print "quiescent_store_apparent_bytes=" $1}'
  modes=(read graph write)
  if [[ "$(printenv AMG_RECOVERY || echo 0)" == 1 ]]; then modes+=(recovery); fi
  for mode in "${modes[@]}"; do
    echo "BEGIN_AMG_STAGE dataset=$n stage=$mode"
    if timeout --kill-after=5s "$stage_seconds" /usr/bin/time -f "metric=$mode-process elapsed_s=%e peak_rss_kib=%M" "$bench" "$mode" "$dir" "$n" "$samples"; then
      echo "END_AMG_STAGE dataset=$n stage=$mode"
    else
      rc=$?; echo "DATASET_FAILED dataset=$n stage=$mode exit=$rc"; failed=1
    fi
  done
  echo "BEGIN_AMG_STAGE dataset=$n stage=active_cli"
  timeout --kill-after=5s "$stage_seconds" /usr/bin/time -f 'metric=cli_active_exact elapsed_s=%e peak_rss_kib=%M' "$cli" --memory "$dir" bench < "$tmp/active.in" > "$tmp/active.out"
  grep -Fxq 'Simulated inference calls: 0' "$tmp/active.out"
  grep -Fxq 'Local memory recalls: 1000' "$tmp/active.out"
  [[ "$(grep -Fxc 'I have status recorded as marker000000 validation passed.' "$tmp/active.out")" == 1000 ]]
  stat -c 'db_bytes_after_writes=%s' "$dir/memory.db"
  du -sb "$dir" | awk '{print "quiescent_store_after_writes_apparent_bytes=" $1}'
  echo "END_AMG_DATASET entities_excluding_owner=$n"
  rm -rf -- "$dir"
done
echo "AMG_BENCH completed failed=$failed"
exit "$failed"
