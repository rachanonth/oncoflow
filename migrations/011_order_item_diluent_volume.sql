BEGIN IMMEDIATE;

-- Optional order-specific diluent volume. NULL preserves the existing behavior
-- of using the selected diluent master's volume.
ALTER TABLE order_items ADD COLUMN diluent_volume_ml REAL
  CHECK (diluent_volume_ml IS NULL OR diluent_volume_ml >= 0);

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','11');

COMMIT;
