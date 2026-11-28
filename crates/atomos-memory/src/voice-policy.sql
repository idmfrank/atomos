-- Legacy on-disk compatibility only; no callable voice interface remains.
-- Schema v2: explicit, expiring disclosure grant for one current revision.
-- The owner-local OS account remains the trust boundary.
CREATE TABLE voice_grant (
    assertion_id INTEGER PRIMARY KEY REFERENCES assertion(id) ON DELETE CASCADE,
    expires_at INTEGER NOT NULL CHECK(expires_at>0)
) STRICT;
CREATE TRIGGER voice_grant_retire AFTER UPDATE OF superseded_at ON assertion
WHEN NEW.superseded_at IS NOT NULL
BEGIN
    DELETE FROM voice_grant WHERE assertion_id=NEW.id;
END;
