# Milestone 5 — Chemotherapy Regimens

## Scope

Milestone 5 adds local regimen authoring over the migrated SQLite data:

```text
Regimen list → search → detail → header/group/item editing → ordering
```

It does not calculate doses, generate chemotherapy orders, enforce maximum or
cumulative dose, evaluate dilution compatibility, or connect to external systems.

## Legacy analysis

The source and migrated record shape is:

| Legacy source | SQLite table | Role |
|---|---|---|
| `TblRegimen` | `regimens` | Master code, name, and seven raw behavior flags |
| `Tblregimen details1` | `regimen_groups` | Treatment/cycle group with note, cycle day, and cycle count |
| `Tblregimen details2` | `regimen_items` | Drug step with raw dose, unit/route text, schedule, ordering, and preparation defaults |

The inspected development database contains 90 regimen headers, 99 groups, and
368 items. These totals include the five synthetic regimen masters and nine
synthetic groups created during Milestone 2 orphan reconciliation.

The two detail tables are materially different parent and child concepts. They
remain separate in SQLite, Rust, and the UI. No destructive merge or schema
migration was performed.

Read-only ACE inspection of the legacy form/report queries established that:

- the protocol form selects detail2 rows through detail1 `code`;
- regimen reports order steps primarily by the legacy two-character `group`;
- `ordering`, `StartD`, `duration`, `dfdiluent`, `dfroute`, and `dfrate` are raw
  stored item fields;
- drug, route, and diluent controls use local Access master tables;
- standard-dose queries call a separate VBA `StandardDose` function, which is
  intentionally outside this milestone.

Direct Access automation was not used because opening forms/modules can execute
legacy code. Available query definitions and the existing VBA object inventory
were inspected read-only; uncertain VBA behavior was not inferred.

## Ordering decision

Legacy ordering is not unique: 101 item `ordering_no` values are NULL and 35
group/ordering combinations contain duplicates. A uniqueness constraint would
break migrated compatibility data, so none was added.

Detail reads use a deterministic compatibility order:

1. legacy `item_group` (NULL/ungrouped first, matching the legacy grouping);
2. explicit `ordering_no` when present;
3. migrated SQLite item ID as the stable tie-breaker.

Move-up/down operations are limited to items with the same detail1 parent and the
same legacy `item_group`. An explicit reorder transaction assigns sequential
ordering values only to that selected bucket. Viewing a regimen never rewrites
legacy ordering.

## Rust boundary and transactions

`src-tauri/src/regimen/` contains typed models, an allow-listed SQLite repository,
validation/service behavior, and Tauri commands. It supports header CRUD, group
add/edit and empty-group removal, item add/edit/remove, deterministic reordering,
and local drug/route/diluent lookups.

All mutations use `BEGIN IMMEDIATE`. The service validates ownership and local
foreign keys before writing. Regimen codes are required, trimmed, preserved, and
checked case-insensitively for duplicates. Optional text becomes `NULL`; cycle,
duration, start-day, and ordering values must be nonnegative integers.

The legacy dose source is text. Numeric dose text is also stored in the existing
parsed numeric column when valid, while nonnumeric expressions remain losslessly
in `legacy_dose_text` with `dose = NULL`. No expression is evaluated.

## Conservative deletion

Regimen deletion is not exposed. Migrated patients, orders, appointments, and 276
appointment-card rows reference regimen records, and the schema uses cascading
detail relationships. A treatment group can only be removed after its items have
been explicitly removed. Individual item removal requires a deliberate UI action.

## User interface

Navigation now includes Patients, Drugs, Regimens, and System Status. The Regimens
workspace provides database-backed code/name search, sorting, keyboard-accessible
rows, structured detail groups, raw legacy flags, name-only local lookup dropdowns,
and inline group/item editors. The clinical boundary is stated on the detail view.

## Synthetic tests

Rust tests cover list/search, Thai text, create/update, duplicate codes, detail
groups/items, raw and numeric doses, nullable route/diluent values, local lookups,
invalid-reference rollback, item removal, guarded group removal, reordering, and
legacy compatibility IDs/duplicate ordering. Frontend tests cover required fields,
Thai preservation, NULL mapping, raw dose expressions, and numeric validation.
