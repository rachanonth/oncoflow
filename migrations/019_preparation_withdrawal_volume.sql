BEGIN IMMEDIATE;

-- Persist the checked withdrawal result so final labels never recalculate it
-- from mutable drug master data.
ALTER TABLE preparation_tasks ADD COLUMN withdrawal_volume_ml TEXT;
ALTER TABLE preparation_output_snapshots ADD COLUMN withdrawal_volume_ml TEXT;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','19');

COMMIT;
