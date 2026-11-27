# M2.5a: Atomic account-bound synthetic inference admission

Status: isolated offline candidate, not merged or wired to production. Base `5f564ad5641ea716d86efc894b2b132aaae9fa77` (PR #29). No transport, credentials, OAuth, private AMG, provider calls, new live command or paid fallback. All three previous owner qualification attempts remain consumed and all 3,072 retained planning units and markers remain unchanged. This namespace never reads or migrates owner records.

## Implementation and boundary

`atomos-core` feature `atomic-admission` enables the Unix-only `atomic_admission` module and the existing strict Responses decoder. `AtomicReplay` exposes explicit initialize/open, pure validation, synchronous synthetic replay and permanent revoke. No public admission token, detachable permit, transport callback or separately dispatchable object exists. Local exact help/name commands remain available with no reservation, even while suspended; invalid/stale requests are still rejected first.

`CompiledRequest::compile` owns the request text and all envelope/model/policy/turn/task/priority/output fields privately. There are no mutation methods. Its SHA-256 identity covers a domain separator, fixed-width little-endian u64 fields in this order: account, host, grant, request ID, fixture time, expiry, model, policy version, turn, task, output ceiling, background bit, text byte length; then exact UTF-8 text bytes. Synchronous replay validates and uses the same immutable object. Recompilation changes identity when any field changes; a consumed request ID cannot admit either original or altered content. No prompt text or provider response is journaled. The unkeyed digest can reveal guesses of low-entropy synthetic content; it is not confidentiality protection or a MAC.

Before admission: valid/nonzero task and turn, bounded nonblank control-free input, output bound, active turn, exact synthetic account/host/grant/model/policy version, model eligibility and all M2.3 fixture eligibility/spending flags, sequential request identity, nondecreasing fixture time, request/window/grant expiry, priority and all Governor budgets. Numeric IDs and evidence are fixtures only. Model eligibility is an explicit policy flag tied to the model in the exact-match header; it is not a live catalog attestation. Task identity is required, immutable and recorded, not external task authorization.

The existing Governor's checked reservation decision is extracted into one internal shared function. Both paths use identical per-request, total, background and interactive-reserve rules. Candidate additions and settlement arithmetic are checked; no byte-to-provider-token claim. Estimate remains the reference fixture formula: UTF-8 bytes + 128 framing units + maximum output. Valid authoritative usage charges input + output; cached/reasoning counts are subsets. Overruns settle actual and halt in one record. Unknown/malformed/cancelled/interrupted outcomes retain the outstanding reservation.

Pinned optional `sha2 = 0.10.9` is justified by exact compiled identity without custom cryptography. It adds seven locked packages (including hashing support) and no default feature/runtime dependency. Candidate core and hashing dependencies compile with Rust 1.75. The broader workspace's pre-existing `getrandom` 0.4.3 cannot be fetched by Cargo 1.75 because of edition 2024; this is not a whole-workspace minimum-version certification. No dependency upgrades are made to address that separate issue.

## Journal and state machine

Only `ROOT/synthetic-atomic-admission-v1/admission.log` is used. ROOT must already be a private directory. Explicit initialization creates a new 0700 namespace and 0600 singly linked regular journal; initialization never overwrites any existing namespace, even partial creation. Open requires the existing directory, exact header and journal. Missing journal, symlink, hard link, broad permissions, invalid/malformed/truncated/duplicated record, overflow, incompatible version or different configuration fails closed. No implicit initialization, migration, reset, truncation, compaction or repair.

Canonical LF-terminated ASCII records, single spaces:

```text
AA1 policy_version model_id model_eligible A1 <exact M2.3 policy header fields>
A request_id now expiry turn task input_bytes max_output I|B sha256 reservation
S request_id actual_total halt_bit
H
```

The A1 fields are account, host, grant, window start/end, grant expiry, eligible/direct-scope/credits-disabled/auto-purchase-disabled flags, app-limit percent, total/background/interactive-reserve/per-request limits. The header is immutable and compared exactly. Admission metadata plus the header binds scope and model; SHA-256 binds exact compiled request bytes and metadata without retaining text. Replay independently validates sequence, timing, metadata bounds, canonical record spelling, budget eligibility and exact reservation formula. S requires the one matching outstanding A; duplicate completion/admission and inconsistent overrun halt are rejected. H is permanent, and can coexist with an outstanding reservation.

State transitions:

1. Ready -> A append and `sync_all` -> outstanding/charged -> synthetic dispatch/read.
2. Valid usage -> S append and `sync_all` -> actual charged; halt bit permanently suspends on overrun or provider authorization/rate failure.
3. Unknown dispatch, panic, cancellation, decoder error or interruption -> outstanding remains charged and blocks further synthetic inference, including after restart.
4. Revocation -> H append and `sync_all` -> permanently halted. No resume or grant rotation.

An exclusive create-new writer lock covers open/replay and the entire instance lifetime. Abnormal process exit leaves a stale lock requiring manual investigation; no automatic lock stealing. Tests of restart with outstanding reservation drop the instance cleanly, leaving A without S; they do not claim automatic recovery from a stale crash lock.

Admission appends one logical record (currently two write syscalls for text/newline) and fsyncs the journal before incrementing the synthetic dispatch counter or reading the response source. No cross-file admission window exists. In-memory state is prevalidated on a bounded clone and published only after successful sync. I/O failure poisons the instance. A torn record blocks reopen; a complete A surviving ambiguous sync failure remains charged after reopen. The operation may have committed even if it returned an I/O error: never retry or infer a refund. Initialize syncs the header, namespace directory and parent directory before returning. Append assumes an existing journal in a trusted local filesystem that honors fsync; this is logical atomic admission with fail-closed torn-record handling, not a claim of sector-atomic writes.

Journal cap 1 MiB; replay reads at most 1 MiB + 1 byte and rejects overflow, records capped at 512 bytes. Replay stores only totals, sequence/time, one pending record and halt, not an unbounded request map. When capacity is exhausted it fails closed; there is no automatic rolling window/reset. Existing decoder retains all its independent bounds. No transaction or lock is held against AMG, which is not accessed.

## Reference and intentional semantic differences

The frozen behavioral reference is PR #29's M2.3 `AuthorizedReplay`, Governor and broker with strict decoder. Their implementations and existing tests remain present. Only the shared reservation decision and crate-private policy helper visibility are changed; existing reference results must remain equivalent.

| Condition | Two-journal reference | Atomic candidate |
| --- | --- | --- |
| Successful admission | policy P fsync then Governor R fsync | one A with identity and reservation, one fsync |
| Budget denial | intent ID consumed, no reservation | no A, no consumed ID or reservation |
| Unknown completed wrapper outcome | D can clear policy pending; reservation retained | pending retained; subsequent inference blocked |
| Crash before reservation | P can exist without charge | no accepted A without its charge |
| Identity | borrowed immutable synchronous request; byte count metadata | owned immutable compiled SHA-256 identity, model/version binding |
| Revocation during outstanding intent | reference rejects while suspended | H can permanently revoke while retaining charge |
| Recovery | two bounded journals, Governor entry map | one bounded journal, constant accounting state |

These differences are intentional conservative candidate semantics, not changed reference behavior. No old ledger is opened by the candidate. Strict decoding remains unchanged; extended reasoning/refusal/auxiliary integration is M2.5b.

## Independent tests

23 new distinct tests: 21 integration tests and two internal arithmetic/I/O fault tests. They cover successful A reservation and settlement; account/host/grant/model/version mismatch; missing/invalid evidence; stale turn and immutable altered request identity; duplicate request IDs/replay; request/grant/window expiry and time rollback; per-request/total/background exhaustion and interactive reserve; concurrent writer; missing/incompatible/truncated/malformed/duplicate/oversize journals; restart outstanding charge; checked arithmetic overflow; durable revoke/provider suspension/overrun; local resolution without charge; denial never reading the source; and 12 equivalent successful mixed-priority reference/candidate requests with exact answers/usage/charges.

Persistence tests use real `/dev/full` write failure and `/dev/null` fsync failure via private unit-only handle replacement, plus journal-capacity failure. No public fault switch exists. A response reader inspects the on-disk A identity/reservation before allowing a read; syscall tracing separately verifies fsync ordering/counts. Tests do not simulate actual power loss or certify storage hardware.

After locked dependency provisioning:

```bash
bash scripts/test.sh
bash scripts/test-responses.sh
cargo build --release --locked --offline -p atomos-core --features responses-replay --example responses_qualify
node --test scripts/auth/protocol.test.mjs scripts/auth/live-probe.test.mjs
bash scripts/test-atomic-admission.sh
AMG_SAMPLES=100 AMG_RECOVERY=1 bash scripts/benchmark-amg.sh 1000 10000 100000
```

## Benchmark methodology and local evidence

`scripts/benchmark-admission.sh`: separate reference and candidate processes, identical fixture policy, 300 successful requests alternating interactive/background, identical synthetic strict SSE, same private temp filesystem/umask and fsync durability. Headers/initialization excluded from journal byte deltas. Admission is measured at the first response Read, before decode and settlement; candidate compilation is measured separately, and validation runs independently without producing a permit. Full completion includes decoder/settlement. Reference has no exposed pure-validation API, so there is no fabricated memory-only reference timing. Restart/replay repeats 30 opens after 300 settled requests. The unchanged reference performs its existing open-time journal/directory syncs; candidate opens already durable state without redundant syncs. Samples include these actual implementations, not normalized syscall costs.

Separate `strace -yy` run measures write/fsync syscalls on journal descriptors between start and first-read markers (three admissions per path); instrumentation is excluded from reported latency/RSS. Missing strace reports pending, never inferred measured counts. GNU time measures process peak RSS per mode. Release binaries: reference-only feature build vs combined reference/candidate feature harness, not separately linked production modules or default CLI overhead. Each request is asserted against independent 47-unit expected usage. Errors abort with failing request index and category; partial/failed runs must not be represented as a successful comparison.

Local WSL2 Ubuntu, Rust 1.100, Linux 5.15.167.4, private `/tmp` filesystem reported ext2/ext3, 2026-11-27. Linux snapshot restores Git text line endings from the Windows CRLF checkout; no owner state copied. These timings are not controlled physical-device measurements.

| Metric | Reference | Candidate |
| --- | ---: | ---: |
| Admission p50 / p95 / p99 (ms), 300 samples | 6.2921 / 8.3572 / 24.9803 | 2.7969 / 3.4981 / 14.3612 |
| Full completion p50 / p95 / p99 (ms) | 13.0139 / 20.5395 / 41.2268 | 5.7204 / 7.0509 / 19.2804 |
| Restart p50 / p95 / p99 (ms), 30 samples | 3.6908 / 4.3270 / 7.9860 | 0.5005 / 0.8189 / 0.8253 |
| Journal write syscalls / fsyncs per admission (trace) | 4 / 2 | 2 / 1 |
| Logical admission records | 2 | 1 |
| Journal bytes per completed request | 52.560 | 104.280 |
| Process peak RSS (KiB) | 2,416 | 2,316 |
| Release harness bytes | 416,528 | 439,944 |
| Final charged units / errors | 14,100 / 0 | 14,100 / 0 |

Final candidate memory-only validation p50/p95/p99: 0.313/0.596/0.773 microseconds; compilation including SHA-256: 2.966/4.983/6.017 microseconds. Candidate 600 append records and 600 file syncs include 300 admissions plus 300 settlements. Background charge 7,050; independent total expectation 14,100 on both paths, zero parity mismatches. Larger candidate journal reflects durable identity/metadata, not accounting drift. This run supersedes the earlier development measurement (reference/candidate admission p95 7.2182/3.7824 ms), after replay rejection of impossible settlement records was tightened. This run is evidence for fewer durable admission syncs, not a general speed/energy claim.

Full regression/decoder/Node and full-scale completion plus exact PR CI evidence are recorded in PROJECT_STATE as verified. Candidate PR CI and hosted full-scale results remain pending until run on an exact committed revision.

## Residual risks and next milestone

Trust the local directory, process owner, filesystem durability and injected fixture time. No same-UID tamper protection, MAC/authenticated evidence, rollback resistance, root protection, secure credential boundary, trusted time, rolling budgets, real eligibility attestation, provider generation ceiling or authoritative real usage reconciliation. Numeric model IDs are synthetic. Unkeyed identity is not a disclosure mechanism. No production transport, credentials, vault changes, live inference or private AMG disclosure. Existing live markers must never be reset.

M2.5b: separately reviewed integration of extended decoder outcomes with account-bound admission/cancellation/settlement, retaining reference parity and privacy/spend gates. Transport and credential work require a later explicit scope and security review. Human review/approval is required before merge; this slice must not be merged by the agent.
