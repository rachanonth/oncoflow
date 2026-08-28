# Milestone 11: Local Inventory Ledger and Stock Management

## Scope

Milestone 11 adds a local, authenticated, append-only inventory ledger for chemotherapy-preparation stock. It does not add hospital integration, purchasing, billing, barcode scanning, vial optimization, preparation quantity inference, or automatic stock deduction from an order or preparation task.

The runtime remains fully local and uses only `oncoflow.db`.

## Pre-implementation verification

The schema-version-5 AppData database was inspected read-only before implementation, without selecting patient-identifying columns:

- schema version: `5`
- `PRAGMA integrity_check`: `ok`
- `PRAGMA foreign_key_check`: zero rows
- drugs: `49`
- drugs with a migrated `Inv` value: `48` (`20` were zero, `1` was NULL)
- inventory-tracked drugs (`InvUse`): `15`
- tracked drugs satisfying the legacy low-stock comparison `Inv <= InvMin`: `1`
- migrated `InvIN` rows: `1` completed row and no pending sent/unsent rows
- historical orders/order items: `1 / 2`
- locally created orders/order items: `1 / 1`
- preparation tasks: `0`

No row contents containing patient identity were printed.

Legacy file baselines:

- `legacy/AllTable.mdb`: SHA-256 `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3`
- `legacy/Cytotoxic V8.0.mdb`: SHA-256 `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D`

## Safely inspected legacy evidence

Inspection used schema/migration reports, saved query SQL, and text exports from a disposable copy of the Access front end with startup disabled. Neither original MDB was opened for writing or modified.

### Migrated fields

`Tbldrug` contains:

| Access field | SQLite field | Observed type/meaning |
| --- | --- | --- |
| `Inv` | `drugs.inventory_qty` | Nullable numeric stock snapshot |
| `InvCut` | `drugs.inventory_cutoff` | Boolean; legacy automatic-deduction mode flag |
| `InvMin` | `drugs.inventory_min` | Nullable numeric minimum |
| `InvMax` | `drugs.inventory_max` | Nullable numeric maximum |
| `InvUse` | `drugs.inventory_enabled` | Boolean tracking selector |

`InvIN` is migrated as `inventory_events` with `legacy_incode`, `legacy_dcode`, `quantity`, `event_at`, `inventory_ok`, `send_order`, and `legacy_user`. The Access source declares no primary key.

### Confirmed behavior

- The `INVCheck` screen filters drugs using `InvUse=True AND marker=True` and displays `Inv`, `InvMin`, and `InvMax`.
- It creates `InvIN` rows with `InvOK=False` and `SendOrder=False`. The entered quantity may default to `InvMax - Inv`.
- Sending a request changes `SendOrder` to true.
- Confirming a sent row adds `In_no` to `Tbldrug.Inv` and then marks the `InvIN` row `InvOK=True`.
- Legacy low stock uses the inclusive comparison `Inv <= InvMin` when `InvUse=True`.
- Legacy order-continuation code can deduct stock automatically. When `InvCut=True`, it subtracts `Int(drug use)+1`; otherwise it subtracts the raw calculated use and records `InvDate`.

The automatic deduction behavior is deliberately **not** ported in Milestone 11. The milestone explicitly forbids automatic preparation deduction, inferred preparation quantities, and unit conversion.

### Partially confirmed behavior

- `InvIN` represents a replenishment/request lifecycle in the legacy interface, but the available evidence does not establish purchasing, vendor, receiving-document, or requisition semantics suitable for a new warehouse workflow.
- `Tbldrug.unitcode` has useful drug-unit labels, but available evidence does not prove that those labels are always the physical inventory-count unit. The UI therefore labels this as a legacy drug unit and does not claim vial, ampoule, mg, or mL semantics.
- `InvMin` and `InvMax` are confirmed display/configuration fields. Only the inclusive minimum comparison is automated. No maximum-based replenishment rule is introduced.

### Unknown/display-only behavior

- `InvCut` is retained and displayed as a legacy deduction setting, but it drives no OncoFlow calculation in this milestone.
- The exact operational meaning of `Change` / `Change Details` is insufficiently established. Those tables are not treated as inventory movements.
- Whether a legacy `InvIN` user/date represents requester, receiver, approver, or another actor is unresolved. No OncoFlow user identity is fabricated from it.

## Compatibility and ledger design

Migration 006 introduces `inventory_movements` as the deterministic balance source:

```text
opening balance (legacy snapshot)
    + subsequent signed movements
    = current balance
```

For each drug whose migrated `inventory_qty` is not NULL, migration 006 creates exactly one `opening_balance` movement, including zero and negative values. The opening movement:

- preserves the exact migrated quantity;
- has `NULL` actor and `NULL` occurrence time because neither should be fabricated;
- records `legacy_drug_inventory` provenance and the legacy drug code;
- is protected by a unique partial index and idempotent insertion.

`drugs.inventory_qty` remains an untouched compatibility snapshot. Runtime balance is `SUM(inventory_movements.quantity_delta)` and the inventory service never edits the snapshot. This prevents two independently editable stock balances.

The completed migrated `InvIN` record is **not** added as another receipt. Legacy evidence shows completed `InvIN` quantities were already added to `Tbldrug.Inv`; counting both would duplicate stock. `inventory_events` remains compatibility provenance and is not an authoritative ledger.

Movement rows are append-only through database triggers and normal OncoFlow commands expose no update/delete operation. Corrections use compensating movements.

### Movement types

- `opening_balance`: migration provenance; actor/time may be unknown
- `receipt`: positive authenticated movement
- `adjustment_increase`: positive authenticated correction
- `adjustment_decrease`: negative authenticated correction
- `manual_issue`: negative authenticated issue

Runtime movement quantities must be finite and non-zero. Receipt and increase inputs are positive; decrease and issue inputs are converted to negative deltas. Adjustments and manual issues require a reason. Movement insertion and the corresponding audit event are one SQLite transaction.

### Negative and low-stock behavior

Negative inventory is valid ledger state. It commits normally, remains visible in movement history, and is labeled `Shortage`. It never blocks orders, regimen initialization, preparation creation, or preparation verification.

Inventory states are deterministic:

- `untracked`: inventory tracking disabled
- `unknown`: tracked but no ledger balance is available
- `shortage`: balance below zero
- `out`: balance equals zero
- `low`: balance is non-negative and `balance <= minimum`
- `normal`: all other tracked balances

The low-stock view includes tracked `shortage`, `out`, and `low` items. The `InvMin` comparison is inclusive, matching confirmed legacy behavior. `InvMax` and `InvCut` remain informational.

Movement-history resulting balances are calculated in ledger insertion order (`id`), not occurrence-time order. This keeps the audit sequence deterministic even when a pharmacist records a backdated business timestamp.

## Authentication, audit, and privacy

All inventory-changing commands obtain the actor from the Rust process-local authenticated session. The frontend cannot submit an actor user ID.

Successful movements atomically append one of:

- `inventory_receipt`
- `inventory_adjustment`
- `inventory_manual_issue`

Audit metadata contains only internal identifiers, movement type, signed quantity delta, and resulting balance. It contains no patient data, passwords, password hashes, or full clinical/order payloads.

## Implementation plan

1. Add migration 006 with the movement ledger, opening-balance transition, indexes, constraints, and append-only triggers; test migration from a populated schema-5 fixture.
2. Add a Rust `inventory` boundary with typed models, SQLite-only repository code, validation/transaction service code, authenticated Tauri commands, deterministic balances, and audit integration.
3. Update Drug Master inventory reads to use ledger-derived current balance while retaining all legacy configuration unchanged.
4. Add the Inventory workspace with local search, tracked/low filters, stock states, detail/history, receipt, adjustment, and manual-issue workflows.
5. Add synthetic Rust/frontend coverage for boundaries, negative stock, Thai text, authentication, atomic audit, rollback, immutability, and non-mutation of clinical/preparation data.
6. Validate schema version/integrity/foreign keys and preservation baselines, then run formatting, strict Clippy, all tests, frontend checks/build, NSIS release build, and normal application startup.

No schema or behavior from Milestone 12 is included.

## Implemented vertical slice

### Database and Rust

- `migrations/006_inventory_ledger.sql` creates the constrained ledger, indexes, idempotent opening transition, and append-only triggers.
- `src-tauri/src/inventory/model.rs` defines typed list, detail, movement, request, result, state, and sort DTOs.
- `repository.rs` contains SQLite-only list/search/filter/detail/history queries and deterministic windowed resulting balances.
- `service.rs` validates inputs, derives the actor from `AuthSession`, records movements and minimal audit metadata in one immediate transaction, and deliberately permits negative results.
- `commands.rs` exposes only typed inventory operations; it exposes no generic SQL or arbitrary actor input.
- Drug Master reads its displayed current quantity from the movement sum after migration 006. It never updates the preserved `drugs.inventory_qty` snapshot.

Typed Tauri commands:

- `list_inventory`
- `get_low_stock_items`
- `get_inventory_item`
- `list_inventory_movements`
- `record_inventory_receipt`
- `record_inventory_adjustment`
- `record_inventory_manual_issue`

### Frontend

The authenticated navigation now includes an Inventory workspace with:

- SQLite-backed code/name search;
- tracked-only and low-stock filters;
- sortable drug, balance, min/max, and state columns;
- explicit `Normal`, `Low`, `Out`, `Shortage`, `Unknown`, and `Untracked` states;
- current/min/max and preserved legacy configuration detail;
- append-only movement history with resulting balance and actor display;
- receipt, adjustment-increase/decrease, and manual-issue entry;
- client validation backed by authoritative Rust validation;
- explicit scope and unresolved-unit notices.

The UI never presents an actor selector. The backend session is authoritative.

## Automated coverage

Rust tests cover:

- migration from a populated schema-5 fixture;
- exact non-NULL, zero, and negative opening preservation;
- idempotent opening insertion;
- preservation of users, audit, orders/items, preparation, and legacy `InvIN` rows;
- deterministic balance and resulting-balance history;
- Thai search/text;
- tracked, untracked, unknown, exact-minimum, and shortage states;
- receipt, both adjustment directions, manual issue, and negative balance;
- zero, negative, non-finite, invalid-date, missing-reason, and invalid-drug rejection;
- authenticated actor attribution and anonymous rejection;
- movement/audit atomic rollback;
- append-only update/delete rejection;
- legacy snapshot immutability and completed `InvIN` non-double-counting.

Frontend tests cover database-search/filter request construction, Thai list/detail rendering, shortage visibility, receipt/adjustment/manual-issue controls, actor history, opening provenance, and form validation.

All fixtures are synthetic.

## Completion validation

The release application was started normally (hidden only for automated validation) against the actual local AppData `oncoflow.db`. It remained running until the validation process deliberately closed it. Migration 006 applied successfully.

Read-only AppData comparison:

| Check | Before | After |
| --- | ---: | ---: |
| schema version | 5 | 6 |
| integrity check | ok | ok |
| foreign-key violations | 0 | 0 |
| all orders / items | 2 / 3 | 2 / 3 |
| OncoFlow-created orders / items | 1 / 1 | 1 / 1 |
| inferred historical orders / items | 1 / 2 | 1 / 2 |
| preparation tasks | 0 | 0 |
| migrated `inventory_events` | 1 | 1 |
| prior audit events | 4 | 4 |
| non-NULL legacy inventory snapshots | 48 | 48 |

Reconciliation after migration:

- opening movements: `48`
- opening quantity sum: `546.1061496734619`
- legacy snapshot sum: `546.1061496734619`
- all-ledger quantity sum before any new user movement: `546.1061496734619`
- opening rows with both unknown actor and unknown occurrence time: `48`
- low-stock items using the confirmed inclusive comparison: `1`

Toolchain validation:

- `cargo fmt --all -- --check`: passed
- `cargo clippy --all-targets --all-features -- -D warnings`: passed
- Rust tests: `105 passed`
- frontend tests: `38 passed` in `12` files
- frontend typecheck: passed
- frontend lint: passed
- frontend production build: passed
- Tauri release build: passed
- normal release application startup: passed
- NSIS installer: produced successfully; MSI was not built
- Git-tracked database/MDB files: `0`
- both legacy MDB SHA-256 values: unchanged from the pre-implementation baselines

## Intentional remaining limitations

- Physical inventory unit semantics remain unresolved; values are preserved without conversion.
- `InvCut` is display-only because its confirmed legacy behavior belongs to prohibited automatic order deduction.
- The migrated completed `InvIN` row is compatibility provenance, not a second receipt ledger entry.
- No stock reservation or automatic clinical/preparation deduction exists. Negative stock remains advisory.
- Append-only triggers protect normal OncoFlow writes, but a direct SQLite administrator can still alter local data; this is not a tamper-proof ledger.
