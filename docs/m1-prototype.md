# M1 implementation slice 1: offline foundation

This is the first executable slice, not the completed voice prototype.

## Run on Linux

Install Rust/Cargo 1.75 or newer and Bash. GNU time is optional for benchmarks. The workspace has no third-party crates or network dependencies.

    bash scripts/test.sh
    cargo run --locked --offline -p atomos-cli --bin atomos
    bash scripts/benchmark.sh

Type "what is your name?" for local resolution, "explain entropy" for a deterministic mock response, and "/quit" to exit. Nothing is sent to OpenAI. The CLI keeps no conversation history yet. The simulated provider is deliberately not connected to the reservation ledger because it makes no remote calls; governor tests exercise the proposed accounting boundary independently.

## Implemented

- Workspace with core library and CLI; std-only, no unsafe code.
- Exact local intent resolution and deterministic mock provider.
- Turn manager with cancellation epochs, bounded transcript size, mute and stale-output rejection.
- Bounded PCM queue, explicit flush and provisional ASR/TTS/audio interfaces. No hardware drivers or speech models.
- Durable single-writer reservation journal with background ceiling, interactive reserve, per-request ceiling, conservative restart accounting and fail-closed corruption handling.
- Unit and integration tests, CLI smoke script and release footprint/process benchmark.
- CI configuration for offline build and tests. CI uses GitHub-hosted runner minutes subject to the repository owner's plan.

## Ledger limitations

The current journal is a fixed budget experiment: no calendar rollover, rolling windows, authentication, trusted provider usage records, cryptographic permits, or automatic stale-lock recovery. All reservations charge the budget until explicitly settled with a final actual count. A cancelled/ambiguous reservation stays charged. The library assumes a private, owner-controlled local directory and a single trusted writer; use a mutex for shared in-process access. A crash may leave writer.lock. Do not delete it until the owner has independently verified that no writer remains; preserve the usage log. Corrupt journals fail closed and require review, not automatic reset. The log is bounded to 4 MiB; rollover/compaction and hardened SQLite storage are future work. No live billing protection is asserted.

## Tests and next gate

The scripts exercise Rust state, local/mock resolution, queue bounds, stale speech suppression, mute, journal reopen, concurrent admission, quotas, duplicate settlement and corruption. The benchmark measures only text simulation and binary/RSS, not acoustic latency. Actual microphone/speaker cancellation must be independently measured at the audio device.

Next: wire one physical audio input/output backend, add local streaming ASR and TTS, replay consented or synthetic speech fixtures, record timing from speech end to audible response and barge-in to actual playback stop. Add durable identity/memory, then integrate the governor with a real broker only after official OpenAI eligibility and spending assurance are established. See [issue #1](https://github.com/idmfrank/atomos/issues/1).

## Baseline verification

PR #2 was merged and its GitHub Actions run passed all 15 tests, CLI smoke and release benchmark. Executable 296,824 bytes; peak RSS 2,068 KiB for 1,000 text-only interactions (no ASR/TTS or ledger in the benchmark). See [run 45744009330](https://github.com/idmfrank/atomos/actions/runs/45744009330). The next experimental slice is described in [local voice spike](local-voice-spike.md).
