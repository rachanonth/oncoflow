BEGIN IMMEDIATE;

ALTER TABLE orders ADD COLUMN weight_kg REAL CHECK (weight_kg IS NULL OR (weight_kg > 0 AND weight_kg <= 500));
ALTER TABLE orders ADD COLUMN height_cm REAL CHECK (height_cm IS NULL OR (height_cm > 0 AND height_cm <= 300));

UPDATE orders
SET weight_kg = (SELECT p.weight_kg FROM patients p WHERE p.id = orders.patient_id),
    height_cm = (SELECT p.height_cm FROM patients p WHERE p.id = orders.patient_id);

UPDATE app_meta SET value = '12' WHERE key = 'schema_version';

COMMIT;
