# M2.1 — Offline inference broker and governor integration

> **Milestone context (reviewed through PR #28, 2026-11-26):** This is the original synthetic broker slice and its historical verification. M2.2–M2.4c added strict and isolated compatibility decoder profiles, synthetic authorization and separate owner-run authentication/qualification. The existing broker remains offline and uses its original strict replay contract; M2.5 integrated account-bound admission is next. See [inference architecture](inference-architecture.md) and [protocol compatibility](m2-protocol-compatibility.md). [Current project state](../PROJECT_STATE.md) is authoritative; status and next-step statements below describe the original milestone unless explicitly updated.


Status: merged in PR #12; locked offline post-merge CI passed. No live inference, credentials, network transport,
provider registration, or remote AMG disclosure.

## Official integration boundary (2026-11-26)

[Sign in with ChatGPT overview](https://developers.openai.com/siwc/token-sharing-open-source)
describes eligible Responses API use by open-source/local apps. The user/client
registration and stable opaque host identity are distinct. Do not reuse another
client's credentials or regenerate host IDs to reset usage. Private prototype
eligibility is not established. [Help Center plan usage](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites)
describes included Work/Codex usage and optional credits. No entitlement or
raw-token quota is inferred from Plus. Owner must verify permitted use and
disable extra credit spending; if no trustworthy subscription-only enforcement
can be established, no live dispatch. Some previously documented subpage URLs
were unavailable when checked; reverify current registration, scopes, endpoints,
models, parameters and error/usage schema before implementing live access.
No unofficial endpoint or paid fallback.

## Implemented

The M2.1 broker remains the reference. [M2.5a](m2-atomic-admission.md) adds an isolated atomic account-bound admission candidate; the shared checked Governor reservation decision preserves existing total/background/per-request/interactive limits. No reference decoder or broker event behavior is replaced. No transport or credential integration is added.

The std-only core `broker` module is an isolated synchronous synthetic replay
harness. The current CLI and voice pipeline remain local/mock; no code path
from conversational AMG to the broker. No network, HTTP, JSON/SSE, tool
execution, OAuth or provider selection. No real data in fixtures.

1. Validate task/turn IDs, active turn, input (1–1024 UTF-8 bytes, nonblank,
   no control characters), output ceiling (1–256 fixture units).
2. Resolve existing exact local commands before reservation. No synthetic
   dispatch/charge for these or invalid/stale requests.
3. Reserve durably via the single-owner governor before incrementing dispatch
   count and reading fixture events. Synthetic estimate = input UTF-8 bytes
   + 128 framing units + maximum output. This is NOT a real tokenizer,
   provider accounting or guaranteed upper bound.
4. Use an internal reservation once in the same synchronous call; no public
   detachable permit, request alteration, dispatch interface or transport.
   Request/task/turn binding is in memory, not a durable digest/expiry/account
   record. No automatic retry or background execution.
5. Replay bounded typed Started/Delta/Completed/Failed/Cancelled events
   (128 maximum, 4096 output bytes maximum). Every event must carry the
   active request's turn. Reject wrong epoch, repeated start, invalid order,
   nonterminal completion, duplicate completion, missing usage, oversized
   stream or control text. Buffer text until validated final settlement;
   no actual incremental speech delivery. No network deadline or asynchronous
   cancellation because there is no transport.
6. Valid final usage is input + output only. Cached input and reasoning
   output are subsets, not extra tokens. Reject impossible fixture arithmetic.
   Unknown/cancelled/malformed outcomes retain full reservation. Charge actual
   authoritative fixture usage even if excessive; a single X settlement
   record persists a halt on input/output/total cap violation. Old S overruns
   also latch a halt on replay. No ledger reset on restart; no automatic
   retries or refunds. H record handles arithmetic overflow without writing
   invalid accounting. Journal corruption/failed persistence fails closed.
7. Rate/authorization failure suspends further synthetic dispatches for the
   current instance; local replies still work. Restart preserves charges
   but not that suspension; live provider suspension/recovery must be durable
   and authoritative before implementation.

Dedicated ledger namespace `SYNTHETIC_LEDGER_ROOT/synthetic-v1`. Numeric R/S/X/H
records only, no user text, prompts, credentials or raw event content. Existing
governor lock and fsync invariants; trusted local directory, not an authenticated
account ledger. The new records are intentionally rejected by older code.
Do not open a real ledger in this example. Limits are fixed local simulation
budgets, not calendar/rolling limits or subscription entitlement. No actual
request identifiers, account aliases, digest, policy/version/time metadata or
remote usage reconciliation yet.

## Run

```bash
cargo fetch --locked
tmp="$(mktemp -d)"
cargo run --locked --offline -p atomos-core --example broker_demo -- "$tmp"
rm -rf -- "$tmp"
```

Expected: local name, synthetic entropy response, one synthetic dispatch,
47 charged fixture units, zero real calls. Run with a fresh directory.

Tests cover local and stale requests, admission denial, exact settlement
without double-counting, restart, cancellation, missing/malformed/duplicate
terminal events, output/event limits, rate/auth suspension, overrun halts
and background reserve. Existing governor tests plus durable below-total
overrun regression. CI includes separate example smoke test.

## Next gates

M2.2: bounded byte-level SSE/JSON parser and differential replay against
documented Responses fixtures, strict event/usage semantics, resource and
cancellation tests. Recheck official provider API, authorization and usage
before designing transport; no automatic provider retries. M2.3: authenticated
owner consent, supported OAuth/host lifecycle, account-specific caps and
durable single-use request identity, clock/window/expiry, secure credential
storage, full compiled request token estimation, provider limit recovery,
and zero-additional-spend gate. M2.4: explicitly authorized live synthetic
text smoke test only after all gates, then local voice and memory disclosure
integration. No private AMG data until authenticated disclosure/security
controls; no live model traffic in CI. Physical voice/AEC and reference-device
power, hard storage quotas, encryption, deletion reconciliation remain open.

## Verified results

[PR CI 45808715905](https://github.com/idmfrank/atomos/actions/runs/45808715905) and [post-merge CI 45833540395](https://github.com/idmfrank/atomos/actions/runs/45833540395) passed on merge f0815cb9a72f7d915fe3ca0a6983290f51ce2fc8. 61 Rust + 11 Python tests, including broker regressions and example smoke: one synthetic dispatch, 47 charged fixture units, zero real provider calls. No-store release CLI 466,464 bytes / 2,704 KiB peak RSS (unchanged binary size; broker is separate). Active 1K benchmark: exact p95 0.0620 ms, graph 0.1891 ms, selective context 0.3194 ms, common-term 0.3896 ms (reference 0.5882 ms), forget 1.5660 ms; 100/100 complete selective packets and zero mismatches in 30 comparisons. Active CLI 4,108 KiB vs same-run no-store 2,672 KiB. No new 100K scale run on M2.1 found; M1.4f remains scale/recovery baseline. No live inference, actual token estimate or account validation. Next: M2.2 bounded SSE/JSON parsing and deterministic replay.
