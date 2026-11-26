#!/usr/bin/env python3
"""Deterministic PCM fixtures and full local stdio protocol replay."""
import pathlib
import struct
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/atomos"
SILENCE = "FRAME " + bytes(640).hex()
SPEECH = "FRAME " + struct.pack("<320h", *([2000] * 320)).hex()


class Replay(unittest.TestCase):
    def setUp(self):
        self.p = subprocess.Popen(
            [str(BIN), "--voice-events", "--aec-verified"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
            encoding="ascii", cwd=ROOT,
        )

    def tearDown(self):
        if self.p.poll() is None:
            self.p.kill()
        self.p.communicate(timeout=5)

    def send(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        result = []
        for _ in range(10):
            reply = self.p.stdout.readline()
            self.assertTrue(reply, "voice process ended")
            reply = reply.rstrip("\n")
            if reply == ".":
                return result
            result.append(reply)
        self.fail("unbounded response")

    def onset(self):
        self.assertEqual(self.send(SPEECH), [])
        self.assertEqual(self.send(SPEECH), [])
        reply = self.send(SPEECH)
        start = next(x for x in reply if x.startswith("START "))
        return int(start.split()[1]), reply

    def final(self, turn):
        for _ in range(14):
            self.assertEqual(len(self.send(SILENCE)), 1)
        self.assertEqual(self.send(SILENCE)[-1], f"FINALIZE {turn}")
        text = "what is your name".encode().hex()
        self.assertEqual(self.send(f"FINAL {turn} {text}"),
                         [f"SPEAK {turn} " + b"I am AtomOS.".hex()])

    def test_multiturn_interrupt_and_mute(self):
        self.assertEqual(self.send(SPEECH), [])
        self.assertEqual(self.send("ARM"), ["ARMED"])
        turn, _ = self.onset()
        self.final(turn)
        next_turn, events = self.onset()
        self.assertEqual(events[:2], [f"CANCEL {turn}", f"START {next_turn}"])
        self.assertEqual(self.send(f"DONE {turn}"), [])
        self.final(next_turn)
        self.assertEqual(self.send("MUTE"), [f"CANCEL {next_turn}", "MUTED"])
        self.assertEqual(self.send(SPEECH), [])
        self.assertEqual(self.send("ARM"), [])
        self.assertEqual(self.send("UNMUTE"), ["IDLE"])
        self.assertEqual(self.send("QUIT"), ["IDLE"])
        self.assertEqual(self.p.wait(timeout=5), 0)

    def test_invalid_frame_fails_closed(self):
        self.p.stdin.write("FRAME 00\n")
        self.p.stdin.flush()
        self.assertNotEqual(self.p.wait(timeout=5), 0)


if __name__ == "__main__":
    unittest.main()
