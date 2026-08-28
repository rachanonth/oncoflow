# Milestone 13: Verified Preparation to Inventory Consumption

## Scope

Milestone 13 connects successful verification of a new local preparation task to the existing append-only inventory ledger. It consumes only a fully `calculated` Milestone 12 whole-container result. It introduces no new preparation formula, unit alias, container inference, order mutation, clinical blocking rule, inventory reservation, purchasing, barcode, or historical backfill.

Inventory remains advisory. Zero, low, and negative balances never block order or preparation behavior.

## Pre-implementation AppData baseline

The actual local schema-version-6 AppData database was inspected read-only using aggregate queries only:

- schema version: `6`
- `PRAGMA integrity_check`: `ok`
- `PRAGMA foreign_key_check`: zero rows
- all orders / items: `3 / 4`
- historical orders / items: `1 / 2`
- OncoFlow-created orders / items: `2 / 2`
- preparation tasks: `0`
- inventory movements: `48`
- inventory movement quantity sum: `546.1061496734619`
- safety acknowledgements: `0`
- audit events: `5`

No patient-identifying values were selected or printed.

Immutable legacy baselines:

- `legacy/AllTable.mdb`: SHA-256 `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3`
- `legacy/Cytotoxic V8.0.mdb`: SHA-256 `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D`

## Evidence and compatibility decision

Milestone 11 established that the legacy application could subtract `[dose]/[dose/pack]` from inventory, including an `InvCut` branch whose exact-integer behavior was not suitable for direct reuse. Milestone 12 separately established the accepted container rule from the saved `Safe Value` query:

```text
FixNumber(ordered dose / dose per pack)
```

Milestone 13 does not port the legacy order-continuation mutation. It introduces a new OncoFlow workflow rule: post the already accepted Milestone 12 result only after authenticated preparation verification. The calculation retains ruleset `legacy-cytotoxic-v8`; the workflow identity is distinct:

```text
oncoflow-preparation-inventory-v1
```

This distinction avoids claiming that the timing, audit, or verification transaction is legacy Access behavior.

## Posting eligibility

Automatic posting requires all of the following:

1. the task is currently `prepared` and passes the existing source-stale and safety-acknowledgement checks;
2. the exact Milestone 12 calculation status is `calculated`;
3. `containers_required` parses as a non-negative, JavaScript/SQLite-safe whole integer;
4. inventory tracking is enabled for the drug;
5. the ledger has an authoritative current balance.

No formula is repeated in the posting service. It consumes the calculation result's `containers_required` field.

Outcomes:

- positive supported count: append one negative `preparation_issue` movement;
- supported zero count: persist `not_required`, with no zero movement;
- partial, unavailable, unsupported, malformed, or overflow result: verify and persist `manual_reconciliation_required`;
- tracking disabled: verify and persist `tracking_disabled`;
- missing ledger balance: verify and persist `manual_reconciliation_required`.

The manual path is not an error and does not block verification. Existing Milestone 11 manual issue/adjustment tools remain the correction/reconciliation path.

## Negative and low-stock behavior

For an automatic issue:

```text
balance before - containers required = balance after
```

The movement commits when the result is zero or negative. The persisted stock state uses the Milestone 11 rules (`normal`, `low`, `out`, or `shortage`) and is presentation-only. It never changes an order, ordered dose, calculated requirement, or preparation decision.

## Migration 007 design

Migration 007:

1. rebuild `inventory_movements` without changing existing rows or IDs, adding:
   - movement type `preparation_issue`;
   - nullable `preparation_task_id` foreign key;
   - checks requiring that link only for preparation issues;
   - a unique partial index enforcing at most one automatic issue per task;
2. restore the existing opening-balance and history indexes and append-only triggers;
3. create append-only `preparation_inventory_postings`, keyed uniquely by preparation task, recording the durable outcome, optional movement link, accepted calculation status/rule/version, exact container count where applicable, authenticated actor, before/after balances, resulting state, reason code, and timestamp;
4. set schema version 7 without inserting a posting or movement for any existing task.

The posting table is a decision/provenance record, not a mutable stock balance. Corrections remain compensating ledger movements.

## Atomic and idempotent behavior

For a supported positive count, one immediate SQLite transaction performs:

1. existing verification validations;
2. mark task verified with authenticated verifier and timestamp;
3. append `preparation_issue` using that verifier;
4. persist the unique task-to-movement posting record;
5. append `preparation_inventory_issued` and `preparation_verified` audit events.

Any failure rolls back all five effects. Database uniqueness exists both on the posting task and the ledger preparation-task reference.

Calling verify again on an already verified task is idempotent: it returns the existing task/provenance without writing. Critically, an existing verified task with no Milestone 13 posting row is returned as pre-integration and is not backfilled.

## Privacy and immutability

Posting/audit metadata contains only internal task, movement, drug, quantity, balance, rule, version, status, and reason identifiers. It contains no patient name, HN, notes, address, password, or order payload.

Normal application writes cannot edit or delete an inventory movement or posting decision. Direct SQLite administrators remain technically capable of changing local data; this is not a cryptographic ledger.

## Implementation plan

1. Add migration 007, register schema version 7, and test upgrade from a populated schema-6 fixture with existing verified/unverified tasks, ledger rows, users, orders, and audit events.
2. Extend typed inventory/preparation models and repository reads for the new movement/provenance state while preserving old rows.
3. Refactor the existing Milestone 12 workspace calculation construction into one shared service helper, then consume that exact result inside the verification transaction.
4. Implement fully atomic supported issue, zero/no-issue, disabled-tracking, and manual-reconciliation paths with Rust-session actor attribution and minimal audit events.
5. Add synthetic Rust tests for exact quantity, idempotency/retry/reload, database uniqueness, zero and already-negative balances, unsupported units, transaction failures, append-only history, pre-integration tasks, stale sources, and clinical-record non-mutation.
6. Add Preparation Workspace provenance UI and frontend tests for posted before/after/shortage, manual reconciliation, reload persistence, and non-blocking behavior.
7. Run schema/integrity/preservation checks, formatting, strict Clippy, Rust/frontend tests, typecheck, lint, production build, NSIS-only Tauri release build, startup validation, Git tracking, and legacy hash comparison.

Milestone 14 is not included.

## Implemented result

- `preparation_issue` is an explicit append-only ledger type. Its quantity is the negative of the accepted whole-container result.
- Both the ledger row and durable posting decision reference the source preparation task. A database uniqueness constraint allows at most one automatic issue for a task.
- A database validation trigger additionally requires the posting actor, task, drug, signed quantity, and movement link to agree.
- Verification derives the actor from the Rust session and performs verification, issue, posting, `preparation_inventory_issued`, and `preparation_verified` audit writes in one immediate transaction.
- Repeated verification returns the existing posting without another write. The behavior survives workspace reload and a fresh authenticated process session.
- Unsupported/partial calculations and an unavailable authoritative balance persist `manual_reconciliation_required`; disabled tracking persists `tracking_disabled`; a supported zero count persists `not_required`. None blocks verification.
- The Preparation Workspace displays automatic issue provenance, authenticated actor, movement reference, accepted rule, before/after balances, and shortage state. It labels unsupported outcomes as manual reconciliation rather than clinical failure.
- The Inventory ledger labels automatic entries as `Preparation issue` and retains the task reference.
- Existing verified tasks without a posting remain visibly pre-integration and are never backfilled.

## Automated coverage

Synthetic Rust coverage includes:

- exact reuse of the Milestone 12 `containers_required` value and unchanged raw `number_of_drug`;
- authenticated actor and minimal audit provenance;
- one issue across repeated command calls, workspace reload, and a new process session;
- database uniqueness and append-only triggers;
- `0 -> -1` and `-2 -> -5` shortage paths;
- zero-required, tracking-disabled, missing-balance, and unsupported-unit outcomes;
- movement failure and final-audit failure rollback with no partial verification, movement, posting, or audit;
- no mutation of ordered dose, regimen, patient identifier, or historical records;
- migration from populated schema 6 without historical backfill.

Frontend coverage verifies automatic issue provenance, before/after balances, non-blocking shortage presentation, unsupported/manual reconciliation, duplicate-free display, reload-compatible task data, and pre-integration labeling.

## Completion validation

The production local AppData database was checked before startup at schema 6 and after a controlled release-app startup at schema 7. Final results:

- schema version: `7`
- `PRAGMA integrity_check`: `ok`
- `PRAGMA foreign_key_check`: zero rows
- all orders / items: `3 / 4` (unchanged)
- historical orders / items: `1 / 2` (unchanged)
- preparation tasks: `0` (unchanged; no backfill)
- inventory movements: `48` (unchanged)
- inventory movement quantity sum: `546.1061496734619` (unchanged)
- preparation postings / automatic issues: `0 / 0`
- audit events: `5` (unchanged)

Acceptance commands passed:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Rust tests: `127 passed`
- frontend tests: `42 passed`
- frontend typecheck, lint, and production build
- Tauri release build and controlled application startup
- NSIS installer generation only

Git tracks no `.db`, `.sqlite`, `.sqlite3`, or `.mdb` file. Both legacy MDB SHA-256 hashes match the pre-implementation baseline.

## Deliberate remaining boundaries

- No historical issue backfill or automatic reconciliation is performed.
- Corrections use compensating Milestone 11 movements; automatic issue/posting history is not editable through OncoFlow.
- Unsupported quantity/unit relationships remain manual and do not prevent verification.
- There is no inventory reservation, preparation reopening/correction workflow, unit conversion, vial sharing, purchasing, barcode, lot/expiry, or administration behavior.
