# Order and drug-line cancellation

Authorized 2026-09-29 for wrong-patient orders and excess/duplicate/unneeded drug lines.
Evidence reviewed: the legacy `order` / `order details` mapping in Milestone 6
records cascade-delete linkage; Milestone 9 records that the legacy print flag
is not proof of preparation. No evidence establishes a safe legacy cancellation
workflow. This is an explicitly requested new workflow, not claimed Access parity.
Neither MDB is modified.

Cancellation is append-only, for OncoFlow-created records. A whole-order cancellation
affects all remaining lines; a line cancellation affects every treatment date for
that line, including future dates. The preview explicitly shows that scope, dates,
existing preparations and inventory postings. Other lines remain usable.

Checking can precede physical preparation. Every existing task therefore requires
an explicit physical-preparation decision; a pending task cannot be claimed as
prepared because preparer and quantity provenance is incomplete. Record that work
through the preparation workflow first. Reasons, actor, timestamp and task decisions
are retained; free-text reasons are not copied into diagnostic/audit payloads.

An immediate SQLite transaction compares a SHA-256 fingerprint of the current
order, lines, preparations, postings and print audit history with the reviewed
preview. Stale/repeated requests fail without partial changes. Server authentication
and LAN write-conflict checks remain in force. Cancelled records cannot be edited,
prepared, checked or printed through the application.

Ward delivery excludes cancelled tasks. Preparation totals exclude cancelled tasks
not physically prepared; physically prepared cancellations are counted separately
and remain in total actual work. Inventory reports retain posted issues and expose
cancelled postings requiring review: cancellation never restores stock automatically.
Existing inventory adjustment tools remain the reconciliation workflow.
Cancelled drug lines are excluded from order-based cumulative exposure inputs;
clinical formulas are unchanged. Cancellation is not an administration reversal.

Printed paper or labels cannot be recalled electronically. The cancellation screen
requires acknowledgement to withdraw previously printed documents and review stock.
The original order remains attached to its original patient. Create and check a new
order for the correct patient. There is no undo or reassignment of cancelled records.

## Usage

Open an order, then use “ยกเลิกรายการยา” on a drug line or select “ทั้ง order”
in the cancellation section. Review the patient, affected date ranges and task
list. Choose actual physical-preparation status for every task, enter a reason,
acknowledge document/stock review, and confirm. The retained history is visible
in the same section. Imported historical orders remain read-only.

## Validation (2026-09-29)

- `cargo fmt` and `cargo test`: 256 Rust tests passed.
- `npm run typecheck`, `npm run lint`, `npm test`: 179 frontend tests passed.
- `npm run build`: successful; existing bundle-size advisory remains.
- Synthetic tests cover partial then whole cancellation, multi-date work,
  active/cancelled report separation, excluded unprepared work, retained stock
  issues, frozen-label rejection, audit rollback, stale/duplicate decisions,
  append-only records, and authenticated LAN conflict handling.
- Existing LAN session-expiry test now advances an explicit test clock rather
  than subtracting eight hours from a Windows monotonic clock near boot.

No production database or legacy MDB was opened or modified for these tests.
Native printer output and clinical operational sign-off are not claimed by these
automated checks.
