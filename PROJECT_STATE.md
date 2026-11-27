# AtomOS — current engineering state

Updated 2026-11-27 (M2.5a candidate development). Verified main baseline: `5f564ad5641ea716d86efc894b2b132aaae9fa77` ([PR #29](https://github.com/idmfrank/atomos/pull/29), documentation reconciliation over PR #28's runtime). PR #29 post-merge CI and full scale verified; measurements below supersede PR #28 while preserving its historical evidence. M2.5a is an isolated offline candidate on `codex/m2-5a-atomic-admission`, not merged. This is the shared engineering handoff; read AGENTS.md first, then this file and the milestone documentation. Verify the checkout and CI before assuming this snapshot is current.

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

| M2.4b–c | PR #27 and #28 merged and verified: separate offline auxiliary and opaque reasoning/refusal profiles; strict decoder/broker and live harness unchanged. 18,000 synthetic replays passed. Actual model event compatibility still unqualified. |

## Latest verified test and benchmark baseline

PR #29 main baseline above: [post-merge CI 45919844209](https://github.com/idmfrank/atomos/actions/runs/45919844209), [full scale 45919947851](https://github.com/idmfrank/atomos/actions/runs/45919947851), both successful. Logs retrieved 2026-11-27. Same 150 distinct tests and 18,000 decoder replays, zero errors. CLI 466,512 bytes; scale no-store RSS 2,588 KiB. Decoder p95 (1/64/4096-byte reads, microseconds): strict 43.7/26.5/26.0, same-wire compat 37.1/26.5/26.2, auxiliary 47.0/31.0/31.0, same-wire extended 38.3/26.2/26.1, reasoning 51.8/32.2/31.7, refusal 58.6/31.2/24.2; combined peak RSS 2,116 KiB. Hosted timing differences are not a runtime performance change.

| PR #29 scale metric (p95 unless noted) | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Exact recall (ms) | 0.0261 | 0.0320 | 0.0474 |
| Graph traversal (ms) | 0.0812 | 0.0828 | 0.0919 |
| Selective context (ms) | 0.1382 | 0.1677 | 0.2051 |
| Common-term context (ms) | 0.2141 | 0.3829 | 1.6446 |
| Reference common term (ms) | 0.3023 | 3.1094 | 24.9433 |
| Remember (ms) | 1.1667 | 1.0571 | 1.1274 |
| Correct (ms) | 1.2695 | 1.1047 | 1.1296 |
| Forget (ms) | 1.1991 | 1.0683 | 1.0602 |
| Active CLI RSS (KiB) | 4,304 | 4,132 | 4,196 |
| Restore (s) | 0.0791 | 0.9551 | 9.8047 |
| Recovery RSS (KiB) | 7,388 | 9,216 | 9,108 |

All three datasets: 300/300 fixture facts, 100/100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery errors/calls. 100K incremental CLI RSS 1,608 KiB. Assertion counts, context/export bytes and DB sizes unchanged from the historical table below.

### Historical PR #28 measurements

2026-11-26 local date, PR #28 [PR CI 45918815563](https://github.com/idmfrank/atomos/actions/runs/45918815563), [post-merge CI 45919051196](https://github.com/idmfrank/atomos/actions/runs/45919051196), [scale 45919185353](https://github.com/idmfrank/atomos/actions/runs/45919185353) passed. 96 Rust + 11 Python + 43 Node = 150 distinct tests; owner WSL last verified 43/43 at 49c6cf2. Default CLI 466,512 bytes. 18,000 decoder replays zero errors, combined process RSS 2,216 KiB. Decoder p95 at 1/64/4096-byte reads (µs): strict 23.4/16.5/16.3, same-wire compat 23.0/16.8/16.1, auxiliary 26.9/19.4/18.7, same-wire extended 22.6/16.6/19.8, opaque reasoning 29.1/20.9/20.1, refusal 22.3/15.6/15.2.

| Full-scale metric (p95 unless noted) | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Active assertions | 2,500 | 25,000 | 250,000 |
| Exact recall (ms) | 0.0619 | 0.0666 | 0.0892 |
| Graph traversal (ms) | 0.1984 | 0.2883 | 0.2099 |
| Selective context (ms) | 0.3305 | 0.3664 | 0.4145 |
| Common-term context (ms) | 0.3612 | 0.7336 | 3.0721 |
| Reference common term (ms) | 0.5762 | 4.4162 | 44.0512 |
| Remember (ms) | 1.4814 | 1.4589 | 1.5711 |
| Correct (ms) | 1.5336 | 1.9090 | 1.9869 |
| Forget (ms) | 1.4328 | 1.4388 | 1.4984 |
| Active CLI RSS (KiB) | 4,104 | 4,080 | 4,144 |
| Ranked context / same-scope bytes (100 requests) | 38,842 / 90,072 | 39,400 / 91,000 | 40,012 / 92,020 |
| Restore (s) | 0.151 | 1.646 | 17.411 |
| Recovery RSS (KiB) | 7,528 | 9,292 | 9,124 |

Same-run no-store CLI RSS 2,576 KiB; 100K incremental 1,568 KiB (1.531 MiB) versus 8 MiB target. All three datasets: 300/300 fixture facts matched, 100/100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery verification errors and remote calls. Hosted-runner variability is not a controlled hardware performance comparison. Bytes are not tokens; RSS is not whole-system RAM; synthetic recall is not general answer quality. Actual power, token savings, physical speech and subscription usage not measured.

## Current working environments

- ChatGPT AtomOS Project: architecture, roadmap, decision and review workspace. GitHub is the shared durable source, not a chat transcript.
- Codex desktop coding session: select connected WSL Ubuntu; open existing Linux checkout (prefer `~/atomos-catalog-test` for current qualification). Owner confirmed x86_64 Ubuntu 24.04.4 LTS, WSL2, Node 24.22.3/npm 11.19.1. Rust/Cargo 1.100.0, GCC 13.3.0, SQLite 3.45.1 verified locally; 26/26 WSL tests at 30df9b6 and decoder built. Main checkout `~/atomos` may have the earlier local password prompt fix; inspect status/diff and do not discard changes.
- Physical reference platform: existing codex-ubuntu remote SSH machine, reserved for later hardware testing once owner returns home. Do not modify SSH aliases, DNS, addresses or keys for temporary network changes.
- GitHub Actions: CI and scale. No credentials or provider inference in CI. Human reviews and merges runtime changes.
- Existing owner vault: `$HOME/.config/atomos-auth` on WSL. Do not read/decrypt outside the existing owner-run utility, copy it, inspect secret-bearing files, print tokens/passphrase, or move it to another environment. Never commit local state. No new registration needed.

## Immediate objective: M2.5a offline atomic admission; live harness closed

**All three authorized attempts consumed.** Preserve the three durable records; cumulative 3,072 local planning units retained, actual provider usage unknown. No fourth attempt, reset, alternate vault, re-registration or unattended inference. First two attempts rejected at headers. Final owner-run result at 49c6cf2 (2026-11-26): HTTP 200, missing content-type and encoding, headers 874 ms, first body byte 875 ms, HTTP body end 1591 ms, 8,465 wire/decoded bytes. Rust category Unsupported; stage part, event_class other, item_class none, 8 validated events. Missing-MIME transport successfully received bounded body and invoked decoder. No terminal response/expected text/actual usage validated. First byte is not TTFT. Unknown event name was not recorded; cannot infer annotation, reasoning, completion, output text or provider defect. No raw stream retained.

Owner spending settings observed enabled 50% app limit, purchased credits access and reload disabled, zero credit balance; analytics lagged attempts, no attributable usage. This is settings evidence, not a billing attestation.

Official [SIWC inference guide](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) and [Responses streaming reference](https://developers.openai.com/api/reference/resources/responses/streaming-events) reviewed 2026-11-26. Official examples handle text delta, failure, completion and do not require every other event to be a text lifecycle event; success requires completed terminal. API reference has additional documented events including annotations. Exact historical event remains unknown.

M2.4b–c offline [protocol compatibility](docs/m2-protocol-compatibility.md) validated queued/null-annotation events, bounded opaque reasoning item and typed refusal, with strict reference and broker unchanged. Exact live event, SDK differential execution and model qualification remain open. M2.5a implements isolated atomic account-bound admission with the existing strict decoder only; extended decoder integration is M2.5b. Secure credential handoff/transport remain future separately scoped work. No raw reasoning, tools, fallback or fourth attempt. Physical hardware qualification may proceed independently.

## Outstanding gates and next direction

- Live stream may contain reasoning items or event shapes unsupported by narrow Rust decoder. Investigate via safe sanitized categories and offline fixtures before separately approved future live calls.
- Live token revocation, integrated account-bound admission, trusted time/rolling budgets, real usage reconciliation, hard provider generation ceiling or explicit acceptable replacement policy, secure credential boundary and production security review remain open. Same-UID/root tampering and vault rollback are not solved.
- M2.5 provisional: integrate safe transport with broker/Token Governor and natural conversation, then authorized minimal AMG context only after privacy/spending gates. M2.6: benchmark actual quality, token use, latency and energy; model routing based on evidence rather than slug names.
- M3 prospective memory/scheduler, bounded autonomous tasks; M4 native bootable OS. Hardware voice wake/endpointing/AEC/barge-in, power/RSS and full speech tests pending. No assumption that WSL measures physical hardware efficiency.

## Maintenance protocol

Every new Codex session: read AGENTS.md and this file, inspect current commit and open PRs, verify evidence and report discrepancies before changing behavior. Follow the [per-commit documentation rule](AGENTS.md#documentation-is-part-of-every-commit): review and update PROJECT_STATE in every authored commit; update README and all affected technical docs in the same change, then reconcile verified evidence after every merge/test run. Preserve historical milestone evidence and explicitly distinguish current instructions from superseded ones. Record exact commit and run URLs, test counts, measured results, remaining gates, next action and evidence provenance. Distinguish owner-reported physical/live results from synthetic CI. Never mark unrun tests as passed or an unqualified feature as complete. Do not include owner identity, secrets, network addresses, private content or raw provider responses in this public repository. Use a reviewed branch/PR for executable changes; do not merge on the owner's behalf.

## Documentation reconciliation

This documentation-only change audits the 24 Markdown files at the PR #28 baseline, updates current architecture/status/benchmark summaries and marks older milestone procedures as historical with links to current guidance. New per-commit documentation rule in AGENTS.md. No runtime or test behavior changes; this documentation PR's CI is pending until actually run. Latest executable evidence remains PR #28. No new inference authorization. Review scope: status, operational guidance and cross-document consistency; not a fresh external dependency, provider contract, full source or security audit. Historical measurements remain labeled and unchanged.

The preceding paragraph is the historical PR #29 scope; its post-merge evidence is now reconciled above. M2.5a adds an opt-in Rust candidate with one durable account/request/reservation transition, shared Governor budget admission rules, immutable SHA-256 compiled identity, strict existing decoder, bounded private journal and permanent suspension. [Implementation, format and semantics](docs/m2-atomic-admission.md). Reference broker/decoders and owner qualification paths remain preserved. All three qualification attempts and 3,072 retained units remain consumed; no owner records accessed.

Per-commit review covers README, architecture, inference, account authorization, broker, protocol compatibility, security and resource docs. Historical milestone/auth/AMG/voice documents retain their evidence and operational boundaries; no behavior in those components changed. No human approval to merge is implied.

### M2.5a local validation, 2026-11-27

WSL2 Linux snapshot of this candidate's runtime: existing locked offline regression, 18,000 decoder replays, all 43 Node tests, candidate feature suite and full AMG 1K/10K/100K passed. 119 Rust + 11 Python + 43 Node = 173 distinct tests (150 preserved + 23 new; repeated feature/compiler executions are not additional distinct tests). Rust 1.75 package-only core check and all-target core tests passed; whole-workspace MSRV remains unverified because existing getrandom 0.4.3 requires newer Cargo. Initial host failures were CRLF shell/fixture execution and a missing prerequisite decoder executable for Node tests; the Linux snapshot restored text line endings and built that executable before rerun. Two new test fixture expectations were corrected to the existing 35 input/12 output units. Assertions were not weakened.

Final 300-request admission benchmark: reference/candidate p50 6.2921/2.7969 ms, p95 8.3572/3.4981 ms, p99 24.9803/14.3612 ms. Trace: 4/2 journal write syscalls and 2/1 fsyncs per admission. Peak RSS 2,416/2,316 KiB, release harness 416,528/439,944 bytes, completed-request journal bytes 52.560/104.280, restart p95 4.3270/0.8189 ms. Exact charge 14,100 units each, zero parity/errors, candidate background 7,050. [Full method and caveats](docs/m2-atomic-admission.md). These final results supersede earlier development p95 7.2182/3.7824 ms; the final candidate rejects impossible usage and post-revocation settlement records.

Local AMG p95 (1K/10K/100K): exact 0.0906/0.1200/0.1850 ms; graph 0.4241/0.4303/0.4291; selective 0.5548/0.6326/0.9881; common 0.7129/1.4725/10.9405 vs reference 1.1686/6.9933/233.5749. Remember 15.2711/11.8953/10.6589; correct 16.8648/10.6373/11.7654; forget 14.0621/12.0705/10.6595. Restore 0.2318/2.4856/25.5890 s, recovery RSS 7,420/9,292/9,232 KiB; active CLI RSS 4,264/4,280/4,300 vs no-store 2,904 KiB. All sizes: 300/300 matched facts, 100 complete packets, precision/recall 1, 0/30 parity mismatches and zero recovery errors/calls. WSL scheduling/storage tails are uncontrolled; not a regression/performance claim against hosted CI. Local default CLI 466,504 bytes versus historical hosted 466,512; shared Governor refactoring can affect linked code size, but this is not a controlled binary-size comparison. Decoder process peak RSS 2,260 KiB, 18,000 samples zero errors; strict p95 at 1/64/4096-byte reads 90.1/50.1/48.6 microseconds.

Candidate PR CI, hosted full-scale benchmark and human review remain pending until verified on committed revisions. Do not merge; no live inference authorized.
