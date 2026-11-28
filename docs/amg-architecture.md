# AtomOS Memory Graph

Local temporal knowledge graph backed by SQLite/FTS5 through typed Rust interfaces. [Roadmap](agent-roadmap.md) | [Architecture](architecture.md).

## Implemented

Owner-scoped identity, entities, typed facts/relationships, source IDs, revision history, exact recall, scoped graph/FTS retrieval and ranked bounded context. Transactional correction/forgetting, integrity checks, explicit migrations, logical export/restore and search rebuild. Conversation is read-only; atomos-memory owns explicit writes. Automatic ingestion/consolidation is not implemented.

Keep values on stdin, not shell arguments. Parameterized SQL, application-controlled queries, bounded traversal and short transactions. Never hold a transaction across inference, capture or external actions. Models cannot author SQL or gain tool permissions through facts.

## Learning target

Start with owner interactions and verified outcomes; add host events and own connection metadata under scope. Separate rotating evidence from selected durable assertions. Track source, observation time, confidence, trust, sensitivity, expiry and derivation. Distinguish observed, inferred, owner-confirmed and verified information. Repetition cannot promote malicious content into instructions.

Memory classes: identity/configuration, bounded working state, selected episodes, semantic assertions, evaluated procedures and commitments. These are targets, not a claim that every class is implemented. New observations can corroborate/propose facts; conflicts and uncertainty remain visible, with no silent permission grants.

The [MVP contract](mvp-contract.md#learning-observation-and-retention) specifies candidate/confirmed evidence, default retention, pause/purge/forget behavior and test gates A08-A10. These automatic-learning controls are requirements, not current AMG functionality.

## Retention and correction

Provide inspect/correct/forget, pause-learning and purge-evidence. Propagate deletion through derived summaries, graph links, indexes and caches. Define backup retention/restore so forgotten facts do not silently return. Logical deletion is not forensic SSD/backup erasure.

Exclude credentials and unrelated payloads; no automatic full-transcript or all-packet archive. Verify deployment capacity/free space privately before quotas; keep the inventory out of public artifacts. Reserve recovery headroom; pause ingestion under contention/disk pressure.

## Compatibility and disclosure

Keep required schema/migration/integrity code for existing stores. Legacy grant tables and retirement triggers are internal compatibility state; no voice recall/grant API exists. Do not rewrite owner stores during cleanup. Private graph data needs explicit disclosure policy and complete request binding before egress. [Resources](resource-budget.md), [security](threat-model.md).
