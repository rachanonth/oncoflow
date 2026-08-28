# Milestone 8 — Clinical Safety and Alerts Integration

## Scope and safety boundary

Milestone 8 evaluates confirmed local legacy safety rules against an editable
OncoFlow order and presents explainable findings for pharmacist review. Evaluation
is read-only. It never changes a dose, line, order, regimen, patient, or historical
record; it never approves or blocks treatment; and it has no external dependency.

All legacy-compatible findings use the immutable ruleset identity:

```text
legacy-cytotoxic-v8
```

Historical migrated orders are not retrospectively evaluated by default. The UI
states that no current warning should be interpreted as evidence of what was shown
when a historical order was created.

## Evidence collection

Evidence was collected without executing forms, reports, saved queries, startup
behavior, or VBA procedures:

- source schema and mapping were read from the Milestone 2 migration report;
- saved query SQL was read through DAO with the front-end opened read-only;
- a disposable copy of `Cytotoxic V8.0.mdb` had its startup form removed before
  Access opened it with automation security set to force-disable macros;
- Access `SaveAsText` exported definitions without opening forms/reports;
- matching function and event-procedure source was inspected as text;
- the disposable copy and exports were deleted;
- both original MDB hashes remained unchanged.

The intentionally excluded `TblAlert` lookup contains 14 alert labels. Its source
codes confirm, among others, dose variance (1), cycle timing (2), concentration
(3), dilution incompatibility (4), SOAP (5), side-effect history (6), laboratory
result (7), cumulative dose (8), pharmacist note (9), appointment card (11),
counselling (12), and out-of-protocol drug (13). These names are evidence only;
`TblAlert` is not a runtime dependency.

## Migrated safety inventory

The actual local AppData database was inspected with read-only aggregate queries:

- schema version 3, `integrity_check=ok`, zero foreign-key violations;
- 49 drugs: 16 with warning text, 13 with `max_dose`, 5 with maximum-dilution
  alert enabled, 3 with cumulative alert enabled, and 5 with a structured
  incompatibility code;
- observed incompatibility codes are `A`, `D`, and `S`;
- all drugs with maximum-dilution or cumulative alert enabled use the migrated
  unit label `mg.`;
- regimen flags: 50 drug-alert, 37 appointment-alert, and 41 counselling-alert;
- the singleton alert flags are note=off, side-effect=on, SOAP=on,
  new-order=on, cycle=on, and plan=on;
- historical baseline remains one order header and two items.

No patient values or order contents were printed.

## Confidence and automation matrix

| Legacy source/rule | Confidence | Milestone 8 behavior |
|---|---|---|
| `warning` | CONFIRMED display field | Show unchanged as an informational drug advisory. Do not interpret it. |
| `MaxDilAlert` + `MaxDilH` | CONFIRMED comparison; PARTIALLY_CONFIRMED numeric representation | Automate only with numeric dose, positive pack strength/volume, a local diluent volume, supported `mg.` unit, and nonnegative threshold. |
| `Dil_Incompat` + `DilCompat` | CONFIRMED | Automate the exact recovered character/category matrix. Unknown codes are unsupported. |
| `CumAlert` + `CumAlertH` | CONFIRMED query/comparison; PARTIALLY_CONFIRMED applicability | Automate only with `mg.` unit, positive stored legacy BSA, numeric compatible-history total, and a nonnegative threshold. Otherwise report unsupported. |
| `Maxdose` | UNKNOWN | Display-only unsupported configuration. No recovered call site or comparison was found. |
| regimen `DAlert` | PARTIALLY_CONFIRMED | Unsupported. The source percentage (`MinMax.MinMax`, observed as 15) was not migrated, and missing unit/BSA behavior is not fully established. |
| `AppAlert` | CONFIRMED presence check | Warn when enabled for the selected regimen and no local patient/regimen appointment exists. |
| `CounselAlert` | CONFIRMED presence check | Warn when enabled and no local pharmaceutical-care record has `p2=true`. |
| `NoteAlert` | CONFIRMED presence check | When enabled, show an informational advisory if a prior order note exists; never expose note text. |
| `SEAlert` | CONFIRMED presence check | When enabled, show an informational advisory if a local side-effect record exists; never expose its contents. |
| `SOAPAlert` | CONFIRMED presence check | When enabled, show an informational advisory if a local SOAP record exists; never expose its contents. |
| `CycleAlert` | PARTIALLY_CONFIRMED | Unsupported. `Count day`, date conversion, multiple regimen-group behavior, and current workflow timing need a separate parity milestone. |
| `NewOrderAlert` | PARTIALLY_CONFIRMED | Unsupported in per-order evaluation. It is a legacy startup/unprinted-work queue check, not an order-line safety rule. |
| `Plan` | UNKNOWN for current workflow | Unsupported pending implementation of the planning workflow. |
| WBC/ANC/platelet/Hb/AST/bilirubin/creatinine thresholds | CONFIRMED storage, UNKNOWN order action | Display as pending/unsupported only. No local laboratory input or confirmed treatment action exists. |
| `AlertRec` | CONFIRMED historical storage | Preserve unchanged; do not use it as current acknowledgement or proof that a recomputed warning existed historically. |

## Confirmed threshold behavior

### Maximum dilution concentration

The order-detail source calculates drug solution volume and final concentration:

```text
drug_volume = order_dose / (dose_per_pack / volume_per_pack)
concentration = order_dose / (diluent_volume + drug_volume)
```

The alert is enabled by `MaxDilAlert` and triggers only when:

```text
concentration > MaxDilH
```

Equality does not trigger. The source labels the values `mg/ml`. OncoFlow performs
no conversion and evaluates only the observed compatible `mg.` configuration.
Fixed-point cross multiplication determines the comparison without binary-float
boundary drift. Any missing, zero-denominator, negative, nonnumeric, or unsupported
unit input produces an explicit unsupported finding instead of a guess.

### Cumulative exposure

When `CumAlert` is enabled, the source uses `DSum([dose], "Print", HN + dcode)`,
divides the result by stored patient BSA, and triggers when the result is greater
than or equal to `CumAlertH`. The `Print` query has no date, `missing`, or `print`
filter. Its inner joins exclude rows lacking its required patient diagnosis/regimen,
drug unit, diluent, route, doctor, or ward relationships.

OncoFlow reproduces that compatible-row scope. Equality triggers. It performs no
unit conversion and evaluates only `mg.` drugs with a positive migrated
`legacy_bsa`; otherwise it reports unsupported. This is a warning only and never
changes the current or historical dose.

### Dilution incompatibility

Recovered `DilCompat` trims the diluent display text and classifies it by whether
it contains `D` and/or `S`/`N`:

| Diluent category | Output |
|---|---|
| `D`, without `S`/`N` | `D` |
| `S` or `N`, without `D` | `S` |
| both | `T` |
| neither | `-` |

The recovered incompatibility matrix is:

| Drug code | Finding condition |
|---|---|
| `D` | diluent category `D` or `T` |
| `S` | diluent category `S` or `T` |
| `A` | diluent category exactly `D` |
| `B` | diluent category exactly `S` |

No other code is interpreted.

## Finding model and acknowledgement

Each finding records deterministic ID, rule ID, ruleset version, severity,
status, title, message, source, evidence values, item association, and whether
review acknowledgement is requested. Only `info` and `warning` severities are
used. Although two legacy dialogs used a `vbCritical` icon, the source did not
define a durable clinical severity model; Milestone 8 does not invent one.

Acknowledgement is deliberately session-only and labelled “Acknowledged for this
review.” It does not mean approval, is not persisted, resets whenever evaluation
is refreshed, and never gates editing or saving. Therefore no acknowledgement
schema migration is justified.

## Schema decision

No schema migration is required. All required supported inputs already exist in
schema version 3, findings are recomputed read-only, and acknowledgement is not a
durable clinical record in this milestone.

## Implementation plan

1. Add a separate `safety` Rust domain with versioned models, pure threshold/code
   rules, allow-listed SQLite reads, an evaluator, and one typed Tauri command.
2. Return no retrospective findings for migrated historical orders.
3. Integrate the command into every order-detail refresh after create, edit,
   regimen initialization, line add/edit/remove, and reordering.
4. Add an explainable persistent safety panel with warning, information, and
   pending-investigation groups plus session-only acknowledgement.
5. Add synthetic Rust and frontend tests for all boundaries, unsupported inputs,
   deterministic results, acknowledgement behavior, and non-mutation.
6. Run the complete Rust, frontend, Tauri, AppData integrity/count, Git tracking,
   startup, and immutable-MDB verification matrix.

## Implemented vertical slice

- `src-tauri/src/safety/model.rs` defines versioned findings, evidence, severity,
  status, and historical-versus-active evaluation modes.
- `src-tauri/src/safety/rules.rs` contains pure, deterministic fixed-point
  comparisons for the confirmed concentration and cumulative thresholds plus the
  exact recovered dilution-code matrix.
- `src-tauri/src/safety/repository.rs` contains allow-listed, read-only SQLite
  queries. The cumulative query deliberately retains the recovered `Print` query's
  inner-join scope and lack of date/printed/missing filters.
- `src-tauri/src/safety/evaluator.rs` combines rule outcomes with confirmed local
  presence checks. Unknown or incomplete rules produce explicit unsupported
  findings rather than inferred clinical behavior.
- `src-tauri/src/safety/commands.rs` exposes only the typed
  `evaluate_order_safety` IPC command; it does not expose SQL or mutation.
- The order detail screen refreshes safety review after creation, header editing,
  regimen initialization, and every line add/edit/remove/reorder. Evaluation
  failure is isolated from the already-validated order transaction.
- The persistent safety panel groups warning, information, and pending findings;
  exposes source, rule version, observed/configured evidence; and offers explicit
  session-only acknowledgement. It never disables editing or saving.

No schema migration was added. Existing `AlertRec` records are neither read as
current acknowledgement nor modified.

## Completion verification — 2026-08-23

The normal AppData database was checked before and after starting the optimized
application. Only aggregate counts were printed.

| Check | Result |
|---|---|
| schema version | `3` |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| historical order headers | 1 before, 1 after |
| historical order items | 2 before, 2 after |
| all order headers/items | 2 / 3 before and after |
| Rust tests | 64 passed |
| `cargo fmt --all -- --check` | passed |
| strict Clippy (`--all-targets --all-features -- -D warnings`) | passed |
| frontend tests | 20 passed |
| frontend typecheck/lint/build | passed |
| optimized Tauri executable | built and started successfully |
| NSIS release bundle | built successfully |
| tracked DB/MDB files | 0 |

The controlled startup used the normal local AppData `oncoflow.db`, remained
running through the startup observation period, and was then stopped by its exact
test process ID. Counts and integrity remained unchanged.

The default all-bundles command also built the optimized executable, but WiX MSI
ICE validation could not run because the host Windows Installer service was not
accessible (`LGHT0217`). The independently selected NSIS release bundle completed;
this is a host MSI-tooling limitation, not a compilation or application-startup
failure.

Final SHA-256 values match the pre-implementation evidence values:

```text
AllTable.mdb       C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3
Cytotoxic V8.0.mdb 2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D
```
