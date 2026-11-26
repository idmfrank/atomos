# M1.4c — Scoped retrieval and context compilation

Status: development branch, CI pending. M1.4b baseline passed [CI](https://github.com/idmfrank/atomos/actions/runs/45755478646).

The read-only MemoryReader now accepts an explicit ContextRequest (anchor entity,
question, required anchor predicates, graph depth and maximum fact count).
All retrieval occurs inside one SQLite read transaction. No raw SQL or FTS
syntax from callers; no embeddings, provider calls, or background learning.

1. Validate anchor, question (512 bytes), up to eight unique non-stopword
   lexical terms (64 bytes each), at most eight required predicates, depth 0–3
   (default 2), max facts 1–50 (default 12).
2. Retrieve required exact anchor facts; record missing ones. Explore incoming
   and outgoing active edges using indexed bounded BFS, cycle detection and
   paths from the explicit anchor. Maximum 32 visited entities and 32
   neighbors per direction/expansion (narrower than administration traversal).
3. Query FTS5 with application-generated quoted OR terms and bound scope /
   reachable subject IDs. Max 50 candidate matches and estimated 16 KiB
   candidate payload. Rank deterministic lexical matches in predicate/value/
   subject, graph distance, FTS position, then recorded time and revision.
   Score is relevance, not truth. No fuzzy entity matching or semantic
   embeddings. FTS does not index entity labels separately.
4. Required facts first, then ranked candidates with their graph connector
   paths. Deduplicate revision IDs. Never include a selected linked fact
   without its connector path. Report missing requirements, visited nodes,
   candidate count, omissions and work truncation explicitly.
5. Compile data-only fact packet with source/revision/recorded timestamps,
   exact encoded byte count and fact count. Reject empty/incomplete results,
   missing mandatory facts, ranked omissions or byte overflow. No partial
   packets. Caller must keep this data separate from trusted policy and user
   instructions. A byte limit is NOT a provider token estimate.

A SQLite progress hook (1000 VM instructions per callback, 2000 callback cap,
250 ms target deadline) and 100 ms busy timeout bound query work; wall-clock
checks between steps. This is cooperative cancellation, not a hard real-time
deadline or a native-code watchdog. Query failure returns no packet. The hook
is cleared after each retrieval. No DB work on microphone callback threads.

## Local usage

With an existing synthetic AMG store:

    printf '%s\n' 'context project what is the validation status?' '/quit' | target/debug/atomos --memory /path/to/store demo

Only explicit text context previews are supported; no automatic broad retrieval
for unrelated mock questions, no voice disclosure, no memory mutation, no
model request or token admission. Existing exact recall and follow-up remain
unchanged. Required predicates are supplied through the typed API (not text
grammar yet). Unknown anchor, over-broad or incomplete retrieval should be
narrowed; the compiler refuses silent omission.

Tests: graph-scoped FTS, deterministic relevance, required facts, connector
paths, deduplication, revision/deletion freshness, literal search, scope,
work/fact/byte limits, no unrelated disclosure, no voice broad context and
zero mock calls for a successful preview. CI fixtures are synthetic.
Full context completeness beyond the explicit query, conflicting/multi-valued
assertions, sensitivity classification, authenticated access, prompt injection
resistance at a real provider boundary, measured token savings, retrieval
recall/precision at scale, active SQLite latency/power and engine comparison
remain unvalidated. M1.4d/e and M2 retain their existing gates.
