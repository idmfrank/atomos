#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
filesystem="$(findmnt -n -o FSTYPE -T "$tmp")"
[[ "$filesystem" != tmpfs ]] || { echo 'Persistent TMPDIR required; tmpfs is not device durability evidence.' >&2; exit 1; }
target="$(realpath -m "${CARGO_TARGET_DIR:-target}")"
cargo build --release --locked --offline -p atomos-tasks --example task_bench
changes="$(git status --porcelain --untracked-files=normal | wc -l)"
echo "TASK_BENCH filesystem=$filesystem commit=$(git rev-parse HEAD) worktree_changes=$changes synthetic_only=1"
/usr/bin/time -f 'task_bench peak_rss_kib=%M elapsed_s=%e' "$target/release/examples/task_bench" "$tmp/state"
wc -c "$target/release/examples/task_bench"
