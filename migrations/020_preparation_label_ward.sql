BEGIN IMMEDIATE;

-- Freeze the order ward for new labels; old snapshots remain unknown.
ALTER TABLE preparation_output_snapshots ADD COLUMN ward_name TEXT;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','20');

COMMIT;
