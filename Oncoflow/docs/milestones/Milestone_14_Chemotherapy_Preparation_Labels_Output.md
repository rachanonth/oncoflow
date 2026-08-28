# Milestone 14 — Chemotherapy Preparation Labels & Preparation Output

Status: complete (2026-08-23)

## Scope and evidence

Milestone 14 produces an OncoFlow-owned chemotherapy preparation label and
pharmacist preparation summary. It does not reproduce a hospital medication
order, prescription, MAR, administration sheet, or take-home label. Barcode and
QR support is deliberately deferred until both the encoded identifier and its
consumer are defined.

The output boundary was reviewed against the migrated schema and Milestones
2–13. The authoritative data available at verification is:

- `preparation_tasks`: snapshotted prescribed dose/unit, diluent, route, rate,
  treatment day, sequence, regimen preparation details, drug detail/storage,
  entered preparation volume/notes, state, and actor timestamps;
- `orders`, `patients`, `regimens`, and `drugs`: identifiers and human-readable
  names which are not currently snapshotted by the preparation task;
- `preparation_inventory_postings`: the accepted Milestone 12 container result,
  posting outcome, calculation provenance, and balance before/after;
- `users`: preparer/verifier display names; and
- `audit_events`: append-only authenticated workflow events.

Legacy preparation/output objects establish that preparation output existed,
but the photographed/non-standard hospital order layouts are not a product
target. No safely established rule supports deriving a beyond-use date,
stability duration, or expiry date. Those values will not be calculated.

## Provenance and confidence matrix

| Output value | Provenance | Confidence / handling |
| --- | --- | --- |
| Ordered dose and dose unit | preparation task snapshot | CONFIRMED; copied verbatim |
| Diluent, route, rate, treatment day | preparation task snapshot | CONFIRMED; copied verbatim |
| Final/preparation volume | user selects the default diluent-volume + calculated-withdrawal-volume suggestion or enters a final volume manually; the selected numeric result is stored on the preparation task | CONFIRMED by product decision; optional when either source volume is unavailable |
| Preparation instructions and storage text | preparation task snapshots | CONFIRMED as reference text only |
| Patient HN/name, regimen name, drug code/name | current related rows at first output generation | CONFIRMED identifiers; frozen in the output snapshot thereafter |
| Preparer/verifier and timestamps | authenticated preparation workflow | CONFIRMED; missing historical actors remain absent |
| Container requirement and inventory issue | durable Milestone 13 posting | CONFIRMED when present; copied without recalculation |
| Withdrawal volume/presentation | Milestone 12 calculation preview only | NOT PERSISTED; omitted rather than recalculated during output |
| Safety state | current safety evaluation | MUTABLE/NOT SNAPSHOTTED; label omits it and does not print warning text |
| BUD, stability, expiry | no confirmed rule | UNKNOWN; never derived or printed as a calculated value |

## Snapshot and migration decision

Migration 008 is justified. Existing preparation snapshots do not include
patient, regimen, or drug display names. Rendering those names dynamically
would allow master-data edits to change a later reprint. Migration 008 adds one
immutable `preparation_output_snapshots` row per preparation task. It is created
only on the first authenticated output request for an already verified task;
the migration does not backfill historical tasks.

The snapshot stores typed, explicitly selected output fields instead of an
arbitrary database row or opaque full-label payload. Database uniqueness makes
generation idempotent, and update/delete triggers make the application-facing
record append-only. Missing historical actor values remain NULL. No preparation,
order, patient, regimen, safety acknowledgement, or inventory row is changed.

Physical paper dimensions are device configuration and remain separate from
clinical label content. The initial UI offers basic local dimensions while the
clinical template identity remains `oncoflow-preparation-label-v1`.

## Print and audit semantics

Final output is available only for a verified task. Milestone 14 does not offer
a draft label, avoiding any chance that an unverified preview resembles final
output. The React view is a visual preview only. Printing does not use the
browser print dialog: Rust rasterizes the immutable typed label to a monochrome
bitmap, wraps it as ESC/POS or TSPL, and submits a `RAW` job to an installed
Windows printer queue with the native spooler API. The transport and renderer
are separate modules. USB or LAN details remain Windows-driver concerns, and
there is no cloud, direct socket, or printer-model dependency.

Printer queue, command language, DPI, dimensions, and gap are workstation-local
settings stored in browser local storage. Queue discovery uses the native
Windows spooler, preserving Unicode queue names. Save/connect stores settings
only; a rasterized Thai/English test label is the actual connection check.

The renderer identity is `oncoflow-raw-label-raster-v1`, separate from both the
clinical ruleset and label content template. Thai text is rendered as bitmap
glyphs using `ab_glyph` and a Thai-capable Windows font (Tahoma, Leelawadee UI,
or Arial fallback), rather than being sent as printer code-page text. No font
binary with unresolved redistribution licensing is copied into the repository.

Because `WritePrinter` proves only that Windows accepted the job—not that paper
physically printed—audit events use the honest names
`preparation_label_print_requested` and
`preparation_label_reprint_requested`. They are appended only after spooler
acceptance. Metadata contains only the preparation task ID, output snapshot ID,
template version, renderer/transport identity, and request ordinal—never patient
name, queue name, or medication content. Output snapshot creation is not itself
recorded as a completed print. The external spooler job cannot be transactionally
rolled back if the later local audit write fails; this is an honest device
boundary and never affects clinical/preparation/inventory state.

Printing or reprinting does not re-verify, recalculate, create inventory
movements, alter safety acknowledgements, or mutate clinical records. Negative
inventory remains non-blocking.

## Implementation plan

1. Add migration 008 and database migration coverage from schema 7 without
   backfilling existing preparations.
2. Add a Rust `output` domain with typed label/summary models, snapshot-only
   repository access, verified/authenticated guards, and minimal print-request
   audit events.
3. Expose typed Tauri commands for output preview, Windows printer enumeration,
   printer test output, and authenticated RAW label printing.
4. Add Settings > Hardware plus a preparation-workspace label preview and
   summary, configurable physical dimensions, final/reprint states, and
   missing-value handling.
5. Add synthetic Rust/frontend tests for provenance, determinism, Thai UTF-8,
   guards, audit, shortage behavior, and clinical/inventory non-mutation.
6. Validate schema 8, SQLite integrity/FKs, record-count invariants, source MDB
   hashes, formatting, Clippy, tests, frontend checks, release build, and normal
   startup. NSIS is the only packaging target in scope.

## Deferred work

- Barcode/QR encoding and scanning.
- Receipt/cash-drawer output, queue deletion, direct USB/LAN access, and
  printer-specific drivers.
- A redistributable bundled Thai font; current raster output uses installed
  Windows fonts and reports an actionable error if none can be loaded.
- Hospital-specific dimensions.
- Durable proof of physical print completion.
- Historical output reconstruction when required provenance is absent.
- BUD/stability/expiry calculations and any new clinical rules.

## Completion validation

Migration 008 was applied by a controlled startup of the optimized application
against the normal local AppData database. The exact launched process remained
running during observation and was then stopped by process ID. Aggregate-only
checks before and after startup were:

| Check | Before | After |
| --- | ---: | ---: |
| schema version | 7 | 8 |
| integrity check | ok | ok |
| foreign-key violations | 0 | 0 |
| all orders / items | 3 / 4 | 3 / 4 |
| historical orders / items | 1 / 2 | 1 / 2 |
| preparation tasks | 0 | 0 |
| inventory movements | 48 | 48 |
| inventory movement quantity sum | 546.1061496734619 | 546.1061496734619 |
| preparation postings / automatic issues | 0 / 0 | 0 / 0 |
| safety acknowledgements / audit events | 0 / 5 | 0 / 5 |
| output snapshot table | absent | present, zero rows |

The migration therefore created no historical output snapshot or print event.
Printing tests use synthetic output only and do not contaminate AppData.

Validation passed:

- `cargo fmt --all -- --check`;
- strict `cargo clippy --all-targets --all-features -- -D warnings`;
- 136 Rust tests, including deterministic Thai raster bytes, ESC/POS/TSPL
  framing, snapshot determinism, verified-only output, audit rollback, and
  clinical/inventory non-mutation;
- 51 frontend tests in 14 files, including preview, summary, reprint, shortage,
  missing fields, printer setup, and unavailable-queue behavior;
- frontend typecheck, lint, and optimized build;
- Tauri optimized build and NSIS-only installer generation;
- normal application startup against AppData;
- no tracked `.db`, `.sqlite`, `.sqlite3`, or `.mdb` file; and
- unchanged legacy MDB SHA-256 hashes.

Windows exposes an online `Xprinter XP-420B` label queue and the queue inventory
was inspected read-only. No unsolicited physical job was sent during automated
validation: command language, loaded stock dimensions/gap, and physical output
must be confirmed with Settings > Hardware > Print test label on the intended
device. This avoids wasting stock or sending an unsupported command language to
a user-owned printer without an explicit operator action.

## RC1 printer-polarity correction

Operator feedback during RC1 established that the Xprinter TSPL RAW path
interprets the original raster polarity in reverse, producing a black label
background with white text. The TSPL encoder now reverses bitmap payload bytes
at the device-language boundary so the physical output is a white background
with black text and rules. ESC/POS output and the immutable clinical label
content are unchanged. Because rendered bytes changed, the renderer identity is
`oncoflow-raw-label-raster-v2`; a physical operator recheck remains required.
