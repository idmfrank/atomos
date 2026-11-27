# AtomOS — current engineering state

Updated 2026-11-26 (owner WSL qualification findings). Verified executable baseline: `5ec8b3f5ef7d7b7683b69e251bfdd01fb949e0d7` ([PR #22](https://github.com/idmfrank/atomos/pull/22)). This is the shared handoff between the ChatGPT AtomOS Project, Codex coding sessions and human maintainers. Read [AGENTS.md](AGENTS.md) first, then this file and the specific milestone documentation. Verify the actual checkout and latest CI before assuming this snapshot is current.

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
| M2.4 | PR #19 merged, PR CI/post-merge CI/full scale all green. One-shot qualification harness ready for owner-run WSL build/preflight. Owner WSL build at 30df9b6 and 26/26 Node tests passed. First live attempt returned HTTP 200 but failed combined media/encoding validation; unknown usage retained, no successful inference validated. Owner refresh then succeeded and reservation remained present. |

## Latest verified test and benchmark baseline

2026-11-26, PR #22 [CI 45913123800](https://github.com/idmfrank/atomos/actions/runs/45913123800) and [post-merge CI 45913847075](https://github.com/idmfrank/atomos/actions/runs/45913847075) passed; owner WSL at 5ec8b3f passed 33/33 Node tests. 86 Rust + 11 Python + 33 Node = 130 distinct tests. PR #23 documentation-only merge 6dca0d5 [CI 45914518212](https://github.com/idmfrank/atomos/actions/runs/45914518212) and [scale 45914610881](https://github.com/idmfrank/atomos/actions/runs/45914610881) passed. Latest measurements below are from those runs. Default CLI 466,512 bytes; decoder p95 16.0/11.6/11.2 microseconds at 1/64/4096-byte reads, RSS 2,204 KiB.

| Full-scale metric (p95 unless noted) | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Active assertions | 2,500 | 25,000 | 250,000 |
| Exact recall (ms) | 0.0602 | 0.0735 | 0.0907 |
| Graph traversal (ms) | 0.1856 | 0.2104 | 0.2175 |
| Selective context (ms) | 0.3252 | 0.3694 | 0.4279 |
| Common-term context (ms) | 0.3767 | 0.7211 | 2.9899 |
| Reference common term (ms) | 0.6271 | 5.2753 | 44.5215 |
| Remember (ms) | 1.9301 | 1.6385 | 1.7605 |
| Correct (ms) | 1.8385 | 1.5412 | 1.8823 |
| Forget (ms) | 1.6882 | 1.4953 | 1.6698 |
| Active CLI RSS (KiB) | 4,276 | 4,200 | 4,248 |
| Ranked context / same-scope bytes (100 requests) | 38,842 / 90,072 | 39,400 / 91,000 | 40,012 / 92,020 |
| Restore (s) | 0.148 | 1.647 | 17.412 |
| Recovery RSS (KiB) | 7,448 | 9,268 | 9,096 |

Same-run no-store CLI RSS 2,620 KiB; 100K incremental 1,628 KiB (1.590 MiB) versus 8 MiB target. All three datasets: 300/300 fixture facts matched, 100/100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery verification errors and remote calls. Hosted-runner variability is not a controlled hardware performance comparison. Bytes are not tokens; RSS is not whole-system RAM; synthetic recall is not general answer quality. Actual power, token savings, physical speech and subscription usage not measured.

## Current working environments

- ChatGPT AtomOS Project: architecture, roadmap, decision and review workspace. GitHub is the shared durable source, not a chat transcript.
- Codex desktop coding session: select connected WSL Ubuntu; open existing Linux checkout (prefer `~/atomos-catalog-test` for current qualification). Owner confirmed x86_64 Ubuntu 24.04.4 LTS, WSL2, Node 24.22.3/npm 11.19.1. Rust/Cargo 1.100.0, GCC 13.3.0, SQLite 3.45.1 verified locally; 26/26 WSL tests at 30df9b6 and decoder built. Main checkout `~/atomos` may have the earlier local password prompt fix; inspect status/diff and do not discard changes.
- Physical reference platform: existing codex-ubuntu remote SSH machine, reserved for later hardware testing once owner returns home. Do not modify SSH aliases, DNS, addresses or keys for temporary network changes.
- GitHub Actions: CI and scale. No credentials or provider inference in CI. Human reviews and merges runtime changes.
- Existing owner vault: `$HOME/.config/atomos-auth` on WSL. Do not read/decrypt outside the existing owner-run utility, copy it, inspect secret-bearing files, print tokens/passphrase, or move it to another environment. Never commit local state. No new registration needed.

## Immediate objective: M2.4 suspended pending transport investigation

**Both authorized attempts are consumed.** Original historical combined header rejection: unknown, 1,024 planning units retained. Owner WSL follow-up at 5ec8b3f: HTTP 200, content_type_class missing, content_encoding_class missing, probe_content_type, headers_ms 1124, elapsed_ms 1125, first_body_ms null, wire_bytes 0, reported_tokens null, unknown usage. Cumulative local reservation 2,048 planning units; actual provider usage unknown for both. Zero received bytes at rejection does not mean empty provider body. No validated response or actual usage; no new live inference authorization. Preserve both records, no third attempt, no deletion, registration changes or alternate vault workaround.

Official [Sign in with ChatGPT inference documentation](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) checked 2026-11-26: public Responses route, stream true, store false, terminal response.completed. Examples do not impose our additional response MIME gate. Missing response header is observed in this client's HTTP response, not proof of provider defect or proof body is SSE. Root cause not yet established. Do not broadly relax declared non-SSE/unknown encoding checks.

Next work is offline only. Proposed loopback experiment adds five tests, pending CI (38 Node, 135 distinct total expected). Native live transport still requires explicit SSE MIME; candidate fixed to 127.0.0.1 with synthetic token and no CLI selection. No ledger or authorization changes.

Investigation: compare documented curl/SDK transport semantics with native HTTPS; test a real local HTTP fixture including missing Content-Type, empty/truncated streams, non-SSE JSON/HTML, malformed events, gzip and body limits. Evaluate a narrowly scoped missing-MIME candidate path that retains all bounds, rejects explicit wrong media/encoding, and requires existing strict terminal lifecycle and usage validation. No live activation or additional reservation command in this investigation. Review provider app usage and current saved cap with owner; seek provider clarification if needed. General live inference remains disabled.

## Outstanding gates and next direction

- Live stream may contain reasoning items or event shapes unsupported by narrow Rust decoder. Investigate via safe sanitized categories and offline fixtures before separately approved future live calls.
- Live token revocation, integrated account-bound admission, trusted time/rolling budgets, real usage reconciliation, hard provider generation ceiling or explicit acceptable replacement policy, secure credential boundary and production security review remain open. Same-UID/root tampering and vault rollback are not solved.
- M2.5 provisional: integrate safe transport with broker/Token Governor and natural conversation, then authorized minimal AMG context only after privacy/spending gates. M2.6: benchmark actual quality, token use, latency and energy; model routing based on evidence rather than slug names.
- M3 prospective memory/scheduler, bounded autonomous tasks; M4 native bootable OS. Hardware voice wake/endpointing/AEC/barge-in, power/RSS and full speech tests pending. No assumption that WSL measures physical hardware efficiency.

## Maintenance protocol

Every new Codex session: read AGENTS.md and this file, inspect current commit and open PRs, verify evidence and report discrepancies before changing behavior. Maintain these files together with README and relevant technical docs when a milestone is merged or validated. Record exact commit and run URLs, test counts, measured results, remaining gates, next action and evidence provenance. Distinguish owner-reported physical/live results from synthetic CI. Never mark unrun tests as passed or an unqualified feature as complete. Do not include owner identity, secrets, network addresses, private content or raw provider responses in this public repository. Use a reviewed branch/PR for executable changes; do not merge on the owner's behalf.
