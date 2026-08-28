# Preparation checking and batch output

## Scope

This post-RC workflow change removes the separate preparation and verification actions from the operator-facing workflow. It does not add or change a clinical calculation, safety rule, eligibility rule, inventory formula, or unit conversion.

The operator workflow is now:

1. An assistant pharmacist or pharmacist creates/edits an order and selects one active pharmacist as the preparation pharmacist for that order.
2. An assistant pharmacist or pharmacist opens the preparation workspace, reviews the preparation data, and selects **Check preparation**.
3. OncoFlow records the order-assigned pharmacist as the preparer and the authenticated user who performed the check as the checker.
4. The same transaction records deterministic inventory consumption (when supported) and append-only audit events.
5. Checked labels may be printed for the whole order, selected drug items, or one corrected drug item.

## Assignment and identity

Migration `013_preparation_assignment.sql` adds the nullable `orders.assigned_preparer_user_id` foreign key and advances the schema version to 13. Existing orders are not backfilled with a fabricated pharmacist.

Only an active, modern Argon2id account whose user type is `pharmacist` can be selected as the preparation pharmacist. Both pharmacist and assistant-pharmacist (`non_pharmacist` compatibility value) accounts may create orders and perform the check. The Rust session supplies the checker identity; the frontend cannot claim an arbitrary actor.

One pharmacist is assigned per order. Supporting different preparers for different drug lines is intentionally out of scope for this change.

## Compatibility and transaction behavior

The persisted terminal state remains `verified` and the existing `verified_*` columns remain in use for database, inventory, label-snapshot, and historical compatibility. Operator-facing text calls this state **Checked**; it is not described as a digital signature, treatment approval, or electronic verification.

The check transaction includes:

- recording the assigned preparation pharmacist and preparation time when absent;
- recording the authenticated checker and check time;
- posting one idempotent preparation inventory issue for a supported container calculation;
- retaining the manual-reconciliation path for unsupported calculations;
- appending minimal inventory and `preparation_checked` audit events.

If any required write fails, all changes roll back. Inventory may become negative and never blocks the check. Existing checked/verified preparations are not reassigned or backfilled.

## Warnings and precautions

Safety calculation code and stored acknowledgements are preserved for compatibility, but warning/precaution review is not part of this operator workflow. The order and preparation screens do not display or require the former safety acknowledgement/verification flow. Reintroducing warning review requires an explicit future release decision.

## Output behavior

The primary label action is **Print all labels in order**. It requires every preparation-eligible item in the order to have a checked final output snapshot; OncoFlow does not silently omit an uninitialized or unchecked item.

Correction paths remain available:

- **Print selected** prints the checked items selected in the order workspace;
- **Preview / print this label** prints a single drug item.

All label actions use the existing local Windows RAW spooler and do not re-check preparation, create another inventory movement, or change an order. A print/reprint audit event is appended using the current authenticated user.

The working formula output is selected by treatment date (Bangkok local date by default), uses existing preparation values/calculation results only, and opens the local system print dialog. It does not introduce a second calculation path.

When the working formula is reopened, initialization refreshes an existing `pending` preparation task if its source order item changed. The refresh keeps the task identity, replaces its source snapshot, clears preparation volume/notes that may refer to the previous order values, and appends a minimal `preparation_source_refreshed` audit event. Prepared or checked tasks are never refreshed automatically and retain the stale-source guard. Batch checking stores the confirmed default final-volume calculation when no manual final volume was entered.

## Known limitations

- The database uses legacy-compatible `verified` names internally even though the UI says **Checked**.
- Working formula output uses the system print dialog; preparation labels use the configured RAW printer queue.
- Physical printer output remains an operator-controlled validation step and is never triggered by automated tests.
- Warning/precaution review is explicitly deferred; the underlying safety implementation is retained but not exposed in this workflow.

## Validation

- `cargo fmt --all -- --check`: passed
- strict Clippy (`--all-targets --all-features -- -D warnings`): passed
- Rust tests: 193 passed
- frontend tests: 105 passed
- frontend typecheck, lint, and production build: passed
- Tauri release build and NSIS-only package: passed
- tracked DB/MDB files: 0
- legacy MDB SHA-256 values: unchanged from baseline

Release artifact: `src-tauri/target/release/bundle/nsis/OncoFlow_0.1.2_x64-setup.exe` (3,405,756 bytes; SHA-256 `EDA698AA59CE4089B6D3D4C4E0AE9123D8A932EABD4DA890653EDAB1EDC59AD5`). No physical print was sent during validation.
