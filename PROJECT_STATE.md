# AtomOS — current engineering state

Updated 2026-11-26 (owner WSL qualification findings). Verified executable baseline: `61a7efc5810c67c20abdc4431f1806f41b40807c` ([PR #27](https://github.com/idmfrank/atomos/pull/27)). This is the shared handoff between the ChatGPT AtomOS Project, Codex coding sessions and human maintainers. Read [AGENTS.md](AGENTS.md) first, then this file and the specific milestone documentation. Verify the actual checkout and latest CI before assuming this snapshot is current.

## Mission and architecture

Always-on, voice-first personal AI operating environment on low-power commodity hardware. Natural conversation quality first; zero incremental AI spend using eligible official ChatGPT plan sharing; persistent identity and autonomy; token, power and hardware efficiency. Privacy/security are mandatory. Local first, zero idle remote inference, no unofficial access or paid fallback. Long-term bootable native OS; Linux-hosted runtime is the current development stage.

- Rust core and CLI: deterministic local operations, turn epochs, bounded playback, voice coordinator, durable reservation ledger. Keep production hot paths in Rust.
- AtomOS Memory Graph (AMG): SQLite temporal typed graph, owner scope, provenance and revisions, scoped graph/FTS retrieval, ranked minimal context, correction/forget, revision-specific expiring voice grants, logical recovery. No raw audio/full transcripts as memory. No remote retrieval; context is data, never instructions. Short serial writes; never hold database transaction across inference.
- Voice: persistent PCM coordinator, endpointing, follow-up, cancellation gated by AEC, optional native Sherpa-ONNX/Kokoro and batch Whisper.cpp/Piper adapters. Physical acoustics and wake word not qualified.
- Inference: local reflex/procedure before remote conversation; synthetic Rust broker, Token Governor, strict SSE text-only decoder and account-bound synthetic authorization. No production remote integration yet.
- Authentication cold path: explicit owner-run Node utility, official OAuth PKCE, signed identity, encrypted private vault, scoped token and read-only catalog. M2.4 has a separate one-shot transport with Rust replay, not a general-purpose inference client.

Design details: [README](README.md), [resource budget](docs/resource-budget.md), [inference architecture](docs/inference-architecture.md), [AMG foundation](docs/m1-amg-foundation.md), [AMG optimization](docs/m1-amg-optimization.md), [account authorization](docs/m2-account-authorization.md), [auth hardening](docs/m2-auth-hardening.md), [M2.4 procedure](docs/m2-first-live-qualification.md).

## Verified milestones

| Milestone | Evidence/status |
| --- | --- |
| M1.1–M1.3 | Local text and voice runtime foundations; synthetic tests passed, hardware voice unqualified. |
| M1.4a–f | AMG foundation, scoped recall, context ranking, grants, recovery, indexing and optimization; synthetic 100K scale passed. |
| M2.1 | Offline broker and durable usage reservation/settlement. |
| M2.2 | Offline bounded Responses SSE replay and strict lifecycle validation. |
| M2.3a | Synthetic account-bound authorization and durable suspension. |
| M2.3b–c | Official OAuth utility, encrypted vault, strict JSON and catalog support. |
| M2.3d | Owner ran actual WSL sign-in, reopened vault, 18/18 local tests, retrieved 10 account-specific model identifiers. Owner screenshot showed enabled 50% personal app limit. Live refresh subsequently verified; revocation not tested. |
| M2.4 | PR #19 merged, PR CI/post-merge CI/full scale all green. One-shot qualification harness ready for owner-run WSL build/preflight. Owner WSL build at 30df9b6 and 26/26 Node tests passed. First two live attempts rejected at headers; third delivered 8,465 bytes and eight validated SSE events, then unsupported event. All three reservations consumed with unknown usage. Owner refresh verified; no successful terminal response validated. |

## Latest verified test and benchmark baseline

2026-11-26 local date, PR #27 [CI 45918304938](https://github.com/idmfrank/atomos/actions/runs/45918304938), [scale 45918448554](https://github.com/idmfrank/atomos/actions/runs/45918448554) passed. 91 Rust + 11 Python + 43 Node = 145 distinct tests; owner WSL last verified 43/43 at 49c6cf2. Default CLI 466,512 bytes; strict p95 38.1/25.5/25.1 µs, same-wire compat 42.9/25.8/25.3 µs, auxiliary 49.1/37.6/30.1 µs (1/64/4096-byte reads); combined RSS 2,144 KiB. 9,000 replays, zero errors.

| Full-scale metric (p95 unless noted) | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Active assertions | 2,500 | 25,000 | 250,000 |
| Exact recall (ms) | 0.0244 | 0.0317 | 0.0504 |
| Graph traversal (ms) | 0.0845 | 0.0876 | 0.1001 |
| Selective context (ms) | 0.1270 | 0.1655 | 0.2163 |
| Common-term context (ms) | 0.1569 | 0.3736 | 1.8022 |
| Reference common term (ms) | 0.2824 | 2.4933 | 28.7214 |
| Remember (ms) | 13.5059 | 2.4336 | 1.6047 |
| Correct (ms) | 5.6239 | 2.1907 | 1.2718 |
| Forget (ms) | 3.9707 | 3.3273 | 1.1674 |
| Active CLI RSS (KiB) | 4,268 | 4,304 | 4,136 |
| Ranked context / same-scope bytes (100 requests) | 38,842 / 90,072 | 39,400 / 91,000 | 40,012 / 92,020 |
| Restore (s) | 0.079 | 0.967 | 10.471 |
| Recovery RSS (KiB) | 7,380 | 9,184 | 9,012 |

Same-run no-store CLI RSS 2,684 KiB; 100K incremental 1,452 KiB (1.418 MiB) versus 8 MiB target. All three datasets: 300/300 fixture facts matched, 100/100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery verification errors and remote calls. Hosted-runner variability is not a controlled hardware performance comparison. Bytes are not tokens; RSS is not whole-system RAM; synthetic recall is not general answer quality. Actual power, token savings, physical speech and subscription usage not measured.

## Current working environments

- ChatGPT AtomOS Project: architecture, roadmap, decision and review workspace. GitHub is the shared durable source, not a chat transcript.
- Codex desktop coding session: select connected WSL Ubuntu; open existing Linux checkout (prefer `~/atomos-catalog-test` for current qualification). Owner confirmed x86_64 Ubuntu 24.04.4 LTS, WSL2, Node 24.22.3/npm 11.19.1. Rust/Cargo 1.100.0, GCC 13.3.0, SQLite 3.45.1 verified locally; 26/26 WSL tests at 30df9b6 and decoder built. Main checkout `~/atomos` may have the earlier local password prompt fix; inspect status/diff and do not discard changes.
- Physical reference platform: existing codex-ubuntu remote SSH machine, reserved for later hardware testing once owner returns home. Do not modify SSH aliases, DNS, addresses or keys for temporary network changes.
- GitHub Actions: CI and scale. No credentials or provider inference in CI. Human reviews and merges runtime changes.
- Existing owner vault: `$HOME/.config/atomos-auth` on WSL. Do not read/decrypt outside the existing owner-run utility, copy it, inspect secret-bearing files, print tokens/passphrase, or move it to another environment. Never commit local state. No new registration needed.

## Immediate objective: M2.4 live harness closed; offline protocol compatibility

**All three authorized attempts consumed.** Preserve the three durable records; cumulative 3,072 local planning units retained, actual provider usage unknown. No fourth attempt, reset, alternate vault, re-registration or unattended inference. First two attempts rejected at headers. Final owner-run result at 49c6cf2 (2026-11-26): HTTP 200, missing content-type and encoding, headers 874 ms, first body byte 875 ms, HTTP body end 1591 ms, 8,465 wire/decoded bytes. Rust category Unsupported; stage part, event_class other, item_class none, 8 validated events. Missing-MIME transport successfully received bounded body and invoked decoder. No terminal response/expected text/actual usage validated. First byte is not TTFT. Unknown event name was not recorded; cannot infer annotation, reasoning, completion, output text or provider defect. No raw stream retained.

Owner spending settings observed enabled 50% app limit, purchased credits access and reload disabled, zero credit balance; analytics lagged attempts, no attributable usage. This is settings evidence, not a billing attestation.

Official [SIWC inference guide](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) and [Responses streaming reference](https://developers.openai.com/api/reference/resources/responses/streaming-events) reviewed 2026-11-26. Official examples handle text delta, failure, completion and do not require every other event to be a text lifecycle event; success requires completed terminal. API reference has additional documented events including annotations. Exact historical event remains unknown.

Current work: [offline protocol compatibility audit](docs/m2-protocol-compatibility.md). M2.4b validated queued state, 0/1 sequence base and explicit null annotations. M2.4c proposes separate bounded opaque reasoning and typed refusal profile, five added Rust tests (150 distinct expected), pending CI. Strict reference, compatibility entry point and live harness unchanged. No raw reasoning, summary streaming, tool calls or fallback. Next work: offline reference-based protocol compatibility audit. Retain strict framing, sequence/accounting/terminal/resource constraints. Compare official SDK event consumption with current narrow state machine on identical synthetic fixtures; classify documented auxiliary events explicitly, reject unsupported capability-bearing events, do not simply ignore arbitrary event types or infer usage from partial streams. Test complete/incomplete/failed terminal and trailing/duplicate/invalid usage. Consider SDK as reference oracle, not an automatic production dependency; measure overhead before choice. No new live reservation command or live execution in this harness. M2.5 account-bound Token Governor integration and hardware qualification can proceed offline while provider compatibility remains outstanding.

## Outstanding gates and next direction

- Live stream may contain reasoning items or event shapes unsupported by narrow Rust decoder. Investigate via safe sanitized categories and offline fixtures before separately approved future live calls.
- Live token revocation, integrated account-bound admission, trusted time/rolling budgets, real usage reconciliation, hard provider generation ceiling or explicit acceptable replacement policy, secure credential boundary and production security review remain open. Same-UID/root tampering and vault rollback are not solved.
- M2.5 provisional: integrate safe transport with broker/Token Governor and natural conversation, then authorized minimal AMG context only after privacy/spending gates. M2.6: benchmark actual quality, token use, latency and energy; model routing based on evidence rather than slug names.
- M3 prospective memory/scheduler, bounded autonomous tasks; M4 native bootable OS. Hardware voice wake/endpointing/AEC/barge-in, power/RSS and full speech tests pending. No assumption that WSL measures physical hardware efficiency.

## Maintenance protocol

Every new Codex session: read AGENTS.md and this file, inspect current commit and open PRs, verify evidence and report discrepancies before changing behavior. Maintain these files together with README and relevant technical docs when a milestone is merged or validated. Record exact commit and run URLs, test counts, measured results, remaining gates, next action and evidence provenance. Distinguish owner-reported physical/live results from synthetic CI. Never mark unrun tests as passed or an unqualified feature as complete. Do not include owner identity, secrets, network addresses, private content or raw provider responses in this public repository. Use a reviewed branch/PR for executable changes; do not merge on the owner's behalf.
