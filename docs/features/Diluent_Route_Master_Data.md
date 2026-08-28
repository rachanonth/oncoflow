# Diluent and Route Master Data

Status: implemented as a focused local master-data extension (2026-08-25). It does not introduce a new clinical calculation, unit conversion, or external dependency.

## Navigation and UI

Authenticated local administrators can use the existing collapsible sidebar group:

```text
Master data
├── Doctors
├── Wards
├── Diluents
├── Routes
└── Diagnosis
```

Diluents and Routes support SQLite-backed listing, Thai UTF-8 search, creation, and inline editing. The UI displays names only for routes. Diluents display the name and the existing optional `volume_ml` reference. Internal IDs and migrated compatibility codes are not shown.

## Compatibility behavior

No schema migration is required. The feature uses the established compatibility-first tables:

- `routes(id, legacy_rcode, route_name)`
- `diluents(id, legacy_dilcode, diluent_name, volume_ml)`

Existing compatibility codes are preserved unchanged when a route or diluent is edited. Newly created records leave the legacy code NULL. No hard-delete command is exposed because drugs, regimens, orders, and preparation snapshots may reference these rows.

Drug, regimen, and order lookup services already read these same local tables. Saved changes therefore become available when those forms next load, while their select controls continue to show option names rather than IDs or compatibility codes.

The optional diluent volume retains its established SQLite meaning in millilitres. Blank becomes SQLite NULL; finite numeric values greater than or equal to zero are accepted. No concentration, withdrawal, dose, container, or inventory formula is performed by this feature.

## Authentication, audit, and privacy

List/create/update commands require a Rust-controlled administrator session. The frontend cannot supply an actor ID. Each create or update is committed atomically with a minimal append-only audit event:

- `route_created`
- `route_updated`
- `diluent_created`
- `diluent_updated`

Audit metadata is empty and does not duplicate lookup names, values, or compatibility codes. Normal application commands do not edit or delete referenced master-data rows.

## Tests

Synthetic Rust coverage verifies Thai create/search/update, NULL volume, decimal volume, invalid and non-finite volume rejection, case-insensitive compatibility-code protection, administrator access, and audit rollback. Frontend coverage verifies Thai rendering, hidden compatibility codes, required-name validation, optional volume parsing, zero, and decimals.

## Release validation

Validation completed on 2026-08-25:

- `cargo fmt --all -- --check`: passed;
- strict Clippy: passed;
- Rust tests: 168 passed;
- frontend tests: 69 passed across 20 files;
- frontend typecheck, lint, and production build: passed;
- Tauri release build and NSIS-only packaging: passed;
- tracked database/MDB files: zero; and
- both reference MDB SHA-256 hashes: unchanged.

Release artifact: `OncoFlow_0.1.2_x64-setup.exe` (3,347,310 bytes; SHA-256 `2438765D6CEA63D19DC5AD05006CBABAE709C4E8B9B2D948F48927C1726A02ED`).
