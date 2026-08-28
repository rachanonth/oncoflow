BEGIN IMMEDIATE;

-- Current inventory is derived from this append-only ledger. The original
-- drugs.inventory_qty value remains an untouched legacy compatibility snapshot.
CREATE TABLE inventory_movements (
  id INTEGER PRIMARY KEY,
  drug_id INTEGER NOT NULL REFERENCES drugs(id) ON DELETE RESTRICT,
  movement_type TEXT NOT NULL CHECK (
    movement_type IN (
      'opening_balance',
      'receipt',
      'manual_issue',
      'adjustment_increase',
      'adjustment_decrease'
    )
  ),
  quantity_delta REAL NOT NULL,
  occurred_at TEXT,
  actor_user_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
  reference_type TEXT,
  reference_id TEXT,
  note TEXT,
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
    OR (movement_type IN ('manual_issue','adjustment_decrease') AND quantity_delta < 0)
  )
);

CREATE UNIQUE INDEX idx_inventory_movements_opening
  ON inventory_movements(drug_id)
  WHERE movement_type = 'opening_balance';
CREATE INDEX idx_inventory_movements_drug
  ON inventory_movements(drug_id, id);
CREATE INDEX idx_inventory_movements_occurred
  ON inventory_movements(occurred_at, id);

-- Preserve every known migrated value exactly, including zero and negative
-- snapshots. No actor or historical occurrence time is fabricated.
INSERT OR IGNORE INTO inventory_movements(
  drug_id,
  movement_type,
  quantity_delta,
  occurred_at,
  actor_user_id,
  reference_type,
  reference_id,
  note
)
SELECT
  id,
  'opening_balance',
  inventory_qty,
  NULL,
  NULL,
  'legacy_drug_inventory',
  legacy_dcode,
  'Migrated Tbldrug.Inv snapshot'
FROM drugs
WHERE inventory_qty IS NOT NULL;

-- OncoFlow corrects inventory with compensating rows. Normal writes can append
-- movements but cannot revise or remove history.
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

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','6');

COMMIT;
