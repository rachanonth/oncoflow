PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- OncoFlow migration - compatibility-first SQLite schema.
-- Dates/times are stored as ISO-8601 TEXT in the new application.
-- Legacy Access identifiers are retained where useful for migration/parity testing.

CREATE TABLE IF NOT EXISTS app_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY,
  legacy_user TEXT UNIQUE,
  username TEXT NOT NULL UNIQUE,
  display_name TEXT,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user','admin')),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS diagnoses (
  id INTEGER PRIMARY KEY,
  legacy_diagcode TEXT UNIQUE,
  diagnosis TEXT NOT NULL,
  warning1 TEXT,
  warning2 TEXT
);

CREATE TABLE IF NOT EXISTS units (
  id INTEGER PRIMARY KEY,
  legacy_unitcode TEXT UNIQUE,
  unit_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS routes (
  id INTEGER PRIMARY KEY,
  legacy_rcode TEXT UNIQUE,
  route_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS diluents (
  id INTEGER PRIMARY KEY,
  legacy_dilcode TEXT UNIQUE,
  diluent_name TEXT NOT NULL,
  volume_ml REAL
);

CREATE TABLE IF NOT EXISTS doctors (
  id INTEGER PRIMARY KEY,
  legacy_doccode TEXT UNIQUE,
  doctor_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wards (
  id INTEGER PRIMARY KEY,
  legacy_wcode TEXT UNIQUE,
  ward_name TEXT NOT NULL,
  telephone TEXT
);

CREATE TABLE IF NOT EXISTS regimens (
  id INTEGER PRIMARY KEY,
  legacy_regcode TEXT UNIQUE,
  regimen_name TEXT NOT NULL,
  marker INTEGER DEFAULT 0 CHECK (marker IN (0,1)),
  flag INTEGER DEFAULT 0,
  cycle_check INTEGER DEFAULT 0 CHECK (cycle_check IN (0,1)),
  auto_mode INTEGER DEFAULT 0 CHECK (auto_mode IN (0,1)),
  drug_alert INTEGER DEFAULT 0 CHECK (drug_alert IN (0,1)),
  appointment_alert INTEGER DEFAULT 0 CHECK (appointment_alert IN (0,1)),
  counsel_alert INTEGER DEFAULT 0 CHECK (counsel_alert IN (0,1))
);

CREATE TABLE IF NOT EXISTS patients (
  id INTEGER PRIMARY KEY,
  legacy_hn TEXT NOT NULL UNIQUE,
  homc_hn TEXT,
  hn3 TEXT,
  hn4 TEXT,
  cancer_no TEXT,
  title TEXT,
  first_name TEXT,
  last_name TEXT,
  weight_kg REAL,
  height_cm REAL,
  birth_date TEXT,
  occupation TEXT,
  address TEXT,
  diagnosis_id INTEGER REFERENCES diagnoses(id),
  regimen_id INTEGER REFERENCES regimens(id),
  appointment_card INTEGER DEFAULT 0 CHECK (appointment_card IN (0,1)),
  counselling INTEGER DEFAULT 0 CHECK (counselling IN (0,1)),
  patient_history TEXT,
  stage TEXT,
  her2 TEXT,
  erpr TEXT,
  cd TEXT,
  mh TEXT,
  allergy TEXT,
  record_by TEXT,
  record_time TEXT,
  treatment_end_date TEXT
);
CREATE INDEX IF NOT EXISTS idx_patients_name ON patients(last_name, first_name);
CREATE INDEX IF NOT EXISTS idx_patients_regimen ON patients(regimen_id);

CREATE TABLE IF NOT EXISTS drugs (
  id INTEGER PRIMARY KEY,
  legacy_dcode TEXT NOT NULL UNIQUE,
  drug_name TEXT NOT NULL,
  unit_id INTEGER REFERENCES units(id),
  dose_per_pack REAL,
  volume_per_pack_ml REAL,
  package TEXT,
  detail TEXT,
  price REAL,
  theory TEXT,
  marker INTEGER DEFAULT 0 CHECK (marker IN (0,1)),
  default_diluent_id INTEGER REFERENCES diluents(id),
  default_route_id INTEGER REFERENCES routes(id),
  default_rate TEXT,
  warning TEXT,
  storage TEXT,
  flag INTEGER DEFAULT 0,
  expiry_time TEXT,
  expiry_storage TEXT,
  max_dose REAL,
  max_dilution_alert REAL,
  max_dilution_hard REAL,
  cumulative_alert REAL,
  cumulative_alert_hard REAL,
  dilution_incompatibility TEXT,
  inventory_cut REAL,
  inventory_min REAL,
  inventory_max REAL,
  inventory_qty REAL,
  inventory_enabled INTEGER DEFAULT 0 CHECK (inventory_enabled IN (0,1)),
  homc_code TEXT
);
CREATE INDEX IF NOT EXISTS idx_drugs_name ON drugs(drug_name);
CREATE INDEX IF NOT EXISTS idx_drugs_inventory ON drugs(inventory_enabled, inventory_qty);

CREATE TABLE IF NOT EXISTS drug_detail_groups (
  id INTEGER PRIMARY KEY,
  legacy_code TEXT,
  drug_id INTEGER NOT NULL REFERENCES drugs(id) ON DELETE CASCADE,
  note TEXT
);

CREATE TABLE IF NOT EXISTS drug_detail_items (
  id INTEGER PRIMARY KEY,
  detail_group_id INTEGER NOT NULL REFERENCES drug_detail_groups(id) ON DELETE CASCADE,
  detail TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS regimen_groups (
  id INTEGER PRIMARY KEY,
  legacy_code TEXT UNIQUE,
  regimen_id INTEGER NOT NULL REFERENCES regimens(id) ON DELETE CASCADE,
  note TEXT,
  cycle_day INTEGER,
  cycle_count INTEGER
);

CREATE TABLE IF NOT EXISTS regimen_items (
  id INTEGER PRIMARY KEY,
  regimen_group_id INTEGER NOT NULL REFERENCES regimen_groups(id) ON DELETE CASCADE,
  drug_id INTEGER REFERENCES drugs(id),
  dose REAL,
  unit_text TEXT,
  route_text TEXT,
  details TEXT,
  item_group TEXT,
  duration TEXT,
  start_day INTEGER,
  ordering_no INTEGER,
  default_diluent_id INTEGER REFERENCES diluents(id),
  default_route_id INTEGER REFERENCES routes(id),
  default_rate TEXT
);
CREATE INDEX IF NOT EXISTS idx_regimen_items_group ON regimen_items(regimen_group_id, ordering_no);

CREATE TABLE IF NOT EXISTS appointments (
  id INTEGER PRIMARY KEY,
  legacy_appid TEXT UNIQUE,
  patient_id INTEGER NOT NULL REFERENCES patients(id),
  appointment_date TEXT NOT NULL,
  diagnosis_id INTEGER REFERENCES diagnoses(id),
  regimen_id INTEGER REFERENCES regimens(id),
  legacy_user TEXT,
  recorded_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_appointments_date ON appointments(appointment_date);
CREATE INDEX IF NOT EXISTS idx_appointments_patient ON appointments(patient_id, appointment_date);

CREATE TABLE IF NOT EXISTS appointment_items (
  id INTEGER PRIMARY KEY,
  appointment_id INTEGER NOT NULL REFERENCES appointments(id) ON DELETE CASCADE,
  drug_id INTEGER REFERENCES drugs(id),
  dose REAL,
  detail TEXT
);

CREATE TABLE IF NOT EXISTS appointment_cards (
  id INTEGER PRIMARY KEY,
  legacy_ccode TEXT,
  regimen_id INTEGER REFERENCES regimens(id),
  cycle_no INTEGER,
  day_no INTEGER,
  appointment_day INTEGER
);

CREATE TABLE IF NOT EXISTS orders (
  id INTEGER PRIMARY KEY,
  legacy_orderid TEXT NOT NULL UNIQUE,
  patient_id INTEGER NOT NULL REFERENCES patients(id),
  ward_id INTEGER REFERENCES wards(id),
  doctor_id INTEGER REFERENCES doctors(id),
  worker INTEGER DEFAULT 0 CHECK (worker IN (0,1)),
  edit_worker TEXT,
  note TEXT,
  order_time TEXT,
  side_effect_flag INTEGER DEFAULT 0 CHECK (side_effect_flag IN (0,1)),
  side_effect_recorder TEXT,
  side_effect_record_time TEXT,
  regimen_id INTEGER REFERENCES regimens(id),
  order_type TEXT,
  appointment_flag INTEGER DEFAULT 0 CHECK (appointment_flag IN (0,1))
);
CREATE INDEX IF NOT EXISTS idx_orders_patient ON orders(patient_id, order_time);

CREATE TABLE IF NOT EXISTS order_items (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
  drug_id INTEGER NOT NULL REFERENCES drugs(id),
  diluent_id INTEGER REFERENCES diluents(id),
  start_date TEXT,
  stop_date TEXT,
  dose REAL,
  route_id INTEGER REFERENCES routes(id),
  schedule_time TEXT,
  number_of_drug REAL,
  missing INTEGER DEFAULT 0 CHECK (missing IN (0,1)),
  printed INTEGER DEFAULT 0 CHECK (printed IN (0,1)),
  rate TEXT,
  ordering_no INTEGER,
  running_no INTEGER,
  running_sum INTEGER,
  inventory_date TEXT
);
CREATE INDEX IF NOT EXISTS idx_order_items_active ON order_items(start_date, stop_date, missing);
CREATE INDEX IF NOT EXISTS idx_order_items_drug ON order_items(drug_id, start_date);

CREATE TABLE IF NOT EXISTS inventory_events (
  id INTEGER PRIMARY KEY,
  legacy_incode TEXT UNIQUE,
  drug_id INTEGER NOT NULL REFERENCES drugs(id),
  quantity REAL,
  event_date TEXT,
  inventory_ok INTEGER DEFAULT 0 CHECK (inventory_ok IN (0,1)),
  send_order INTEGER DEFAULT 0 CHECK (send_order IN (0,1)),
  legacy_user TEXT
);
CREATE INDEX IF NOT EXISTS idx_inventory_pending ON inventory_events(inventory_ok, send_order, event_date);

CREATE TABLE IF NOT EXISTS side_effect_catalog (
  id INTEGER PRIMARY KEY,
  legacy_secode TEXT UNIQUE,
  side_effect_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS side_effect_grades (
  id INTEGER PRIMARY KEY,
  adverse_event TEXT NOT NULL,
  short_name TEXT,
  grade1 TEXT,
  grade2 TEXT,
  grade3 TEXT,
  grade4 TEXT,
  grade5 TEXT,
  remark TEXT,
  also_consider TEXT
);

CREATE TABLE IF NOT EXISTS side_effect_records (
  id INTEGER PRIMARY KEY,
  order_id INTEGER REFERENCES orders(id),
  patient_id INTEGER NOT NULL REFERENCES patients(id),
  side_effect_id INTEGER REFERENCES side_effect_catalog(id),
  side_effect_date TEXT,
  drug_admin_date TEXT,
  management TEXT,
  suspected_drug TEXT,
  grade TEXT,
  recorder TEXT,
  record_time TEXT
);
CREATE INDEX IF NOT EXISTS idx_side_effect_patient ON side_effect_records(patient_id, side_effect_date);

CREATE TABLE IF NOT EXISTS drug_administration (
  id INTEGER PRIMARY KEY,
  patient_id INTEGER NOT NULL REFERENCES patients(id),
  drug_id INTEGER REFERENCES drugs(id),
  cycle INTEGER,
  administration_date TEXT,
  side_effect_flag INTEGER DEFAULT 0 CHECK (side_effect_flag IN (0,1)),
  details TEXT,
  recorder TEXT,
  record_time TEXT
);

CREATE TABLE IF NOT EXISTS pharmcare_soap (
  id INTEGER PRIMARY KEY,
  legacy_soapcode TEXT UNIQUE,
  patient_id INTEGER REFERENCES patients(id),
  problem TEXT,
  soap_date TEXT,
  recorder TEXT,
  note TEXT,
  problem_type TEXT,
  pcode TEXT
);

CREATE TABLE IF NOT EXISTS pharmcare_records (
  id INTEGER PRIMARY KEY,
  legacy_prcode TEXT UNIQUE,
  order_id INTEGER REFERENCES orders(id),
  patient_id INTEGER REFERENCES patients(id),
  visit_date TEXT,
  p1 INTEGER DEFAULT 0 CHECK (p1 IN (0,1)),
  p2 INTEGER DEFAULT 0 CHECK (p2 IN (0,1)),
  p3 INTEGER DEFAULT 0 CHECK (p3 IN (0,1)),
  p4 INTEGER DEFAULT 0 CHECK (p4 IN (0,1)),
  p5 INTEGER DEFAULT 0 CHECK (p5 IN (0,1)),
  p6 INTEGER DEFAULT 0 CHECK (p6 IN (0,1)),
  p7 INTEGER DEFAULT 0 CHECK (p7 IN (0,1)),
  p8 INTEGER DEFAULT 0 CHECK (p8 IN (0,1)),
  p9 INTEGER DEFAULT 0 CHECK (p9 IN (0,1)),
  note TEXT,
  user_practice TEXT,
  edit_practice TEXT
);

CREATE TABLE IF NOT EXISTS problems (
  id INTEGER PRIMARY KEY,
  legacy_procode TEXT UNIQUE,
  patient_id INTEGER REFERENCES patients(id),
  problem_code TEXT,
  problem_date TEXT,
  problem_time TEXT,
  note TEXT,
  cleared INTEGER DEFAULT 0 CHECK (cleared IN (0,1)),
  cleared_by TEXT
);

CREATE TABLE IF NOT EXISTS problem_catalog (
  id INTEGER PRIMARY KEY,
  legacy_problemcode TEXT UNIQUE,
  problem_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY,
  legacy_planid TEXT UNIQUE,
  patient_id INTEGER REFERENCES patients(id),
  topic TEXT,
  plan_text TEXT,
  plan_date TEXT,
  plan_by TEXT,
  edit_by TEXT,
  edit_date TEXT,
  inactive INTEGER DEFAULT 0 CHECK (inactive IN (0,1)),
  inactive_by TEXT
);

CREATE TABLE IF NOT EXISTS dtp_categories (
  id INTEGER PRIMARY KEY,
  legacy_id TEXT UNIQUE,
  category TEXT,
  subcategory1 TEXT,
  subcategory2 TEXT
);

CREATE TABLE IF NOT EXISTS interventions (
  id INTEGER PRIMARY KEY,
  legacy_intcode TEXT UNIQUE,
  patient_id INTEGER REFERENCES patients(id),
  intervention_date TEXT,
  dtp_id INTEGER REFERENCES dtp_categories(id),
  dtp_detail TEXT,
  intervention_to TEXT,
  intervention_type TEXT,
  intervention_detail TEXT,
  response TEXT,
  note TEXT,
  intervention_by TEXT
);

CREATE TABLE IF NOT EXISTS pharmacist_notes (
  id INTEGER PRIMARY KEY,
  legacy_ncode TEXT UNIQUE,
  patient_id INTEGER REFERENCES patients(id),
  note_date TEXT,
  note_time TEXT,
  note TEXT,
  hold INTEGER DEFAULT 0 CHECK (hold IN (0,1)),
  note_by TEXT,
  unhold_by TEXT
);

CREATE TABLE IF NOT EXISTS alert_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  note_alert INTEGER DEFAULT 1 CHECK (note_alert IN (0,1)),
  side_effect_alert INTEGER DEFAULT 1 CHECK (side_effect_alert IN (0,1)),
  soap_alert INTEGER DEFAULT 1 CHECK (soap_alert IN (0,1)),
  new_order_alert INTEGER DEFAULT 1 CHECK (new_order_alert IN (0,1)),
  cycle_alert INTEGER DEFAULT 1 CHECK (cycle_alert IN (0,1)),
  plan_alert INTEGER DEFAULT 1 CHECK (plan_alert IN (0,1)),
  platelet_threshold REAL,
  bilirubin_threshold REAL,
  creatinine_threshold REAL,
  hospital TEXT
);

CREATE TABLE IF NOT EXISTS alert_records (
  id INTEGER PRIMARY KEY,
  patient_id INTEGER REFERENCES patients(id),
  alert_code TEXT,
  alert_date TEXT,
  alert_type TEXT,
  management TEXT,
  current_user TEXT,
  view_note INTEGER DEFAULT 0 CHECK (view_note IN (0,1)),
  lab_result_date TEXT,
  lab_type TEXT
);

CREATE TABLE IF NOT EXISTS legacy_specialty_records (
  id INTEGER PRIMARY KEY,
  patient_id INTEGER NOT NULL REFERENCES patients(id),
  source_table TEXT NOT NULL CHECK (source_table IN ('CA Breast','CA Coloretal','DTPs','F/U schedule')),
  legacy_payload_json TEXT NOT NULL,
  migrated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS audit_log (
  id INTEGER PRIMARY KEY,
  occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  user_id INTEGER REFERENCES users(id),
  action TEXT NOT NULL,
  entity_type TEXT,
  entity_id TEXT,
  details_json TEXT
);

INSERT OR IGNORE INTO alert_settings(id) VALUES (1);
INSERT OR REPLACE INTO app_meta(key,value) VALUES ('schema_version','1');
