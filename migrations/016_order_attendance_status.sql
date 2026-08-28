BEGIN IMMEDIATE;

-- Order workflow state is separate from legacy order/item dates. Historical
-- migrated rows remain explicitly legacy rather than being declared active.
ALTER TABLE orders ADD COLUMN workflow_status TEXT NOT NULL DEFAULT 'active'
  CHECK (workflow_status IN ('active','on_hold','legacy'));
ALTER TABLE orders ADD COLUMN workflow_status_reason TEXT;
ALTER TABLE orders ADD COLUMN workflow_status_changed_at TEXT;
ALTER TABLE orders ADD COLUMN workflow_status_changed_by_user_id INTEGER
  REFERENCES users(id) ON DELETE RESTRICT;

UPDATE orders
SET workflow_status='legacy'
WHERE oncoflow_created=0;

CREATE INDEX idx_orders_workflow_status
  ON orders(workflow_status,order_time,id);

-- These rows are append-only attendance/scheduling events. A no-show date is
-- excluded from preparation. A rescheduled event makes the items that were
-- due on its related no-show date available on the new effective date without
-- changing legacy start/stop dates.
CREATE TABLE order_status_events (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
  event_type TEXT NOT NULL CHECK (event_type IN ('no_show','rescheduled')),
  from_status TEXT NOT NULL CHECK (from_status IN ('active','on_hold')),
  to_status TEXT NOT NULL CHECK (to_status IN ('active','on_hold')),
  effective_date TEXT NOT NULL CHECK (length(effective_date)=10),
  related_date TEXT CHECK (related_date IS NULL OR length(related_date)=10),
  actor_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (
    (event_type='no_show' AND from_status='active' AND to_status='on_hold' AND related_date IS NULL)
    OR
    (event_type='rescheduled' AND from_status='on_hold' AND to_status='active' AND related_date IS NOT NULL)
  )
);

CREATE UNIQUE INDEX uq_order_status_event_no_show
  ON order_status_events(order_id,effective_date)
  WHERE event_type='no_show';
CREATE UNIQUE INDEX uq_order_status_event_reschedule_source
  ON order_status_events(order_id,related_date)
  WHERE event_type='rescheduled';
CREATE UNIQUE INDEX uq_order_status_event_reschedule_target
  ON order_status_events(order_id,effective_date)
  WHERE event_type='rescheduled';
CREATE INDEX idx_order_status_events_effective
  ON order_status_events(order_id,event_type,effective_date,id);

CREATE TRIGGER order_status_events_no_update
BEFORE UPDATE ON order_status_events
BEGIN
  SELECT RAISE(ABORT,'order status events are append-only');
END;

CREATE TRIGGER order_status_events_no_delete
BEFORE DELETE ON order_status_events
BEGIN
  SELECT RAISE(ABORT,'order status events are append-only');
END;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','16');

COMMIT;
