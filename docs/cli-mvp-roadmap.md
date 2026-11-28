# AtomOS CLI MVP roadmap

Decision: owner-approved scope change, 2026-11-27. The first usable MVP is a single-owner text CLI on an existing Linux host, accessed locally or over the owner's existing SSH connection. This supersedes the voice-first MVP and its acoustic, audio-hardware and boot-image exit criteria. It changes development priorities, not implemented behavior or live-inference authorization.

[Current state](../PROJECT_STATE.md) | [README](../README.md) | [Architecture](architecture.md) | [Security](threat-model.md) | [Resources](resource-budget.md)

## Product boundary

- One owner, one Linux host, one active conversation and one foreground process.
- Plain terminal input/output. SSH supplies remote login and transport; AtomOS adds no web server, network listener, SSH server or authentication service.
- Optional existing `tmux` session for process continuity across SSH disconnects. This is an operator convenience, not implemented AtomOS session recovery, automatic startup or credential unlocking. No installation or SSH configuration changes are part of this decision.
- A thin CLI calls shared Rust conversation, memory and inference interfaces. Later browser/voice adapters should reuse those interfaces after a separate scope decision.
- Local commands and supported memory queries remain useful offline. A provider failure must leave the local interface usable.

## Existing capability versus MVP work

The CLI currently supports local/mock text interaction, persistent identity and opt-in read-only conversational AMG access. The separate `atomos-memory` administration executable owns explicit remember/correct/forget/history/export operations. Open-ended conversation still uses mock reasoning. M2.5a atomic admission and extended decoder profiles are isolated synthetic components; they are not connected to a production provider.

The MVP needs a coherent owner command contract, bounded in-session conversational context, cancellation/error/status handling and reviewed integration of these components. The [session contract](cli-session-contract.md) now specifies proposed commands and bounds; memory mutations remain in the separate administration executable. The commands exist only on the implementation branch; full validation/review and deployment remain pending. Do not document proposed commands as runnable.

Durable memory initially means approved identity/configuration and explicitly committed facts. Full transcript persistence, automatic extraction and replay of complete conversations are outside the MVP. Working conversation context may be volatile and must have explicit bounds. Restart must not resurrect deleted facts or imply that an interrupted remote request was refunded.

## Delivery sequence

Historical milestone numbers and evidence remain intact. M2.5a is complete as an offline synthetic slice; this roadmap does not rename completed work.

| Stage | Scope | Exit / authorization boundary |
| --- | --- | --- |
| M1 through M2.5a | Preserve completed text, AMG, admission, decoder and optional voice foundations | Historical tests and measured limits remain valid for their actual scope; voice hardware is unqualified |
| Linux host and local CLI deployment | [Minimal owner-local setup](linux-cli-deployment.md), verified main build, release hashes, generic identity store and running local/mock tmux session | Complete at PR #34 main; all 173 host tests and SSD scale/recovery passed; no provider or private-data deployment |
| M2.5b: complete offline, PR #36 merged | [Offline typed decoder outcome contract](m2-text-outcomes.md) with atomic admission, cancellation and settlement, focused on text CLI needs | Completed/refused/failed outcomes, known/unknown usage, durable suspension and strict-reference parity covered by independent fixtures; no transport or credentials |
| CLI integration: candidate implemented, validation pending | Implement the [session contract](cli-session-contract.md): opt-in terminal mode, bounded local-only working turns, read-only memory and visible mock/synthetic outcome/accounting status | Complete local owner journey works over terminal I/O; no automatic memory mutation or private provider disclosure |
| Provider integration gate and slice | Separately review secure credential handoff, account/model eligibility, trusted admission time, spending controls, output/unknown-usage exposure, cancellation/deadlines and reconciliation before enabling an adapter | Approved transport implementation may be tested offline; any new live test requires separate explicit owner authorization and a reviewed procedure |
| CLI MVP acceptance | Run the owner journey over existing SSH; qualify real text conversation only after the provider gate | Local persistence/failure tests and authorized provider results are recorded separately; until then label the interface local/mock |
| After MVP | Evaluate browser or voice access from actual use; separately consider reminders, scheduling and approved tools | New interface/execution surfaces require their own scope and security review |
| Long-term | Autonomous tasks, bootable OS, satellites, distributed operation and OS specialization | Deferred; no automatic promotion into the CLI MVP |

## CLI MVP acceptance criteria

1. Start manually on the existing Linux host through a local terminal or SSH, without a desktop, microphone, speaker or speech-model installation.
2. Conduct bounded text turns with clear local/mock/provider status. A successful real provider conversation is a later gated acceptance result, not a current claim.
3. Recall supported facts locally with zero inference. Explicitly remember a synthetic fact, restart, recall it, correct it, verify the new value, forget it and confirm it no longer appears in supported retrieval. Reuse the administration CLI until a unified command surface is deliberately implemented.
4. Preserve identity and explicitly committed facts across process restart. Do not promise persistence of volatile working turns, scheduled jobs or full transcripts.
5. Show meaningful local status for unavailable storage, denied admission, exhausted budget, provider failure, cancellation and unknown usage. Unsupported questions must not be presented as successfully answered.
6. Handle cancellation, EOF and session exit without unaccounted retry or duplicate dispatch. Define disconnect behavior explicitly; SSH disconnection is not proof of upstream cancellation. An optional multiplexer retains a running process but does not solve recovery.
7. Keep outstanding ambiguous usage charged and blocked according to admission policy after restart. Test actual process termination separately from clean close/reopen; stale crash locks require investigation, never automatic deletion.
8. Measure local response latency, context compilation, durable admission/settlement, startup, process RSS, storage and binary size. Report provider timing and usage separately when authorized.
9. Keep idle inference at zero, retain no paid fallback and require all remote work to pass through one account-bound admission path. No unattended retries or background inference in the MVP.
10. Demonstrate memory/credential separation and terminal/log redaction with synthetic data. Authenticated SSH access does not authorize sending stored facts to a provider; private AMG disclosure remains separately gated.

## Deferred work and retained assets

Voice development and physical qualification are paused. Preserve the PCM coordinator, adapters, fixtures, tests and historical benchmarks. Keep existing offline regression checks; do not delete code or relax assertions merely because voice is deferred. Wake word, VAD tuning, ASR/TTS quality, AEC, barge-in, far-field acoustics and audio power budgets apply only if voice is resumed.

A browser UI is a later option. HTTP hosting, browser sessions, web authentication and frontend work are not dependencies of this CLI. Automatic daemon startup, service management, multi-user sessions, reminders, tool execution, autonomous workflows, model routing, cross-device sync, custom images and new hardware purchases are also deferred.

## Safety and data boundaries

The existing trusted-local-directory model and same-UID/root/rollback limitations still apply. Terminal echo, scrollback, multiplexer buffers, shell history, redirected output and host logs can retain text. Do not print secrets or raw provider diagnostics, introduce transcript logging, or pass private fact values on command lines. Existing memory administration accepts values on stdin.

All three historical owner qualification attempts remain consumed, with 3,072 retained local planning units and unknown actual provider usage. Preserve the existing records; do not reset markers, register another vault/account, or reopen the historical harness. This roadmap authorizes documentation only, not live inference, credential access, private memory deployment, installs or host configuration changes.

M2.5b implementation/tests and full offline validation passed directly on the authorized host and in CI; PR #36 is merged and post-merge CI passed. The installed local/mock release remains unchanged, and no separate task handoff is needed.

## Immediate next step

The [M2.5b offline implementation contract](m2-text-outcomes.md) merged in PR #34. The [Linux development/local deployment host](linux-cli-deployment.md) is ready and running the existing local/mock CLI. It defines an explicit extended atomic replay entry point and Local/Completed/Refused/Failed outcomes, durable usage/suspension rules, cancellation and persistence-fault cases, frozen-reference parity and required test/benchmark evidence. Its runtime implementation and offline evidence are complete on PR #36. That slice is now merged at e3ebf24 with successful post-merge CI. The [CLI session contract](cli-session-contract.md) now has an implementation candidate requiring full validation and review; it defines the proposed command surface, local-only working history, typed status and precise synchronous termination limits. Provider access remains a separately gated slice.
