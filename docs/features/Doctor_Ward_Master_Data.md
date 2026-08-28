# Doctor and Ward Master Data

Status: implemented as a focused post-RC1 local master-data feature (2026-08-25). This is not a new clinical milestone and does not change any clinical rule.

## Scope and navigation

Authenticated local administrators can open the new sidebar group:

```text
Master data
├── Doctors
├── Wards
├── Diluents
├── Routes
└── Diagnosis
```

The two pages support local database-backed listing, Thai-aware search, creation, and editing. Doctor name and ward name are required; ward telephone is optional. Internal numeric database IDs and legacy codes are not shown in the UI.

The normal order lookups already read the same `doctors` and `wards` tables, so additions and corrections become available the next time an order form loads. Dropdowns continue to display option names only.

The Master data sidebar group is collapsed by default and uses the same accessible expand/collapse behavior as the Medication management and Settings groups. It expands automatically while either master-data page is active.

## Database and compatibility

No schema migration is required. The feature uses the established tables:

- `doctors(id, legacy_doccode, doctor_name)`
- `wards(id, legacy_wcode, ward_name, telephone)`

Schema version remains 9. Existing legacy codes remain preserved internally during name edits, but new records do not require them. Row identifiers are never silently renumbered. No delete command is provided because existing orders can reference these rows. Edits update the shared local master record; they do not update order rows or create external dependencies.

## Security, transactions, and privacy

All six typed commands—list/create/update for doctors and wards—require a Rust-controlled authenticated administrator session. The frontend cannot supply an acting user ID and cannot execute arbitrary SQL.

Creates and updates use an immediate SQLite transaction and append a corresponding audit event atomically:

- `doctor_created`
- `doctor_updated`
- `ward_created`
- `ward_updated`

Audit metadata is deliberately empty; it does not duplicate names, codes, or telephone values. A failed audit insert rolls back the master-data change. The module remains entirely local to `oncoflow.db`.

## Validation

- names are trimmed, required, and limited to 200 Unicode characters;
- optional legacy codes are trimmed, blank-to-NULL, limited to 50 characters, and duplicate codes are rejected case-insensitively;
- optional ward telephone is trimmed, blank-to-NULL, and limited to 100 characters; and
- Thai names are stored and searched as UTF-8 without conversion.

Synthetic tests cover Thai create/search/update, nullable values, duplicate codes, administrator enforcement, privacy-safe audit data, and transaction rollback. No MDB file is read or modified by this feature.

## Release validation

Validation completed on 2026-08-25:

- `cargo fmt --all -- --check`: passed;
- strict Clippy: passed;
- Rust tests: 165 passed;
- frontend tests: 65 passed across 19 files;
- frontend typecheck, lint, and production build: passed;
- Tauri release build and NSIS-only packaging: passed;
- tracked database/MDB files: zero; and
- both reference MDB SHA-256 hashes: unchanged.

The latest combined master-data installer is documented in `Diluent_Route_Master_Data.md`. The installed/running OncoFlow process and its AppData database were not interrupted or modified during the build.
