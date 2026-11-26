#!/usr/bin/env python3
"""Deterministic PCM fixtures and full local stdio protocol replay."""
import importlib.util
import pathlib
import struct
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/atomos"
SILENCE = "FRAME " + bytes(640).hex()
SPEECH = "FRAME " + struct.pack("<320h", *([2000] * 320)).hex()
spec = importlib.util.spec_from_file_location("live_voice", ROOT / "scripts/live-voice.py")
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)


class Replay(unittest.TestCase):
    def setUp(self):
        self.bridge = live.Bridge(BIN, True)
        self.addCleanup(self.bridge.close)

    def send(self, line):
        return self.bridge.send(line)

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
        self.assertEqual(self.bridge.proc.wait(timeout=5), 0)

    def test_invalid_frame_fails_closed(self):
        result = subprocess.run(
            [str(BIN), "--voice-events"], input="FRAME 00\n",
            capture_output=True, text=True, timeout=5, cwd=ROOT,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Invalid voice event", result.stderr)


class StartupAndDeadline(unittest.TestCase):
    def test_startup_dispatches_without_relocking_stdin(self):
        # Independent of the adapter. A deadlocked process is killed by run().
        for options in ([], ["--aec-verified"]):
            with self.subTest(options=options):
                result = subprocess.run(
                    [str(BIN), "--voice-events", *options],
                    input="ARM\nQUIT\n", capture_output=True, text=True,
                    timeout=5, cwd=ROOT,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "ARMED\n.\nIDLE\n.\n")

    def test_unresponsive_child_is_killed_and_connection_invalidated(self):
        with tempfile.TemporaryDirectory() as tmp:
            child = pathlib.Path(tmp) / "unresponsive"
            child.write_text("#!/usr/bin/env python3\nimport time\ntime.sleep(60)\n")
            child.chmod(0o700)
            bridge = live.Bridge(child, request_timeout=0.25)
            try:
                with self.assertRaises(TimeoutError):
                    bridge.send("ARM")
                self.assertIsNotNone(bridge.proc.poll())
                self.assertTrue(bridge.proc.stdin.closed)
                self.assertTrue(bridge.proc.stdout.closed)
                with self.assertRaises(RuntimeError):
                    bridge.send("ARM")
            finally:
                bridge.close()


if __name__ == "__main__":
    unittest.main()
