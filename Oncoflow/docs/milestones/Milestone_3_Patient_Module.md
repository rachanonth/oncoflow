# Milestone 3 — Patient module

## Scope

Milestone 3 implements the first runtime workflow over the local `oncoflow.db`:

```text
Patient list → database search → patient detail → create/edit patient
```

It does not implement orders, regimen authoring, clinical calculations, inventory, authentication conversion, networking, or external integration.

## Data and schema decision

The existing Milestone 2 `patients` table is the source of truth. Inspection found:

- `legacy_hn TEXT NOT NULL UNIQUE` for the clinical HN;
- nullable legacy identity, demographic, measurement, and clinical fields;
- local foreign keys to `diagnoses` and `regimens`;
- the existing name index and unique HN index;
- `record_time` for the last recorded/updated value;
- three migrated development records, including the compatibility patient created for a legacy orphan;
- `PRAGMA integrity_check = ok` and no foreign-key violations.

No schema migration was required. Existing IDs and all compatibility records remain unchanged. New and edited records trim accidental whitespace, use the existing SQLite-generated internal ID, preserve the entered HN, and update `record_time`.

## Rust boundary

The patient module is separated into:

- `model.rs`: command DTOs and patient representations;
- `repository.rs`: allow-listed SQLite queries only;
- `service.rs`: trimming, validation, duplicate handling, and transactions;
- `commands.rs`: typed Tauri IPC commands and privacy-safe errors.

Commands:

- `list_patients`
- `get_patient`
- `create_patient`
- `update_patient`
- `patient_form_options`

List queries select only summary fields. Large text values such as address, allergy, and patient history are loaded only for a single detail record. Search is performed in SQLite against HN, first name, and last name. `%` and `_` in user searches are treated literally, and Thai Unicode text is passed through unchanged.

## Validation and write behavior

- HN is required, trimmed, limited to 64 characters, and checked case-insensitively for duplicates.
- Empty optional text is stored as `NULL`.
- Birth and treatment-end dates must be valid `YYYY-MM-DD` calendar dates.
- Weight must be greater than zero and no more than 500 kg when supplied.
- Height must be greater than zero and no more than 300 cm when supplied.
- Diagnosis and regimen IDs must exist in their local lookup tables.
- Create and update execute in `BEGIN IMMEDIATE` transactions.
- Failed validation, duplicate detection, missing lookup data, and missing patient updates do not partially modify a record.
- HN and SQLite patient ID are never silently renumbered.

Database errors returned to the UI are generic. The backend does not log patient names, addresses, histories, allergies, or other row contents.

## User interface

OncoFlow opens on the Patients workspace. It provides:

- debounced database-side search;
- sortable HN, patient name, and update columns;
- loading, empty, and error states;
- row selection, double-click opening, Enter-key opening, and an explicit open button;
- logically grouped patient details;
- create and edit forms using local diagnosis/regimen master data;
- field-level and server validation feedback;
- System Status under Settings.

Only implemented navigation destinations are displayed.

## Synthetic test coverage

Rust tests use temporary SQLite databases and synthetic records to cover:

- lookup by HN;
- HN and Thai-name search;
- create and update;
- duplicate HN rejection;
- nullable optional fields;
- strict dates and numeric limits;
- invalid lookup rollback;
- preservation of internal patient IDs during edits.

No real patient values are present in tests, fixtures, snapshots, or documentation.

Frontend unit tests cover required HN validation, date and measurement validation,
NULL conversion, whitespace trimming, and Thai-name preservation. They use synthetic
values only.
