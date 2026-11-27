# M1 slice 2: local audio path

> **Milestone context (reviewed through PR #28, 2026-11-26):** This documents the original batch audio smoke-test slice; [persistent conversation](persistent-conversation.md) implements the subsequent coordinator and optional warm adapter. Physical microphone/speaker, native model quality, wake word and AEC remain unqualified. Stub/CI success is not hardware evidence. [Current project state](../PROJECT_STATE.md) is authoritative; status and next-step statements below describe the original milestone unless explicitly updated.


Status: merged, automated stub integration tests passed; physical validation pending. This is a deliberately bounded hardware smoke test, not the final conversational engine.

## Purpose

Exercise a real microphone -> local ASR -> Rust agent -> local TTS -> speaker path with a repeatable fixture mode. All reasoning remains mock/local. Use installed executables and models; no runtime model downloads, API keys or OpenAI calls. The helper is Linux-specific and uses Bash only for early audio orchestration; the long-lived runtime remains Rust.

## Prerequisites

- Linux x86-64 host, Rust/Cargo, Bash, GNU coreutils (timeout, realpath, stat), and ALSA utilities (arecord, aplay).
- A locally installed whisper.cpp whisper-cli and compatible English ggml model; an installed Piper CLI and ONNX voice plus matching .onnx.json. Consult [Whisper CLI](https://github.com/ggml-org/whisper.cpp/tree/master/examples/cli) and [Piper CLI](https://github.com/OHF-Voice/piper1-gpl/blob/main/docs/CLI.md). Pin and record actual versions/checksums before benchmark comparisons. Review Piper's GPL licensing and individual model licenses before distribution; no engines or weights are bundled here.
- Audio input and output devices, preferably with hardware DSP/AEC. Use arecord -L and aplay -L to enumerate ALSA devices. Do not assume echo cancellation is active.
- /dev/shm mounted as tmpfs. tmpfs can swap; disable or encrypt swap for privacy-sensitive tests. Run as an unprivileged user, never root. Use trusted offline binaries and local model files only.

## Run

    bash scripts/test.sh
    bash scripts/test-voice.sh
    cargo build --release --locked --offline -p atomos-cli
    export WHISPER_MODEL=/absolute/path/to/ggml-base.en.bin
    export PIPER_MODEL=/absolute/path/to/voice.onnx
    # Optionally set WHISPER_BIN, PIPER_BIN, CAPTURE_DEVICE, PLAYBACK_DEVICE.
    bash scripts/voice-smoke.sh --record

Say "What is your name?" during the default five-second capture; the expected spoken answer is "I am AtomOS." RECORD_SECONDS allows 1–10 seconds. Press Ctrl+C to cancel capture, recognition, synthesis or playback. No automatic listening follows the turn.

For a consented or synthetic local WAV (16 kHz mono PCM recommended):

    bash scripts/voice-smoke.sh --fixture /path/to/test.wav --no-playback

The fixture is not deleted; scratch audio and transcript files are held in a private tmpfs directory and removed on normal exit and handled signals. A hard kill or machine crash can leave scratch files; inspect/remove stale private files after verifying no session uses them. Do not retain or upload recordings by default. Child output and audio files have size limits; stages have bounded timeouts. Raw text is not printed by the harness. The helper trusts installed binaries; it is not a network sandbox.

## Tests

The CI stub tests use fake executables to verify the actual argument/data flow, successful fixture processing, playback gating, empty/oversized/control-character transcript rejection, ASR/TTS failures and missing fixture handling. They do not test audio quality or real engine compatibility. The Rust CLI rejects oversized input before trimming and adds a single-turn interface. Token ledger reservations are bound to an open instance to prevent cross-ledger or reopened-instance settlement; outstanding reservations still remain charged. Persistent authoritative reconciliation remains future work.

## Physical acceptance checklist

- Record hardware SKU, OS, audio devices, exact engine and model versions.
- Verify local capture, intelligible voice playback, correct recognition and local name response.
- Test normal exit and Ctrl+C during playback; confirm actual speaker stops, not only process termination.
- Test capture/speaker disconnect and invalid model path; verify clean failure.
- Confirm no extra remote calls and no persistent raw audio from the harness.
- Record coarse per-stage times first; use monotonic instrumentation and acoustic loopback for precise end-of-speech and barge-in measurements.

## Limitations and next gate

Fixed-duration, batch ASR, cold-start TTS and manual cancellation are intentional limitations of this slice. It does NOT meet the streaming, full-duplex, wake-word, VAD or natural barge-in requirements. Coarse elapsed stage timing is not an end-to-end conversational latency benchmark. Actual mic/speaker and model tests remain pending. Next implement a persistent native audio worker, streaming ASR (evaluate sherpa-onnx), warm neural TTS, endpointing and real near-end-speech interruption with verified AEC. Compare Piper against a more natural voice such as Kokoro. Do not move to live OpenAI inference until the local path and subscription-only controls are validated.
