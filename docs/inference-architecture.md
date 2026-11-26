# OpenAI-native inference architecture

Status: proposed, M0. [Architecture](architecture.md) | [Resource budget](resource-budget.md). No live inference is enabled by this document.

## Integration contract

Use official [Sign in with ChatGPT](https://developers.openai.com/siwc/token-sharing-open-source) documentation and verify current app eligibility, scopes, supported models, endpoint requirements and spending controls before any implementation is enabled against a real account. Private prototype eligibility is unresolved. ChatGPT Plus is not an API-key credit balance or a guaranteed raw-token quota. No undocumented endpoints, credential reuse from other clients, browser automation, account rotation or paid fallback.

Implement the supported OAuth flow with PKCE and securely stored registration/credentials; use only the authorized account and scope. Discover supported capabilities; authorized streaming Responses requests use the documented parameters (including store:false and stream:true where required). Keep conversation state locally. Do not assume access to ChatGPT memory/history or zero provider retention. Do not assume audio/Realtime is included. Verify owner has disabled additional credit spending in provider controls; if subscription-only operation cannot be established, keep live adapter disabled. The application does not change billing settings.

## Admission and accounting

Local intent resolver -> context compilation -> estimate -> governor reservation -> broker -> stream -> reconcile. The broker is the only inference egress and accepts single-use permits tied to request digest, account, maximum output, expiry and task ID. Revalidate any altered request. Concurrency reservations and durable usage updates must be atomic. An admission permit is not authorization to execute tools.

Local defaults: 750,000 raw tokens per calendar month, 10% maximum background allocation, 30% reserve for interactive tasks, routine 1024 input / 256 output target, routine total ceiling 1500. Use configurable rolling short-window ceilings and per-task call/deadline limits in addition to monthly budget. Do not invent provider quota or reset timing; provider limits override local estimates. Define UTC for accounting windows and record policy versions. All retries, follow-up requests and consolidation consume the same budget. Never double-count reasoning tokens already included in reported output.

Ledger fields: request ID, task ID, turn/epoch, account alias, model, priority, reservation estimate, actual input/output, cached/reasoning detail where supplied, status, timestamp, policy version. Never store OAuth tokens or complete prompts in the usage ledger. For cancellation, ambiguous dispatch or missing usage, conservatively charge the outstanding reservation until authoritative reconciliation; do not assume a refund. Handle crash recovery and expired undispatched permits distinctly from possibly dispatched calls. Estimates may be imperfect; surface discrepancies and block further admissions when over budget.

## Context compiler

Preserve owner instructions, applicable safety policy, recent turn state, task-relevant typed facts and only needed tool schemas. Use AMG exact entity/property lookup, bounded graph traversal, scoped FTS5 and local ranking initially; optional local embeddings only after benchmark. See [AMG architecture](amg-architecture.md). Preserve provenance, validity, conflicts and scope; return ambiguity/truncation metadata. No remote retrieval/routing requirement. Use stable prompt prefixes where appropriate; prompt caching is not assumed to reduce subscription usage. Keep essential context even if a task requires larger budget; ask the owner or defer rather than silently degrade accuracy. Untrusted content remains clearly delimited. Store facts with source, timestamp and confirmation status. No remote model call to select a model for every request.

## Voice integration

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

M1.1–M1.4b automated tests passed. The runtime can answer exact read-only AMG recall locally; no model call for those queries, missing facts or storage failures. M1.4a has data-only exact context formatting and administrative graph/FTS retrieval. M1.4c will extend relevance-ranked read-only retrieval and context compilation; no live dispatch. No live broker, authorized OAuth integration, policy-aware final context compiler, calendar/rolling ledger or subscription eligibility validation yet. Governor accounting remains separate until invariants can be preserved. See [README](../README.md).
