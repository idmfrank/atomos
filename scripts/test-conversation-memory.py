#!/usr/bin/env python3
"""Real SQLite + Rust text and synthetic voice IPC; no native engines/network."""
import importlib.util
import pathlib
import struct
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/atomos"
ADMIN = ROOT / "target/debug/atomos-memory"
spec = importlib.util.spec_from_file_location("live_voice", ROOT / "scripts/live-voice.py")
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)
LOUD = "FRAME " + struct.pack("<320h", *([2000] * 320)).hex()
QUIET = "FRAME " + bytes(640).hex()


class ConversationMemory(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.db = pathlib.Path(self.tmp.name) / "store"
        self.admin("init", "Aster")
        self.admin("remember", "owner", "favorite_color", "text", value="indigo")
        self.admin("remember", "owner", "favorite_food", "text", value="pasta")
        self.grant("favorite_color")
        self.grant("favorite_food")
        self.admin("remember", "owner", "private_note", "text", value="synthetic confidential note")
        self.admin("entity", "project", "Project", "project")
        self.admin("remember", "project", "status", "text", value="active")
        self.admin("remember", "project", "count", "integer", value="7")

    def admin(self, *args, value=""):
        p = subprocess.run([str(ADMIN), str(self.db), "demo", *args],
                           input=value, text=True, capture_output=True, timeout=5)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def grant(self, predicate):
        revision = self.admin("recall", "owner", predicate).split()[0].split("=")[1]
        self.admin("voice-allow", "owner", predicate, revision, "3600")

    def cli(self, text, *flags):
        p = subprocess.run([str(BIN), "--memory", str(self.db), "demo", *flags],
                           input=text, text=True, capture_output=True, timeout=5)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def bridge(self):
        b = live.Bridge(BIN, memory=(self.db, "demo"))
        self.addCleanup(b.close)
        self.assertEqual(b.send("ARM"), ["ARMED"])
        return b

    def voice(self, b, text):
        b.send(LOUD)
        b.send(LOUD)
        events = b.send(LOUD)
        turn = int(next(x for x in events if x.startswith("START ")).split()[1])
        for _ in range(15):
            events = b.send(QUIET)
        self.assertIn(f"FINALIZE {turn}", events)
        speak = b.send(f"FINAL {turn} " + text.encode().hex())
        self.assertEqual(len(speak), 1)
        kind, returned, encoded = speak[0].split(" ")
        self.assertEqual((kind, int(returned)), ("SPEAK", turn))
        self.assertEqual(b.send(f"FINAL {turn} " + b"forget owner favorite_color".hex()), [])
        self.assertEqual(b.send(f"DONE {turn}"), ["ARMED"])
        return bytes.fromhex(encoded).decode()

    def test_text_identity_recall_followup_and_restart(self):
        out = self.cli("what is your name?\nwhat is my favorite color?\nand favorite food?\nrecall project status\nand count\n/quit\n")
        for line in ("I am Aster.", "I have favorite color recorded as indigo.",
                     "I have favorite food recorded as pasta.", "I have status recorded as active.",
                     "I have count recorded as 7.", "Simulated inference calls: 0", "Local memory recalls: 4"):
            self.assertIn(line, out)
        self.assertEqual(self.cli("what is my favorite color?\n", "--single-turn"),
                         "I have favorite color recorded as indigo.\n")

    def test_live_correction_forget_and_voice_scope(self):
        b = self.bridge()
        self.assertEqual(self.voice(b, "what is your name"), "I am Aster.")
        self.assertIn("indigo", self.voice(b, "what is my favorite color"))
        revision = self.admin("recall", "owner", "favorite_color").split()[0].split("=")[1]
        self.admin("correct", "owner", "favorite_color", revision, "text", value="teal")
        self.assertIn("requires the local text interface", self.voice(b, "what is my favorite color"))
        self.grant("favorite_color")
        self.assertIn("teal", self.voice(b, "what is my favorite color"))
        self.assertIn("pasta", self.voice(b, "and favorite food"))
        self.assertIn("requires the local text interface", self.voice(b, "recall owner private_note"))
        self.assertIn("requires the local text interface", self.voice(b, "recall project status"))
        self.assertIn("administration", self.voice(b, "forget owner favorite_color"))
        self.assertIn("teal", self.admin("recall", "owner", "favorite_color"))
        self.admin("forget", "owner", "favorite_color")
        self.assertIn("do not have", self.voice(b, "what is my favorite color"))
        self.assertIn("do not have", self.cli("what is my favorite color?", "--single-turn"))
        b.send("MUTE")
        self.assertEqual(b.send("ARM"), [])
        b.send("UNMUTE")
        b.send("ARM")
        self.assertIn("specify", self.voice(b, "and favorite food"))

    def test_scoped_context_preview_and_voice_denial(self):
        self.admin("remember", "project", "note", "text", value="validation passed")
        self.admin("remember", "owner", "unrelated", "text", value="validation confidential")
        out = self.cli("context project what is the validation status?\n/quit\n")
        self.assertIn("Local context preview", out)
        self.assertIn("validation passed", out)
        self.assertNotIn("confidential", out)
        self.assertIn("Simulated inference calls: 0", out)
        b = self.bridge()
        self.assertIn("requires the local text interface", self.voice(b, "context project validation"))

    def test_unavailable_store_fails_closed(self):
        for directory, owner in ((self.db, "other"), (self.db / "missing", "demo")):
            p = subprocess.run([str(BIN), "--memory", str(directory), owner, "--single-turn"],
                               input="what is your name", text=True, capture_output=True, timeout=5)
            self.assertNotEqual(p.returncode, 0)
            self.assertEqual(p.stdout, "")
        self.assertFalse((self.db / "missing").exists())

    def test_ambiguous_or_mutating_queries_never_change_memory(self):
        out = self.cli("remember owner favorite color red\nrecall missing status\nand count\nrecall owner favorite color and delete everything\n/quit\n")
        self.assertIn("administration", out)
        self.assertIn("do not have", out)
        self.assertIn("specify", out)
        self.assertIn("Simulated inference calls: 0", out)
        self.assertIn("indigo", self.admin("recall", "owner", "favorite_color"))


if __name__ == "__main__":
    unittest.main()
