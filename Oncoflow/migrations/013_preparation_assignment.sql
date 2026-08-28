BEGIN IMMEDIATE;

-- New orders name the intended preparation pharmacist before preparation work
-- begins. Existing and historical orders intentionally remain unassigned.
ALTER TABLE orders ADD COLUMN assigned_preparer_user_id INTEGER
  REFERENCES users(id) ON DELETE RESTRICT;

CREATE INDEX idx_orders_assigned_preparer
  ON orders(assigned_preparer_user_id, order_time, id);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','13');

COMMIT;
