# M1.4a — AMG durable foundation

Status: development branch; CI and physical resource benchmarks pending.
See [AMG architecture](amg-architecture.md).

This first vertical slice introduces a separate Rust library and explicit local
administration binary. It has no network or model dependency. Existing text
and voice entry points remain unchanged; voice cannot authorize memory writes.

Reference storage: system-linked SQLite (>=3.37 with FTS5 and unixepoch support,
test actual runtime; SQLite 3.38+ recommended), using pinned rusqlite 0.32.1.
SQLite is not encrypted. Use only synthetic data for this prototype. The
single-owner scope is an integrity check, not authentication; trust the local OS
account and private directory. Symlink/hardlink checks are basic safeguards,
not protection against a hostile host or same-UID processes. Schema version 1
is explicit; unknown versions or missing stores fail rather than reset.

The schema stores identity, entities, typed single-valued assertions (text,
integer, boolean, entity-valued edge), source IDs, recorded-time revisions
and active FTS5 entries. Corrections require an expected revision; new fact,
source, supersession and index changes are transactional. Forget removes all
revisions for the exact property and orphan sources. Physical erasure is not
guaranteed (FTS internals, backups, SSD and journal artifacts). No raw audio,
automatic transcript capture or model-authored SQL. Entity labels are explicit
metadata; aliases, entity deletion/merge, multiple-valued/conflicting claims,
world-valid intervals/as-of queries, full export and authenticated memory UI
remain future work.

Graph lookup uses indexed incoming and outgoing edges, depth <=3, fan-out
32 per direction per expansion, 256 visited node cap, 50 results and estimated
16 KiB payload cap; returns truncation rather than claiming complete results.
FTS accepts a literal phrase, not raw query syntax. Exact lookup and simple
data-only context compilation are implemented. No full policy-aware context
compiler, relevance fusion or voice integration yet. The formatted context is
untrusted data; no provider is connected. Revisions refer to recorded history,
not inferred truth or validity. No automatic overwrite of facts.

Linux development prerequisites: Rust/Cargo, Bash, Python 3, GNU coreutils,
pkg-config and SQLite development library with FTS5. Explicitly provision
Cargo dependencies with cargo fetch --locked after the generated lockfile is
committed. Builds/tests then run with --locked --offline. No dependency fetch
or model download during runtime. Existing CLI benchmark excludes AMG; report
new memory process and storage separately.

```bash
cargo build --workspace --locked --offline
# Temporary synthetic store; no private data.
umask 077
tmp="$(mktemp -d)"
target/debug/atomos-memory "$tmp/store" demo init AtomOS
printf '%s' indigo | target/debug/atomos-memory "$tmp/store" demo remember owner color text
target/debug/atomos-memory "$tmp/store" demo recall owner color
# Use the revision printed by recall for correction.
printf '%s' teal | target/debug/atomos-memory "$tmp/store" demo correct owner color 1 text
target/debug/atomos-memory "$tmp/store" demo context owner color
target/debug/atomos-memory "$tmp/store" demo forget owner color
rm -rf -- "$tmp"
```

Tests: restart identity and facts, optimistic correction, deletion/history/FTS,
scope, explicit init, malformed values, invalid reference rollback, graph cycles,
fan-out/truncation, literal FTS, context budgets and unsupported schema.
Shell integration exercises separate processes. Existing voice regressions
continue to run.

Resource settings: 4 KiB pages, 2 MiB SQLite cache target, rollback journal,
FULL synchronous, foreign keys, secure_delete ON, 250 ms busy timeout,
128 MiB main DB page cap. This is not a certified 256 MiB aggregate quota.
Full query execution deadlines, disk-full/crash tests, power-failure recovery,
backup/restore, export, migrations beyond v1, resource benchmarks and engine
comparison remain required. M1.4a is not the full M1.4 acceptance gate.
