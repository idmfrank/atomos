-- AMG v1: a single owner per store; recorded-time revisions, not inferred valid time.
CREATE TABLE identity (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    agent_id TEXT NOT NULL UNIQUE,
    owner_scope TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;
CREATE TABLE entity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_scope TEXT NOT NULL REFERENCES identity(owner_scope),
    entity_key TEXT NOT NULL,
    label TEXT NOT NULL,
    kind TEXT NOT NULL,
    UNIQUE(owner_scope, entity_key),
    UNIQUE(owner_scope, id)
) STRICT;
CREATE TABLE source (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_scope TEXT NOT NULL REFERENCES identity(owner_scope),
    kind TEXT NOT NULL CHECK(kind='owner_local'),
    recorded_at INTEGER NOT NULL,
    UNIQUE(owner_scope,id)
) STRICT;
CREATE TABLE assertion (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_scope TEXT NOT NULL,
    subject_id INTEGER NOT NULL,
    predicate TEXT NOT NULL,
    value_kind TEXT NOT NULL CHECK(value_kind IN ('text','integer','boolean','entity')),
    text_value TEXT,
    int_value INTEGER,
    target_id INTEGER,
    source_id INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    superseded_at INTEGER,
    supersedes_id INTEGER REFERENCES assertion(id) ON DELETE SET NULL,
    FOREIGN KEY(owner_scope,subject_id) REFERENCES entity(owner_scope,id),
    FOREIGN KEY(owner_scope,target_id) REFERENCES entity(owner_scope,id),
    FOREIGN KEY(owner_scope,source_id) REFERENCES source(owner_scope,id),
    CHECK(
      (value_kind='text' AND text_value IS NOT NULL AND int_value IS NULL AND target_id IS NULL) OR
      (value_kind='integer' AND int_value IS NOT NULL AND text_value IS NULL AND target_id IS NULL) OR
      (value_kind='boolean' AND int_value IS NOT NULL AND int_value IN (0,1) AND text_value IS NULL AND target_id IS NULL) OR
      (value_kind='entity' AND target_id IS NOT NULL AND text_value IS NULL AND int_value IS NULL)
    )
) STRICT;
-- Initial API treats a predicate as single-valued per subject.
CREATE UNIQUE INDEX current_property ON assertion(owner_scope,subject_id,predicate)
    WHERE superseded_at IS NULL;
CREATE INDEX assertion_history ON assertion(owner_scope,subject_id,predicate,id);
CREATE INDEX outgoing_edge ON assertion(owner_scope,subject_id,id)
    WHERE superseded_at IS NULL AND target_id IS NOT NULL;
CREATE INDEX incoming_edge ON assertion(owner_scope,target_id,id)
    WHERE superseded_at IS NULL AND target_id IS NOT NULL;
CREATE VIRTUAL TABLE fact_search USING fts5(predicate, body, tokenize='unicode61');
CREATE TRIGGER assertion_insert AFTER INSERT ON assertion WHEN NEW.superseded_at IS NULL
BEGIN
    INSERT INTO fact_search(rowid,predicate,body)
    VALUES(NEW.id, NEW.predicate,
        COALESCE(NEW.text_value,CAST(NEW.int_value AS TEXT),
          (SELECT entity_key FROM entity WHERE id=NEW.target_id)));
END;
CREATE TRIGGER assertion_retire AFTER UPDATE OF superseded_at ON assertion
WHEN OLD.superseded_at IS NULL AND NEW.superseded_at IS NOT NULL
BEGIN
    DELETE FROM fact_search WHERE rowid=OLD.id;
END;
CREATE TRIGGER assertion_delete AFTER DELETE ON assertion
BEGIN
    DELETE FROM fact_search WHERE rowid=OLD.id;
END;
