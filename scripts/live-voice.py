#!/usr/bin/env python3
"""Optional local Linux hardware adapter. Explicit, bounded session only.
No network calls or model downloads. Requires owner-installed local engines.
"""
import argparse
import os
import pathlib
import queue
import select
import subprocess
import sys
import threading
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
FRAME_SAMPLES = 320
FRAME_BYTES = FRAME_SAMPLES * 2
MAX_LINE = 17000
MAX_REPLY = 10 * MAX_LINE


class Bridge:
    """Private bounded stdio client for the Rust voice coordinator."""

    def __init__(self, binary, aec=False):
        args = [str(binary), "--voice-events"]
        if aec:
            args.append("--aec-verified")
        self.proc = subprocess.Popen(args, stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, bufsize=0)
        self.in_fd = self.proc.stdin.fileno()
        self.out_fd = self.proc.stdout.fileno()
        os.set_blocking(self.in_fd, False)
        self.pending = bytearray()

    def send(self, command):
        data = (command + "\n").encode("ascii")
        if len(data) > MAX_LINE:
            raise ValueError("oversized command")
        deadline = time.monotonic() + 3
        while data:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([], [self.in_fd], [], remaining)[1]:
                raise TimeoutError("Rust command write timeout")
            try:
                data = data[os.write(self.in_fd, data):]
            except BlockingIOError:
                continue
        result = []
        size = 0
        while True:
            split = self.pending.find(b"\n")
            if split >= 0:
                if split > MAX_LINE:
                    raise ValueError("oversized response line")
                line = bytes(self.pending[:split]).decode("ascii")
                del self.pending[:split + 1]
                if line == ".":
                    return result
                result.append(line)
                size += len(line) + 1
                if len(result) > 9 or size > MAX_REPLY:
                    raise ValueError("oversized response")
                continue
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([self.out_fd], [], [], remaining)[0]:
                raise TimeoutError("Rust response timeout")
            chunk = os.read(self.out_fd, 4096)
            if not chunk:
                raise RuntimeError("Rust voice coordinator exited")
            self.pending.extend(chunk)
            if len(self.pending) > MAX_REPLY:
                raise ValueError("oversized response")

    def close(self):
        # Do not leave a process holding captured audio if a worker failed.
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=2)
        self.proc.stdin.close()
        self.proc.stdout.close()


class Recognition:
    """Keep the model resident; each utterance gets a fresh decode stream."""

    def __init__(self, recognizer, np):
        self.recognizer = recognizer
        self.np = np
        self.stream = None
        self.turn = None

    def start(self, turn):
        self.stream = self.recognizer.create_stream()
        self.turn = turn

    def decode(self):
        # Bound unexpected ready-loop behavior.
        for _ in range(256):
            if not self.recognizer.is_ready(self.stream):
                return
            self.recognizer.decode_stream(self.stream)
        raise RuntimeError("ASR decode iteration limit")

    def feed(self, turn, data):
        if self.turn != turn or len(data) != FRAME_BYTES:
            raise ValueError("invalid recognition frame")
        samples = self.np.frombuffer(data, dtype="<i2").astype(self.np.float32) / 32768.0
        self.stream.accept_waveform(16000, samples)
        self.decode()

    def finish(self, turn):
        if self.turn != turn:
            raise ValueError("invalid recognition turn")
        self.stream.accept_waveform(16000, self.np.zeros(8000, dtype=self.np.float32))
        self.stream.input_finished()
        self.decode()
        text = self.recognizer.get_result_all(self.stream).text.strip()
        self.cancel(turn)
        if len(text.encode("utf-8")) > 8192:
            raise ValueError("transcript exceeds bound")
        return text

    def cancel(self, turn):
        if self.turn == turn:
            self.stream = None
            self.turn = None


class Playback:
    """Warm local synthesis worker; at most one queued job plus one active.
    Native generation is not necessarily interruptible; cancelled audio is discarded.
    """

    def __init__(self, tts, generation, sd, np, device=None):
        self.tts = tts
        self.generation = generation
        self.sd = sd
        self.np = np
        self.device = device
        self.jobs = queue.Queue(maxsize=1)
        self.completed = queue.Queue(maxsize=4)
        self.failed = threading.Event()
        self.shutdown = threading.Event()
        self.current = None
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()

    def submit(self, turn, text):
        self.cancel()
        flag = threading.Event()
        self.current = (turn, flag)
        self.jobs.put_nowait((turn, text, flag))

    def cancel(self, turn=None):
        if self.current is not None:
            active, flag = self.current
            if turn is None or active == turn:
                flag.set()
                self.current = None
        # A cancelled queued job can be discarded rather than delaying its successor.
        try:
            pending = self.jobs.get_nowait()
        except queue.Empty:
            return
        if turn is None or pending[0] == turn:
            pending[2].set()
        else:
            self.jobs.put_nowait(pending)

    def run(self):
        while not self.shutdown.is_set():
            try:
                turn, text, cancelled = self.jobs.get(timeout=0.1)
            except queue.Empty:
                continue
            try:
                if cancelled.is_set():
                    continue
                started = time.monotonic()
                audio = self.tts.generate(text, self.generation)
                if cancelled.is_set() or self.shutdown.is_set():
                    continue
                samples = self.np.asarray(audio.samples, dtype=self.np.float32)
                rate = int(audio.sample_rate)
                if rate < 8000 or rate > 96000 or not 0 < len(samples) <= rate * 30:
                    raise ValueError("invalid synthesized audio size")
                print(f"turn={turn} synthesis_ms={(time.monotonic()-started)*1000:.1f}",
                      file=sys.stderr)
                with self.sd.OutputStream(samplerate=rate, channels=1,
                                          dtype="float32", device=self.device,
                                          blocksize=max(1, rate // 50),
                                          latency="low") as output:
                    for offset in range(0, len(samples), max(1, rate // 50)):
                        if cancelled.is_set() or self.shutdown.is_set():
                            output.abort()
                            break
                        # Output timestamps are software enqueue times, not acoustic onset.
                        if offset == 0:
                            print(f"turn={turn} first_playback_enqueue_ms="
                                  f"{(time.monotonic()-started)*1000:.1f}", file=sys.stderr)
                        if output.write(samples[offset:offset + max(1, rate // 50)].reshape(-1, 1)):
                            raise RuntimeError("audio playback underflow")
                    if not cancelled.is_set() and not self.shutdown.is_set():
                        output.stop()
                if not cancelled.is_set() and not self.shutdown.is_set():
                    self.completed.put_nowait(turn)
            except Exception:
                # Do not print engine exception text: it may contain a transcript.
                self.failed.set()
                return

    def close(self):
        self.shutdown.set()
        self.cancel()
        self.thread.join(timeout=2)
        if self.thread.is_alive():
            print("Native audio worker did not exit promptly; process shutdown required.",
                  file=sys.stderr)


def apply(bridge, asr, playback, actions):
    """Execute bounded coordinator actions. Return False when capture must stop."""
    for action in actions:
        kind, *args = action.split(" ")
        if kind == "START":
            asr.start(int(args[0]))
        elif kind == "PCM":
            asr.feed(int(args[0]), bytes.fromhex(args[1]))
        elif kind == "FINALIZE":
            turn = int(args[0])
            started = time.monotonic()
            text = asr.finish(turn)
            print(f"turn={turn} finalize_ms={(time.monotonic()-started)*1000:.1f}",
                  file=sys.stderr)
            if not apply(bridge, asr, playback,
                         bridge.send(f"FINAL {turn} " + text.encode("utf-8").hex())):
                return False
        elif kind == "SPEAK":
            playback.submit(int(args[0]), bytes.fromhex(args[1]).decode("utf-8"))
        elif kind == "CANCEL":
            turn = int(args[0])
            playback.cancel(turn)
            asr.cancel(turn)
        elif kind in ("IDLE", "MUTED"):
            playback.cancel()
            return False
        elif kind != "ARMED":
            raise ValueError("unknown coordinator action")
    return True


def capture_session(bridge, asr, playback, sd, device, max_seconds):
    # Only start the microphone after explicit owner activation. Stop on any
    # lost frame, overload or device discontinuity; never commit truncated audio.
    frames = queue.Queue(maxsize=16)
    lost = threading.Event()

    def callback(indata, count, _time, status):
        if status or count != FRAME_SAMPLES:
            lost.set()
            return
        try:
            frames.put_nowait(bytes(indata))
        except queue.Full:
            lost.set()

    if not apply(bridge, asr, playback, bridge.send("ARM")):
        return
    deadline = time.monotonic() + max_seconds
    print("Local microphone active. Ctrl+C stops. No audio is saved.", file=sys.stderr)
    with sd.RawInputStream(samplerate=16000, blocksize=FRAME_SAMPLES,
                           channels=1, dtype="int16", device=device,
                           callback=callback):
        while time.monotonic() < deadline:
            if lost.is_set() or playback.failed.is_set():
                raise RuntimeError("audio discontinuity or synthesis failure")
            try:
                done = playback.completed.get_nowait()
                if not apply(bridge, asr, playback, bridge.send(f"DONE {done}")):
                    break
            except queue.Empty:
                pass
            try:
                frame = frames.get(timeout=0.5)
            except queue.Empty:
                raise RuntimeError("microphone stalled") from None
            if lost.is_set() or len(frame) != FRAME_BYTES:
                raise RuntimeError("invalid capture frame")
            if not apply(bridge, asr, playback, bridge.send("FRAME " + frame.hex())):
                break
    print("Local microphone inactive.", file=sys.stderr)


def existing_file(value):
    path = pathlib.Path(value).expanduser().resolve(strict=True)
    if not path.is_file():
        raise ValueError("model file required")
    return str(path)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary", default=str(ROOT / "target/release/atomos"))
    for arg in ("encoder", "decoder", "joiner", "asr-tokens",
                "tts-model", "tts-voices", "tts-tokens"):
        p.add_argument("--" + arg, required=True)
    p.add_argument("--tts-data-dir", required=True)
    p.add_argument("--tts-lexicon", default="")
    p.add_argument("--speaker-id", type=int, default=0)
    p.add_argument("--input-device", type=int)
    p.add_argument("--output-device", type=int)
    p.add_argument("--max-seconds", type=int, default=180)
    p.add_argument("--aec-verified", action="store_true",
                   help="only use after physical echo-cancellation verification")
    a = p.parse_args()
    if not 1 <= a.max_seconds <= 600:
        p.error("--max-seconds must be 1..600")

    # Optional dependencies: CI protocol tests do not import these.
    import numpy as np
    import sherpa_onnx
    import sounddevice as sd

    recognizer = sherpa_onnx.OnlineRecognizer.from_transducer(
        tokens=existing_file(a.asr_tokens), encoder=existing_file(a.encoder),
        decoder=existing_file(a.decoder), joiner=existing_file(a.joiner),
        num_threads=2, sample_rate=16000, feature_dim=80,
        decoding_method="greedy_search", enable_endpoint_detection=False,
        provider="cpu",
    )
    data_dir = pathlib.Path(a.tts_data_dir).expanduser().resolve(strict=True)
    if not data_dir.is_dir():
        raise ValueError("local TTS data directory required")
    kokoro = sherpa_onnx.OfflineTtsKokoroModelConfig(
        model=existing_file(a.tts_model), voices=existing_file(a.tts_voices),
        tokens=existing_file(a.tts_tokens), data_dir=str(data_dir),
        lexicon=existing_file(a.tts_lexicon) if a.tts_lexicon else "",
    )
    config = sherpa_onnx.OfflineTtsConfig(
        model=sherpa_onnx.OfflineTtsModelConfig(
            kokoro=kokoro, num_threads=2, provider="cpu", debug=False),
        max_num_sentences=1,
    )
    if not config.validate():
        raise ValueError("TTS configuration rejected")
    tts = sherpa_onnx.OfflineTts(config)
    generation = sherpa_onnx.GenerationConfig()
    generation.sid = a.speaker_id
    generation.speed = 1.0

    bridge = Bridge(pathlib.Path(a.binary).resolve(strict=True), a.aec_verified)
    playback = Playback(tts, generation, sd, np, a.output_device)
    asr = Recognition(recognizer, np)
    try:
        input("Press Enter to activate the local microphone (Ctrl+C cancels): ")
        capture_session(bridge, asr, playback, sd, a.input_device, a.max_seconds)
    except KeyboardInterrupt:
        print("Session cancelled.", file=sys.stderr)
    finally:
        playback.cancel()
        asr.cancel(asr.turn)
        try:
            bridge.send("STOP")
        except (OSError, RuntimeError, ValueError, TimeoutError):
            pass
        bridge.close()
        playback.close()


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        # Report exception type only: native engine errors may contain speech.
        print(f"Local voice adapter stopped: {type(exc).__name__}", file=sys.stderr)
        sys.exit(1)
