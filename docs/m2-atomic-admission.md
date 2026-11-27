# M2.5a: Atomic account-bound synthetic inference admission

Status: isolated offline implementation; initial [PR #30](https://github.com/idmfrank/atomos/pull/30) merged externally, current admission-context follow-up [PR #31](https://github.com/idmfrank/atomos/pull/31) for human review. Verified executable follow-up `3895b2bd1dd3c113397b9d685be03350156aa88d` passed CI/full scale; current documentation-only reconciliation does not change that code. Original base `5f564ad5641ea716d86efc894b2b132aaae9fa77` (PR #29). Not wired to production. No transport, credentials, OAuth, private AMG, provider calls, new live command or paid fallback. All three previous owner qualification attempts remain consumed and all 3,072 retained planning units and markers remain unchanged. This namespace never reads or migrates owner records.

## Implementation and boundary

`atomos-core` feature `atomic-admission` enables the Unix-only `atomic_admission` module and the existing strict Responses decoder. `AtomicReplay` exposes explicit initialize/open, pure validation, synchronous synthetic replay and permanent revoke. No public admission token, detachable permit, transport callback or separately dispatchable object exists. Local exact help/name commands remain available with no reservation, even while suspended; invalid/stale requests are still rejected first.

`CompiledRequest::compile` owns the request text and all envelope/model/policy/turn/task/priority/output fields privately. There are no mutation methods. Its SHA-256 identity covers a domain separator, fixed-width little-endian u64 fields in this order: account, host, grant, request ID, fixture time, expiry, model, policy version, turn, task, output ceiling, background bit, text byte length; then exact UTF-8 text bytes. Synchronous replay validates and uses the same immutable object. Recompilation changes identity when any field changes; a consumed request ID cannot admit either original or altered content. No prompt text or provider response is journaled. The unkeyed digest can reveal guesses of low-entropy synthetic content; it is not confidentiality protection or a MAC.

Before admission: valid/nonzero task and turn, bounded nonblank control-free input, output bound, active turn, exact synthetic account/host/grant/model/policy version, model eligibility and all M2.3 fixture eligibility/spending flags, sequential request identity, nondecreasing fixture time, request/window/grant expiry, priority and all Governor budgets. Numeric IDs and evidence are fixtures only. Model eligibility is an explicit policy flag tied to the model in the exact-match header; it is not a live catalog attestation. Task identity is required, immutable and recorded, not external task authorization.

Every validation/replay call requires a fresh `Context { now, active_turn, task }` supplied by the trusted caller. Compilation time remains immutable in the digest; admission time is independently checked against it, the last admission, request expiry and grant/window end. The active task must match as well as the active turn. A request compiled while valid but presented after expiry is denied before reading. A time earlier than compilation or the last admission is rollback. The A record stores admission time; it must not reuse a stale compilation timestamp as current time. Context is caller state, never authorization or a dispatch permit; a trustworthy real clock/task manager remains an integration gate.

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
| Admission p50 / p95 / p99 (ms), 300 samples | 5.8979 / 8.6549 / 31.4788 | 3.5324 / 5.2026 / 18.7092 |
| Full completion p50 / p95 / p99 (ms) | 12.0638 / 24.9337 / 49.2294 | 7.4768 / 10.6349 / 33.9943 |
| Restart p50 / p95 / p99 (ms), 30 samples | 3.4235 / 4.6039 / 63.7204 | 0.4296 / 0.5260 / 0.5605 |
| Journal write syscalls / fsyncs per admission (trace) | 4 / 2 | 2 / 1 |
| Logical admission records | 2 | 1 |
| Journal bytes per completed request | 52.560 | 104.280 |
| Process peak RSS (KiB) | 2,312 | 2,336 |
| Release harness bytes | 416,528 | 440,152 |
| Final charged units / errors | 14,100 / 0 | 14,100 / 0 |

Final candidate memory-only validation p50/p95/p99: 0.533/0.940/1.222 microseconds; compilation including SHA-256: 4.890/6.990/8.118 microseconds. Candidate 600 append records and 600 file syncs include 300 admissions plus 300 settlements. Background charge 7,050; independent total expectation 14,100 on both paths, zero parity mismatches. Larger candidate journal reflects durable identity/metadata, not accounting drift. This run includes fresh admission context checks and supersedes earlier development p95 7.2182/3.7824 ms and initially committed 57c1739 p95 8.3572/3.4981 ms. That earlier commit's [CI](https://github.com/idmfrank/atomos/actions/runs/46022140907) and [full scale](https://github.com/idmfrank/atomos/actions/runs/46022161437) passed; final-revision hosted evidence below supersedes those initial runs. These uncontrolled runs support fewer durable syncs, not a general speed/energy claim.

### Verified hosted results

Executable `3895b2bd1dd3c113397b9d685be03350156aa88d`: [CI 46023799399](https://github.com/idmfrank/atomos/actions/runs/46023799399), [full scale 46023520346](https://github.com/idmfrank/atomos/actions/runs/46023520346). All 173 distinct tests, Rust 1.75 core targets, 18,000 decoder samples and 1K/10K/100K AMG scales passed. Measurements below use the same methodology as the local run; no live requests.

| Hosted metric | Reference | Candidate |
| --- | ---: | ---: |
| Admission p50 / p95 / p99 (ms), 300 samples | 0.332693 / 0.428936 / 0.631346 | 0.161890 / 0.206696 / 0.363438 |
| Full completion p50 / p95 / p99 (ms) | 0.687799 / 0.885192 / 1.596407 | 0.344821 / 0.427323 / 1.004940 |
| Restart p50 / p95 / p99 (ms), 30 samples | 0.344601 / 0.384500 / 0.417929 | 0.237361 / 0.266946 / 0.296169 |
| Journal write syscalls / fsyncs per admission (trace) | 4 / 2 | 2 / 1 |
| Journal bytes per completed request | 52.560 | 104.280 |
| Process peak RSS (KiB) | 2,240 | 2,200 |
| Release harness bytes | 416,520 | 440,144 |
| Final charge / errors / parity mismatches | 14,100 / 0 / 0 | 14,100 / 0 / 0 |

Candidate validation p50/p95/p99 0.080/0.171/0.290 microseconds; compilation 0.391/0.651/1.122 microseconds. Candidate background charge 7,050; 600 logical writes/syncs include settlement. Toolchain/runner differences prevent controlled comparisons with local WSL or historical runs. Full-scale measurements and elevated 10K write tail are retained in PROJECT_STATE and README. Human review remains pending. Initial main `2bd3adef18abeaaf2a659696ab2944cd506d927c` also passed post-merge CI/scale; no merge was performed by this agent.

## Residual risks and next milestone

Trust the local directory, process owner, filesystem durability and injected fixture time. No same-UID tamper protection, MAC/authenticated evidence, rollback resistance, root protection, secure credential boundary, trusted time, rolling budgets, real eligibility attestation, provider generation ceiling or authoritative real usage reconciliation. Numeric model IDs are synthetic. Unkeyed identity is not a disclosure mechanism. No production transport, credentials, vault changes, live inference or private AMG disclosure. Existing live markers must never be reset.

M2.5b: separately reviewed integration of extended decoder outcomes with account-bound admission/cancellation/settlement, retaining reference parity and privacy/spend gates. Transport and credential work require a later explicit scope and security review. Human review/approval is required before merge; this slice must not be merged by the agent.
