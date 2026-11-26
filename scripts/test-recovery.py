#!/usr/bin/env python3
"""Synthetic abrupt writer termination against real SQLite rollback journal."""
import pathlib
import subprocess
import tempfile
import unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
ADMIN = ROOT / "target/debug/atomos-memory"

class Recovery(unittest.TestCase):
    def test_killed_writer_rolls_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = pathlib.Path(tmp) / "store"
            def admin(*args, value=""):
                return subprocess.run([str(ADMIN), str(store), "demo", *args],
                    input=value, capture_output=True, text=True, timeout=5, check=True).stdout
            admin("init", "Aster")
            before = admin("remember", "owner", "note", "text", value="committed")
            # Force rollback journal activity with a write transaction. Kill
            # without closing or committing the connection.
            child = subprocess.run(["python3", "-c",
                "import sqlite3,os,sys; c=sqlite3.connect(sys.argv[1]); "
                "c.execute('PRAGMA cache_size=1'); c.execute('BEGIN IMMEDIATE'); "
                "c.execute('UPDATE assertion SET text_value=?',('uncommitted',)); os._exit(23)",
                str(store / "memory.db")], timeout=5)
            self.assertEqual(child.returncode, 23)
            self.assertEqual(admin("recall", "owner", "note"), before)
            self.assertEqual(admin("verify"), "integrity ok\n")

if __name__ == "__main__":
    unittest.main()
