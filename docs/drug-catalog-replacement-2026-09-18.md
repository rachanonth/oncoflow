# Drug catalog replacement — 2026-09-18

Source: [ข้อมูลยาเคมีบำบัด สำหรับลงโปรแกรม](https://docs.google.com/spreadsheets/d/1DWEZZ5cTKOeOR7ggxBjEL8fumqYn8RYeAqe71UKtIhw/edit), tab `ชีต1`, 44 named drug rows.

## User decisions

- Replace the catalog, including exact source `dcode` values. Old order history
  need not be retained where it depends on removed drugs.
- Written unit and diluent names take precedence over conflicting legacy lookup
  codes. Leave default diluent volume unspecified when the code is missing,
  unknown, or points to a different name. Do not infer volume from preparation text.
- Remove complete regimen groups containing removed drugs; retain complete
  unaffected groups. Do not leave a partial regimen after removing its drugs.

## Mapping

The original import mapping in `src-tauri/src/migration/mapping.rs` was inspected.
The new sheet has 28 columns, including descriptive unit/diluent/route labels,
and does not provide prices, inventory settings/balances, or historical HOMC codes.
These absent fields are retained on matching drugs; new drugs use schema defaults.
All supplied fields replace their previous values, including blank optional cells.
Yes/No remains boolean; numeric values and concentration text are copied without
clinical recalculation. Expiry time is formatted as `HH:MM:SS`.

Exact source codes replace zero-padded old codes. Source 45 maps to the existing
Asparaginase IV record (`OF-D000051`), and source 58 maps to the existing Dacarbazine
record (`OF-D000050`). The old UFT at code 45 is removed. The snapshot and audit
retain provenance; no source lookup code is treated as an internal SQLite ID.

Written units and routes resolve to existing labels. A diluent with a matching
code and label retains its known volume. Otherwise a matching label with NULL
volume is reused or created. Existing shared diluent definitions are not rewritten.

## Maintenance operation

`drug-sheet-import --replace-catalog` is an explicit, offline maintenance mode.
It refuses an empty/invalid source, duplicate codes, ambiguous lookups, unexpected
prepared-history dependencies, client-mode fallback, and active database ownership.
A dry run executes the entire transaction on an in-memory SQLite backup and rolls
it back. Apply requires a new, validated recovery backup, uses foreign keys and
one immediate transaction, and appends a catalog replacement audit event.

The obsolete inventory rows are removed only under this explicitly authorized
replacement. Their delete-protection trigger is restored verbatim within the same
transaction; failures restore both rows and trigger. Prepared outputs, audit history,
patients, users, and unaffected regimen groups are not rewritten.

```powershell
src-tauri/target/debug/drug-sheet-import.exe `
  --database "$env:APPDATA/com.laste.oncoflow/oncoflow.db" `
  --input .tmp/drug-replacement-2026-09-18.json `
  --source-url https://docs.google.com/spreadsheets/d/1DWEZZ5cTKOeOR7ggxBjEL8fumqYn8RYeAqe71UKtIhw/edit `
  --replace-catalog --remove-affected-regimen-groups `
  --map-code 45=OF-D000051 --map-code 58=OF-D000050
```

Add `--apply --backup <new-backup-path>` only after reviewing the dry-run report.
The two map-code options describe the pre-replacement catalog and are for this
one-time replacement; omit them for a later replacement of the resulting catalog.
Run against the authoritative local database with the app/server closed. In LAN
mode, perform maintenance on the server machine, never through a network share.

## Tests

Synthetic tests use the actual migrations and cover exact code reassignment,
label precedence, unknown diluent volumes, whole-group removal, preserved inventory
for retained drugs, mandatory regimen decisions, dry-run isolation, verified backup,
audit append, and rollback of deletions and trigger changes after a later failure.

## Applied and verified

- Final catalog: 44 drugs with exact source codes (43 updated, Pemetrexed added).
- Removed: 26 obsolete drugs; 48 complete affected regimen groups containing 284
  lines; 2 obsolete order lines; 22 obsolete inventory movements; 17 obsolete
  drug-detail groups and their dependent detail items.
- Kept all 37 unaffected tables unchanged. Retained inventory balances, existing
  prepared outputs, audit history, patients, users, and complete unaffected groups
  were checked against the backup. Existing shared lookup definitions remain unchanged.
- Unspecified default diluent volumes: codes 2, 6, 8, 9, 10, 100, 11, 17, 26, 27,
  29, 30, 35, 39, 47, 63, 81, 82, 83, 90, 96, 98.
- Source SHA-256: `b84c23025204badcd984f8cb0dad0b06ba207a379bc51d9798417a622facbafd`.
- Backup: `outputs/drug-replacement-2026-09-18-before.db`.
- Report: `outputs/drug-replacement-2026-09-18-report.json`.
- Independent verification checked every supplied drug field and lookup label,
  exact remaining codes, approved dependent deletions, all original trigger
  definitions, source audit hash, database integrity, and zero foreign-key violations.
- Validation: `cargo fmt`; all 241 Rust tests (including LAN/server and importer
  tests); frontend type checking and lint; all 145 frontend tests passed.
