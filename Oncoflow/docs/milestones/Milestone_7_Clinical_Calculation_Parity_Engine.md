# Milestone 7 — Clinical Calculation Parity Engine

## Scope and safety boundary

Milestone 7 introduces a pure, deterministic Rust engine for the narrowly
established legacy behavior of `StandardDose`, `ANCCal`, `ANCGrade`, `Platelet`,
`LabMinMax`, and `FixNumber`.

The engine returns a value, status, ruleset identity, confidence, non-identifying
inputs, trace, and warnings. It does not read or write SQLite, mutate patients,
regimens, orders, or order items, approve/block treatment, make recommendations,
or connect to any external database. Order integration is outside this milestone.

The immutable ruleset identifier is:

```text
legacy-cytotoxic-v8
```

## Pre-milestone runtime verification

The normal packaged OncoFlow application was launched against its actual local
AppData database before implementation. Normal startup had already applied
migration 003. A privacy-safe aggregate check produced:

| Check | Result |
|---|---:|
| Schema version | `3` |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| Migrated historical orders | 1 |
| Migrated historical order items | 2 |
| OncoFlow-created orders | 1 |
| OncoFlow-created order items | 1 |

The historical baseline remains exactly one header and two items. No row contents
or patient-identifying fields were printed.

## Safe evidence collection

Evidence was collected without executing saved queries, forms, reports, startup
behavior, or legacy calculation functions:

- DAO opened both MDBs read-only to inspect saved query SQL and local schema;
- a disposable copy of `Cytotoxic V8.0.mdb` had its `StartUpForm` property removed;
- Office automation security was forced to disable macros before the copy opened;
- Access `SaveAsText` exported the `Function` module and object definitions without
  opening any form/report;
- only the six target bodies and matching definition lines were examined;
- the disposable copy and exports were deleted;
- the original front-end hash remained
  `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D`.

No full VBA function was invoked to manufacture expected outputs. Reference cases
are derived from recovered source and documented VBA coercion behavior.

## Rule inventory

Confidence applies to the engine's supported input subset. Locale-dependent
`Variant`/`IsNumeric` forms, external lookups, and missing-value coercions are
explicitly excluded when they cannot be reproduced safely.

### `StandardDose` — PARTIALLY_CONFIRMED

Recovered signature:

```text
Function StandardDose(Number, Surface As Variant) As String
```

Confirmed behavior:

- `IsNumeric(Number)` selects the numeric branch;
- calculate `Surface * Number`;
- a product below 10 is returned without explicit rounding;
- a product equal to or above 10 is passed through VBA `Int` (floor);
- a nonnumeric `Number` is split only by total character length:
  lengths 3/4/5/6/7/8/9 use left/right widths 1/1, 1/2, 2/2, 2/3,
  3/3, 3/4, and 4/4 respectively;
- the middle character is not validated as a delimiter;
- the lower product is passed through `Int`;
- because `Dim A, B, C, D, E As Integer` declares only `E` as an Integer,
  the upper product is first coerced to a signed 16-bit VBA Integer (nearest,
  midpoint-to-even) and can overflow, while `D` remains a Variant;
- output is text, with range values joined by `" - "`.

The saved `~sq_fStandard dose` query calls
`standarddose([dose],[bsa])`, filters `bsa <> 0`, and joins local patient,
regimen, regimen-detail, drug, and unit data. No maximum-dose or other safety rule
is called by this function.

Inputs are raw dose text/value and surface value. The source names BSA but does not
establish a unit. NULL `Variant` behavior and locale/currency forms accepted by
Access `IsNumeric` were not captured as reference outputs, so the engine returns an
explicit unsupported result for those inputs.

### `FixNumber` — CONFIRMED for decimal inputs

Recovered signature:

```text
Function FixNumber(Number As Variant) As String
```

If the value is nonnumeric, the function exits with its default empty String.
Otherwise it returns `Int(Number) + 1` when `Number > Int(Number)`, else
`Int(Number)`. This is mathematical ceiling, including negative values; it is not
VBA `Round` and does not use midpoint-to-even rounding.

The saved `Safe Value` query calls `FixNumber([dose]/[dose/pack])` to obtain whole
package count. NULL/malformed input maps to an unavailable engine result with a
trace noting the legacy empty String. Locale-specific currency/thousands syntax is
unsupported rather than guessed.

### `ANCCal` — PARTIALLY_CONFIRMED

Recovered signature:

```text
Function ANCCal(Number1, Number2 As Variant) As String
```

The legacy function treats `Number1` as HN and `Number2` as result date, then
queries linked external CBC rows named exactly `Neutrophil` and `WBC count` with
lab code `CBC   `. If records exist, it computes:

```text
(neutrophil × WBC) / 100
```

There is no explicit rounding. Missing rows leave local variables at Empty/zero;
the unusual VBA gate `If W And N <> 0 Then` prevents assignment for ordinary zero
or missing cases, leaving the default empty String. Units are not stated in the
source.

OncoFlow does not reproduce the forbidden HN/date external lookup. Its pure rule
accepts supplied local numeric WBC and neutrophil values. The gate's VBA numeric
logical coercion is preserved for the supported fixed-point subset and traced.
Missing/zero inputs return unavailable, and negative values remain calculated with
an explicit warning because the legacy source contains no validation.

### `ANCGrade` — CONFIRMED thresholds, PARTIALLY_CONFIRMED lookup/gate

`ANCGrade` duplicates the same external CBC lookup and ANC expression. Its exact
comparison order is:

| Calculated ANC | Legacy output |
|---:|---|
| `> 1500` | `-` |
| `>= 1000` and `<= 1500` | `1` |
| `>= 500` and `< 1000` | `2` |
| `>= 100` and `< 500` | `3` |
| `< 100` | `4` |

Equality at 1500 is grade 1, not `-`. Negative direct values are grade 4 because
the source contains no nonnegative guard. ANC calculation and grade classification
remain separate pure functions. The external lookup is not implemented.

### `Platelet` — PARTIALLY_CONFIRMED

Recovered `Platelet(Number1, Number2)` performs no grading, normalization, or
threshold calculation. It looks up external CBC rows whose result name is
`Platelet count` and lab code matches `CBC*`, then returns `real_res` unchanged as
String. Missing data leaves the default empty String. Units are not stated.

The pure OncoFlow rule therefore accepts an already supplied raw local lab value
and returns it unchanged. HN/date lookup is unsupported. Direct form code elsewhere
uses `Val(real_res) <= DLookup("Platelet", "MinMax")` for an alert. That separate
alert workflow is not silently folded into the `Platelet` function.

The source `MinMax` row contains a raw platelet threshold of 100000; this is
configuration evidence only, with no inferred unit or recommendation.

### `LabMinMax` — CONFIRMED transformation, UNKNOWN purpose

Recovered signature:

```text
Function LabMinMax(Number As Variant) As String
```

If `IsNumeric(Number)` is true, the input is assigned to a String and returned.
Otherwise the function returns `-`. No explicit rounding or min/max comparison is
performed. No saved query, form, or report call site was recovered, so the name
must not be interpreted as generalized laboratory range logic. NULL is nonnumeric
and returns `-`.

### Supporting `MinMax` configuration — CONFIRMED storage, UNKNOWN units

The single local `MinMax` row contains raw thresholds: WBC 3000, ANC 1500,
Platelet 100000, Hb 8, AST 150, Bilirubin 3, and Creatinine 2. The migration stores
these values in `alert_settings`. This milestone does not create generalized lab
alerts or reinterpret their units/boundaries beyond the source code above.

## Numeric compatibility strategy

The engine uses an internal checked fixed-point decimal instead of binary
floating point. It implements:

- exact decimal parsing for an invariant, documented subset;
- checked multiplication and division by powers of ten;
- VBA `Int` floor behavior, including negatives;
- mathematical ceiling for `FixNumber`;
- midpoint-to-even integer coercion for the one recovered VBA Integer assignment;
- signed 16-bit overflow detection for that assignment;
- deterministic non-scientific text output.

Scientific notation can be normalized deterministically. Locale-specific currency,
date, thousands-separator, and hexadecimal forms accepted by some Access
`IsNumeric` configurations return `unsupported` rather than being guessed.

## Result and status model

Every rule returns `ClinicalCalculationResult<T>` with:

- optional `value`;
- `status`: `calculated`, `unavailable`, `unsupported`, or `legacy_error`;
- `ruleset` and `rule_id`;
- confidence;
- non-identifying named inputs;
- ordered trace steps;
- warnings.

Trace data never includes HN, patient names, diagnoses, medication combinations,
or notes.

## Schema decision

No schema migration is required. Results are not persisted, and the mathematical
modules have no database or Tauri dependency. Historical orders and every other
clinical record remain untouched.

The Tauri boundary contains six explicit verification commands—one per target
rule—and no generic calculator or SQL command. Those commands accept only the
non-identifying values required by each pure function and return the same result
and trace model. They have no database handle and therefore cannot write an order.

## Implemented structure

```text
src-tauri/src/clinical/
├── anc.rs
├── commands.rs
├── decimal.rs
├── lab.rs
├── model.rs
├── platelet.rs
├── rounding.rs
├── standard_dose.rs
├── trace.rs
└── tests.rs

tests/fixtures/clinical/
└── legacy_cytotoxic_v8.json
```

The fixture corpus contains 66 synthetic/reference cases: 17 `StandardDose`,
8 `FixNumber`, 12 `ANCCal`, 13 `ANCGrade`, 7 `Platelet`, and 9 `LabMinMax`.
Every fixture records an evidence note and confidence status. It contains no HN,
name, address, order contents, or other patient-identifying data.

## Implementation and parity plan

1. Add the versioned result/trace model and checked decimal compatibility core.
2. Implement only the supported behavior documented above; return explicit status
   for excluded external, NULL, locale-specific, overflow, or malformed cases.
3. Store synthetic JSON fixtures with expected status/value, evidence note, and
   confidence.
4. Test typical/zero/NULL/minimum/maximum/threshold-adjacent/decimal/negative/
   malformed cases, deterministic repetition, and ruleset identity.
5. Test that running every calculation leaves synthetic patient, regimen, order,
   and item rows unchanged.
6. Run the full Rust, frontend, Tauri, SQLite integrity, source-hash, and Git
   tracking verification matrix.

## Explicitly deferred or unsupported

- all external HN/date lab lookup behavior;
- clinical interpretation or recommendations from ANC/platelet values;
- maximum/cumulative dose, BSA derivation, dose adjustment, dilution compatibility,
  alerts, and order mutation;
- locale-dependent `Variant` coercions without captured reference cases;
- any rule outside the six named targets and their fixed-point support operations.

## Completion verification

The built release application started successfully against the normal AppData
database. A final read-only aggregate check after all calculation tests and the
startup check produced the same results as the pre-milestone baseline:

| Check | Final result |
|---|---:|
| Schema version | `3` |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| Migrated historical orders | 1 |
| Migrated historical order items | 2 |
| OncoFlow-created orders | 1 |
| OncoFlow-created order items | 1 |

Validation completed successfully with:

- `cargo fmt --all -- --check`;
- strict `cargo clippy --all-targets --all-features -- -D warnings`;
- all 51 Rust tests, including non-mutation snapshots;
- all 17 frontend tests;
- frontend typecheck, lint, and production build;
- Tauri release compilation plus MSI and NSIS bundle creation;
- hidden application startup.

The source hashes after completion remain:

| Immutable source | SHA-256 |
|---|---|
| `legacy/AllTable.mdb` | `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3` |
| `legacy/Cytotoxic V8.0.mdb` | `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D` |

`git ls-files` reports no tracked `.mdb`, `.db`, `.sqlite`, or `.sqlite3` file.
