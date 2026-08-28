BEGIN IMMEDIATE;

-- Rebuild the append-only ledger to add a structured preparation-task link and
-- the new automatic issue type. Existing movement IDs and values are copied
-- exactly. No preparation task is backfilled or issued by this migration.
DROP TRIGGER inventory_movements_reject_update;
DROP TRIGGER inventory_movements_reject_delete;
DROP INDEX idx_inventory_movements_opening;
DROP INDEX idx_inventory_movements_drug;
DROP INDEX idx_inventory_movements_occurred;

ALTER TABLE inventory_movements RENAME TO inventory_movements_v6;

CREATE TABLE inventory_movements (
  id INTEGER PRIMARY KEY,
  drug_id INTEGER NOT NULL REFERENCES drugs(id) ON DELETE RESTRICT,
  movement_type TEXT NOT NULL CHECK (
    movement_type IN (
      'opening_balance',
      'receipt',
      'manual_issue',
      'adjustment_increase',
      'adjustment_decrease',
      'preparation_issue'
    )
  ),
  quantity_delta REAL NOT NULL,
  occurred_at TEXT,
  actor_user_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
  reference_type TEXT,
  reference_id TEXT,
  note TEXT,
  preparation_task_id INTEGER REFERENCES preparation_tasks(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (
    (
      movement_type = 'opening_balance'
      AND occurred_at IS NULL
      AND actor_user_id IS NULL
    ) OR (
      movement_type <> 'opening_balance'
      AND occurred_at IS NOT NULL
      AND actor_user_id IS NOT NULL
      AND quantity_delta <> 0
    )
  ),
  CHECK (
    movement_type = 'opening_balance'
    OR (movement_type IN ('receipt','adjustment_increase') AND quantity_delta > 0)
    OR (
      movement_type IN ('manual_issue','adjustment_decrease','preparation_issue')
      AND quantity_delta < 0
    )
  ),
  CHECK (
    (movement_type = 'preparation_issue' AND preparation_task_id IS NOT NULL)
    OR (movement_type <> 'preparation_issue' AND preparation_task_id IS NULL)
  )
);

INSERT INTO inventory_movements(
  id,drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
  reference_type,reference_id,note,preparation_task_id,created_at
)
SELECT
  id,drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
  reference_type,reference_id,note,NULL,created_at
FROM inventory_movements_v6;

DROP TABLE inventory_movements_v6;

CREATE UNIQUE INDEX idx_inventory_movements_opening
  ON inventory_movements(drug_id)
  WHERE movement_type = 'opening_balance';
CREATE UNIQUE INDEX idx_inventory_movements_preparation_issue
  ON inventory_movements(preparation_task_id)
  WHERE movement_type = 'preparation_issue';
CREATE INDEX idx_inventory_movements_drug
  ON inventory_movements(drug_id, id);
CREATE INDEX idx_inventory_movements_occurred
  ON inventory_movements(occurred_at, id);

CREATE TRIGGER inventory_movements_reject_update
BEFORE UPDATE ON inventory_movements
BEGIN
  SELECT RAISE(ABORT, 'inventory movements are append-only');
END;

CREATE TRIGGER inventory_movements_reject_delete
BEFORE DELETE ON inventory_movements
BEGIN
  SELECT RAISE(ABORT, 'inventory movements are append-only');
END;

-- One append-only decision row records what verification did with inventory.
-- Absence for an already verified task means pre-Milestone-13 provenance and is
-- intentionally never backfilled.
CREATE TABLE preparation_inventory_postings (
  id INTEGER PRIMARY KEY,
  preparation_task_id INTEGER NOT NULL UNIQUE
    REFERENCES preparation_tasks(id) ON DELETE RESTRICT,
  status TEXT NOT NULL CHECK (
    status IN (
      'posted',
      'manual_reconciliation_required',
      'not_required',
      'tracking_disabled'
    )
  ),
  inventory_movement_id INTEGER UNIQUE
    REFERENCES inventory_movements(id) ON DELETE RESTRICT,
  containers_required INTEGER
    CHECK (
      containers_required IS NULL OR
      (containers_required >= 0 AND containers_required <= 9007199254740991)
    ),
  balance_before REAL,
  balance_after REAL,
  resulting_stock_state TEXT CHECK (
    resulting_stock_state IS NULL OR
    resulting_stock_state IN ('normal','low','out','shortage')
  ),
  calculation_status TEXT NOT NULL CHECK (
    calculation_status IN ('calculated','partially_calculated','unavailable','unsupported')
  ),
  calculation_ruleset_version TEXT NOT NULL,
  calculation_rule_id TEXT NOT NULL,
  workflow_rule_id TEXT NOT NULL,
  reason_code TEXT NOT NULL,
  actor_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (
    (
      status = 'posted'
      AND inventory_movement_id IS NOT NULL
      AND containers_required > 0
      AND balance_before IS NOT NULL
      AND balance_after IS NOT NULL
      AND resulting_stock_state IS NOT NULL
    ) OR (
      status = 'not_required'
      AND inventory_movement_id IS NULL
      AND containers_required = 0
      AND balance_before IS NULL
      AND balance_after IS NULL
      AND resulting_stock_state IS NULL
    ) OR (
      status IN ('manual_reconciliation_required','tracking_disabled')
      AND inventory_movement_id IS NULL
      AND balance_before IS NULL
      AND balance_after IS NULL
      AND resulting_stock_state IS NULL
    )
  )
);

CREATE INDEX idx_preparation_inventory_postings_movement
  ON preparation_inventory_postings(inventory_movement_id);

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

CREATE TRIGGER preparation_inventory_postings_reject_update
BEFORE UPDATE ON preparation_inventory_postings
BEGIN
  SELECT RAISE(ABORT, 'preparation inventory postings are append-only');
END;

CREATE TRIGGER preparation_inventory_postings_reject_delete
BEFORE DELETE ON preparation_inventory_postings
BEGIN
  SELECT RAISE(ABORT, 'preparation inventory postings are append-only');
END;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','7');

COMMIT;
