PRAGMA foreign_keys=OFF;
BEGIN IMMEDIATE;

-- This trigger joins the parent table and must be recreated around the SQLite
-- table rebuild so every intermediate schema remains valid.
DROP TRIGGER preparation_inventory_postings_validate_issue;

-- Preparation is performed for a treatment date, not merely once per order
-- line. Preserve every existing task and attribute it to the best durable date
-- already present on its source: explicit administration start, order date, or
-- (only when neither exists) the task creation date.
CREATE TABLE preparation_tasks_v14 (
  id INTEGER PRIMARY KEY,
  source_order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
  source_order_item_id INTEGER NOT NULL REFERENCES order_items(id) ON DELETE RESTRICT,
  preparation_date TEXT NOT NULL,
  drug_id INTEGER NOT NULL REFERENCES drugs(id) ON DELETE RESTRICT,
  state TEXT NOT NULL DEFAULT 'pending'
    CHECK (state IN ('pending','prepared','verified')),
  snapshot_ordered_dose_text TEXT,
  snapshot_dose_unit_text TEXT,
  snapshot_diluent_id INTEGER REFERENCES diluents(id) ON DELETE RESTRICT,
  snapshot_diluent_name TEXT,
  snapshot_diluent_volume_ml REAL
    CHECK (snapshot_diluent_volume_ml IS NULL OR snapshot_diluent_volume_ml >= 0),
  snapshot_route_id INTEGER REFERENCES routes(id) ON DELETE RESTRICT,
  snapshot_route_name TEXT,
  snapshot_rate_text TEXT,
  snapshot_treatment_day TEXT,
  snapshot_start_date TEXT,
  snapshot_stop_date TEXT,
  snapshot_sequence_no INTEGER,
  snapshot_regimen_details TEXT,
  snapshot_drug_detail TEXT,
  snapshot_drug_storage TEXT,
  preparation_volume_ml REAL
    CHECK (preparation_volume_ml IS NULL OR preparation_volume_ml >= 0),
  preparation_notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  prepared_at TEXT,
  verified_at TEXT,
  prepared_by_user_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
  verified_by_user_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
  CHECK (
    (state = 'pending' AND prepared_at IS NULL AND verified_at IS NULL) OR
    (state = 'prepared' AND prepared_at IS NOT NULL AND verified_at IS NULL) OR
    (state = 'verified' AND prepared_at IS NOT NULL AND verified_at IS NOT NULL)
  ),
  UNIQUE(source_order_item_id, preparation_date)
);

INSERT INTO preparation_tasks_v14(
  id,source_order_id,source_order_item_id,preparation_date,drug_id,state,
  snapshot_ordered_dose_text,snapshot_dose_unit_text,
  snapshot_diluent_id,snapshot_diluent_name,snapshot_diluent_volume_ml,
  snapshot_route_id,snapshot_route_name,snapshot_rate_text,
  snapshot_treatment_day,snapshot_start_date,snapshot_stop_date,
  snapshot_sequence_no,snapshot_regimen_details,snapshot_drug_detail,
  snapshot_drug_storage,preparation_volume_ml,preparation_notes,
  created_at,updated_at,prepared_at,verified_at,
  prepared_by_user_id,verified_by_user_id
)
SELECT
  t.id,t.source_order_id,t.source_order_item_id,
  COALESCE(
    NULLIF(substr(i.start_date,1,10),''),
    NULLIF(substr(o.order_time,1,10),''),
    substr(t.created_at,1,10)
  ),
  t.drug_id,t.state,
  t.snapshot_ordered_dose_text,t.snapshot_dose_unit_text,
  t.snapshot_diluent_id,t.snapshot_diluent_name,t.snapshot_diluent_volume_ml,
  t.snapshot_route_id,t.snapshot_route_name,t.snapshot_rate_text,
  t.snapshot_treatment_day,i.start_date,i.stop_date,
  t.snapshot_sequence_no,t.snapshot_regimen_details,t.snapshot_drug_detail,
  t.snapshot_drug_storage,t.preparation_volume_ml,t.preparation_notes,
  t.created_at,t.updated_at,t.prepared_at,t.verified_at,
  t.prepared_by_user_id,t.verified_by_user_id
FROM preparation_tasks t
JOIN order_items i ON i.id=t.source_order_item_id
JOIN orders o ON o.id=t.source_order_id;

DROP TABLE preparation_tasks;
ALTER TABLE preparation_tasks_v14 RENAME TO preparation_tasks;

CREATE INDEX idx_preparation_tasks_order
  ON preparation_tasks(source_order_id, preparation_date, snapshot_sequence_no, id);
CREATE INDEX idx_preparation_tasks_state
  ON preparation_tasks(preparation_date, state, updated_at, id);

CREATE TRIGGER preparation_inventory_postings_validate_issue
BEFORE INSERT ON preparation_inventory_postings
WHEN NEW.status = 'posted'
  AND NOT EXISTS (
    SELECT 1
    FROM inventory_movements m
    JOIN preparation_tasks t ON t.id = NEW.preparation_task_id
    WHERE m.id = NEW.inventory_movement_id
      AND m.movement_type = 'preparation_issue'
      AND m.preparation_task_id = NEW.preparation_task_id
      AND m.drug_id = t.drug_id
      AND m.actor_user_id = NEW.actor_user_id
      AND m.quantity_delta = -NEW.containers_required
  )
BEGIN
  SELECT RAISE(ABORT, 'preparation inventory posting does not match its issue movement');
END;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','14');

COMMIT;
PRAGMA foreign_keys=ON;
