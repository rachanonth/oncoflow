# Drug master spreadsheet update

Source: [oncoflowdrugdata](https://docs.google.com/spreadsheets/d/1e57D4NH9mBopXV-s7ZoyMQsnuzv8zt_qPscGMKMR-8A/edit), Sheet1, read on 2026-09-15.

The spreadsheet is a one-time data source. Normal OncoFlow operation remains offline.
The legacy MDB files are not modified. The snapshot and runtime database are not committed.

## Mapping and review

The 32 source headers match the existing `Tbldrug` mapping in
`src-tauri/src/migration/mapping.rs`. The import retains the source configuration,
including Thai text, numeric thresholds, and Yes/No semantics. It does not derive
concentration, change calculations, or correct suspected clinical inconsistencies.
Currency display formatting is removed from `price` without converting its value;
time strings are normalized to `HH:MM:SS`. Empty optional cells become NULL.

Numeric legacy codes are matched ignoring leading zeros. Lookup codes are resolved
through `legacy_unitcode`, `legacy_rcode`, and `legacy_dilcode`, never interpreted
as SQLite row IDs. Ambiguous normalized codes abort the import.

User-confirmed identity decisions:

- Source code 45 (Asparaginase IV) is assigned the new code `OF-D000051`.
  Existing code 45 (UFT) is preserved.
- Source code 58 (Dacarbazine) updates existing `OF-D000050`.
- Other matching rows keep their existing IDs and codes. Repeated names with
  distinct source codes remain separate records.

Unnamed placeholders are skipped. Rows using undefined unit code 4 or diluent
code 59 are withheld until their definitions are provided. No lookup is invented.

`Inv` is excluded: existing `inventory_qty` and inventory movements are preserved,
and new drugs do not receive a fabricated opening balance. Inventory configuration
(`InvUse`, `InvCut`, `InvMin`, `InvMax`) is imported. Drugs absent from the sheet and
the unresolved legacy compatibility record are preserved.

## Offline utility

`src-tauri/src/bin/drug_sheet_import.rs` is available behind `migration-cli`.
It accepts a local JSON array of rows, including the exact header row. First run
without `--apply` to review counts and rejected rows. Identity differences must be
reviewed before applying: the utility cannot decide whether two drug names refer
to the same product.

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --features migration-cli --bin drug-sheet-import -- `
  --database "$env:APPDATA/com.laste.oncoflow/oncoflow.db" `
  --input .tmp/drug-sheet-2026-09-15.json `
  --source-url https://docs.google.com/spreadsheets/d/1e57D4NH9mBopXV-s7ZoyMQsnuzv8zt_qPscGMKMR-8A/edit `
  --map-code 45=OF-D000051 --map-code 58=OF-D000050
```

To apply the reviewed plan, add `--apply --backup <new-backup-path>`.
The backup path must not already exist. A verified SQLite backup is required
before applying the transaction. Foreign keys remain enabled. Every imported
drug receives an audit event with source URL, source code, and snapshot SHA-256.
Failures roll back all drug and audit changes; rejected source rows are reported
and the valid rows can proceed. Repeated imports preserve IDs and do not add
duplicate records, but do append new audit events.

## Verification

Synthetic tests cover leading-zero lookup resolution, unknown and ambiguous
codes, numeric/boolean/time parsing, dry-run behavior, rollback after a later-row
failure, backup validity, preserved IDs and links, excluded stock balances, and
repeat imports. The live result is compared against its pre-import backup to
verify unaffected tables and existing inventory values.

## Applied result — 2026-09-15

- Updated 48 existing drugs and inserted 19 new drugs; final catalog: 69 records.
- Skipped 28 unnamed placeholders.
- Pending definitions: code 50 (7.5%NaHCO3) and code 51 (NaHCO3) require
  unit code 4; code 96 (Pemetrexed) requires diluent code 59.
- Recovery backup: `outputs/drug-import-2026-09-15-before.db`.
- Import report: `outputs/drug-import-2026-09-15-report.json`.
- Source SHA-256: `51279a4a08843ee61ae01dc79efe6c65305f40e02ffb417a2605880511154864`.
- Independent readback checked all 30 mapped fields for all 67 imported rows,
  all 44 unaffected tables, existing IDs/codes/stock, preserved UFT and compatibility
  rows, and 67 appended audit records. Integrity passed with zero foreign-key violations.
- `cargo fmt`, 227 Rust tests, frontend type checking, 142 frontend tests, and
  lint of `src`, `vite.config.ts`, and `eslint.config.js` passed.
- Full `npm run lint` remains blocked by three existing `console` no-undef errors
  in `.tmp/drug_master_export/build_drug_master.mjs`, unrelated to this import.
