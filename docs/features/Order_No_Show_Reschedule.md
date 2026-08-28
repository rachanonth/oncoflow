# Order no-show and reschedule workflow

## Decision

OncoFlow handles attendance as an exception-only workflow. Normal orders remain `active` and require no daily attendance confirmation. A user acts only when a patient does not attend.

The implementation deliberately does not change an order item's `start_date` or `stop_date` to represent attendance. Those fields retain their original prescribing/scheduling provenance. `stopped` is not used for a no-show because it would imply a permanent clinical discontinuation.

## Workflow

### No show

From the Preparation Queue or Order detail, an authenticated user records **Patient did not attend** for a specific preparation date. In one transaction OncoFlow:

1. appends a `no_show` order-status event;
2. changes the current order workflow status from `active` to `on_hold`;
3. records the authenticated actor and timestamp;
4. appends the minimal `order_no_show_recorded` audit event.

The held order immediately leaves the Preparation Queue. Repeating the same command is idempotent and does not add duplicate status or audit rows.

If a preparation item for that date is already `prepared` or internally `verified`/Checked, no-show recording is rejected. This prevents an already-prepared or inventory-consumed record from being hidden without an explicit correction/reconciliation workflow. A merely initialized `pending` task is retained unchanged as provenance and is not moved or deleted.

### Continue order

Order detail shows **Continue order on new date** while the order is on hold. The user selects only the new date. In one transaction OncoFlow:

1. appends a `rescheduled` event linked to the missed date;
2. returns the order from `on_hold` to `active`;
3. records the authenticated actor and timestamp;
4. appends the minimal `order_rescheduled` audit event.

The original `order_time`, item `start_date`, and item `stop_date` remain unchanged. For the new preparation date, the queue selects the same item set that was due on the missed date. This allows a rescheduled continuing order to appear even if the new date falls outside the original item range, without rewriting the source order.

The original no-show date and the dates between it and the selected continuation date remain unavailable for preparation. The order does not reappear automatically on an intermediate date. On the selected continuation date, a new preparation task uses that date while retaining the original item-date snapshot. Dates after the continuation date resume their normal stored schedule without shifting any source date.

## Persistence

Migration `016_order_attendance_status.sql` advances the schema to version 16 and adds:

- `orders.workflow_status`: `active`, `on_hold`, or `legacy`;
- current status reason/actor/time fields;
- append-only `order_status_events` for `no_show` and `rescheduled` events;
- uniqueness constraints for idempotency and unambiguous reschedule source/target dates.

Existing OncoFlow orders become `active`. Migrated historical orders become explicitly `legacy`; no attendance events or user identities are fabricated. Update/delete triggers protect event history through normal SQLite writes.

## Privacy and audit

The backend derives the actor from the Rust-authenticated process session. The frontend cannot submit an actor ID. Audit metadata contains only internal order/event identifiers, workflow state, and dates. It does not contain patient names, HN, drug data, or clinical notes.

## Limits

- This change implements only `active ↔ on_hold` for the no-show/reschedule exception.
- Permanent stop, cancellation, appointment reminders, and broader scheduling management are not introduced.
- An already-prepared/Checked no-show requires a future controlled preparation/inventory correction workflow; OncoFlow does not guess or reverse inventory automatically.
- Existing pending tasks from the missed date are preserved, not silently deleted or moved.

## Automated validation

Tests cover migration from schema 15, no fabricated events, preserved order/item dates, status/audit atomicity, idempotent command retry, append-only history, prepared-record protection, held-order queue exclusion, rescheduled-date queue inclusion, source-item preservation, supported older-backup migration, and future-schema rejection.

- Rust tests: 202 passed
- Frontend tests: 112 passed
- `cargo fmt --all -- --check`: passed
- strict Clippy: passed
- frontend typecheck, lint, and production build: passed
- Tauri release and NSIS-only bundle: passed

Release artifact: `src-tauri/target/release/bundle/nsis/OncoFlow_0.1.2_x64-setup.exe`. The release binary was not launched against the real AppData database during automated validation; schema 16 will apply through the existing backed-up migration path on the next normal application start.
