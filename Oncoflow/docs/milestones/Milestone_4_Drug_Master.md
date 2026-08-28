# Milestone 4 — Drug Master

## Scope

Milestone 4 adds the local Drug Master workflow:

```text
Drug list → SQLite search/filter → drug detail → create/edit drug
```

The module displays and maintains migrated configuration. It does not implement
orders, regimen execution, drug calculations, inventory transactions, receiving,
requisition, external lookups, or network integration.

## Schema inspection and decision

The Milestone 2 `drugs` table is the source of truth. The inspected development
database contains 49 drug records: 48 from `Tbldrug` and one labeled compatibility
drug required by orphaned legacy detail rows. Related local lookup counts are three
units, eight routes, and 50 diluents.

The existing table already provides:

- required unique `legacy_dcode` and required `drug_name`;
- nullable unit, route, and diluent foreign keys;
- preparation, warning, storage, expiry, safety, and inventory columns;
- indexes on drug name and inventory state;
- preserved legacy mapping and compatibility fields.

No schema migration was required. Existing IDs, raw values, relationships, current
inventory quantities, and compatibility rows remain unchanged unless the user edits
that specific record.

## Legacy semantics preserved

Source metadata and the importer mapping establish that:

- `MaxDilAlert` is an Access Yes/No value stored in `max_dilution_alert`;
- `CumAlert` is an Access Yes/No value stored in `cumulative_alert`;
- `InvCut` is an Access Yes/No value stored in `inventory_cut`;
- `MaxDilH` and `CumAlertH` are the associated raw numeric thresholds;
- `theory` is retained as raw compatibility text in SQLite;
- `homc_code` is displayed only as a legacy mapping code and has no external behavior.

The UI therefore presents the three Yes/No values as nullable enable flags. It does
not reinterpret them as calculated quantities. No `StandardDose`, maximum-dose,
cumulative-dose, dilution compatibility, or regimen-dose algorithm is present.

## Rust boundary

`src-tauri/src/drug/` contains:

- `model.rs`: typed request, response, detail, input, and lookup DTOs;
- `repository.rs`: allow-listed SQLite queries and local lookup joins;
- `service.rs`: normalization, validation, duplicate handling, and transactions;
- `commands.rs`: typed Tauri IPC and safe error responses.

Commands:

- `list_drugs`
- `get_drug`
- `create_drug`
- `update_drug`
- `drug_form_options`

List queries omit long detail, warning, storage, and incompatibility text. Search is
performed in SQLite against drug code and drug name, with literal wildcard handling
and Unicode preservation. Inventory-enabled filtering and pagination are also
database-backed.

## Validation and write behavior

- drug code and name are trimmed and required;
- duplicate codes are rejected case-insensitively;
- blank optional text becomes SQLite `NULL`;
- editable numeric configuration must be finite and non-negative;
- inventory maximum cannot be below inventory minimum;
- unit, route, and diluent IDs must exist in local lookup tables;
- create and update use `BEGIN IMMEDIATE` transactions;
- failed lookup or validation cannot partially create or update a drug;
- edits preserve internal IDs, current inventory quantity, legacy mapping code, and
  compatibility metadata not exposed as editable fields.

Current inventory quantity is read-only until the later inventory workflow milestone.

## User interface

The navigation now contains Patients, Drugs, and Settings/System Status. Drug Master
provides:

- debounced database search and inventory-state filtering;
- sortable, paginated, keyboard-accessible list rows;
- loading, empty, and error states;
- structured identity, preparation, safety, inventory, and legacy detail sections;
- create/edit forms populated from local unit, route, and diluent tables;
- explicit notices that safety values are raw legacy configuration.

## Synthetic test coverage

Rust tests cover list/filter, get-by-ID/code, code/name/Thai search, create, update,
duplicate rejection, nullable fields, non-negative numeric validation, inventory
min/max validation, lookup retrieval, and transaction rollback.

Frontend unit tests cover required fields, numeric and min/max validation, whitespace
and NULL conversion, Thai text preservation, and nullable legacy flags. All fixtures
are synthetic and contain no patient data.
