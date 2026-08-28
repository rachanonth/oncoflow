# OncoFlow Migration Blueprint

Legacy source: Cytotoxic V8.0 -> Tauri 2 + React + TypeScript + Rust + SQLite

## 1. Scope and source files

Analyzed legacy files:

- `Cytotoxic V8.0.mdb` — Access front end/application database (~14 MB)
- `AllTable.mdb` — canonical migration copy of the Access data/backend database (~1.5 MB); database password has been removed

The front end contains forms, reports, saved queries, menu/toolbar definitions, VBA string pools, and links to the data database. The data MDB contains the clinical/application tables and relationships.

A legacy linked-table path is embedded in the front end:

```text
C:\Ctx\Tbl\All Table.mdb
```

The linked Access connection metadata contains:

```text
MS Access;PWD=table;
```

This confirms the front end was designed as a classic Access split database: application MDB + linked data MDB.

### Current migration filename and password state

For OncoFlow migration work, the backend database is now standardized as:

```text
AllTable.mdb
```

The database-level password has been removed from this working copy. The strings `C:\Ctx\Tbl\All Table.mdb` and `MS Access;PWD=table;` above are preserved because they describe the **legacy front-end linkage**, not the current migration file.

If `Cytotoxic V8.0.mdb` must continue to run during the migration, its linked tables must either be relinked to `AllTable.mdb` or a compatibility copy must remain at the legacy path/name. Do not silently modify the legacy front end without recording that change.

---

## 2. High-level conclusion

For a single-user replacement, SQLite is appropriate for the local application data. The safest migration architecture is:

```text
React + TypeScript UI
        |
        | Tauri invoke()
        v
Rust application/domain layer
  |-- auth
  |-- patients
  |-- orders
  |-- regimens
  |-- clinical rules
  |-- pharmacy care
  |-- inventory
  |-- alerts
  |-- reporting
  |-- hospital/HOMC adapter
        |
        +--> SQLite: oncoflow.db
        +--> external hospital/HOMC source (read-only adapter where required)
```

Important: this application is not self-contained. The front end references many `dbo_*` objects from a separate hospital/HOMC system. SQLite replaces the local backend represented by `AllTable.mdb`; it does **not** automatically replace the external hospital source.

---

## 3. Legacy application object inventory

### Module-backed forms detected

At least 54 module-backed forms were recoverable from the front-end MDB, plus additional no-module subforms/forms in the Access object catalog.

Major forms include:

- Login
- Order
- order details subform / old version
- Nurse
- AdmitPatient / AdmitPatientNew / AdmitPatientWait / AdmitOPDPatient
- PharmCareMain
- PharmRec / PharmRecSubform / PharmrecSubFFMain
- SOAP Main
- Problem
- Planning
- Side Effect
- Alert
- INVCheck
- Protocol
- Standard dose
- Review Data / Review Data By Drug
- Data of All Patient
- AppCardNew
- Appdate / AppPatient
- HOMCMed
- NewLab / LabSubform
- Patho
- Search
- Change / Change Password
- User
- Edit Patient
- Edit Drug
- Edit Regimen
- Edit Doctor
- Edit Ward
- Edit Route
- Edit Unit
- Edit Diluent
- Edit diagnosis
- Edit SE

### Reports detected

Module-backed reports include:

- Validation
- Drug Use by Ward
- Number Each Drug Use
- Standard Treatment Protocol
- Patient History / old version
- Pharmacist note
- Print Continue Label
- Review data for Print
- Review side effect
- Medication Profile Cont subform

The Access catalog also contains additional reports including medication profiles, drug/diluent usage, one-day/continuous print labels, return-drug reporting, workload summaries, patient counts, monthly reports, and appointment-card reports.

---

## 4. Menu and workflow structure

The embedded Access menu/toolbar definitions show these top-level workflows.

### Operational

- Order
- Change
- Pharmaceutical Care
- Problem
- Print Continue
- Nurse
- Alerting System
- Logout/Login

### Configuration / master data

- Edit Patient
- Edit Drug
- Edit Doctor
- Edit Ward
- Edit Route
- Edit Unit
- Edit Diluent
- Edit diagnosis
- Edit Regimen / Treatment Protocol
- Appointment Card
- Side Effect setup
- User / Change Password

### Review / reporting

- Data of All Patient
- Review Data By Drug
- INVCheck
- Validation
- DisplayDetailandSummary
- Return drug
- Cytotoxic drug information
- Diluent/stability information

This should map to a much simpler Tauri navigation model rather than reproducing Access menus exactly.

Recommended UI routes:

```text
/login
/dashboard
/patients
/patients/:id
/orders
/orders/:id
/regimens
/drugs
/inventory
/pharm-care/:patientId
/appointments
/alerts
/reports
/settings/*
```

---

## 5. Core local data model recovered from `AllTable.mdb`

### Patient

Legacy `Tblpatient` fields include, among others:

```text
HN
HN3
HN4
CAno
bname
fname
lname
weight
high
endtreat
birthdate
occup
address
diagcode
regcode
appcard
counselling
PHistory
Stage
Her2
ERPR
Allergy
record
recordtime
```

The patient table also participates in relationships to diagnosis, regimen, orders, cancer-specific records, and AN linking.

### Drug master

Legacy `Tbldrug` includes:

```text
dcode
dname
unitcode
dose/pack
vol/pack
package
Detail
price
theory
marker
dfdiluent
dfroute
dfrate
warning
storage
flag
exptime
expstore
Maxdose
MaxDilAlert
MaxDilH
CumAlert
CumAlertH
Dil_Incompat
InvCut
InvMin
InvMax
InvUse
HOMCCode
```

This table mixes master data, preparation defaults, clinical safety limits, and inventory configuration.

### Orders

Legacy `order` includes:

```text
orderid
wcode
doccode
worker
edit worker
note
ordertime
SideEffect
SErecorder
SErecordtime
regcode
Type
```

Legacy `order details` includes:

```text
orderid
dcode
dilcode
start
stop
dose
rcode
time
noofdrug
missing
print
rate
ordering
running
runsum
InvDate
```

This is a conventional order-header/order-line model and maps cleanly to SQLite.

### Regimens

`TblRegimen` contains regimen metadata and alert/configuration flags.

`Tblregimen details1` appears to represent regimen group/header-level detail, including fields such as:

```text
regcode
code
note
cday
ncycle
```

`Tblregimen details2` contains individual treatment items:

```text
code
drug
dose
unit
route
details
group
duration
StartD
ordering
dfdiluent
dfroute
dfrate
```

### Inventory

`InvIN` includes:

```text
Incode
dcode
In_no
Indate
InvOK
SendOrder
user
```

Inventory limits and enablement live in `Tbldrug` (`InvCut`, `InvMin`, `InvMax`, `InvUse`, and an inventory quantity field referenced by the application).

### Pharmaceutical care / clinical documentation

Recovered local tables include:

- `Pharmcare`
- `PharmCareRec`
- `Problem`
- `Planning`
- `PNote`
- `DTPs`
- `TblDTPCat`
- `Intervention`
- `TblIntType`
- `TblIntTo`
- `TblResponse`
- `SideEffect`
- `TblSideEffect`
- `TblGradingSE`
- `Drug Administration`
- `AlertRec`
- `MinMax`

There are also breast- and colorectal-cancer-specific legacy tables and follow-up scheduling data.

---

## 6. Relationships recovered

Important relationships embedded in the backend include:

```text
Tblpatient.hn            -> order.hn
order.orderid            -> order details.orderid
Tbldrug.dcode            -> order details.dcode
Tbldiluent.dilcode       -> order details.dilcode
Tblroute.rcode            -> order details.rcode
Tbldoctor.doccode        -> order.doccode
Tblward.wcode            -> order.wcode
TblDiagnosis.diagcode    -> Tblpatient.diagcode
TblRegimen.regcode       -> Tblpatient.regcode
Tblunit.unitcode         -> Tbldrug.unitcode
Appoint.appid            -> Appoint details.appid
Tblpatient.hn            -> ANLink.hn
Tblpatient.hn            -> CA Breast.hn
Tblpatient.hn            -> CA Coloretal.hn
```

These should become explicit SQLite foreign keys rather than relying on Access form logic.

---

## 7. External hospital/HOMC integration

The front-end MDB references external `dbo_*` objects. Confirmed names include:

```text
dbo_PATIENT
dbo_Ipd_h
dbo_OPD_H
dbo_Appoint
dbo_PATDIAG
dbo_ICD101
dbo_Ward
dbo_docc
dbo_Med_inv
dbo_Patmed
dbo_InvReqH
dbo_InvReqD
dbo_Lab
dbo_Labre_s
dbo_Labres_d
dbo_Labres_m
dbo_Labtype
dbo_medalery
dbo_XresHis
```

The Access app uses these for:

- patient demographics
- IPD admissions
- OPD visits
- hospital appointments
- diagnoses / ICD data
- ward and physician information
- current/home medication
- medication requests
- laboratory results
- pathology/text results
- medication allergy information

### Migration recommendation

Create a Rust trait/interface so the application does not depend on a specific ODBC implementation throughout the codebase:

```rust
trait HospitalGateway {
    fn patient(&self, hn: &str) -> Result<Option<HospitalPatient>>;
    fn admissions(&self, hn: &str) -> Result<Vec<Admission>>;
    fn appointments(&self, hn: &str) -> Result<Vec<HospitalAppointment>>;
    fn medications(&self, hn: &str) -> Result<Vec<Medication>>;
    fn labs(&self, hn: &str) -> Result<Vec<LabResult>>;
    fn diagnoses(&self, hn: &str) -> Result<Vec<Diagnosis>>;
    fn allergies(&self, hn: &str) -> Result<Vec<Allergy>>;
}
```

Implement the actual hospital connector separately. Keep it read-only unless there is a documented write requirement.

---

## 8. Business and clinical logic requiring parity tests

The VBA string pool exposes a central `Function` module with many reusable functions. Detected names include:

```text
StandardDose
FixNumber
HomcDate
FormatDate543
LabMinMax
ApplyHN
ApplyWFPlan
ApplyPlan
ApplyProblemWF
ApplyPharmCare
ApplyPharmCareWF
ApplyProblem
ApplyPharmRecWF
ApplyPharmRec
ApplySE
EditOrder
FindStart
FindHOMC
ANCCal
ANCGrade
HOMCDISC
Platelet
KeyDate
Prac
Practice
Findlast
CheckPlan
OpenOrder
ClearProblem
StopDate
CVCPlus
FindRegist
FindNPt
CheckPatho
AppD
FindCardio
CardioShow
DxMsg
AppFdate
CSLCont
```

These are migration-critical because the new UI can look correct while silently producing different clinical results.

### Confirmed rule categories

#### Standard dose / BSA

Saved queries call:

```text
standarddose([dose],[bsa])
```

Patient weight, height, age, and BSA are used throughout treatment views.

#### ANC

The VBA module contains `ANCCal` and `ANCGrade` and queries hospital CBC results for WBC and neutrophil values. This must be ported with test vectors captured from Access.

#### Platelets

A `Platelet` function queries CBC platelet values and is used in alert/clinical logic.

#### Dilution compatibility

The function pool contains `DilCompat`, with string markers indicating checks involving Dextrose and NSS. Drug master fields also include `Dil_Incompat`, `MaxDilAlert`, and `MaxDilH`.

#### Maximum / cumulative dose

Drug fields include:

```text
Maxdose
CumAlert
CumAlertH
```

Queries aggregate total dose by patient and drug, including BSA-normalized cumulative values.

#### Inventory replenishment

The login flow previously supplied shows a rule equivalent to:

```text
if current_inventory <= inventory_min and inventory_enabled:
    item requires replenishment
```

It also checks for a pending `InvIN` record where `InvOK = false` and `SendOrder = true` before opening the inventory-check workflow.

#### Thai Buddhist/Gregorian date conversion

Functions and expressions include `HomcDate`, `FormatDate543`, `Y543`, and `KeyDate`. The new application should store dates internally as Gregorian ISO-8601 and convert only at presentation/integration boundaries.

---

## 9. Legacy security findings

`TblUser` has fields:

```text
user
name
password
group
```

The legacy login VBA compares the entered password directly with the stored value. The front-end also contains a hard-coded `user` / `password` login path in one event handler.

Do not migrate this behavior.

New design:

```text
users
- id
- username
- display_name
- password_hash
- role
- active
```

Use a modern password hash in the Rust layer and never expose hashes to the frontend.

For a genuinely single-user application, another valid option is to remove application-level login entirely and rely on the workstation account, but only if the existing audit/role behavior is not required.

---

## 10. Tauri module layout

Recommended Rust structure:

```text
src-tauri/src/
  lib.rs
  state.rs
  db/
    mod.rs
    migrations.rs
    repositories/
  auth/
    mod.rs
  patients/
    mod.rs
    model.rs
    service.rs
  drugs/
  regimens/
  orders/
  inventory/
  appointments/
  pharmcare/
  alerts/
  clinical/
    dose.rs
    anc.rs
    platelet.rs
    dilution.rs
    cumulative_dose.rs
    dates.rs
  hospital/
    mod.rs
    gateway.rs
    models.rs
    legacy_connector.rs
  reports/
  audit/
```

Recommended frontend:

```text
src/
  routes/
  features/
    auth/
    patients/
    orders/
    drugs/
    regimens/
    inventory/
    pharmcare/
    appointments/
    alerts/
    reports/
  components/
  api/
    commands.ts
  types/
```

### Boundary rule

Clinical calculations, validation, persistence, password verification, and hospital integration belong in Rust. React should handle interaction and presentation.

---

## 11. SQLite strategy

A compatibility-first initial SQLite schema has been produced as `001_initial.sql`.

It contains 37 application tables and explicit foreign keys for the main domain. It intentionally retains `legacy_*` identifiers to make Access-to-SQLite reconciliation possible.

A blank database generated from this migration is included as `oncoflow_empty.db`.

### Why compatibility first

Do not immediately redesign every Access table into an ideal normalized model. First:

1. migrate all records without loss;
2. reproduce Access behavior;
3. parity-test clinical calculations and reports;
4. run old and new versions side-by-side;
5. refactor schema only after parity is demonstrated.

This is safer for a clinical application than mixing behavioral changes with platform migration.

---

## 12. Suggested migration pipeline

```text
AllTable.mdb
     |
     v
Access extraction tool
     |
     +--> raw CSV/JSON snapshots
     |
     v
migration-transform
     |
     v
oncoflow.db
```

Recommended staging layout:

```text
migration/
  raw/
    Tblpatient.csv
    Tbldrug.csv
    TblRegimen.csv
    Tblregimen_details1.csv
    Tblregimen_details2.csv
    order.csv
    order_details.csv
    ...
  transform/
  reports/
    row_counts.json
    rejected_rows.csv
    foreign_key_errors.csv
    reconciliation.json
```

Never make the production MDB the only copy used during migration. Work from read-only copies and retain cryptographic hashes of the source files.

---

## 13. Data migration order

Recommended order to satisfy foreign keys:

```text
1. users
2. units / routes / diluents / doctors / wards
3. diagnoses
4. regimens
5. drugs
6. patients
7. regimen groups/items
8. appointments + appointment items
9. orders + order items
10. inventory
11. side effects / grading
12. pharmaceutical care / problems / planning / interventions
13. alerts and notes
14. specialty cancer/follow-up tables
```

After each stage, compare legacy and new row counts and validate foreign-key coverage.

---

## 14. Report migration

Do not port Access report definitions mechanically. Classify them by output type:

### Operational labels

Examples:

- Print Continue Label
- Print Label
- one-day labels

These should be implemented with a deterministic print/PDF template and test fixtures.

### Clinical profiles

Examples:

- Patient History
- Medication Profile
- Pharmacist note

Implement as structured printable views / PDF outputs.

### Aggregate/statistical reports

Examples:

- Drug Use by Ward
- Number Each Drug Use
- monthly report by drug/ward
- workload summary

Implement as parameterized SQL queries over SQLite and export to PDF/CSV as needed.

---

## 15. First parity-test suite

Before replacing Access, create fixture patients/orders and assert these behaviors against Access and Rust:

1. Standard dose for representative BSA values.
2. ANC calculation from WBC + neutrophil input.
3. ANC grading boundaries.
4. Platelet alert boundaries.
5. Maximum single dose alert.
6. Cumulative dose soft/hard alerts.
7. Dilution compatibility combinations.
8. Default diluent, route, and infusion rate.
9. Active-order date logic (`start <= today <= stop`, not missing).
10. Inventory minimum/reorder behavior.
11. Appointment-card cycle/day generation.
12. Thai/Gregorian date conversion around year boundaries.
13. Side-effect/adverse-event grade lookup.
14. Alert status / acknowledgement behavior.
15. Report/label drug ordering and sequence numbers.

No clinical calculation should be considered migrated solely because it compiles.

---

## 16. Items that still require exact extraction

The low-level inspection recovered object names, relationships, many record-source SQL statements, linked paths, table/field names, and VBA function identifiers. Before executing the actual data move, perform a structured Access dump to capture:

- exact Access column types, sizes, defaults, required/nullability rules
- exact row counts
- all indexes
- every saved query definition
- full VBA source text, not only recoverable string pools
- macro definitions
- report layout measurements
- external `dbo_*` connection configuration/DSN
- all current user data

The current blueprint deliberately marks inferred mappings as migration design, not as proof of exact legacy types.

---

## 17. Recommended implementation sequence

### Milestone 1 — migration foundation

- create Tauri application shell
- add SQLite migrations
- implement repository layer
- create MDB extraction/export script on a Windows machine with Access/ACE, or use a compatible extractor
- produce raw snapshots and reconciliation report

### Milestone 2 — read-only clinical viewer

- patient search
- patient summary
- diagnosis/regimen
- treatment history
- current orders
- lab/HOMC read-only integration

### Milestone 3 — order workflow

- order entry
- regimen-to-order generation
- dose/dilution/rate defaults
- clinical alerts
- print labels

### Milestone 4 — pharmaceutical care

- SOAP
- problems
- DTP/interventions
- planning
- side effects
- pharmacy care record

### Milestone 5 — inventory and reporting

- inventory checks/reorder
- usage summaries
- validation report
- patient/medication profiles

### Milestone 6 — cutover

- complete parity tests
- frozen MDB backup
- final delta migration
- read-only legacy period
- retire Access only after reconciliation

---

## 18. Canonical OncoFlow source names

Use these names consistently in Codex, scripts, documentation, and migration commands:

```text
legacy/Cytotoxic V8.0.mdb   # legacy Access front end; reference only
legacy/AllTable.mdb         # unlocked backend migration source
migrations/001_initial.sql  # SQLite schema migration
oncoflow.db                 # runtime SQLite database
```

Do not reintroduce the old backend filename `All Table(1).mdb` in new code. The legacy embedded path `C:\Ctx\Tbl\All Table.mdb` may still appear in reverse-engineering notes because it is part of the original Access application metadata.

---

## 19. Deliverables produced in this analysis

- `OncoFlow_Migration_Blueprint.md` — this migration specification
- `access_object_inventory.csv` — detected forms/reports/functions/external-table/menu metadata
- `001_initial.sql` — compatibility-first SQLite schema
- `oncoflow_empty.db` — validated blank SQLite database created from the schema

The next engineering task is the **extract-transform-load utility** from `AllTable.mdb` into this SQLite schema, followed by parity tests for the `Function` module.
