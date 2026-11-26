#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
m=target/debug/atomos-memory
db="$tmp/store"
first="$("$m" "$db" test init AtomOS)"
[[ "$("$m" "$db" test identity)" == "$first name=\"AtomOS\"" ]]
saved="$(printf %s indigo | "$m" "$db" test remember owner color text)"
[[ "$("$m" "$db" test recall owner color)" == "$saved" ]]
revision="$(printf '%s\n' "$saved" | sed -n 's/^revision=\([0-9]*\).*/\1/p')"
[[ -n "$revision" ]]
printf %s teal | "$m" "$db" test correct owner color "$revision" text
"$m" "$db" test context owner color | grep -Fq 'text:"teal"'
"$m" "$db" test search indigo | grep -Fxq 'truncated=false'
"$m" "$db" test forget owner color | grep -Fxq 'deleted=2'
[[ "$("$m" "$db" test recall owner color)" == 'not found' ]]
[[ "$("$m" "$db" test search teal)" == 'truncated=false' ]]
if "$m" "$db" other identity; then echo 'Scope rejection failed' >&2; exit 1; fi
echo 'PASS: AMG learn, restart, recall, correct, forget and scope rejection'
