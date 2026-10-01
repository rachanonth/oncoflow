PRAGMA foreign_keys=ON;
BEGIN IMMEDIATE;

CREATE TABLE order_cancellations (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
  item_id INTEGER REFERENCES order_items(id) ON DELETE RESTRICT,
  reason TEXT NOT NULL CHECK (length(trim(reason)) BETWEEN 1 AND 500),
  actor_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
ALTER TABLE orders ADD COLUMN cancellation_id INTEGER REFERENCES order_cancellations(id);
ALTER TABLE order_items ADD COLUMN cancellation_id INTEGER REFERENCES order_cancellations(id);
ALTER TABLE preparation_tasks ADD COLUMN cancellation_id INTEGER REFERENCES order_cancellations(id);
ALTER TABLE preparation_tasks ADD COLUMN cancelled_after_preparation INTEGER NOT NULL DEFAULT 0
  CHECK (cancelled_after_preparation IN (0,1));
CREATE INDEX idx_order_cancellations_order ON order_cancellations(order_id,id);

CREATE TRIGGER order_cancellations_no_update BEFORE UPDATE ON order_cancellations
BEGIN SELECT RAISE(ABORT,'cancellations are append-only'); END;
CREATE TRIGGER order_cancellations_no_delete BEFORE DELETE ON order_cancellations
BEGIN SELECT RAISE(ABORT,'cancellations are append-only'); END;
CREATE TRIGGER cancelled_order_no_update BEFORE UPDATE ON orders WHEN OLD.cancellation_id IS NOT NULL
BEGIN SELECT RAISE(ABORT,'cancelled orders are read-only'); END;
CREATE TRIGGER cancelled_item_no_update BEFORE UPDATE ON order_items WHEN OLD.cancellation_id IS NOT NULL
BEGIN SELECT RAISE(ABORT,'cancelled items are read-only'); END;
CREATE TRIGGER cancelled_task_no_update BEFORE UPDATE ON preparation_tasks WHEN OLD.cancellation_id IS NOT NULL
BEGIN SELECT RAISE(ABORT,'cancelled preparations are read-only'); END;
CREATE TRIGGER cancelled_task_no_delete BEFORE DELETE ON preparation_tasks WHEN OLD.cancellation_id IS NOT NULL
BEGIN SELECT RAISE(ABORT,'cancelled preparations are retained'); END;
CREATE TRIGGER cancelled_item_no_delete BEFORE DELETE ON order_items WHEN OLD.cancellation_id IS NOT NULL
BEGIN SELECT RAISE(ABORT,'cancelled items are retained'); END;
CREATE TRIGGER cancelled_order_no_item BEFORE INSERT ON order_items
WHEN EXISTS(SELECT 1 FROM orders WHERE id=NEW.order_id AND cancellation_id IS NOT NULL)
BEGIN SELECT RAISE(ABORT,'cancelled orders are read-only'); END;
CREATE TRIGGER cancelled_source_no_preparation BEFORE INSERT ON preparation_tasks
WHEN EXISTS(SELECT 1 FROM orders WHERE id=NEW.source_order_id AND cancellation_id IS NOT NULL)
  OR EXISTS(SELECT 1 FROM order_items WHERE id=NEW.source_order_item_id AND cancellation_id IS NOT NULL)
BEGIN SELECT RAISE(ABORT,'cancelled sources cannot be prepared'); END;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','21');
COMMIT;
