# Daily diluent usage

## Evidence and agreed rule

The user supplied a paper report on 2026-10-01 showing diluent name, volume,
bottle count, and a Dose total. Its six groups total 19 bottles: D-5-W 250 mL
(3), NSS 100 mL (5), NSS 1000 mL (2), NSS 250 mL (5), NSS 50 mL (1), and NSS
500 mL (3). The legacy inventory and blueprint identify diluent maintenance and
drug/diluent usage reports, but do not contain the exact report query. Exact
Access query parity is not claimed. No legacy database or linked query is changed
or executed.

The user explicitly confirmed: only Checked tasks on the selected preparation
date; a preparation with two recorded final containers contributes two bottles.
This uses the existing final-container rule: container count does not divide or
recalculate the stored dose or diluent volume.

## Behavior

- Reports → จำนวนใช้ Diluent รายวัน, defaulting to the Bangkok local date.
- Authenticated Rust command `get_diluent_usage_report` queries `oncoflow.db`.
  LAN/host clients use the existing authenticated encrypted API.
- Filter exact `preparation_date`, `state='verified'`, and no cancellation.
  This is the selected day's checked, non-cancelled workload, not a stock ledger
  or an accounting of wastage from cancelled preparations.
- Group by trimmed snapshot diluent name and snapshot diluent volume in mL.
  Different sizes remain separate; identical names/sizes are combined even if
  master data contains duplicate IDs. Current master data is not substituted.
- Bottle counts sum `final_container_count`; Dose counts count preparation tasks.
  Thus a two-bottle task contributes 2 bottles and 1 Dose. Totals are computed in
  Rust. No stock is deducted and no clinical calculation runs.
- Unknown names/volumes have explicit rows and a note; counts remain in the total.
  Zero volume remains zero and is not replaced by a default volume.
- Date picker, refresh, empty/error/loading states, and system print action.
  Print is disabled until data for the selected date is ready. A4 portrait output
  uses 12 mm margins, repeating table headers, and hides app controls.

## Validation

Synthetic tests cover the example's six groups and 19 bottles, multibottle tasks,
duplicate rows, numeric size grouping, exact dates, excluded pending/prepared/
cancelled tasks, missing values, zero volume, read-only behavior, authentication,
date validation, migrated schema compatibility, Thai date display, navigation,
and loading print guards. Physical printer output remains operator validation.
