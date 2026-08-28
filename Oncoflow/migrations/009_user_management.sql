BEGIN IMMEDIATE;

-- Administrative privilege remains in users.role. User type is separate
-- identity metadata and must not silently become a clinical permission rule.
ALTER TABLE users ADD COLUMN user_type TEXT NOT NULL DEFAULT 'non_pharmacist'
  CHECK (user_type IN ('pharmacist','non_pharmacist'));

-- Existing usable OncoFlow identities have already participated in the
-- pharmacist-oriented workflow. Disabled legacy identities remain explicitly
-- non-pharmacist until a local administrator creates/manages a modern account.
UPDATE users
SET user_type='pharmacist'
WHERE credential_kind='argon2id';

CREATE INDEX idx_users_management
  ON users(credential_kind, active, user_type, display_name, id);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','9');

COMMIT;
