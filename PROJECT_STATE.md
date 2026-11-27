# AtomOS — current engineering state

Updated 2026-11-26 (owner WSL qualification findings). Verified executable baseline: `e4234174a4d17e4663bd0729daaca71cdc48f702` ([PR #24](https://github.com/idmfrank/atomos/pull/24)). This is the shared handoff between the ChatGPT AtomOS Project, Codex coding sessions and human maintainers. Read [AGENTS.md](AGENTS.md) first, then this file and the specific milestone documentation. Verify the actual checkout and latest CI before assuming this snapshot is current.

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

2026-11-26 local date, PR #24 [CI 45914977721](https://github.com/idmfrank/atomos/actions/runs/45914977721) and [scale 45915135417](https://github.com/idmfrank/atomos/actions/runs/45915135417) passed. 86 distinct Rust + 11 Python + 38 Node = 135 tests. Owner WSL 33/33 at 5ec8b3f. Default CLI 466,512 bytes; decoder p95 23.4/15.0/13.4 microseconds at 1/64/4096-byte reads; RSS 2,236 KiB. Five real loopback groups passed.

| Full-scale metric (p95 unless noted) | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Active assertions | 2,500 | 25,000 | 250,000 |
| Exact recall (ms) | 0.0583 | 0.0715 | 0.0968 |
| Graph traversal (ms) | 0.1643 | 0.1844 | 0.1873 |
| Selective context (ms) | 0.2763 | 0.3548 | 0.3958 |
| Common-term context (ms) | 0.3271 | 0.7151 | 3.4682 |
| Reference common term (ms) | 0.5440 | 5.1282 | 44.6299 |
| Remember (ms) | 1.2080 | 1.2186 | 1.1835 |
| Correct (ms) | 1.3188 | 1.2593 | 1.3060 |
| Forget (ms) | 1.1900 | 1.0989 | 1.1810 |
| Active CLI RSS (KiB) | 4,304 | 4,304 | 4,116 |
| Ranked context / same-scope bytes (100 requests) | 38,842 / 90,072 | 39,400 / 91,000 | 40,012 / 92,020 |
| Restore (s) | 0.151 | 1.673 | 17.309 |
| Recovery RSS (KiB) | 7,536 | 9,240 | 9,048 |

Same-run no-store CLI RSS 2,768 KiB; 100K incremental 1,348 KiB (1.316 MiB) versus 8 MiB target. All three datasets: 300/300 fixture facts matched, 100/100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery verification errors and remote calls. Hosted-runner variability is not a controlled hardware performance comparison. Bytes are not tokens; RSS is not whole-system RAM; synthetic recall is not general answer quality. Actual power, token savings, physical speech and subscription usage not measured.

## Current working environments

- ChatGPT AtomOS Project: architecture, roadmap, decision and review workspace. GitHub is the shared durable source, not a chat transcript.
- Codex desktop coding session: select connected WSL Ubuntu; open existing Linux checkout (prefer `~/atomos-catalog-test` for current qualification). Owner confirmed x86_64 Ubuntu 24.04.4 LTS, WSL2, Node 24.22.3/npm 11.19.1. Rust/Cargo 1.100.0, GCC 13.3.0, SQLite 3.45.1 verified locally; 26/26 WSL tests at 30df9b6 and decoder built. Main checkout `~/atomos` may have the earlier local password prompt fix; inspect status/diff and do not discard changes.
- Physical reference platform: existing codex-ubuntu remote SSH machine, reserved for later hardware testing once owner returns home. Do not modify SSH aliases, DNS, addresses or keys for temporary network changes.
- GitHub Actions: CI and scale. No credentials or provider inference in CI. Human reviews and merges runtime changes.
- Existing owner vault: `$HOME/.config/atomos-auth` on WSL. Do not read/decrypt outside the existing owner-run utility, copy it, inspect secret-bearing files, print tokens/passphrase, or move it to another environment. Never commit local state. No new registration needed.

## Immediate objective: M2.4 suspended pending transport investigation

**Both authorized attempts are consumed.** Original historical combined header rejection: unknown, 1,024 planning units retained. Owner WSL follow-up at 5ec8b3f: HTTP 200, content_type_class missing, content_encoding_class missing, probe_content_type, headers_ms 1124, elapsed_ms 1125, first_body_ms null, wire_bytes 0, reported_tokens null, unknown usage. Cumulative local reservation 2,048 planning units; actual provider usage unknown for both. Zero received bytes at rejection does not mean empty provider body. No validated response or actual usage; no new live inference authorization. Preserve both records, no deletion, registration changes or alternate vault workaround. Existing attempts cannot be repeated; any new final compatibility test needs a separately reviewed gate and new owner consent.

Official [Sign in with ChatGPT inference documentation](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) checked 2026-11-26: public Responses route, stream true, store false, terminal response.completed. Examples do not impose our additional response MIME gate. Missing response header is observed in this client's HTTP response, not proof of provider defect or proof body is SSE. Root cause not yet established. Do not broadly relax declared non-SSE/unknown encoding checks.

PR #24 real loopback investigation passed: absent MIME permits only strictly validated SSE; empty/JSON/HTML/malformed/truncated/invalid usage rejected. Actual provider body unknown. Owner confirmed spending controls: app enabled 50% cap, purchased credit access and automatic reload disabled, zero credit balance; aggregate analytics lag both attempts, no attributable consumption. No provider settings change requested.

Current proposed revision introduces a final explicitly gated compatibility qualification, superseding the previous stop only after reviewed merge and new owner approval. Both old records retained, new separate marker binds both historical hashes, reserves 1024 additional planning units (3072 total initially). No fourth attempt, no replay. Old commands still strict, new path missing-MIME only, Rust parser unchanged in acceptance; adds static decoder stage/event/item classes and validated-event count, no raw content. Expected 43 Node / 140 distinct tests pending CI. Rebuild Rust decoder before local tests; check app controls and authorization again. See latest section of qualification doc for exact gates. No unattended execution.

## Outstanding gates and next direction

- Live stream may contain reasoning items or event shapes unsupported by narrow Rust decoder. Investigate via safe sanitized categories and offline fixtures before separately approved future live calls.
- Live token revocation, integrated account-bound admission, trusted time/rolling budgets, real usage reconciliation, hard provider generation ceiling or explicit acceptable replacement policy, secure credential boundary and production security review remain open. Same-UID/root tampering and vault rollback are not solved.
- M2.5 provisional: integrate safe transport with broker/Token Governor and natural conversation, then authorized minimal AMG context only after privacy/spending gates. M2.6: benchmark actual quality, token use, latency and energy; model routing based on evidence rather than slug names.
- M3 prospective memory/scheduler, bounded autonomous tasks; M4 native bootable OS. Hardware voice wake/endpointing/AEC/barge-in, power/RSS and full speech tests pending. No assumption that WSL measures physical hardware efficiency.

## Maintenance protocol

Every new Codex session: read AGENTS.md and this file, inspect current commit and open PRs, verify evidence and report discrepancies before changing behavior. Maintain these files together with README and relevant technical docs when a milestone is merged or validated. Record exact commit and run URLs, test counts, measured results, remaining gates, next action and evidence provenance. Distinguish owner-reported physical/live results from synthetic CI. Never mark unrun tests as passed or an unqualified feature as complete. Do not include owner identity, secrets, network addresses, private content or raw provider responses in this public repository. Use a reviewed branch/PR for executable changes; do not merge on the owner's behalf.
