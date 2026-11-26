# M1.4d — Memory disclosure and logical recovery

Status: merged in PR #8; locked offline post-merge CI passed. Synthetic data only.

Schema v2 adds explicit revision-specific voice grants expiring after 1–86,400
seconds (wall UTC, no clock tampering defense). Voice still requires opt-in
session and permits only owner favorite_color/favorite_food/favorite_music text
values; name uses existing launch consent. Grant checked in same read snapshot
as the fact. No grant for other predicates, historical or non-text values.
Correction retires the grant, deletion cascades, explicit revocation is
immediate for subsequent reads; in-flight/spoken output cannot be retracted.
No remote-disclosure capability exists; all provider disclosure remains denied.
Local text assumes trusted OS operator, not authenticated human identity.
No broad voice context. This is not a full sensitivity taxonomy or authentication.

Existing v1 stores require an explicit, transactional admin migration. Stop
all runtime sessions and back up the private store offline first. Never
auto-migrate or overwrite. Old voice allowlist no longer grants access.

Commands (atomos-memory DIR OWNER ...):
- migrate
- verify
- voice-allow owner favorite_color REVISION TTL_SECONDS
- voice-revoke owner favorite_color
- export PATH
- restore PATH --accept-snapshot (DIR must not exist)

Logical export is a versioned UTF-8 tab-delimited format with fixed table
tags I/E/S/A/Q, typed decimal integers, hex UTF-8 text, explicit null markers,
and terminal row count. 8 MiB / 20,000 rows / 4096 bytes per line maximum;
oversized exports fail, never silently truncate. Captures identity, entities,
source provenance, all surviving revisions and AUTOINCREMENT high water
marks in one transaction; no derived index or voice grant. No raw SQL from
export is executed. Restore validates format/scope/keys/types, inserts through
fixed statements in one new private store transaction, rebuilds FTS and checks
integrity before publishing schema version. Restore does not overwrite or
merge. Invalid data rolls back; failed initialization may leave an unusable
new directory, which the operator must inspect/remove. Existing store remains
untouched. Export files are new 0600 files in an existing private directory;
sync file and directory. Plaintext, not cryptographically authenticated or
encrypted; protect backups, use only trusted local exports. Row count detects
truncation, not malicious modification. No protection from hostile same-UID
processes. Verify runs SQLite integrity/foreign-key, revision/provenance,
FTS content/index and voice grant checks.

Older snapshots can resurrect deleted memories if explicitly restored; the
--accept-snapshot flag acknowledges restoration of that point in time, not
proof of deletion reconciliation. Do not restore old exports into production;
review deletion/retention before activation and retire obsolete backups.
Restored grants are always absent. Identity is preserved for disaster recovery;
do not run the restored and original stores as independent agents.
No real private data until authenticated controls and encryption/key recovery.

Existing 128 MiB main DB page cap stays in place; 8 MiB logical export cap is
a separate development bound. Aggregate storage quota, retention/automatic
purging, authenticated owner policy, durable deletion reconciliation,
cryptographic backup protection, large streaming export, filesystem-full and
power-failure testing remain open. Tests exercise logical roundtrip, high
water marks, grant expiration/revocation, correction, deletion, v1 migration,
bad backup, inconsistent FTS and abrupt uncommitted SQLite writer exit.
The crash test is process-exit simulation, not power-loss certification.

[Post-merge CI 45798237897](https://github.com/idmfrank/atomos/actions/runs/45798237897): 44 Rust and 11 Python tests passed, including all four recovery tests and abrupt writer rollback. No-store release CLI 464,912 bytes, peak RSS 2,608 KiB. No actual power-loss or private-data certification.
