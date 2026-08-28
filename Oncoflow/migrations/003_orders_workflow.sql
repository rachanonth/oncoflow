BEGIN IMMEDIATE;

-- OncoFlow-created orders are editable drafts in Milestone 6. Imported rows
-- remain historical and read-only without introducing a clinical status model.
ALTER TABLE orders ADD COLUMN oncoflow_created INTEGER NOT NULL DEFAULT 0
  CHECK (oncoflow_created IN (0,1));

-- Preserve raw order/regimen text and provenance without interpreting legacy
-- dose, schedule, group, duration, or start-day semantics.
ALTER TABLE order_items ADD COLUMN legacy_dose_text TEXT;
ALTER TABLE order_items ADD COLUMN source_regimen_item_id INTEGER
  REFERENCES regimen_items(id) ON DELETE SET NULL;
ALTER TABLE order_items ADD COLUMN regimen_dose_text TEXT;
ALTER TABLE order_items ADD COLUMN regimen_unit_text TEXT;
ALTER TABLE order_items ADD COLUMN regimen_route_text TEXT;
ALTER TABLE order_items ADD COLUMN regimen_details TEXT;
ALTER TABLE order_items ADD COLUMN regimen_item_group TEXT;
ALTER TABLE order_items ADD COLUMN regimen_duration TEXT;
ALTER TABLE order_items ADD COLUMN regimen_start_day INTEGER;
ALTER TABLE order_items ADD COLUMN regimen_ordering_no INTEGER;

CREATE INDEX IF NOT EXISTS idx_orders_time ON orders(order_time, id);
CREATE INDEX IF NOT EXISTS idx_order_items_ordering
  ON order_items(order_id, ordering_no, id);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','3');

COMMIT;
