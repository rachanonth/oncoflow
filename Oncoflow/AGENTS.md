# OncoFlow Migration Project

## Goal

Migrate the legacy Microsoft Access oncology pharmacy application to a single-user desktop application named **OncoFlow** using:

- Tauri 2
- React
- TypeScript
- Rust
- SQLite

## Canonical source and target filenames

Use these names consistently in new code, scripts, tests, and documentation:

```text
legacy/Cytotoxic V8.0.mdb   # legacy Access front end; reference only
legacy/AllTable.mdb         # unlocked Access backend migration source
migrations/001_initial.sql  # compatibility-first SQLite migration
oncoflow.db                 # runtime SQLite database
```

The database-level password has been removed from `AllTable.mdb`.

The legacy front end may still contain the historical linked-table path:

```text
C:\Ctx\Tbl\All Table.mdb
```

and historical connection metadata:

```text
MS Access;PWD=table;
```

Treat those strings as legacy evidence only. Do not use them in new OncoFlow code.

If the legacy Access front end must remain operational during migration, relink it explicitly to `AllTable.mdb` or keep a compatibility copy at the old path. Do not silently alter legacy linkage.

## Core migration principle

Do not rewrite the Access application blindly.

Preserve behavior first:

1. inspect the relevant legacy objects;
2. document the rule;
3. create a parity test where possible;
4. implement equivalent behavior;
5. verify results against the legacy application;
6. refactor only after parity is demonstrated.

Clinical calculations must not be changed merely because a different formula appears more modern or elegant. Flag discrepancies for review.

## Architecture

Frontend (`src/`):

- React + TypeScript
- presentation and user interaction only

Backend (`src-tauri/src/`):

- Rust
- SQLite access
- business rules
- clinical calculations
- validation
- import/export

Database:

- SQLite
- canonical runtime filename: `oncoflow.db`

### Database architecture rule

OncoFlow is a fully local, single-user desktop application. All application data must be stored in the local SQLite database `oncoflow.db`.

Do not:

- connect to HOMC;
- connect to a hospital SQL Server;
- query legacy `dbo_*` tables;
- synchronize with external databases;
- create a `HospitalGateway` or `HomcGateway`;
- require network connectivity for normal operation.

Treat legacy references to external `dbo_*` tables as legacy-only evidence. Exclude them from the new architecture unless a later task explicitly changes this rule. `AllTable.mdb` is the sole legacy data source for migration into `oncoflow.db`.

Do not put clinical calculations in React components.
Do not execute arbitrary clinical SQL directly from UI components.
Use Rust commands/services as the application boundary.

## Target Rust modules

```text
src-tauri/src/
  auth/
  db/
  patient/
  drug/
  regimen/
  order/
  clinical/
  inventory/
  appointment/
  pharmcare/
  alert/
  report/
```

## Important clinical functions from Access

Investigate and preserve the behavior of legacy functions including:

- StandardDose
- FixNumber
- HomcDate
- FormatDate
- FormatDate543
- LabMinMax
- ApplyHN
- ApplyWFPlan
- ApplyPlan
- ApplyProblemWF
- ApplyProblem
- ApplyPharmRecWF
- ApplyPharmRec
- FindStart
- FindHOMC
- DilCompat
- ANCCal
- ANCGrade
- Platelet
- KeyDate
- Practice
- Findlast
- CheckPlan

Treat these as migration targets. Create tests before replacing them when the legacy behavior can be reproduced.

## Database strategy

The first SQLite schema is compatibility-first.

Do not aggressively normalize tables during the first migration phase. First achieve data and behavioral parity, then refactor in later migrations.

Always enable SQLite foreign keys:

```sql
PRAGMA foreign_keys = ON;
```

## Legacy external hospital/HOMC references

The Access application references historical hospital/HOMC objects such as:

- dbo_PATIENT
- dbo_OPD_H
- dbo_Ipd_h
- dbo_Appoint
- dbo_PATDIAG
- dbo_ICD101
- dbo_Labres_d
- dbo_Labres_m
- dbo_Med_inv
- dbo_InvReqH
- dbo_InvReqD
- dbo_medalery
- dbo_Ward
- dbo_docc

These names document legacy behavior only. Do not connect to them, convert them automatically into local SQLite tables, or create gateway abstractions for them. The new application must operate entirely from `oncoflow.db` without network connectivity.

## Security

- Do not commit real patient databases to Git.
- Do not log passwords, authentication secrets, or patient-identifying clinical data.
- Legacy application user passwords may be plaintext; do not reproduce plaintext password storage in OncoFlow.
- The removed MDB database password must not be reintroduced into new code or configuration.

## Development workflow

Before implementing a substantial feature:

1. inspect the relevant legacy object(s);
2. document assumptions;
3. implement a small vertical slice;
4. add tests;
5. run `cargo fmt`;
6. run `cargo test`;
7. run frontend type checking/lint/tests;
8. keep commits small and focused.

Do not modify either legacy MDB unless the task explicitly requires it. Prefer read-only access and disposable copies for migration experiments.

## Milestone 1

Milestone 1 is infrastructure, not a full oncology application:

1. Tauri application boots.
2. SQLite database initializes as `oncoflow.db`.
3. migrations run.
4. Rust DB layer works.
5. backend health/status command works.
6. patient list can be read from synthetic/test data.
7. patient detail can be displayed.
8. tests pass.

Do not implement chemotherapy calculations until their legacy behavior has been documented.
