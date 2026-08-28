BEGIN IMMEDIATE;

-- A final preparation output is frozen once, from an already verified task.
-- The migration intentionally does not backfill existing preparations. Human-
-- readable master-data values are copied by the Rust output service on first
-- generation so later master-data edits cannot change a reprint.
CREATE TABLE preparation_output_snapshots (
  id INTEGER PRIMARY KEY,
  preparation_task_id INTEGER NOT NULL UNIQUE
    REFERENCES preparation_tasks(id) ON DELETE RESTRICT,
  template_version TEXT NOT NULL,
  generated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

  source_order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
  source_order_item_id INTEGER NOT NULL REFERENCES order_items(id) ON DELETE RESTRICT,
  order_reference TEXT NOT NULL,
  patient_identifier TEXT NOT NULL,
  patient_name TEXT,
  regimen_name TEXT,
  treatment_at TEXT,
  treatment_day TEXT,

  drug_code TEXT NOT NULL,
  drug_name TEXT NOT NULL,
  ordered_dose_text TEXT,
  dose_unit_text TEXT,
  diluent_name TEXT,
  diluent_volume_ml REAL,
  final_volume_ml REAL,
  route_name TEXT,
  infusion_rate_or_duration TEXT,
  preparation_instructions TEXT,
  preparation_notes TEXT,
  storage_reference TEXT,

  prepared_by_display_name TEXT,
  prepared_at TEXT,
  verified_by_display_name TEXT,
  verified_at TEXT NOT NULL,

  inventory_posting_status TEXT,
  inventory_movement_id INTEGER REFERENCES inventory_movements(id) ON DELETE RESTRICT,
  containers_required INTEGER,
  inventory_balance_before REAL,
  inventory_balance_after REAL,
  inventory_stock_state TEXT,
  calculation_ruleset_version TEXT,
  calculation_rule_id TEXT,

  CHECK (template_version = 'oncoflow-preparation-label-v1'),
  CHECK (length(trim(order_reference)) > 0),
  CHECK (length(trim(patient_identifier)) > 0),
  CHECK (length(trim(drug_code)) > 0),
  CHECK (length(trim(drug_name)) > 0),
  CHECK (diluent_volume_ml IS NULL OR diluent_volume_ml >= 0),
  CHECK (final_volume_ml IS NULL OR final_volume_ml >= 0),
  CHECK (containers_required IS NULL OR containers_required >= 0),
  CHECK (
    inventory_stock_state IS NULL OR
    inventory_stock_state IN ('normal','low','out','shortage')
  )
);

CREATE INDEX idx_preparation_output_order
  ON preparation_output_snapshots(source_order_id, id);

-- Output snapshots are append-only through normal OncoFlow/database writes.
CREATE TRIGGER preparation_output_snapshots_reject_update
BEFORE UPDATE ON preparation_output_snapshots
BEGIN
  SELECT RAISE(ABORT, 'preparation output snapshots are append-only');
END;

CREATE TRIGGER preparation_output_snapshots_reject_delete
BEFORE DELETE ON preparation_output_snapshots
BEGIN
  SELECT RAISE(ABORT, 'preparation output snapshots are append-only');
END;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','8');

COMMIT;
