# M1.4a — AMG durable foundation

> **Milestone context (reviewed through PR #28, 2026-11-26):** The original schema v1 and verification numbers below are historical. Later milestones implemented v2/v3 migration, scoped retrieval, revision-specific voice grants, indexed deletion, optimized retrieval and verified logical recovery. Active scale benchmarks now pass through 100K synthetic entities. See [AMG architecture](amg-architecture.md), [optimization](m1-amg-optimization.md) and [current resource budget](resource-budget.md). Private deployment remains gated. [Current project state](../PROJECT_STATE.md) is authoritative; status and next-step statements below describe the original milestone unless explicitly updated.


Status: merged and locked offline CI passed. Active AMG resource benchmarks pending.
See [AMG architecture](amg-architecture.md).

This first vertical slice introduces a separate Rust library and explicit local
administration binary. It has no network or model dependency. At this slice, text and voice entry points were unchanged. M1.4b subsequently integrates read-only recall; see [M1.4b](m1-amg-conversation.md). Voice cannot authorize memory writes.

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
Cargo dependencies with cargo fetch --locked using the committed Cargo.lock.
The new dependency set has not been validated on Rust 1.75; CI uses the installed newer toolchain. Builds/tests then run with --locked --offline. No dependency fetch
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

## Initial validation

[Bootstrap Actions run 45754526687](https://github.com/idmfrank/atomos/actions/runs/45754526687) passed all 35 Rust tests (6 new memory tests), 5 existing Python tests and CLI/stub audio/AMG process-level smoke checks. The generated dependency lockfile is committed without manual version selection; the temporary generation step is removed. The committed-lockfile build passed after merge in [run 45754844830](https://github.com/idmfrank/atomos/actions/runs/45754844830), including 35 Rust and 5 Python tests and separate-process memory lifecycle checks. Existing text-only core release executable: 314,224 bytes, peak RSS 2,088 KiB; excludes the memory binary and SQLite runtime. No AMG resource measurements yet.
