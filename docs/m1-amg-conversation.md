# M1.4b — Read-only conversational AMG integration

> **Milestone context (reviewed through PR #28, 2026-11-26):** The original read-only conversation interface is retained. M1.4d adds explicit revision-specific expiring demo voice grants; M1.4e–f add deletion and retrieval improvements. Use the [current AMG architecture](amg-architecture.md) and [recovery/disclosure](m1-amg-recovery.md) for the newer boundary; voice is not authentication. Physical speech remains unqualified. [Current project state](../PROJECT_STATE.md) is authoritative; status and next-step statements below describe the original milestone unless explicitly updated.


Status: merged in PR #6; locked offline post-merge CI passed. Physical audio and active-store resource validation pending. M1.4a was merged and validated in [run 45754844830](https://github.com/idmfrank/atomos/actions/runs/45754844830) (35 Rust, 5 Python tests plus smoke checks).

The Rust CLI and voice event coordinator now accept an explicit existing AMG store through --memory DIR OWNER before the mode flag. Without the flag, prior local/mock behavior is unchanged. A separate SQLite read-only connection and MemoryReader trait expose identity and exact lookup only. No implicit creation, migration, fact mutation, SQL execution or remote inference. Wrong scope, missing store and unsupported schema fail closed at startup. Busy timeout is 100 ms; this is a lock wait limit, not a full query execution deadline. Indexed reads happen after a valid final turn, never in the microphone callback.

Supported local grammar (case-insensitive English, optional final question mark):
- what is your name / what's your name: use stored display name
- what is my favorite color: exact owner property (spaces become underscores)
- recall project status: exact entity/property
- and count: use previous successfully recalled subject; read again, never cache value
- remember/forget/correct/delete/learn commands require explicit local administration

Unknown open questions retain mock behavior. Missing facts and storage failures return local status instead of inventing knowledge or sending data to a provider. Context holds only a subject key; clear on unrelated/failed turns, mute, cancellation, idle/session expiry and restart. No persistent transcript or automatic learning. A valid final transcript is required before any memory read. Late finals cannot trigger retrieval. Prior spoken responses cannot be recalled or retroactively erased.

Voice is not authentication. Opt-in voice memory is limited to synthetic demo owner favorite_color, favorite_food and favorite_music text values plus agent display name; arbitrary entity/property and history access are denied. This is a narrow demo allowlist, not a sensitivity classifier: never store real private data in a demo field. Any person in earshot can hear speech. Text mode assumes a trusted local operator; owner scope is not authentication. Real private voice reads/writes require authenticated authorization and sensitivity controls. Personality remains an unimplemented configuration feature; name is loaded from identity, stored data is never treated as instructions.

```bash
cargo fetch --locked
cargo build --workspace --locked --offline
umask 077
tmp="$(mktemp -d)"
target/debug/atomos-memory "$tmp/store" demo init Aster
printf %s indigo | target/debug/atomos-memory "$tmp/store" demo remember owner favorite_color text
printf '%s\n' 'what is your name?' 'what is my favorite color?' '/quit' | target/debug/atomos --memory "$tmp/store" demo
rm -rf -- "$tmp"
```

The optional live-voice.py adapter accepts --memory DIR OWNER in addition to its existing model/device flags. No actual native speech or acoustic evaluation is implied. Synthetic fixture tests exercise actual Rust IPC and SQLite, persistent name, exact/follow-up lookup, concurrent administrator correction and deletion, stale finals, mutation refusal, voice restrictions, wrong scope and absent store.

The CLI now links atomos-memory and system SQLite; measure size/RSS separately from the prior dependency-free core. The default text benchmark has no active store, so it does not measure AMG working-set cost. M1.4c will expand retrieval/context compilation, M1.4d recovery/security/export, M1.4e engine/resource comparison. Full authorization, physical audio validation and OpenAI eligibility remain gates.

## Verified post-merge results

[Commit 7e523bd](https://github.com/idmfrank/atomos/commit/7e523bdc1c139f3555fce39eda887b0655676d33), [Actions run 45755478646](https://github.com/idmfrank/atomos/actions/runs/45755478646): 35 Rust and 9 Python tests passed, including the four new tests for identity/follow-up/restart, live correction/deletion and voice restrictions, unavailable stores, and mutation refusal. CLI, stub audio and AMG smoke checks passed. No remote inference. The release CLI measured 366,440 bytes and 2,644 KiB peak RSS for 1,000 local text interactions without an active store (prior M1.4a: 314,224 bytes and 2,040 KiB). Not a benchmark of SQLite working-set, real ASR/TTS, system power or acoustic latency. Rust/Cargo 1.100.0 tested; minimum-version compatibility remains unverified.
