-- Schema v3: index referencing columns used during deletion and verification.
-- Shared provenance sources must survive until their last assertion is removed.
CREATE INDEX assertion_source ON assertion(owner_scope,source_id);
CREATE INDEX assertion_supersedes ON assertion(supersedes_id);
CREATE TRIGGER assertion_delete_source AFTER DELETE ON assertion
BEGIN
    DELETE FROM source
    WHERE owner_scope=OLD.owner_scope AND id=OLD.source_id
      AND NOT EXISTS(
          SELECT 1 FROM assertion
          WHERE owner_scope=OLD.owner_scope AND source_id=OLD.source_id
      );
END;
