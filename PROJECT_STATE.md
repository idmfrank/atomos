# AtomOS — current engineering state

Updated 2026-11-26 (owner WSL qualification findings). Verified executable baseline: `6dff9641f6a2ff91b0cd0ad5f4690d5cbbc20aa2` ([PR #19](https://github.com/idmfrank/atomos/pull/19)). This is the shared handoff between the ChatGPT AtomOS Project, Codex coding sessions and human maintainers. Read [AGENTS.md](AGENTS.md) first, then this file and the specific milestone documentation. Verify the actual checkout and latest CI before assuming this snapshot is current.

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

2026-11-26, merged executable commit above. [PR CI 45873933139](https://github.com/idmfrank/atomos/actions/runs/45873933139), [post-merge CI 45874158043](https://github.com/idmfrank/atomos/actions/runs/45874158043), [scale 45874467379](https://github.com/idmfrank/atomos/actions/runs/45874467379). 86 distinct Rust + 11 Python + 26 Node (18 auth, 8 probe) = 123 distinct passing tests; zero actual inference calls in CI. Locked dependencies provisioned before offline test/build. Default release CLI 466,512 bytes. Separate synthetic decoder: 2,497-byte stream, 1,000 repetitions per chunk size; p95 40.9/22.4/22.0 microseconds at 1/64/4096-byte reads; zero errors; decoder process peak RSS 2,188 KiB.

| Full-scale metric (p95 unless noted) | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Active assertions | 2,500 | 25,000 | 250,000 |
| Exact recall (ms) | 0.0625 | 0.0666 | 0.0768 |
| Graph traversal (ms) | 0.1885 | 0.2309 | 0.2152 |
| Selective context (ms) | 0.3122 | 0.3751 | 0.3957 |
| Common-term context (ms) | 0.3860 | 0.7245 | 3.1946 |
| Reference common term (ms) | 0.8638 | 4.4316 | 43.8345 |
| Remember (ms) | 1.3741 | 1.5225 | 1.6706 |
| Correct (ms) | 1.5467 | 1.4977 | 1.7307 |
| Forget (ms) | 1.3533 | 1.3180 | 1.5756 |
| Active CLI RSS (KiB) | 4,188 | 4,040 | 4,324 |
| Ranked context / same-scope bytes (100 requests) | 38,842 / 90,072 | 39,400 / 91,000 | 40,012 / 92,020 |
| Restore (s) | 0.146 | 1.634 | 17.352 |
| Recovery RSS (KiB) | 7,124 | 9,296 | 9,028 |

Same-run no-store CLI RSS 2,588 KiB; 100K incremental 1,736 KiB (~1.70 MiB) versus 8 MiB target. All three datasets: 300/300 fixture facts matched, 100/100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery verification errors and remote calls. Hosted-runner variability is not a controlled hardware performance comparison. Bytes are not tokens; RSS is not whole-system RAM; synthetic recall is not general answer quality. Actual power, token savings, physical speech and subscription usage not measured.

## Current working environments

- ChatGPT AtomOS Project: architecture, roadmap, decision and review workspace. GitHub is the shared durable source, not a chat transcript.
- Codex desktop coding session: select connected WSL Ubuntu; open existing Linux checkout (prefer `~/atomos-catalog-test` for current qualification). Owner confirmed x86_64 Ubuntu 24.04.4 LTS, WSL2, Node 24.22.3/npm 11.19.1. Rust/compiler/SQLite prerequisites not yet verified locally. Main checkout `~/atomos` may have the earlier local password prompt fix; inspect status/diff and do not discard changes.
- Physical reference platform: existing codex-ubuntu remote SSH machine, reserved for later hardware testing once owner returns home. Do not modify SSH aliases, DNS, addresses or keys for temporary network changes.
- GitHub Actions: CI and scale. No credentials or provider inference in CI. Human reviews and merges runtime changes.
- Existing owner vault: `$HOME/.config/atomos-auth` on WSL. Do not read/decrypt outside the existing owner-run utility, copy it, inspect secret-bearing files, print tokens/passphrase, or move it to another environment. Never commit local state. No new registration needed.

## Immediate objective: M2.4 transport qualification recovery

**Stop gate:** The original reservation is consumed. Owner reported state unknown, category probe_content_type, no reported usage, retained 1,024 planning units. Subsequent interactive refresh succeeded, active unexpired session and qualification_attempted:true. Exact response MIME and encoding were not recorded; root cause unknown. Do not run qualify-once again, delete the record, create another vault, or re-register to circumvent it. The diagnostic patch adds allowlisted HTTP status/media/encoding categories and bounded gzip handling; these changes are not proof of the actual root cause. A separate reviewed and explicitly owner-authorized qualification design is needed before any additional POST. The instructions below are historical preflight for the original attempt, not authority to repeat it.

Codex can perform read-only environment checks, build and synthetic tests. It must not initiate a provider inference, prompt for secrets in agent chat, refresh/revoke tokens, delete reservations or change provider settings. The owner performs interactive commands in their terminal. Live execution needs specific owner acknowledgement even if earlier project discussion approved the milestone.

1. Verify working directory, Git status, hostname/kernel and current commit; preserve existing user edits. Check `rustc --version`, `cargo --version`, `node --version`, `gcc --version`, `pkg-config --modversion sqlite3`. If dependencies missing, report and request owner approval for installation. Follow [official rustup](https://rustup.rs/) if needed.
2. Update clean qualification worktree, provision locked dependencies and build the existing decoder. Run offline tests:

   ```bash
   cd ~/atomos-catalog-test
   git fetch origin main
   git switch --detach origin/main
   cargo fetch --locked
   cargo build --release --locked --offline -p atomos-core --features responses-replay --example responses_qualify
   node --test scripts/auth/protocol.test.mjs scripts/auth/live-probe.test.mjs
   ```

   Expect 26/26 Node tests at the verified baseline; diagnostic patch adds 3 tests, pending CI. Report exact commit, versions, counts, failures. No inference during these commands.
3. Owner runs `node scripts/auth/atomos-auth.mjs status "$HOME/.config/atomos-auth"` in interactive terminal and supplies sanitized status only. Require active grant, unexpired access, no pending registration, `qualification_attempted:false`. If expired, stop and review refresh separately. Do not re-register.
4. Owner verifies saved app limit below 100% (last observed 50%) on eligible personal ChatGPT plan. Current documented route does not support `max_output_tokens`; a local response deadline or cap does not guarantee provider generation stops. Do not claim the 1,024-token reservation is a hard server ceiling or plan quota. Owner must accept this one-time exception explicitly. Follow [the exact owner-run qualification procedure](docs/m2-first-live-qualification.md); do not execute it unattended.
5. Single fixed nonpersonal prompt, `gpt-6.2-sol`, official Responses endpoint, `store:false`, `stream:true`, fresh same-account catalog, no AMG, tools, retries, auto refresh or paid fallback. Durable intent in `m2-4-probe.json` before POST, bound to account/host/request hash, 30s HTTP deadline, 256 KiB body limit, 3s Rust decoder subprocess. One attempt per vault; unknown retains reservation, validated actual settles. No raw stream or output logged. Do not delete/reset marker or repeat after failure, including crash/overrun.
6. Acceptance: owner reports sanitized JSON; valid terminal usage, exact expected text boolean, no unsupported stream/lifecycle events, durable settlement, check provider usage and any credit use if visible. First-body-byte time is not TTFT. Do not infer provider consumption from local raw token totals. Report discrepancies and stop rather than retry.

## Outstanding gates and next direction

- Live stream may contain reasoning items or event shapes unsupported by narrow Rust decoder. Investigate via safe sanitized categories and offline fixtures before separately approved future live calls.
- Live token revocation, integrated account-bound admission, trusted time/rolling budgets, real usage reconciliation, hard provider generation ceiling or explicit acceptable replacement policy, secure credential boundary and production security review remain open. Same-UID/root tampering and vault rollback are not solved.
- M2.5 provisional: integrate safe transport with broker/Token Governor and natural conversation, then authorized minimal AMG context only after privacy/spending gates. M2.6: benchmark actual quality, token use, latency and energy; model routing based on evidence rather than slug names.
- M3 prospective memory/scheduler, bounded autonomous tasks; M4 native bootable OS. Hardware voice wake/endpointing/AEC/barge-in, power/RSS and full speech tests pending. No assumption that WSL measures physical hardware efficiency.

## Maintenance protocol

Every new Codex session: read AGENTS.md and this file, inspect current commit and open PRs, verify evidence and report discrepancies before changing behavior. Maintain these files together with README and relevant technical docs when a milestone is merged or validated. Record exact commit and run URLs, test counts, measured results, remaining gates, next action and evidence provenance. Distinguish owner-reported physical/live results from synthetic CI. Never mark unrun tests as passed or an unqualified feature as complete. Do not include owner identity, secrets, network addresses, private content or raw provider responses in this public repository. Use a reviewed branch/PR for executable changes; do not merge on the owner's behalf.
