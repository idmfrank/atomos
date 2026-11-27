# M1.4f — Retrieval optimization, streaming recovery and verification

> **Milestone context (reviewed through PR #28, 2026-11-26):** M1.4f's reference and optimized measurements below are historical verified results. Subsequent CI has repeatedly exercised the same synthetic 1K/10K/100K workload; consult the current [resource budget](resource-budget.md) and [README](../README.md) for latest measurements. Hardware efficiency, private-data deployment and aggregate hard quota remain unqualified. [Current project state](../PROJECT_STATE.md) is authoritative; status and next-step statements below describe the original milestone unless explicitly updated.


Status: merged in PR #11; post-merge and combined scale validation passed.

## Reference baseline and decision

PR #10 merged at aaef767f30bdcd815ba603547aa991630837c7f3. Post-merge
[CI 45803143396](https://github.com/idmfrank/atomos/actions/runs/45803143396):
46 Rust + 11 Python tests, smoke checks, 1k benchmark; executable 464,912 bytes,
text-only peak RSS 2,776 KiB. Original scale run [45801571163](https://github.com/idmfrank/atomos/actions/runs/45801571163)
was cancelled during 10k writes; no 100k results. Corrected individual
[10k](https://github.com/idmfrank/atomos/actions/runs/45803643785) and
[100k](https://github.com/idmfrank/atomos/actions/runs/45803919302) runs
passed, as did [combined 45804554627](https://github.com/idmfrank/atomos/actions/runs/45804554627).
Combined elapsed 59 s including compilation; workload ~21 s. AMD EPYC 7763,
SQLite 3.46.1, 100 samples (30 common-term).

| Metric | 1k | 10k | 100k |
| --- | ---: | ---: | ---: |
| Assertions | 2500 | 25000 | 250000 |
| Main DB bytes | 1216512 | 11186176 | 116084736 |
| Exact p95 ms | 0.0755 | 0.0679 | 0.0854 |
| Graph p95 ms | 0.2022 | 0.1932 | 0.2188 |
| Context p95 ms | 0.3538 | 0.3345 | 0.3750 |
| Common-term p95 ms | 0.6135 | 4.9301 | 44.8992 |
| Forget p95 ms | 2.0212 | 1.6081 | 1.9931 |
| Write phase seconds | 0.51 | 0.75 | 3.06 |
| Reader peak RSS KiB | 5100 | 6620 | 6528 |
| Active CLI peak RSS KiB | 4108 | 4080 | 4228 |
| Ranked bytes / 100 packets | 38842 | 39400 | 40012 |
| All scope bytes / 100 packets | 90072 | 91000 | 92020 |

No-store CLI RSS 2600 KiB; fixture key precision/recall 100%, successful
packets 100/100 each. 1k forget p95 dropped from 301.4151 ms in original
manual run to 2.0212 ms in final (~149x; runner variability applies).
Original 10k write phase interrupted after >8 minutes. Schema v3 targeted
cleanup and indexes explain the improvement. No provider calls.

Retain SQLite + FTS5 as the Linux implementation for M2 development:
measured selective retrieval and memory are comfortably within targets.
Defer equivalent redb/Cozo adapters pending owner review; not an empirical
cross-engine ranking or permanent lock-in. Keep typed interfaces and logical
export. Real hardware, authorization and recovery gates remain.

## FTS optimization and differential verification

Inspired by Anthropic's [Rust migration report](https://www.anthropic.com/research/migrating-api-to-rust),
use the unchanged query as a reference for synthetic tests, rather than a
new lexical approximation. A bounded indexed probe retrieves up to 129
active row IDs within the already selected graph scope, in the same
snapshot. If at most 128 assertions occupy a rowid span under 4096,
add an inclusive FTS rowid range covering all of them. Empty scope uses
an impossible range. Otherwise use the original query. The redundant
predicate preserves the scope filter, unicode61 tokenizer, global BM25
rank, tie-breaking, candidate limit, provenance and compiler. The deadline,
progress hook and failure policy remain. No new dependency/schema change.
No cross-language inference loop.

Compare exact ranked results and packet bytes for synthetic multilingual,
typed, temporal, cycle, missing, required, over-budget and fallback cases.
Benchmark common terms on optimized and reference paths; check expected
keys, full result and packet equality when reference succeeds; report
reference errors separately. No private data or duplicate model execution.
Local experiment with 250k synthetic FTS rows showed potential benefit,
not production benchmark evidence. Retain reference for future regressions;
shared stages mean differential tests alone cannot detect bugs common to
both paths. Keep independent fixture tests.

## Streaming backup and disk accounting

Logical format unchanged for canonical exports. Limit increases from 8 MiB
and 20k records to 256 MiB and 1M records, 4096 bytes per line. Buffered
streaming export, bounded line parser, ordered ID validation and two-pass
restore (validate before creating directory, then insert transactionally
with cached prepared statements). No whole-file buffers or all-record
vector. Reject malformed/oversized input and preserve identity, provenance,
history and high water marks. Backup path must be outside active store.
New file only; remove newly created file on reported failure, interrupted
export may leave truncated file (parser rejects missing trailer). Plaintext
and trusted local snapshots only. No cryptographic authenticity, no
retention reconciliation. Existing canonical v1 logical format readable;
ordered IDs and LF delimiters required. Restore still omits grants.
New benchmark recovery phase opt-in via AMG_RECOVERY=1; manual scale
workflow enables it. Separate timing/RSS and identity/sample verification,
not a full byte-for-byte database equivalence proof.

Admin `storage` reports main DB, aggregate apparent/allocated file bytes
(512-byte blocks), 256 MiB target and over-target status for the dedicated
directory, including sidecars present at sampling. Unexpected symlink,
hard link or subdirectory rejected. Sampling is not synchronized with
concurrent writes; use quiescent store. Hard quota explicitly unverified.
Existing 128 MiB main DB page cap unchanged. Backups and restore staging
need separately budgeted storage. For deployment enforce dedicated
filesystem/project quota including journal/temp and reserve headroom; test
SQLITE_FULL/rollback and filesystem/power loss before claiming aggregate
quota. No code-only hard quota promise.

## Verification loops

Fast: synthetic fixture tests, reference comparisons, single 1k CI benchmark.
Outer: human-reviewed PR, locked offline CI, manual scale benchmark with
recovery and independent target-device testing. Later M2: one provider
response replayed through local parsers, never duplicated inference or
side effects. Memory loop: AGENTS.md rules, regression tests and recorded
decision history. No autonomous production shadowing or private telemetry.

## Verified M1.4f results

[Post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45807144684): 51 Rust and 11 Python tests passed; text-only release CLI 466,464 bytes, peak RSS 2,796 KiB. [Scale CI](https://github.com/idmfrank/atomos/actions/runs/45807347862): 90 common-term comparisons, zero mismatches/reference errors, 100% fixture precision/recall and 300/300 complete selective packets. At 100k entities, reference p95 60.7457 ms vs candidate 5.9669 ms, selective context p95 0.4233 ms, active CLI RSS 4,152 KiB vs 2,568 KiB. Export 600,005 records / 85,222,570 bytes in 3.209 s, restore 18.202 s and verify 2.467 s, recovery RSS 9,156 KiB. 1k and 10k recovery passed too. No hard quota, target-device or cryptographic recovery certification. Retain SQLite for M2; no equivalent alternative-engine benchmarks.
