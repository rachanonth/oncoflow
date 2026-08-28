# Diagnosis Master Data

Status: implemented as a focused post-RC1 local master-data feature (2026-08-25). This is a usability extension only; it adds no clinical rule or external dependency.

## Navigation and workflow

Authenticated local administrators can open:

```text
Master data
└── Diagnosis
```

The page supports SQLite-backed listing, Thai UTF-8 search, creation, and inline name editing. It displays the diagnosis name only. Internal numeric IDs, compatibility codes, and legacy warning fields are not exposed in the UI.

The patient create/edit form already loads its diagnosis options from the same local `diagnoses` table. New or renamed entries therefore appear the next time patient-form options are loaded, and the dropdown continues to display only the option name.

## Compatibility boundary

No schema migration is required and schema version remains 9. The established compatibility table remains:

```text
diagnoses(id, legacy_diagcode, diagnosis, warning1, warning2)
```

`legacy_diagcode` is deliberately absent from the Diagnosis management DTOs, commands, search query, editor, and list. The database column remains physically present to preserve migrated relationships and provenance. Creating a diagnosis leaves it NULL; renaming an existing diagnosis updates only `diagnosis` and preserves `legacy_diagcode`, `warning1`, and `warning2` byte-for-byte.

The hidden warning fields are not reinterpreted as safety behavior. There is no delete command because patient and regimen records may reference diagnosis rows. Existing IDs are never renumbered.

## Authentication, audit, and privacy

The typed `list_diagnoses`, `create_diagnosis`, and `update_diagnosis` commands require a Rust-controlled administrator session. The frontend cannot provide an actor ID or arbitrary SQL.

Creates and edits run in immediate SQLite transactions with an append-only audit event:

- `diagnosis_created`
- `diagnosis_updated`

Audit metadata is empty and does not duplicate the diagnosis name or any compatibility fields. An audit failure rolls back the diagnosis change.

## Validation and tests

Names are trimmed, required, and limited to 200 Unicode characters. Synthetic Rust tests verify Thai create/search/update behavior, administrator enforcement, preservation of hidden legacy fields, NULL compatibility code for new entries, and transaction rollback. Frontend tests verify Thai rendering, required-name validation, and absence of IDs/codes from rendered content.

No legacy MDB file is read or modified by this feature.

## Release validation

Validation completed on 2026-08-25:

- `cargo fmt --all -- --check`: passed;
- strict `cargo clippy --all-targets --all-features -- -D warnings`: passed;
- Rust tests: 172 passed;
- frontend tests: 73 passed across 21 files;
- frontend typecheck, lint, and production build: passed;
- Tauri release build and NSIS-only packaging: passed;
- tracked database/MDB files: zero; and
- both reference MDB SHA-256 hashes: unchanged.

Release artifact: `OncoFlow_0.1.2_x64-setup.exe` (3,366,843 bytes; SHA-256 `E5695A5AD82AC324B458338A36D1A10E15FB6067BCD0AF1F94056B9F4286382F`).
