# AllTable.mdb to OncoFlow SQLite mapping

## Scope and authority

`legacy/AllTable.mdb` is the only data source for this import. `legacy/Cytotoxic V8.0.mdb` is reference-only and is not read by the importer. The importer does not query or import `dbo_*` linked tables and creates no network or hospital database dependency.

The exact inspected source inventory is embedded in `migration/reports/migration_report.json` under `source_schema`. For every one of the 52 local source tables it records:

- every source column in ordinal order;
- ACE/OLE DB data type and text size;
- source nullability;
- declared primary-key columns;
- source row count and import disposition.

This machine-generated inventory is the authority when it differs from older blueprint notes.

## Extraction architecture

The supported command is:

```powershell
pwsh -File migration/import_alltable.ps1 -Replace
```

The wrapper opens the MDB with `Microsoft.ACE.OLEDB.16.0` and `Mode=Read`. It filters `MSys*` and `dbo_*` names before reading rows, removes the `TblUser.password` column inside the reader process, and streams the remaining typed rows to the Rust loader over stdin. No raw CSV or JSON snapshot containing patient data is written to disk.

Rust creates a new temporary SQLite database, applies all migrations, imports inside one transaction, runs validation, and only then replaces the requested output. The original destination remains untouched if extraction, mapping, insertion, or validation fails.

## Type conversion rules

| Access value | SQLite representation | Rule |
|---|---|---|
| Yes/No | `INTEGER` | `False/No/0` → `0`; `True/Yes/1/-1` → `1`; other values fail conversion. |
| Date/Time | ISO-8601 `TEXT` | Dates use `YYYY-MM-DD`, timestamps use `YYYY-MM-DDTHH:MM:SS`, and time-only fields use `HH:MM:SS`. |
| Null | `NULL` | Preserved unless the destination is required; required exceptions are explicitly reported. |
| Byte/Integer/Long | `INTEGER` | Converted without renumbering when used as a legacy identifier. |
| Single/Double/Currency | `REAL` | Converted through the ACE numeric value, not locale-formatted text. |
| Text/Memo | UTF-8 `TEXT` | ACE returns Unicode; conversion is strict and aborts on invalid UTF-16, invalid UTF-8, or U+FFFD replacement characters. |

The inspected database contains Thai text. The current report records Thai-cell and replacement-character counts; the import aborts rather than silently replacing undecodable characters.

## Imported table and column mappings

Columns not listed in this section are either documented under compatibility/unmapped fields or are reported as intentionally excluded. Destination `id` values are set from numeric Access identifiers where safe. Text identifiers are preserved in `legacy_*` columns and resolved to SQLite foreign keys.

| Access source | SQLite destination | Column mapping |
|---|---|---|
| `TblUser` | `users` | `user` → `legacy_user`,`username`; `name` → `display_name`; role defaults to `user`; account is disabled. Password is not extracted. |
| `Tblunit` | `units` | `unitcode` → `legacy_unitcode`; `unitname` → `unit_name`. |
| `Tblroute` | `routes` | `rcode` → `legacy_rcode`; `rname` → `route_name`. |
| `Tbldiluent` | `diluents` | `dilcode` → `legacy_dilcode`; `dilname` → `diluent_name`; `vol` → `volume_ml`. |
| `Tbldoctor` | `doctors` | `doccode` → `legacy_doccode`; `docname` → `doctor_name`. |
| `Tblward` | `wards` | `wcode` → `legacy_wcode`; `wname` → `ward_name`; `tel` → `telephone`. |
| `TblDiagnosis` | `diagnoses` | `diagcode` → `id`,`legacy_diagcode`; `diagnosis`; `warning1`,`warning2` preserved as `0`/`1` text because the initial destination columns are text. |
| `TblRegimen` | `regimens` | `regcode` → `id`,`legacy_regcode`; `Regimen` → `regimen_name`; all seven Access flags → corresponding `INTEGER` flags. |
| `Tbldrug` | `drugs` | `dcode` → `legacy_dcode`; `dname`; unit/diluent/route codes → resolved IDs; pack, price, theory, defaults, warnings, storage, expiry, dose/dilution/cumulative/inventory fields → corresponding columns; `exp`,`reg` → compatibility columns. `HOMCCode` is preserved only as legacy text and creates no external dependency. |
| `Tblpatient` | `patients` | `hn` → `legacy_hn`; `xn` → `legacy_xn`; demographics, measurements, local diagnosis/regimen codes, counselling/history/cancer fields, allergy and recorder fields → corresponding columns; `endtreat`,`age`,`bsa`,`sex`,`tel` → compatibility columns. |
| `TblDrug Details1` | `drug_detail_groups` | `code` → `id`,`legacy_code`; `dcode` → resolved `drug_id`; `note`. |
| `TblDrug Details2` | `drug_detail_items` | `code` → resolved `detail_group_id`; `detail`. |
| `Tblregimen details1` | `regimen_groups` | `code` → `id`,`legacy_code`; `regcode` → resolved `regimen_id`; `note`,`cday`,`ncycle`. |
| `Tblregimen details2` | `regimen_items` | `code` → resolved group; `drug` → resolved drug; `dose` → parsed numeric `dose` plus lossless `legacy_dose_text`; unit, route, details, group, duration, start/order fields and defaults → corresponding columns. |
| `Appoint` | `appointments` | `appid` → `id`,`legacy_appid`; HN/diagnosis/regimen → resolved IDs; appointment/record dates and user → corresponding columns. |
| `Appoint details` | `appointment_items` | `appid` → resolved appointment; `adcode` → resolved drug; `adose`,`adetail`. |
| `TblAppCard` | `appointment_cards` | `ccode` → `legacy_ccode`; `Regimen` → resolved regimen; `aCyc`,`aDay`,`Day_no`. |
| `order` | `orders` | `orderid` → `id`,`legacy_orderid`; HN/ward/doctor/regimen → resolved IDs; appointment flag, note, dates, recorder, type and legacy worker/side-effect/medication-error text → corresponding compatibility columns. The initial numeric `worker` and side-effect flag are not inferred from user-code/text fields. |
| `order details` | `order_items` | order/drug/diluent/route codes → resolved IDs; start/stop/dose/time/quantity/flags/rate/order/run/inventory fields → corresponding columns. Milestone 6 adds nullable raw-dose/regimen snapshot fields for new local orders only; imported rows remain unchanged. |
| `InvIN` | `inventory_events` | `Incode` → `legacy_incode`; `dcode` → resolved drug; quantity/date/status flags/user → corresponding columns. |
| `TblSideEffect` | `side_effect_catalog` | `SEcode` → `id`,`legacy_secode`; `sideE` → `side_effect_name`. |
| `TblGradingSE` | `side_effect_grades` | `ID` → `id`; adverse event, short name, grades 1–5, remark and also-consider text → corresponding columns. |
| `SideEffect` | `side_effect_records` | `orderid` → `id` and optional resolved order; HN/side-effect code → resolved IDs; dates, management, suspected drug, grade and recorder fields → corresponding columns. Old memo duplicates are not mapped. |
| `Drug Administration` | `drug_administration` | HN → resolved patient; cycle/date/side-effect/details/recorder fields → corresponding columns. Source has no drug code, so `drug_id` remains NULL. |
| `Pharmcare` | `pharmcare_soap` | `SOAPcode` → `id`,`legacy_soapcode`; HN → patient; problem/date/recorder/note/type plus `S`,`O`,`A`,`P` → explicit compatibility columns. |
| `PharmCareRec` | `pharmcare_records` | `PRcode` → `id`,`legacy_prcode`; order/HN → optional resolved IDs; visit, P1–P9, note and practice fields → corresponding columns. |
| `TblProblem` | `problem_catalog` | `Problemcode` → `id`,`legacy_problemcode`; `problemname`. |
| `Problem` | `problems` | `procode` → `id`,`legacy_procode`; HN → patient; problem code/date/time/by/note/clear fields → corresponding columns. |
| `Planning` | `plans` | `planID` → `id`,`legacy_planid`; HN → patient; topic/plan/date/by/edit fields; `Unhold`/`UnholdBy` follow the initial compatibility schema's inactive fields and remain flagged for workflow review. |
| `TblDTPCat` | `dtp_categories` | `ID` → `id`,`legacy_id`; category and two subcategories. |
| `Intervention` | `interventions` | `IntCode` → `id`,`legacy_intcode`; HN/DTP → resolved IDs; date, detail, target/type/response codes, note/by and performed flag → corresponding columns. |
| `PNote` | `pharmacist_notes` | `ncode` → `id`,`legacy_ncode`; HN → patient; date/time/note/hold/by fields → corresponding columns. |
| `MinMax` | `alert_settings` | alert flags, label number and WBC/ANC/platelet/Hb/AST/bilirubin/creatinine thresholds plus hospital label → singleton settings row. Values are migrated only; no clinical interpretation is implemented. |
| `AlertRec` | `alert_records` | HN → optional patient; alert code/date/type/management/user/view/lab fields → corresponding columns. `HNTypLabDate` is not confidently understood and is not mapped. |
| `CA Breast`, `CA Coloretal`, `DTPs`, `F/U schedule` | `legacy_specialty_records` | Every non-password field is preserved as JSON with an explicitly resolved or synthetic patient parent. No clinical interpretation is applied. |

## Declared source keys and relationships

The exact primary keys are in the report inventory. Important declared keys include `Tblpatient.hn`, `Tbldrug.dcode`, `TblDiagnosis.diagcode`, `TblRegimen.regcode`, `order.orderid`, `Appoint.appid`, `TblUser.user`, and the master code fields for unit, route, diluent, doctor, ward and response lookups.

ACE reports declared relationships for patient→order, order→order details, appointment→appointment details, drug/diluent/route→order details, doctor/ward→order, regimen→patient, diagnosis→appointment, unit→drug, and specialty/admission patient links. The importer additionally validates all SQLite relationships with `PRAGMA foreign_key_check`.

## Explicit legacy repairs

The source contains relationship violations. They are not silently discarded:

- missing regimen masters referenced by regimen groups receive labeled synthetic regimen rows;
- missing regimen groups referenced by regimen items receive labeled synthetic group rows under one synthetic unresolved parent;
- missing drug-detail groups receive labeled synthetic group rows under one synthetic unresolved drug;
- a specialty-only patient identifier receives a labeled synthetic patient row;
- optional missing order references are stored as NULL and reported;
- one NULL diluent name and NULL drug-detail descriptions receive labeled compatibility placeholders;
- a completely unusable drug-administration row with no HN is skipped and reported.

The generated report lists identifiers only where required to reconcile non-patient master-data orphans. It never contains patient names, addresses, notes, or passwords.

## Intentionally excluded local tables

| Table | Reason |
|---|---|
| `ANLink` | Empty admission-link staging table; no local destination. |
| `Appdate` | Temporary appointment workflow state; canonical appointments come from `Appoint`. |
| `Change`, `Change Details` | Legacy inventory-change workflow has no compatible destination. |
| `PI` | Semantics are undocumented and no compatible destination exists. |
| `PrescriptionDetails` | Legacy medication-error structure has no compatible destination. |
| `TblAlert` | Alert-name lookup has no compatible destination. |
| `TblECOG` | ECOG lookup has no compatible destination. |
| `TblIntTo`, `TblIntType`, `TblResponse` | Codes remain on imported interventions; lookup-name tables have no destination. |
| `TblME` | Medication-error lookup has no compatible destination. |
| `TblOccupation` | Occupation text is preserved directly on each patient. |
| `ราคายาเดิม` | Historical price archive; the current drug master comes from `Tbldrug`. |

These exclusions and their row counts are present in every migration report. Any future decision to model them requires a separate reviewed schema migration.

## Fields requiring later review

- `Planning.Unhold` semantics were inherited from the initial `inactive` mapping and need workflow verification before write functionality.
- `AlertRec.HNTypLabDate` is not confidently understood and is not mapped.
- `SideEffect.SideEffect_old` and `SideEffect.SideEffect_old2` appear to be superseded memo fields and are not mapped.
- `Tblpatient.save`, `Tblpatient.age`, and `Tblpatient.bsa` include Access workflow/derived state. Age and BSA are preserved as legacy values only; they are not recalculated.
- Non-numeric regimen dose expressions are preserved in `legacy_dose_text`; no dose calculation or interpretation occurs in this milestone.
- Legacy user group `1` has no reviewed role definition. The username/display name are retained, role defaults to `user`, and the account remains disabled.
