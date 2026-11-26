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

## Optional hardware adapter

Use an owner-installed, compatible Sherpa-ONNX Python package (the adapter uses OnlineRecognizer.from_transducer, get_result_all and GenerationConfig), NumPy and python-sounddevice/PortAudio. Record exact package versions, model hashes and licenses before tests. The candidate [streaming transducer API](https://github.com/k2-fsa/sherpa-onnx/blob/master/sherpa-onnx/python/sherpa_onnx/online_recognizer.py) and [Kokoro configuration](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/kokoro.html) are upstream references; packages and models are not bundled or installed automatically. Test on Linux x86-64. This bridge is a development adapter, not a permanent Python dependency for the OS.

    cargo build --release --locked --offline -p atomos-cli
    python3 scripts/live-voice.py --help
    python3 scripts/live-voice.py \
      --encoder /models/asr/encoder.onnx \
      --decoder /models/asr/decoder.onnx \
      --joiner /models/asr/joiner.onnx \
      --asr-tokens /models/asr/tokens.txt \
      --tts-model /models/kokoro/model.onnx \
      --tts-voices /models/kokoro/voices.bin \
      --tts-tokens /models/kokoro/tokens.txt \
      --tts-data-dir /models/kokoro/espeak-ng-data

Paths are examples; use a compatible model release's actual files. Some voices require --tts-lexicon. Set device IDs explicitly where necessary. The adapter loads both models once, creates a new ASR stream per turn, feeds frames during speech, and runs synthesis on a separate warm worker. Kokoro currently generates an entire bounded utterance before playback, not incremental synthesis. Press Enter to activate and Ctrl+C to cancel. The session ends after eight seconds of armed silence or the configured wall-clock limit. Microphone callbacks are bounded and overflows abort rather than committing partial audio. During playback the default half-duplex controller discards capture frames. Only pass --aec-verified after measuring actual near-end speech and playback echo; the flag is an operator assertion, not AEC implementation. Model synthesis may not be cancellable until the native call returns; cancelled audio is discarded, but actual device stop latency remains unmeasured. The adapter logs coarse monotonic stage timings, not physical end-to-end response or acoustic barge-in latency.

CI uses fake recognition/synthesis for the adapter boundary and actual Rust stdio IPC; it does not install speech dependencies or test native devices. The adapter itself must be verified with the specified engines and model releases on real hardware before claiming voice quality. No remote inference or spending is enabled.
