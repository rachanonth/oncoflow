BEGIN IMMEDIATE;

-- Freeze display-only master data when the checked output snapshot is first
-- created. Expiration is not stored here: it is derived from the frozen
-- duration and the first successful label print time.
ALTER TABLE preparation_output_snapshots ADD COLUMN hospital_name TEXT;
ALTER TABLE preparation_output_snapshots ADD COLUMN warning_text TEXT;
ALTER TABLE preparation_output_snapshots ADD COLUMN expiry_time_text TEXT;
ALTER TABLE preparation_output_snapshots ADD COLUMN expiry_storage_text TEXT;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','18');

COMMIT;
