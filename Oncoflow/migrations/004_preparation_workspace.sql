BEGIN IMMEDIATE;

-- A preparation task records exactly what was reviewed for an OncoFlow-created
-- order item. Source order values remain authoritative and are never updated by
-- this workflow. Historical imported orders are intentionally not backfilled.
CREATE TABLE preparation_tasks (
  id INTEGER PRIMARY KEY,
  source_order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
  source_order_item_id INTEGER NOT NULL UNIQUE REFERENCES order_items(id) ON DELETE RESTRICT,
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
  CHECK (
    (state = 'pending' AND prepared_at IS NULL AND verified_at IS NULL) OR
    (state = 'prepared' AND prepared_at IS NOT NULL AND verified_at IS NULL) OR
    (state = 'verified' AND prepared_at IS NOT NULL AND verified_at IS NOT NULL)
  )
);

CREATE INDEX idx_preparation_tasks_order
  ON preparation_tasks(source_order_id, snapshot_sequence_no, id);
CREATE INDEX idx_preparation_tasks_state
  ON preparation_tasks(state, updated_at, id);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','4');

COMMIT;
