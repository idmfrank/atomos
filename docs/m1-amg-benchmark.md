# M1.4e — Active AMG benchmark and engine evaluation

Status: SQLite reference harness on development branch; CI and scale measurements pending. No final engine selection or measured token savings.

## Reproduce

```bash
cargo fetch --locked
AMG_SAMPLES=100 bash scripts/benchmark-amg.sh 1000
AMG_SAMPLES=100 bash scripts/benchmark-amg.sh 1000 10000 100000
```

CI uses 1k; the manual AMG scale benchmark workflow supports 1k/10k/100k or an individual size. Synthetic temporary private stores, no network at runtime, no credentials. Actual v2 schema, foreign keys, FTS triggers, FULL synchronous rollback journal, 128 MiB page cap. Bulk fixture seed is not representative of individually committed writes. Each size N contains N entities plus owner, N/2 disconnected pairs, two text assertions per entity (status and unrelated narrative) and one edge per pair, 2.5N active assertions and sources. Each pair shares a unique lexical marker. Deterministic sample order, 100 samples by default; common-term stress 30. Sparse graph baseline, not exhaustive personal memory.

Production MemoryReader exact lookup, graph/FTS and end-to-end context timing; graph-only uses the admin neighbors API in a separate process. Actual durable remember, correct, forget, reopen and verify. Correctness checks subject/predicate and values, connector evidence, no unrelated results, deletion. Precision/recall are fixture-key metrics; failed retrievals count as missed facts, complete packets separately. Common-term failures visible but not CI correctness gate because bounded cancellation may reject them. Percentiles p50/p95/p99/max over attempted operations include failed requests; do not treat failures as successful timing. Process-cold means new connection/process, not OS page cache eviction. CI runner is not reference hardware; no absolute latency gate. Fixture correctness required.

GNU time peak RSS for seed, read, graph, write and same release CLI with/without active store, 1,000 local interactions (no-store name vs active exact recall), zero mock/remote calls in both. Delta is workload-level estimate, not exact SQLite allocation or PSS. Report CPU, RAM, toolchain, SQLite version, executable sizes, quiescent database and directory apparent bytes before/after writes. Transient journal peaks, allocated blocks, power and acoustic latency not measured. Large datasets may exceed the page cap; report failure rather than resize silently.

Context bytes: actual ranked packet versus all five facts in the same two-entity scope encoded by the same compiler. No tokenizer/model or transcript baseline; byte savings are not token savings or proof of reasoning quality. High-degree and required facts tested separately; broader scale fixtures for temporal history, ambiguity, varied queries remain open.

## Alternative-engine qualification

| Requirement | SQLite reference | redb candidate | CozoDB candidate |
| --- | --- | --- | --- |
| Embedded transactions | System SQLite adapter | Upstream ACID KV | Upstream Datalog/relational graph |
| Exact lookup | Indexed SQL | Key/index design needed | Relation schema needed |
| Graph | Bounded indexed BFS | Application adjacency/BFS | Native Datalog; bound work |
| Text | FTS5 and rank | Equivalent tokenizer/index/rank needed | Upstream FTS; verify parity |
| AMG v2 policy/recovery | Prototype tested | Adapter not implemented | Adapter not implemented |
| Performance | Harness | Not measured | Not measured |

Sources: [redb](https://github.com/cberner/redb), [redb documentation](https://docs.rs/redb/latest/redb/), [CozoDB](https://github.com/cozodb/cozo), [CozoDB documentation](https://docs.cozodb.org/en/latest/). Advertised capabilities, not verified AMG adapters. Evaluate pinned version, license, maintenance, build footprint, MSRV, backend and durability before dependency changes. Cozo over SQLite is a graph query layer over SQLite, not an independent physical engine; label backend.

An alternative must match fixture, typed exact/graph/FTS semantics, scope, revisions, expiry, index invalidation, portable restore and failure behavior with equivalent durability. Use same workload and count failures; measure separate process footprint and all indexes/sidecars. Do not compare bare KV lookup to full SQLite context. No new alternative-engine dependency in this PR.

SQLite remains provisional. M1.4e not complete until equivalent candidates are tested or owner explicitly accepts deferral. Remaining gates: reference hardware/PSS/energy, transient journal, full disk/power loss, high-degree and temporal scale workload, aggregate quota, authenticated owner policy, encrypted recovery, deletion reconciliation, physical voice and provider eligibility.
