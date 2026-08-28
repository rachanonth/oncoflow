# Milestone 6 — Chemotherapy Orders

## Scope and architecture

Milestone 6 adds the local order workflow:

```text
Patient → order history → order detail
Orders → local search → create/edit → manage ordered drug lines
New order → optional raw regimen copy
```

All reads and writes use `oncoflow.db`. The module has no HOMC, `dbo_*`, hospital,
network, synchronization, or external API dependency. Neither legacy MDB was
modified, and no Access form, report, query, or VBA procedure was executed.

This milestone stores and presents order data. It does not calculate BSA, ANC,
platelet grades, patient doses, dose adjustments, maxima, cumulative exposure,
dilution compatibility, warnings, inventory, or final clinical decisions.

## Safely inspected legacy structure

Read-only ACE metadata and saved object definitions established the following
source shape. `AllTable.mdb` contains one order header and two order-detail rows in
the migration baseline.

### `order` → `orders`

| Access field | SQLite field | Observed meaning / handling |
|---|---|---|
| `orderid` | `id`, `legacy_orderid` | Legacy primary identifier; preserved |
| `hn` | `patient_id` | Local patient relationship |
| `wcode` | `ward_id` | Local ward relationship |
| `doccode` | `doctor_id` | Local doctor relationship |
| `worker` | `legacy_worker` | Recorder code/text; display-only |
| `edit worker` | `edit_worker` | Editor code/text; display-only |
| `app` | `appointment_flag` | Legacy appointment-origin flag |
| `note` | `note` | Stored order note |
| `ordertime` | `order_time` | Stored order date/time |
| `SideEffect` | `side_effect_text` | Legacy free text, not converted to a Boolean |
| `SErecorder` | `side_effect_recorder` | Side-effect recorder metadata |
| `SErecordtime` | `side_effect_record_time` | Side-effect record time |
| `ME` | `medication_error_text` | Legacy medication-error text; display-only |
| `regcode` | `regimen_id` | Local regimen relationship in SQLite |
| `Type` | `order_type` | Raw two-character value; meaning remains unresolved |

The initial schema's numeric `worker` and `side_effect_flag` are not inferred from
the Access recorder/text fields. The importer already preserves those values in
explicit compatibility columns.

### `order details` → `order_items`

| Access field | SQLite field | Observed meaning / handling |
|---|---|---|
| `orderid` | `order_id` | Header relationship; Access declares cascade delete |
| `dcode` | `drug_id` | Local drug relationship |
| `dilcode` | `diluent_id` | Optional local diluent relationship |
| `start`, `stop` | `start_date`, `stop_date` | Stored administration date bounds |
| `dose` | `dose` | Stored numeric value; historical values are never recalculated |
| `rcode` | `route_id` | Optional local route relationship |
| `time` | `schedule_time` | Stored schedule time |
| `noofdrug` | `number_of_drug` | Likely quantity, but not safely proven; labelled “Legacy quantity” |
| `missing` | `missing` | Raw flag used by legacy active/printing filters |
| `print` | `printed` | Legacy label-print control; display-only here |
| `rate` | `rate` | Stored free-text rate |
| `ordering` | `ordering_no` | Line sequence/order |
| `running`, `runsum` | `running_no`, `running_sum` | Legacy label sequence metadata; display-only |
| `InvDate` | `inventory_date` | Inventory-related date; display-only |

Declared Access relationships connect patient, doctor, and ward to the order
header; order to details; and drug, diluent, and route to details. Access does not
declare a regimen-to-order relationship, though the migrated SQLite schema safely
resolves `regcode` to `regimens` and enforces that local foreign key.

## Saved-query and report findings

Saved definitions were inspected as text only:

- the order-detail drug selector uses local `Tbldrug` rows and the legacy marker;
- route and diluent controls use the local master tables;
- a rate selector uses distinct previously stored order-detail rate text;
- print queries derive days, totals, and packaging values and filter by stored
  date, `missing`, and `print` fields;
- an update query marks printed rows; this printing workflow is not implemented;
- medication-profile/label definitions display `ordering`, `running`, and
  `runsum` as stored metadata;
- no safely inspected query establishes regimen-to-order calendar scheduling.

These findings are descriptive only. None of the legacy calculations, printing
updates, functions, or label behavior was ported.

## Minimal schema migration

`migrations/003_orders_workflow.sql` raises the schema to version 3 and adds:

- `orders.oncoflow_created`, default `0`, solely to distinguish migrated
  historical rows from new editable OncoFlow rows;
- `order_items.legacy_dose_text` for exact raw manual expressions;
- nullable source-regimen and raw snapshot columns for dose/unit/route/detail,
  legacy group, duration, start day, and ordering;
- indexes for order-date and per-order item-order reads.

Existing orders default to historical/read-only. No record counts, identifiers,
relationships, or migrated field values are changed. This is provenance, not a
new clinical order status model.

## Runtime behavior and safety

The Rust `order` boundary separates DTOs, SQLite repository code, transactional
validation/service behavior, and Tauri commands. Search is performed in SQLite by
order ID, HN, first/last name, regimen, and optional date range. Search text is not
logged.

New identifiers are generated under an immediate SQLite transaction: the internal
integer ID is the next local ID and the compatibility identifier is
`OF-########`. Existing identifiers are never renumbered or overwritten.

Imported orders are read-only. The application exposes no order-delete command.
Opening either a historical or new order performs no writes. Header and detail
mutations verify provenance and every supplied local reference in the same
transaction. Newly added lines receive predictable sequence positions; reordering
must include every current line exactly once. Historical NULL or duplicate
ordering values are not normalized.

When explicitly requested, regimen initialization creates the header and copies
all regimen steps in one transaction using the established deterministic legacy
group/item order. It copies the local drug, default route/diluent/rate, numeric
dose where present, exact raw dose text, and raw regimen snapshots. It leaves
start/stop dates, schedule time, and quantity NULL. `start_day` and `duration` are
not translated into dates. A missing local drug rolls back the entire new order.

## User interface

Navigation includes Patients, Drugs, Regimens, Orders, and System Status. Orders
are accessible globally and from Patient → Order History. The UI includes local
search/date filters, keyboard-openable rows, loading/error/empty states, historical
read-only detail, explicit order creation, header editing for new orders, and
inline line add/edit/remove/reorder controls. Lookup dropdowns display names only.

Ambiguous fields are explicitly labelled as raw or legacy. The order detail page
states the clinical calculation boundary and keeps side-effect, medication-error,
print, running, and inventory metadata display-only.

## Automated coverage

Synthetic Rust tests cover patient history, global search, Thai text, historical
reads without mutation, local create/update, invalid patient/drug/regimen and
optional-reference rejection, item add/edit/remove/reorder, raw dose expression
preservation, NULL route/diluent handling, date/numeric validation, transactional
regimen copying and rollback, local lookups, and schema migration preservation.

Frontend tests cover required header fields, raw two-character type validation,
Thai note trimming, NULL lookup conversion, raw dose preservation, optional
route/diluent handling, date-range validation, and numeric validation. Fixtures
contain synthetic data only and never touch the development database.

## Unresolved semantics

The following values remain deliberately uninterpreted: header `Type`; detail
`noofdrug`; `running`; `runsum`; the clinical significance of `missing`; and the
relationship between regimen `start_day`/`duration` and order dates. The legacy
printing/label, inventory, and clinical calculation workflows require separate
parity analysis in later milestones.
