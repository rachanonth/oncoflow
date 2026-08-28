BEGIN IMMEDIATE;

-- Optional workstation guidance is separate from immutable OncoFlow page copy.
-- The text is operational UI configuration, not a clinical rule.
CREATE TABLE page_guidance (
  page_key TEXT PRIMARY KEY,
  guidance TEXT NOT NULL CHECK (length(trim(guidance)) BETWEEN 1 AND 500),
  updated_by_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','10');

COMMIT;
