#!/usr/bin/env python3
"""Heuristic public-text guard. Reports locations/categories, never matched text.

Not a secret scanner or a complete privacy classifier: manual review is required.
No deployment-specific denylist is embedded in this public file.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys

RULES = {
    "deployment-inventory": re.compile(
        r"\bowner[-\s]+(?:reported|reports?)\b[^\n]{0,120}\b\d+\s*(?:[KMGT]i?B|terabytes?|gigabytes?)\b"
        r"|\brecorded\s+hardware\s*:|\binstalled(?:\s+local/mock\s+CLI)?\s+release\s*[:=]",
        re.I,
    ),
    "processor-inventory": re.compile(r"\b(?:i[3579]-\d{4,5}[a-z]*|Ryzen\s+[3579]\s+\d{4,5}[a-z]*)\b", re.I),
    "private-address": re.compile(r"(?<![\w.])(?:10(?:\.\d{1,3}){3}|192\.168(?:\.\d{1,3}){2}|172\.(?:1[6-9]|2\d|3[01])(?:\.\d{1,3}){2})(?![\w.])"),
    "private-home-path": re.compile(r"/ho" r"me/(?!runner/|<)[^/\s<>]+/|[A-Z]:\\Users\\[^\\\s<>]+\\"),
    "target-login": re.compile(r"\bssh\s+(?:-[^\s]+\s+)*-l\s+(?!<)[a-z_][\w.-]*|\b[a-z_][\w.-]*@(?:[\w-]+\.)+[a-z]{2,}\b", re.I),
}


def findings(text):
    return [(number, kind) for number, line in enumerate(text.splitlines(), 1)
            for kind, rule in RULES.items() if rule.search(re.sub(r"\b[\w.-]+@(?:[\w-]+\.)*(?:invalid|example)\b", "<synthetic-address>", line))]


def self_test():
    cases = [
        "owner " + "reports " + "64 TB storage",
        "i7-" + "12345",
        ".".join(["192", "168", "1", "40"]),
        "/home/" + "private-fixture/file",
        "ssh -l " + "fixture target.example",
    ]
    assert all(findings(case) for case in cases)
    assert not findings("Use a configured store cap of 256 MiB; http://127.0.0.1:1455 is a protocol fixture.")
    assert not findings("fixture@example.invalid")


def main():
    self_test()
    root = Path(__file__).resolve().parents[1]
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=root
    ).decode("utf-8").split("\0")
    issues = []
    for name in names:
        path = root / name
        if not name or not path.is_file() or path.suffix not in {
            ".md", ".rs", ".py", ".mjs", ".sh", ".sql", ".toml", ".yml", ".yaml", ".json", ".sse", ".in", ".service"
        }:
            continue
        for line, kind in findings(path.read_text(encoding="utf-8")):
            issues.append((name, line, kind))
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if event_path:
        event = json.loads(Path(event_path).read_text(encoding="utf-8"))
        pr = event.get("pull_request", {})
        text = "\n".join([pr.get("title") or "", pr.get("body") or "",
                          (event.get("head_commit") or {}).get("message") or ""])
        issues.extend(("public-event-metadata", line, kind) for line, kind in findings(text))
    for name, line, kind in issues:
        print(f"PUBLICATION_CHECK {name}:{line} category={kind}")
    print("Publication pattern check failed; review privately." if issues else
          "Publication pattern check passed; manual metadata/log/artifact review remains required.")
    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
