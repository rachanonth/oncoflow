# Ward delivery report

## Evidence and scope

The user-supplied paper example (2026-09-29) defines the layout: Thai title,
Buddhist preparation date, Bangkok print time, ward headings, numbered patients,
one line per medication with ordered dose/unit and diluent/volume, ward totals,
recipient/time blanks, and a grand total. Identical medication lines stay separate.
No identifying information from the example is copied into fixtures.

The legacy object inventory and migration blueprint were inspected. They list
ward-related reports, including `Drug Use by Ward`, but do not establish the
query behind this particular paper form. A read-only DAO catalog inspection
did not return and was cancelled. Exact Access query parity is unverified;
this feature follows the supplied paper layout and the documented scope below.
Neither legacy MDB is modified, and no linked hospital query is executed.

## OncoFlow rules

- Reports → ใบส่งยาหอผู้ป่วย; select preparation date and optionally one ward.
- Preparation → พิมพ์ใบส่งยา opens the same report in a dialog with the queue's
  selected preparation date. Queue search/source filters do not restrict the
  report; it includes all checked tasks for the date and selected ward(s).
- Default source is checked (`verified`) preparation tasks for that exact date.
  Pending/prepared tasks and tasks from other dates are excluded. Opening or
  printing does not initialize tasks, check them, issue stock, or mark delivery.
- Rust queries `oncoflow.db` behind authentication; LAN/host mode uses the same
  read-only command over the existing authenticated API. No pagination cutoff.
- Dose and diluent fields come from the checked preparation snapshot. Volume is
  the recorded diluent volume, not a recalculated final volume. Missing values are
  explicit. Drug/patient/ward names come from local master/order records.
- Group by ward ID, then patient ID (not display name). HN is included alongside
  the patient name. Repeated preparations and multiple orders remain separate.
- One preparation task counts as one “รายการ”; these totals are not vial or bag
  counts. Totals cover only the selected ward(s). No preparer name is printed.
- Printed output hides navigation and filters, wraps long text, keeps patient
  blocks together where possible, and uses the system print dialog. Recipient
  and receipt-time fields are blanks for handwriting, with no electronic receipt.
- The screen preview uses a 210 × 297 mm A4 sheet with 12 mm padding. Printing
  uses a named A4 portrait page with 12 mm margins (186 × 273 mm content area),
  without additional document padding. Longer reports flow onto subsequent A4
  pages; individual medication lines avoid splitting. Other report page sizes
  are unaffected. The preview can shrink to fit a narrow window.

## Validation

Synthetic tests cover checked/date filtering on the migrated schema, preserved
duplicates and snapshots, null wards, authentication, Thai date/time, same-name
patients, totals, missing fields, and the loading print guard. Physical printing
and exact legacy query parity remain operator validation steps.
