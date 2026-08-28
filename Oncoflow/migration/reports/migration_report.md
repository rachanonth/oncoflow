# OncoFlow MDB migration report

This report contains counts and schema diagnostics only. It intentionally omits patient names, addresses, clinical notes, and passwords.

- Source: `AllTable.mdb`
- Source SHA-256: `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3`
- Destination: `oncoflow.db`
- Schema version: 2
- SQLite integrity check: `ok`
- SQLite foreign-key violations: 0

- Source tables inspected: 52
- Text cells decoded: 4635
- Thai text cells detected: 215
- Unicode replacement characters: 0

- Destination text cells: 4408
- Destination Thai text cells: 139
- Destination replacement characters: 0

- Destination NOT NULL violations: 0

## Table counts

| Source table | Destination table | Source | Imported | Skipped | Errors | Synthetic | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| ANLink | — | 0 | 0 | 0 | 0 | 0 | intentionally_excluded |
| AlertRec | alert_records | 1 | 1 | 0 | 0 | 0 | imported |
| Appdate | — | 1 | 0 | 1 | 0 | 0 | intentionally_excluded |
| Appoint | appointments | 1 | 1 | 0 | 0 | 0 | imported |
| Appoint details | appointment_items | 2 | 2 | 0 | 0 | 0 | imported |
| CA Breast | legacy_specialty_records | 0 | 0 | 0 | 0 | 0 | imported |
| CA Coloretal | legacy_specialty_records | 0 | 0 | 0 | 0 | 0 | imported |
| Change | — | 1 | 0 | 1 | 0 | 0 | intentionally_excluded |
| Change Details | — | 1 | 0 | 1 | 0 | 0 | intentionally_excluded |
| DTPs | legacy_specialty_records | 0 | 0 | 0 | 0 | 0 | imported |
| Drug Administration | drug_administration | 1 | 0 | 1 | 1 | 0 | imported_with_skips |
| F/U schedule | legacy_specialty_records | 1 | 1 | 0 | 0 | 0 | imported |
| Intervention | interventions | 1 | 1 | 0 | 0 | 0 | imported |
| InvIN | inventory_events | 1 | 1 | 0 | 0 | 0 | imported |
| MinMax | alert_settings | 1 | 1 | 0 | 0 | 0 | imported |
| PI | — | 5 | 0 | 5 | 0 | 0 | intentionally_excluded |
| PNote | pharmacist_notes | 0 | 0 | 0 | 0 | 0 | imported |
| PharmCareRec | pharmcare_records | 1 | 1 | 0 | 0 | 0 | imported |
| Pharmcare | pharmcare_soap | 1 | 1 | 0 | 0 | 0 | imported |
| Planning | plans | 1 | 1 | 0 | 0 | 0 | imported |
| PrescriptionDetails | — | 1 | 0 | 1 | 0 | 0 | intentionally_excluded |
| Problem | problems | 1 | 1 | 0 | 0 | 0 | imported |
| SideEffect | side_effect_records | 2 | 2 | 0 | 0 | 0 | imported |
| TblAlert | — | 14 | 0 | 14 | 0 | 0 | intentionally_excluded |
| TblAppCard | appointment_cards | 276 | 276 | 0 | 0 | 0 | imported |
| TblDTPCat | dtp_categories | 34 | 34 | 0 | 0 | 0 | imported |
| TblDiagnosis | diagnoses | 55 | 55 | 0 | 0 | 0 | imported |
| TblDrug Details1 | drug_detail_groups | 34 | 34 | 0 | 0 | 7 | imported |
| TblDrug Details2 | drug_detail_items | 202 | 202 | 0 | 0 | 0 | imported |
| TblECOG | — | 6 | 0 | 6 | 0 | 0 | intentionally_excluded |
| TblGradingSE | side_effect_grades | 55 | 55 | 0 | 0 | 0 | imported |
| TblIntTo | — | 8 | 0 | 8 | 0 | 0 | intentionally_excluded |
| TblIntType | — | 15 | 0 | 15 | 0 | 0 | intentionally_excluded |
| TblME | — | 52 | 0 | 52 | 0 | 0 | intentionally_excluded |
| TblOccupation | — | 3 | 0 | 3 | 0 | 0 | intentionally_excluded |
| TblProblem | problem_catalog | 3 | 3 | 0 | 0 | 0 | imported |
| TblRegimen | regimens | 85 | 85 | 0 | 0 | 5 | imported |
| TblResponse | — | 3 | 0 | 3 | 0 | 0 | intentionally_excluded |
| TblSideEffect | side_effect_catalog | 67 | 67 | 0 | 0 | 0 | imported |
| TblUser | users | 1 | 1 | 0 | 0 | 0 | imported |
| Tbldiluent | diluents | 50 | 50 | 0 | 0 | 0 | imported |
| Tbldoctor | doctors | 2 | 2 | 0 | 0 | 0 | imported |
| Tbldrug | drugs | 48 | 48 | 0 | 0 | 1 | imported |
| Tblpatient | patients | 2 | 2 | 0 | 0 | 1 | imported |
| Tblregimen details1 | regimen_groups | 90 | 90 | 0 | 0 | 9 | imported |
| Tblregimen details2 | regimen_items | 368 | 368 | 0 | 0 | 0 | imported |
| Tblroute | routes | 8 | 8 | 0 | 0 | 0 | imported |
| Tblunit | units | 3 | 3 | 0 | 0 | 0 | imported |
| Tblward | wards | 2 | 2 | 0 | 0 | 0 | imported |
| order | orders | 1 | 1 | 0 | 0 | 0 | imported |
| order details | order_items | 2 | 2 | 0 | 0 | 0 | imported |
| ราคายาเดิม | — | 24 | 0 | 24 | 0 | 0 | intentionally_excluded |

## Migration issues

- **warning / source_null_skipped** — row was skipped because required source column 'hn' is NULL
- **warning / resolved_specialty_patient_orphan** — created a synthetic patient parent for a specialty-only legacy record
- **warning / orphan_reference** — optional relationship 'order_id' was stored as NULL because no destination row exists (legacy identifier `14917`)
- **warning / orphan_reference** — optional relationship 'order_id' was stored as NULL because no destination row exists (legacy identifier `23406`)
- **warning / orphan_reference** — optional relationship 'order_id' was stored as NULL because no destination row exists (legacy identifier `23922`)
- **warning / required_text_placeholder** — NULL source value for 'detail' was replaced with a labeled compatibility placeholder
- **warning / required_text_placeholder** — NULL source value for 'detail' was replaced with a labeled compatibility placeholder
- **warning / required_text_placeholder** — NULL source value for 'detail' was replaced with a labeled compatibility placeholder
- **warning / required_text_placeholder** — NULL source value for 'detail' was replaced with a labeled compatibility placeholder
- **warning / required_text_placeholder** — NULL source value for 'detail' was replaced with a labeled compatibility placeholder
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `27`)
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `29`)
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `3`)
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `4`)
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `45`)
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `48`)
- **warning / resolved_legacy_orphan** — created a synthetic drug-detail group because the referenced legacy group row is missing (legacy identifier `50`)
- **warning / required_text_placeholder** — NULL source value for 'diluent_name' was replaced with a labeled compatibility placeholder
- **warning / resolved_legacy_orphan** — created a synthetic regimen because the referenced legacy master row is missing (legacy identifier `15`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen because the referenced legacy master row is missing (legacy identifier `34`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen because the referenced legacy master row is missing (legacy identifier `36`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen because the referenced legacy master row is missing (legacy identifier `37`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `104`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `126`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `136`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `38`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `67`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `70`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `75`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `78`)
- **warning / resolved_legacy_orphan** — created a synthetic regimen group because the referenced legacy group row is missing (legacy identifier `83`)

## Credential handling

Legacy plaintext passwords were not copied. Imported legacy users are disabled and contain a non-credential placeholder until a later authentication migration is explicitly designed.
