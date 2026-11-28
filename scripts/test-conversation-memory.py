#!/usr/bin/env python3
"""Real SQLite and Rust text integration; no network."""
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/atomos"
ADMIN = ROOT / "target/debug/atomos-memory"
class ConversationMemory(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.db = pathlib.Path(self.tmp.name) / "store"
        self.admin("init", "Aster")
        self.admin("remember", "owner", "favorite_color", "text", value="indigo")
        self.admin("remember", "owner", "favorite_food", "text", value="pasta")
        self.admin("remember", "owner", "private_note", "text", value="synthetic confidential note")
        self.admin("entity", "project", "Project", "project")
        self.admin("remember", "project", "status", "text", value="active")
        self.admin("remember", "project", "count", "integer", value="7")

    def admin(self, *args, value=""):
        p = subprocess.run([str(ADMIN), str(self.db), "demo", *args],
                           input=value, text=True, capture_output=True, timeout=5)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def cli(self, text, *flags):
        p = subprocess.run([str(BIN), "--memory", str(self.db), "demo", *flags],
                           input=text, text=True, capture_output=True, timeout=5)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def test_text_identity_recall_followup_and_restart(self):
        out = self.cli("what is your name?\nwhat is my favorite color?\nand favorite food?\nrecall project status\nand count\n/quit\n")
        for line in ("I am Aster.", "I have favorite color recorded as indigo.",
                     "I have favorite food recorded as pasta.", "I have status recorded as active.",
                     "I have count recorded as 7.", "Simulated inference calls: 0", "Local memory recalls: 4"):
            self.assertIn(line, out)
        self.assertEqual(self.cli("what is my favorite color?\n", "--single-turn"),
                         "I have favorite color recorded as indigo.\n")

    def test_live_text_correction_and_forget(self):
        revision = self.admin("recall", "owner", "favorite_color").split()[0].split("=")[1]
        self.admin("correct", "owner", "favorite_color", revision, "text", value="teal")
        self.assertEqual(self.cli("what is my favorite color?", "--single-turn"),
                         "I have favorite color recorded as teal.\n")
        self.admin("forget", "owner", "favorite_color")
        self.assertIn("do not have", self.cli("what is my favorite color?", "--single-turn"))

    def test_scoped_context_preview(self):
        self.admin("remember", "project", "note", "text", value="validation passed")
        self.admin("remember", "owner", "unrelated", "text", value="validation confidential")
        out = self.cli("context project what is the validation status?\n/quit\n")
        self.assertIn("Local context preview", out)
        self.assertIn("validation passed", out)
        self.assertNotIn("confidential", out)
        self.assertIn("Simulated inference calls: 0", out)

    def test_removed_voice_modes_are_rejected(self):
        for flags in (("--voice-events",), ("--voice-events", "--aec-verified")):
            p = subprocess.run([str(BIN), *flags], input="ARM\n", text=True,
                               capture_output=True, timeout=5)
            self.assertNotEqual(p.returncode, 0)
            self.assertEqual(p.stdout, "")

    def test_removed_grant_commands_cannot_modify_memory(self):
        before = self.admin("recall", "owner", "favorite_color")
        for args in (("voice-allow", "owner", "favorite_color", "1", "3600"),
                     ("voice-revoke", "owner", "favorite_color")):
            p = subprocess.run([str(ADMIN), str(self.db), "demo", *args],
                               text=True, capture_output=True, timeout=5)
            self.assertNotEqual(p.returncode, 0)
            self.assertEqual(p.stdout, "")
            self.assertEqual(self.admin("recall", "owner", "favorite_color"), before)

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
