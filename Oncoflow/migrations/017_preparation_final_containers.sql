BEGIN IMMEDIATE;

-- Final administration containers are distinct from source vials/ampoules
-- consumed from inventory. Existing preparations remain one final container.
ALTER TABLE preparation_tasks ADD COLUMN final_container_count INTEGER NOT NULL DEFAULT 1
  CHECK (final_container_count BETWEEN 1 AND 20);

-- The allocation is copied into the immutable output snapshot so later task
-- edits can never change the contents or number of labels on reprint.
ALTER TABLE preparation_output_snapshots ADD COLUMN final_container_count INTEGER NOT NULL DEFAULT 1
  CHECK (final_container_count BETWEEN 1 AND 20);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','17');

COMMIT;
