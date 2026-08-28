BEGIN IMMEDIATE;

-- Organization identity used on non-clinical application output.
CREATE TABLE IF NOT EXISTS application_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  hospital_name TEXT CHECK (
    hospital_name IS NULL OR
    (length(trim(hospital_name)) BETWEEN 1 AND 160 AND
     instr(hospital_name, char(10)) = 0 AND
     instr(hospital_name, char(13)) = 0)
  ),
  updated_by_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO application_settings(id,hospital_name) VALUES(1,NULL);
INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','15');

COMMIT;
