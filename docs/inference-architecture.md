# OpenAI-native inference architecture

Status: target architecture; M2.1–M2.4c isolated offline broker, account policy and decoder profiles verified. Actual owner OAuth/catalog/refresh succeeded. Three fixed live attempts consumed without validated completion or usage. M2.5a isolated atomic admission merged with PR #31 time/task follow-up merged externally and verified offline; M2.5b offline decoder integration is merged in PR #36 with successful post-merge CI; no general live inference. [Current state](../PROJECT_STATE.md) | [Architecture](architecture.md) | [Resource budget](resource-budget.md).

## CLI MVP scope

The [CLI roadmap](cli-mvp-roadmap.md) is the active product boundary. Text input/output replaces voice as the first interface; one owner and one active conversation are sufficient. Current text interaction remains local/mock. The [M2.5b offline outcome/admission/cancellation/settlement contract](m2-text-outcomes.md) is implemented and verified offline on PR #36, followed by the [specified CLI session integration](cli-session-contract.md). Transport, secure credentials, real account controls and private AMG disclosure remain separately gated.

The CLI is a thin adapter: no terminal command can bypass admission, change billing route or silently authorize memory egress. Define bounded working-turn retention, cancellation, EOF and disconnect behavior. No full transcript persistence or automatic memory extraction. Synthetic current decoders buffer until validated terminal/settlement; choosing future incremental display requires explicit outcome/accounting semantics.

## Deployed local/mock boundary

The [Linux CLI deployment](linux-cli-deployment.md#deployed-session-release) at merged PR #38 is tested and running under the explicitly selected owner account. Rust stays on the hot path; Node/Python/compiler tools support offline development and are not resident inference workers. No real account session, transport, provider model, private AMG disclosure or inference quota is enabled. The M2.5b source is merged and verified offline; default deployment does not enable synthetic replay or a provider.

The owner authorized host provisioning and local deployment, not a fourth live qualification attempt. Exact host/account/key data remain local. Local recall and bounded context use measured host resources before future remote admission; context bytes and RSS do not establish actual token, power or billing savings.

M2.5b's extended synthetic replay and typed Local/Completed/Refused/Failed results are implemented and verified offline on PR #36. Known usage settles; unknown outcomes stay charged/pending; refusal contains no answer text. This is library integration only, not a provider adapter or conversational CLI integration.

## CLI session and future context binding

The [session contract](cli-session-contract.md) is implemented and verified offline on PR #38; it is not an enabled provider client. Explicit fixed-fixture replay uses the same current-question admission and v1 ledger. Working turns are bounded, volatile and local-only; they do not enter the digest or reservation estimate. Before any provider sends history or AMG context, define a complete owned/canonical request that binds every byte and obeys reviewed disclosure and input/output bounds. Never bolt extra history onto a previously authorized current-question request. Status distinguishes mock calls/synthetic units from real usage, and no automatic retry is introduced. Cooperative replay tests do not establish signal handling, blocking-I/O cancellation or a production deadline.

## Integration contract

Use official [Sign in with ChatGPT](https://developers.openai.com/siwc/token-sharing-open-source) documentation and verify current app eligibility, scopes, supported models, endpoint requirements and spending controls before any implementation is enabled against a real account. Owner prototype authorization and catalog retrieval succeeded; continuing eligibility and production policy must still be verified. ChatGPT Plus is not an API-key credit balance or a guaranteed raw-token quota. No undocumented endpoints, credential reuse from other clients, browser automation, account rotation or paid fallback.

Implement the supported OAuth flow with PKCE and securely stored registration/credentials; use only the authorized account and scope. Discover supported capabilities; authorized streaming Responses requests use the documented parameters (including store:false and stream:true where required). Keep conversation state locally. Do not assume access to ChatGPT memory/history or zero provider retention. Do not assume audio/Realtime is included. Verify owner has disabled additional credit spending in provider controls; if subscription-only operation cannot be established, keep live adapter disabled. The application does not change billing settings.

## Admission and accounting

M2.5a implements an isolated, opt-in [atomic synthetic admission candidate](m2-atomic-admission.md), not the target live egress below. An owned immutable request and account/model/policy identity are checked before one fsynced admission/reservation record. No public detachable permit exists. Reference two-journal authorization, broker and strict/extended decoder implementations remain available; the preserved strict method uses the strict decoder. The [M2.5b contract](m2-text-outcomes.md) now has a separate explicit extended replay method; the existing strict method remains the reference. Final results distinguish local/completed/refused/failed outcomes. Refusal carries validated usage without text; failed known usage settles, failed unknown usage remains pending; rate/auth and overruns suspend durably. Storage faults suppress delivery. No journal-format, parser-capability or transport change is included.

Local intent resolver -> context compilation -> estimate -> governor reservation -> broker -> stream -> reconcile. The broker is the only inference egress and accepts single-use permits tied to request digest, account, maximum output, expiry and task ID. Revalidate any altered request. Concurrency reservations and durable usage updates must be atomic. An admission permit is not authorization to execute tools.

Proposed local planning defaults (not provider entitlement or implemented monthly quota): 750,000 raw tokens per calendar month, 10% maximum background allocation, 30% reserve for interactive tasks, routine 1024 input / 256 output target, routine total ceiling 1500. Use configurable rolling short-window ceilings and per-task call/deadline limits in addition to monthly budget. Do not invent provider quota or reset timing; provider limits override local estimates. Define UTC for accounting windows and record policy versions. All retries, follow-up requests and consolidation consume the same budget. Never double-count reasoning tokens already included in reported output.

Ledger fields: request ID, task ID, turn/epoch, account alias, model, priority, reservation estimate, actual input/output, cached/reasoning detail where supplied, status, timestamp, policy version. Never store OAuth tokens or complete prompts in the usage ledger. For cancellation, ambiguous dispatch or missing usage, conservatively charge the outstanding reservation until authoritative reconciliation; do not assume a refund. Handle crash recovery and expired undispatched permits distinctly from possibly dispatched calls. Estimates may be imperfect; surface discrepancies and block further admissions when over budget.

## Context compiler

Preserve owner instructions, applicable safety policy, recent turn state, task-relevant typed facts and only needed tool schemas. Use AMG exact entity/property lookup, bounded graph traversal, scoped FTS5 and local ranking initially; optional local embeddings only after benchmark. See [AMG architecture](amg-architecture.md). Preserve provenance, validity, conflicts and scope; return ambiguity/truncation metadata. No remote retrieval/routing requirement. Use stable prompt prefixes where appropriate; prompt caching is not assumed to reduce subscription usage. Keep essential context even if a task requires larger budget; ask the owner or defer rather than silently degrade accuracy. Untrusted content remains clearly delimited. Store facts with source, timestamp and confirmation status. No remote model call to select a model for every request.

## Deferred voice integration

Voice work and physical qualification are paused. This retained target applies only if a later voice interface is separately selected; it does not block text provider integration or CLI acceptance.

Local wake, VAD, streaming ASR, endpointing and local TTS consume no remote model tokens. Do not infer on speculative ASR partials. Stream text deltas to a bounded clause segmenter, then local TTS. On barge-in, stop playback locally and cancel upstream work where supported; reconcile outstanding usage. Record what was delivered, not merely generated. More advanced voice integrations require verified subscription eligibility and measured quality improvement.

## Failure modes and testing

| Condition | Behavior |
| --- | --- |
| No eligible provider/consent or spending assurance | Local mode; no live dispatch. |
| Local budget exhausted | Reject or defer; local commands and speech still function. |
| Provider usage limit | Suspend remote inference; present accurate status and provider settings guidance. No retry loop around limit. |
| Transient network/5xx | Bounded backoff with jitter and fresh admission; no duplicate side effects. |
| Revoked/expired authorization | Pause until supported refresh or owner reauth succeeds. |
| Unsupported model/parameter | Fail safely; rediscover documented capabilities rather than silently switch to paid API. |
| Stream interruption or unknown usage | Cancel, persist outcome and conservatively reconcile. |

M1: deterministic mock provider, no network, tests for reservation concurrency, restart, cancellation, zero idle calls and local resolution. M2: gated real integration, capability checks, streamed response and quota failure tests, measured usage. Use no real account secrets in CI. Define an explicit user-authorized live test mode after eligibility verification. Report measured raw tokens and successful tasks per 1000 tokens, plus quality and provider limits separately.

## Implementation status

M1.1–M1.4f merged and automated/scale validation passed. AMG remains local-only.
SQLite + FTS5 retained for M2 development; selective retrieval and memory
benchmarks pass on CI; no cross-engine superiority claim. M2.1 merged and passed [CI](https://github.com/idmfrank/atomos/actions/runs/45833540395) (61 Rust and 11 Python tests). It implements
an isolated typed synthetic replay broker, durable admission/settlement,
conservative unknown usage, persistent overrun halt and local-first
resolution. At M2.1 no live transport, OAuth, private-memory context, or actual
token estimation existed. Existing conversational CLI/voice remain mock-only. See [M2.1 implementation](m2-inference-broker.md) for
implemented invariants versus remaining request-binding, parsing, account,
window, consent and live integration gates.

M2.2 adds an opt-in [bounded SSE/JSON replay decoder](m2-responses-replay.md), with admission before reading, strict single-message text event lifecycle, identity/sequence/usage validation and differential typed-broker replay. Completed and validated failed/incomplete usage settle through the same governor. Unknown usage retains reservation. No HTTP, live streaming speech, credentials or real inference; narrow text profile is not full API compatibility. Feature-enabled CI and decoder benchmark passed.

M2.3 [synthetic account authorization](m2-account-authorization.md): immutable fixed grant/window, scope checks, sequential durable request intent, expiry, conservative recovery and persistent suspension. Live dispatch remains disabled. Official direct subscription route does not support max_output_tokens; local limits cannot guarantee generation ceiling. Personal-plan credit controls documented, but actual eligibility, authoritative preference verification, credential/OAuth lifecycle and transport controls remain unresolved. No paid fallback.

M2.3a merged and tested. M2.3b adds [official auth-only utility](m2-official-sign-in.md): short-lived Linux Node standard crypto and protected files; no runtime integration, provider inference or credential exchange in CI. Owner eligibility and secure deployment review required. No change to M2.3c spending/output bounds gates.

M2.3b merged and offline-tested. M2.3c [hardening](m2-auth-hardening.md) adds passphrase-encrypted auth profile, bounded duplicate-key audit, owner-run read-only account-specific model discovery and checkout v7.0.1. No live inference, bearer handoff, actual eligibility proof or autonomous credential unlock. Transport and authoritative spending/generation-bound verification remain gates.

M2.4 [qualification harness](m2-first-live-qualification.md): explicitly owner-run fixed nonpersonal one-shot, same account catalog and encrypted session, separate non-reusable fsynced intent/settlement, bounded HTTPS stream replay through existing Rust decoder. This is not integrated governor admission or conversational transport; no arbitrary prompts, no automatic retry, no paid fallback. A provider generation cap is unavailable; require informed acceptance for this one experiment only. No runtime integration or validated inference result. The three authorized historical attempts are consumed; final HTTP body received and eight SSE events validated, then unsupported event. Preserve all 3,072 local planning units (actual usage unknown), no fourth attempt.

M2.4b–c [protocol compatibility](m2-protocol-compatibility.md) validated separate offline queued/null-annotation, bounded opaque reasoning and typed refusal profiles. No SDK parity, full model compatibility, raw reasoning, tool execution or automatic fallback. M2.5a binds synthetic compiled request/account/model/turn with atomic reservation. M2.5b integrates typed outcomes, cancellation, settlement and durable synthetic suspension offline on PR #36, preserving refusal without a normal answer or retry; its full offline evidence is in the outcome contract. CLI integration follows; secure credentials and transport require a separate gate. No production inference until separately reviewed security, consent and spending policy. Existing live harness stays closed.

## Session candidate implementation

The new shared Session wraps the existing extended replay in one synchronous call, with turn ownership, post-settlement stale-delivery suppression and retention only after successful output/flush. AtomicReplay.status is a read-only accounting view, not a dispatch permit. Unknown usage, storage failure and durable halt retain the existing state-machine semantics. The CLI remains local/mock or explicit fixed-fixture replay; full regression, resource and CI evidence passed. [Exact tested commits and measurements](resource-budget.md#cli-session-verified-evidence).

PR #38 default local/mock session deployment at e7e8fc9 is verified. The installed CLI uses bounded local-only working turns and read-only memory; optional session-replay was not installed. Provider, vault, real account usage and private context egress remain disabled. [Deployment evidence](linux-cli-deployment.md#deployed-session-release).
