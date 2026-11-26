# M1.3: Persistent conversational coordinator

Status: proposed implementation, pending CI and real hardware evaluation. No remote inference.

## What this slice implements

A single long-lived Rust process accepts bounded 20 ms, 16 kHz mono PCM frames, gates speech on explicit activation, streams frame actions to an ASR worker, commits on 300 ms of quiet, and accepts only the matching final transcript. It opens an eight-second follow-up window after playback. Speech onset is three loud frames (60 ms of audio), with up to 100 ms volatile pre-roll. The VAD is a simple RMS threshold (500 PCM units), not a trained speech classifier. It can be fooled by noise and pauses. No wake-word detector yet.

When the operator explicitly verifies acoustic echo cancellation, the coordinator accepts near-end speech during playback and emits cancellation before opening a new turn. Without that configuration it defaults to half-duplex, avoiding speaker feedback triggering a new conversation. The adapter must actually stop hardware playback, discard stale output and clear its recognizer stream. The core does not implement AEC. Mute and stop invalidate active turn IDs; late finals and late playback completions are ignored. A device discontinuity should stop the session rather than commit truncated audio. Frame-count timeouts bound listening (30 seconds), finalization (5 seconds), speaking (30 seconds) and follow-up. These are sample-time limits: a stalled adapter needs a separate wall-clock watchdog.

## Local adapter protocol

Run atomos --voice-events (optional --aec-verified only after acoustic validation). One command line per request, response lines terminated with a single dot. Commands: ARM, FRAME <640-byte lowercase hex PCM>, FINAL <turn-id> <UTF-8 hex text>, DONE <turn-id>, STOP, MUTE, UNMUTE, QUIT. Responses: ARMED, START <id>, PCM <id> <hex>, FINALIZE <id>, SPEAK <id> <hex text>, CANCEL <id>, IDLE, MUTED. Trust boundary: local private stdio to an owner-operated worker; not a public network service. Protocol errors terminate the process. No credentials, model downloads or raw-audio logging. Text inference is the existing mock/local resolver; this is not an OpenAI adapter.

## Test

    bash scripts/test.sh
    python3 scripts/test-streaming-voice.py

The deterministic fixture tests use generated PCM amplitudes and mock transcripts, not recorded human audio. They validate the control path, including multi-turn reuse, endpointing, no partial inference, stale IDs, mute and cancellation, timeout and malformed input. They cannot establish acoustic accuracy, actual speaker cancellation or latency.

## Next gate

Use the optional local Sherpa-ONNX adapter to verify real streaming ASR and warm TTS, and measure performance on the selected host. Keep the default half-duplex path until the physical AEC reference is verified. Replace the energy gate with a tested speech VAD and local wake-word engine. Instrument true acoustic onset/offset and device playback before claiming barge-in latency. No automatic always-listening service or real inference yet.
