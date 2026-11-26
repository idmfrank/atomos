#!/usr/bin/env python3
"""Exercise real Rust protocol from the optional adapter without native models."""
import importlib.util
import pathlib
import struct
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("live_voice", ROOT / "scripts/live-voice.py")
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)


class FakeAsr:
    def __init__(self):
        self.turn = None
        self.count = 0

    def start(self, turn):
        self.turn = turn

    def feed(self, turn, data):
        assert turn == self.turn and len(data) == 640
        self.count += 1

    def finish(self, turn):
        assert turn == self.turn
        self.turn = None
        return "what is your name"

    def cancel(self, turn):
        if self.turn == turn:
            self.turn = None


class FakePlayback:
    def __init__(self):
        self.current = None
        self.cancelled = []

    def submit(self, turn, text):
        self.current = (turn, text)

    def cancel(self, turn=None):
        if self.current and (turn is None or self.current[0] == turn):
            self.cancelled.append(self.current[0])
            self.current = None


class AdapterTest(unittest.TestCase):
    def test_streaming_model_boundary(self):
        bridge = live.Bridge(ROOT / "target/debug/atomos", True)
        asr, playback = FakeAsr(), FakePlayback()
        loud = "FRAME " + struct.pack("<320h", *([2000] * 320)).hex()
        quiet = "FRAME " + bytes(640).hex()
        try:
            def send(command):
                return live.apply(bridge, asr, playback, bridge.send(command))

            self.assertTrue(send("ARM"))
            for _ in range(3):
                self.assertTrue(send(loud))
            self.assertEqual(asr.count, 3)
            for _ in range(15):
                self.assertTrue(send(quiet))
            self.assertEqual(asr.count, 18)
            self.assertEqual(playback.current, (1, "I am AtomOS."))
            for _ in range(3):
                self.assertTrue(send(loud))
            self.assertEqual(playback.cancelled, [1])
            self.assertEqual(asr.turn, 2)
            self.assertFalse(send("MUTE"))
            self.assertIsNone(asr.turn)
            self.assertIsNone(playback.current)
        finally:
            bridge.close()


if __name__ == "__main__":
    unittest.main()
