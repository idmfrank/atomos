# M2.5b: Offline text outcomes and atomic settlement contract

Status: accepted contract (merged PR #34); runtime implementation and independent tests are now drafted on a review branch. Compilation, complete validation and benchmarks are pending. The installed CLI remains local/mock; no provider connection is introduced. Baseline `c563599f2e2c8b5abb7e03c75bb85e67e831e75d` (merged PR #33) passed [post-merge CI 46029012445](https://github.com/idmfrank/atomos/actions/runs/46029012445). It has the same executable behavior as PR #31. The last full 1K/10K/100K run remains [46026109238](https://github.com/idmfrank/atomos/actions/runs/46026109238) on PR #32 `5bca8a15e8245e9631d8a978a988c01cdc812f7a`. These are baseline results, not M2.5b validation.

[CLI roadmap](cli-mvp-roadmap.md) | [Current state](../PROJECT_STATE.md) | [Atomic admission](m2-atomic-admission.md) | [Protocol profiles](m2-protocol-compatibility.md) | [Inference architecture](inference-architecture.md)

## Objective and bounded scope

Connect the existing extended Responses decoder to the existing atomic synthetic admission state machine. Deliver typed text outcomes suitable for a later thin CLI, without adding a transport, new decoder capability or conversational command surface. Keep one owner, one synchronous request and the current trusted private Unix directory model.

The baseline already has `responses::decode_extended`, `ExtendedTerminal::{Completed, Refused, Failed}`, bounded diagnostics, immutable `CompiledRequest`, fresh `Context`, durable A/S/H records and strict `AtomicReplay::run_sse`. Reuse these; do not build another parser, ledger, policy engine or service.

No network/HTTP, OAuth or vault access, provider calls, private AMG disclosure, local memory mutations, transcript persistence, incremental terminal output, browser UI, SSH/service configuration, voice development, scheduling, tools, model routing, automatic retries or paid fallback. The CLI session/command integration is a subsequent slice. Preserve all three consumed owner attempts, existing markers and 3,072 retained local planning units.

## Reference and API boundary

- Freeze strict atomic replay at the baseline above as the behavioral reference. Preserve its public signature, error mapping, accepted/denied profiles, timing/task validation, local behavior, reservation, suspension and cancellation semantics. Existing two-journal authorization, broker, decoder entry points and owner qualification paths remain intact.
- Add an explicit opt-in `AtomicReplay::run_sse_extended` using the same immutable request, fresh admission context, finite `Read` source and cooperative cancellation closure. It returns a typed final outcome, not a dispatch permit. Keep it under the existing `atomic-admission` feature; no default CLI wiring or new dependency is expected.
- Define a public final-result enum in the atomic module. The proposed name is `ReplayOutcome`, with variants `Local(Answer)`, `Completed(Answer)`, `Refused { turn, usage }` and `Failed { turn, failure, usage: Option<Usage> }`. Keep storage/admission/decoder/cancellation/overrun faults in the existing `Error` channel. Equivalent names are acceptable if the semantics below and tests stay exact.
- Local answers retain `simulated=false` and no usage. Completed synthetic answers retain `simulated=true` and validated usage. Refused/failed results carry no answer, raw refusal, reasoning or provider-message field. They are mutually exclusive outcomes; no fallback request is dispatched.
- Select the extended profile explicitly by this method. Do not negotiate, auto-upgrade, silently retry a strict failure with extended decoding, or expose a profile switch after admission. In this synthetic slice the trusted caller selects the entry point; do not claim the existing digest or journal independently attests a decoder profile or real model capability.
- A small internal shared admission/settlement helper is acceptable. Prove preservation of the strict reference through independent expectations and baseline/candidate differential checks; do not replace reference tests with two calls into the same new helper.

## Admission and privacy invariants

Use the same `CompiledRequest`, digest, policy/model binding, priority, estimates, checked arithmetic, fresh clock/turn/task checks, budgets, private namespace, exclusive writer, bounds and fail-closed behavior as M2.5a. Validate before reading the source. Exact supported local commands stay available without inference charge, including while remote admission is suspended; malformed/stale turn/task requests still fail first.

Write and sync one A admission/reservation record before synthetic dispatch/read. Do not create a public reusable permit, allow request substitution, remove fsync, open owner records or change the journal/header/digest format. The existing v1 journal stores accounting state, not a durable transcript or a record of every typed response outcome. A settled refusal/failure must not be described as recoverable response history.

Run the existing `decode_extended` once. Preserve strict JSON, complete finite stream/EOF validation, identity/sequence/lifecycle checks and all current caps: 256 KiB wire, 32 KiB frame, 128 events, 4096 text bytes, and bounded opaque encrypted reasoning content. Do not broaden annotations, summaries, reasoning streams, tools, audio or unknown-event acceptance to satisfy a fixture. Only validated ordinary completed text may be returned.

The existing decoder discards refusal/failed text and opaque reasoning before return. Do not reconstruct, return, persist, log or include that content in Debug/status/error output. Keep diagnostics to existing fixed labels and counts. Public results may expose bounded synthetic usage and typed failures, not raw provider strings, IDs or headers.

## Outcome and settlement matrix

In the table, known usage means the entire finite stream passed the selected decoder, including EOF/trailing checks, and usage passes existing subset/arithmetic/representability rules. Merely seeing a terminal event or tokens in malformed input is insufficient.

| Decoded condition | Durable action | Returned result / next admission |
| --- | --- | --- |
| Supported local command | No A/S and no inference dispatch | Local answer; no charge; preserve existing local behavior |
| Admission denied | No response read, A, charge or consumed request ID | Existing typed admission error |
| Completed ordinary text, valid usage within all bounds | S with actual input + output, halt=0, synced before return | Completed answer; next sequential request may be admitted |
| Refused, valid usage within bounds | Same S settlement, halt=0 | Typed Refused with turn/usage, no text, no retry; next request may be admitted |
| Failed/incomplete with valid usage, ordinary failure | Same S settlement, halt=0 | Typed Failed with known usage, no partial text; no automatic retry |
| Rate/authorization failure with valid usage | S with actual and halt=1 | Typed Failed; durable permanent suspension |
| Failed/incomplete with no usage, ordinary failure | No S/refund; leave A outstanding and charged | Typed Failed with usage=None; pending state blocks later admission and survives clean reopen |
| Rate/authorization failure with no usage | H synced; retain outstanding A and full charge | Typed Failed with usage=None; durable permanent suspension |
| Valid usage exceeds total reservation OR input OR output bound | S with actual and halt=1 | Error::Overrun takes precedence over Completed/Refused/Failed; no answer |
| Invalid/unrepresentable usage, decoder error, missing terminal, I/O or cancellation | No S/refund; retain outstanding A | Existing error mapping; no answer; subsequent admission blocked |
| A/S/H persistence failure | Poison instance; publish no successful in-memory transition or result | Storage error; no answer/refusal/failed outcome released as durably completed |

Charge input + output once. Cached input and reasoning output are subsets, not extra charge. Preserve existing actual-usage representability guard, checked arithmetic and overrun behavior. Do not cap an excessive authoritative fixture total down to its reservation or budget. If usage cannot be safely represented/validated, retain the reservation and fail closed rather than fabricate settlement.

A known-usage failed response can settle and allow later admission unless it is rate/auth or overrun; an unknown-usage failed response remains pending. A future CLI must inspect both the typed outcome and suspension/accounting state. Do not flatten all failures into a successful empty answer or claim the reservation was refunded.

## Cancellation, delivery and persistence failures

Match the current strict path's conservative sequence: validation/local resolution, durable A, then decoder cancellation checks. If cancellation is already true when decoding begins, the extended method retains the admitted reservation without reading a byte; no pre-admission cancellation optimization or refund is introduced in this slice. Document this behavior independently of future transport preflight.

Cancellation before first read, between fragmented reads, after terminal bytes but before successful finite-stream/EOF validation, or at the decoder's final cancellation check returns cancellation with outstanding charge. A buffered terminal followed by malformed/trailing data or I/O failure is not a successful settlement. No partial text is released. Once decoding succeeds, complete synchronous settlement; there is no new cancellation point that can split a committed S from a claimed unknown outcome. An interrupt before public delivery does not undo a completed S. Future transport/CLI cancellation and stale-display ownership need their own integration contract.

Cooperative checks cannot interrupt a blocking `Read` or `sync_all`. Use finite synthetic readers; bound fault tests externally. This slice adds no network deadline, signal handler or SSH disconnect guarantee.

Inject write and fsync faults privately in unit tests at admission, settlement and H revocation. Do not expose production fault toggles or weaken file validation. A settlement/H failure returns storage error and blocks further admission in that instance. No public outcome/text is released after such failure. On clean reopen, a complete surviving S may have settled even if fsync returned an error; a surviving A without S remains pending; a torn record is corrupt. Test/classify these cases without assuming a refund, retrying the same ID or automatically repairing/truncating the log.

Retain the lock investigation boundary: clean close/reopen tests demonstrate replayed accounting; forced process termination demonstrates a stale lock and refused reopen. Test both, with a bounded subprocess and synthetic temp state. Never silently remove a stale lock to report successful automatic recovery.

## Independent test contract

Preserve all existing 173 distinct baseline tests and 18,000 decoder benchmark samples. Add integration/unit tests with independently specified text, outcome, usage, charge, background charge, suspension, dispatch count, next ID and journal transitions. Table-driven cases may share setup; do not count repeated compiler/feature executions as distinct tests.

| Group | Required cases and independent checks |
| --- | --- |
| Equivalent strict wire | Frozen strict reference vs extended atomic candidate on exact same policy/request/stream; answer, usage, charge and journal accounting parity across 1/7/64/4096-byte reads; strict still rejects auxiliary/reasoning/refusal fixtures |
| Accepted extended wire | Existing auxiliary fixture and opaque-reasoning-plus-text fixture complete with exact expected text/usage; raw opaque values absent from returned/debug/status/journal data |
| Refusal | Existing refusal fixture produces typed Refused, empty/no text surface, exact 35 input + 12 output = 47 units; cached=10/reasoning=4 remain subsets; exactly one dispatch and settlement; next ID works and settled charge survives clean reopen |
| Failed/incomplete | Each with/without validated usage, including failure after refusal or reasoning; no text; known charge settles, unknown retains full A; failure kind and state match matrix |
| Permanent suspension | Rate and authorization failures with/without usage; reopen preserves halt and charge; local commands still work; no automatic retry |
| Overrun and invalid usage | Total overrun, input-only overrun and output-only overrun even when total fits reservation; check S halt and actual charge; invalid cached/reasoning subsets and arithmetic/representability limits retain pending charge |
| Denial and identity | Account/host/grant/model/policy mismatch, task/turn mismatch, stale admission time/expiry, budgets, duplicate/altered ID and revoked grant: no response reads or additional dispatch; reuse baseline assertions without weakening |
| Cancellation and malformed streams | Before first read, between reads, terminal-before-EOF and final-check cancellation; interrupted reader/I/O, missing/duplicate/trailing/mismatched terminal, invalid usage/JSON/limits; no text/refund; pending blocks restart admission |
| Persistence faults | Write and fsync failure at A, S and H; no output on failure, poisoned instance, correct conservative clean reopen or corruption classification; full journal capacity during settlement also covered |
| Crash and writer | Existing exclusive-writer tests retained; actual bounded child termination after durable A leaves lock and prevents reopen; separately test clean A-only and settled/refused reopening |
| Privacy and isolation | No raw refusal/reasoning/provider envelope/credentials in results/Debug/errors/journal; no network/owner vault/private AMG; no new public permit/fault switch |
| Local behavior | Exact local commands remain free and available while suspended; invalid/stale input rejected as before; no decoder source reads |

Use known fixture values as independent expectations, not candidate-derived oracle output. For each intentional extended-versus-strict acceptance difference, assert both sides and document it. The same-wire baseline must remain frozen by commit or checked-in reference harness; running only the newly refactored strict and extended methods is insufficient to establish unchanged historical behavior.

## Benchmark contract

Preserve the existing reference-versus-atomic admission benchmark. Add an isolated comparison between the frozen M2.5a strict atomic path and extended atomic candidate on identical strict fixture bytes, request sequences, initial ledger state, policies, filesystem, durability and build conditions. Compile reference and candidate separately if needed to avoid contaminating the reference with shared-helper changes.

- Use 300 successful requests per equal-wire path, alternating interactive/background; expect exactly 14,100 total and 7,050 background fixture units. Both must use one synced A before first read and one synced S for each settled request.
- Separately benchmark auxiliary, reasoning and refusal workloads. Do not compare their different wire sizes as same-input parser overhead. Refusal is an expected typed outcome, not answer success or a hidden retry; report completed/refused/failed/cancelled/unknown counts and unexpected errors.
- Report p50/p95/p99 admission-at-first-read, full decode/settlement completion, pure compilation/validation and restart/replay latency; samples and failures; process peak RSS, release harness bytes and journal bytes/request.
- Measure actual journal write/fsync counts and ordering in separate tracing runs; instrumentation stays out of reported latency/RSS. Missing tracing is pending evidence, never an inferred measured count. Include settlement durability when reporting whole-request counts.
- Assert expected charges after each request and clean reopen. Report accounting/answer/outcome mismatches explicitly; never print a hardcoded parity success without assertions that support it.
- Preserve slow tails, caps and all checks. Fewer bytes or hosted timings are not actual token savings, power, physical hardware or provider compatibility evidence. Compare default CLI footprint separately; an isolated feature harness is not CLI overhead.

For the runtime PR, run locked offline regression, feature-enabled admission/outcome tests, existing 18,000 decoder samples, all 43 Node authentication tests, Rust 1.75 candidate core all-target tests and full AMG 1K/10K/100K scale/recovery. Provision dependencies only through existing approved processes; no new dependency or installation is needed by this contract. Build the existing release `responses_qualify` prerequisite before Node live-probe fixture tests. All tests/benchmarks use synthetic data and zero provider calls. Exact new script/example names are an implementation choice and must be documented only once implemented.

## Delivery and definition of done

Implement on a reviewed branch from verified main, preserving unrelated local edits. Include runtime, independent tests, benchmarks and affected documentation in authored commits under [AGENTS.md](../AGENTS.md). Record implementation decisions, intentional profile differences, commands, exact tested commits/run links, distinct test counts, full measurements and pending gates. Update README, PROJECT_STATE, this contract's status, protocol/atomic/inference architecture, resource and security docs where affected; review the remaining documentation.

Acceptance requires: strict historical behavior preserved; extended outcomes match the matrix; no text/refusal before required durable settlement; unknown usage remains charged/pending; permanent suspension and privacy bounds hold; meaningful fault/crash coverage passes; equal-wire parity and resource evidence are reported; all required regression/scale checks pass. No fixed new-test count or unmeasured performance promise is imposed.

Open a PR and report evidence for human review. Do not merge, mark the implementation verified before checks pass, unlock a real account, reopen the consumed live harness or begin the later CLI command/transport slice. This document is the concrete handoff contract; its creation alone is not an implementation result.
