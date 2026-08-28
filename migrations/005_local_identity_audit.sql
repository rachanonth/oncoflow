BEGIN IMMEDIATE;

-- Legacy imported users retain a deliberately unusable credential placeholder.
-- Only an explicit first-run bootstrap can establish a new Argon2id credential.
ALTER TABLE users ADD COLUMN credential_kind TEXT NOT NULL DEFAULT 'legacy_disabled'
  CHECK (credential_kind IN ('legacy_disabled','argon2id'));
-- SQLite cannot add a column with a non-constant default to a populated table.
-- Existing imported identities inherit their original creation timestamp;
-- all new/changed modern credentials set updated_at explicitly in Rust.
ALTER TABLE users ADD COLUMN updated_at TEXT;
UPDATE users SET updated_at=COALESCE(created_at,CURRENT_TIMESTAMP);
ALTER TABLE users ADD COLUMN password_changed_at TEXT;

-- Existing Milestone 9 preparation rows remain unattributed. OncoFlow never
-- backfills the current user into an earlier task.
ALTER TABLE preparation_tasks ADD COLUMN prepared_by_user_id INTEGER
  REFERENCES users(id) ON DELETE RESTRICT;
ALTER TABLE preparation_tasks ADD COLUMN verified_by_user_id INTEGER
  REFERENCES users(id) ON DELETE RESTRICT;

CREATE TABLE safety_acknowledgements (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
  preparation_task_id INTEGER REFERENCES preparation_tasks(id) ON DELETE RESTRICT,
  order_item_id INTEGER REFERENCES order_items(id) ON DELETE RESTRICT,
  finding_id TEXT NOT NULL,
  finding_fingerprint TEXT NOT NULL CHECK (length(finding_fingerprint)=64),
  rule_id TEXT NOT NULL,
  ruleset_version TEXT NOT NULL,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  acknowledged_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  source_snapshot_stale INTEGER NOT NULL DEFAULT 0
    CHECK (source_snapshot_stale IN (0,1)),
  CHECK (
    (preparation_task_id IS NULL AND order_item_id IS NULL) OR
    (preparation_task_id IS NOT NULL AND order_item_id IS NOT NULL)
  )
);

CREATE UNIQUE INDEX idx_safety_acknowledgement_current
  ON safety_acknowledgements(
    order_id,
    IFNULL(preparation_task_id,0),
    finding_fingerprint
  );
CREATE INDEX idx_safety_acknowledgement_order
  ON safety_acknowledgements(order_id, acknowledged_at, id);

CREATE TABLE audit_events (
  id INTEGER PRIMARY KEY,
  occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  user_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
  event_type TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json))
);

CREATE INDEX idx_audit_events_entity
  ON audit_events(entity_type, entity_id, occurred_at, id);
CREATE INDEX idx_audit_events_user
  ON audit_events(user_id, occurred_at, id);

-- Normal application/database writes may append events but cannot revise them.
CREATE TRIGGER audit_events_reject_update
BEFORE UPDATE ON audit_events
BEGIN
  SELECT RAISE(ABORT, 'audit events are append-only');
END;

CREATE TRIGGER audit_events_reject_delete
BEFORE DELETE ON audit_events
BEGIN
  SELECT RAISE(ABORT, 'audit events are append-only');
END;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','5');

COMMIT;
