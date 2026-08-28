BEGIN IMMEDIATE;

-- Compatibility-only columns required to preserve values that do not have a
-- lossless destination in the initial schema. These are not clinical rules.
ALTER TABLE patients ADD COLUMN legacy_xn TEXT;
ALTER TABLE patients ADD COLUMN treatment_ended INTEGER CHECK (treatment_ended IN (0,1));
ALTER TABLE patients ADD COLUMN legacy_age REAL;
ALTER TABLE patients ADD COLUMN legacy_bsa REAL;
ALTER TABLE patients ADD COLUMN sex TEXT;
ALTER TABLE patients ADD COLUMN telephone TEXT;

ALTER TABLE drugs ADD COLUMN legacy_exp INTEGER;
ALTER TABLE drugs ADD COLUMN legacy_reg TEXT;

ALTER TABLE regimen_items ADD COLUMN legacy_dose_text TEXT;

ALTER TABLE orders ADD COLUMN legacy_worker TEXT;
ALTER TABLE orders ADD COLUMN side_effect_text TEXT;
ALTER TABLE orders ADD COLUMN medication_error_text TEXT;

ALTER TABLE pharmcare_soap ADD COLUMN subjective TEXT;
ALTER TABLE pharmcare_soap ADD COLUMN objective TEXT;
ALTER TABLE pharmcare_soap ADD COLUMN assessment TEXT;
ALTER TABLE pharmcare_soap ADD COLUMN plan_text TEXT;

ALTER TABLE interventions ADD COLUMN intervention_performed INTEGER
  CHECK (intervention_performed IN (0,1));

ALTER TABLE problems ADD COLUMN problem_by TEXT;

ALTER TABLE alert_settings ADD COLUMN label_number INTEGER;
ALTER TABLE alert_settings ADD COLUMN wbc_threshold REAL;
ALTER TABLE alert_settings ADD COLUMN anc_threshold REAL;
ALTER TABLE alert_settings ADD COLUMN haemoglobin_threshold REAL;
ALTER TABLE alert_settings ADD COLUMN ast_threshold REAL;

INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','2');

COMMIT;
