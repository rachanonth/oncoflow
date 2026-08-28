# Milestone 12: Preparation Quantity and Container-Use Parity

## Scope

Milestone 12 adds a deterministic, explainable, read-only preparation calculation for eligible chemotherapy-preparation order items. It does not change an order, preparation record, regimen, patient, or inventory movement. It introduces no automatic stock issue, purchasing, barcode, lot/expiry, cross-patient sharing, or drug-name-specific behavior.

The runtime remains local-only and SQLite-backed. The existing marker-based preparation selector is unchanged.

## Pre-implementation verification

The schema-version-6 AppData database was inspected read-only without selecting patient-identifying values:

- schema version: `6`
- `PRAGMA integrity_check`: `ok`
- `PRAGMA foreign_key_check`: zero rows
- all orders / items: `3 / 4`
- preparation tasks: `0`
- inventory movements: `48`
- inventory ledger raw total: `546.1061496734619`
- drugs: `49`
- drugs with `dose_per_pack` / `volume_per_pack_ml`: `48 / 48`
- drugs with a non-blank raw `package`: `46`
- marker-eligible drugs with non-negative presentation values: `35`
- marker-eligible local order items currently available for a live preview: `0`

These counts differ from the Milestone 11 completion snapshot because local development records were created after that milestone. They are the Milestone 12 preservation baseline; no record was changed during inspection.

Legacy file baselines:

- `legacy/AllTable.mdb`: SHA-256 `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3`
- `legacy/Cytotoxic V8.0.mdb`: SHA-256 `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D`

## Safely inspected legacy evidence

Inspection used migrated schema reports, saved query SQL, and Access query definitions opened read-only. No form, macro, startup behavior, action query, or VBA procedure was executed, and neither source MDB was modified.

### Confirmed

- `Tbldrug.[dose/pack]` is used by the saved `Safe Value` query as the denominator of ordered dose: `[dose]/[dose/pack] AS noavg`.
- `Safe Value` calls `FixNumber([dose]/[dose/pack]) AS fixno`. Milestone 7 established that `FixNumber` returns the mathematical ceiling. This confirms upward whole-container/package rounding for this legacy calculation.
- `Safe Value` calculates `[fixno]-[noavg] AS Theorysafe`. This confirms an unused package-equivalent calculation, but it does not establish whether that remainder is waste or reusable.
- The `Print`, `Print Continue`, and `Print WF One Day` queries calculate `(1/([dose/pack]/[vol/pack]))*[dose] AS sumvol`. Algebraically this is ordered dose multiplied by volume per pack and divided by dose per pack.
- Those queries obtain the order dose and the drug presentation/unit through the same drug relationship. There is no separate legacy order-dose unit conversion expression.
- The raw `package` field is displayed by legacy preparation/print queries. Observed marker-eligible values include `Amp.`, `Vial`, `VIAL`, and `vial`; OncoFlow therefore preserves and displays the raw label rather than assuming every container is a vial.
- The Milestone 11 evidence shows legacy inventory deduction uses `[dose]/[dose/pack]` as `DrugUse`, connecting the legacy inventory quantity to package/container use. Milestone 12 uses only the whole-container requirement for a read-only projection and performs no deduction.

### Partially confirmed

- `dose_per_pack` represents the presentation amount used by the legacy container and withdrawal formulas. Its amount unit is the drug's migrated unit label. Exact pharmaceutical unit semantics are only considered compatible when the order-dose unit label and presentation unit label match after trimming and ASCII case folding; no aliases or conversions are introduced.
- `volume_per_pack_ml` is the legacy presentation volume used by the confirmed withdrawal formula. The migrated column name and existing Drug Master UI label it in mL. This does not authorize conversion from any other volume unit.
- The inventory ledger quantity behaves as a legacy package/container count in the inspected deduction path. Because Milestone 11 documented the physical inventory unit as unresolved, projection is labeled as raw package/container units and remains advisory.
- The legacy `theory` field is related to the `Safe Value` workflow but its complete persistence/use semantics are not established. It remains preserved raw configuration and is not an input to the calculation.

### Unknown / display-only

- `order_items.number_of_drug` originated from `order details.noofdrug`. Milestone 6 intentionally labels it `Legacy quantity`; evidence does not prove that it always means containers, withdrawal volume, or another quantity. It is preserved and displayed as a raw legacy reference but is not silently compared as an equivalent value.
- The raw `Detail` preparation text and `marker` selector do not define unit conversions.
- Whether an unused amount is discarded, reusable, shared, or priced as waste is not established. The result says `unused amount`, never `waste` or `reusable remainder`.
- Multi-dose container reuse and cross-patient sharing are not established and are not implemented.

## Unit-semantics confidence matrix

| Concept | Source | Confidence | Milestone 12 behavior |
| --- | --- | --- | --- |
| Ordered amount | order item dose text/value | CONFIRMED | Authoritative input; never overwritten |
| Ordered amount unit | regimen unit text, falling back to drug unit | PARTIALLY_CONFIRMED | Must be present and match presentation unit exactly after conservative label normalization |
| Amount per container | `drugs.dose_per_pack` | CONFIRMED for the legacy formula | Parsed as fixed-point; must be positive |
| Amount-per-container unit | migrated drug unit | PARTIALLY_CONFIRMED | No alias, scaling, or cross-unit conversion |
| Volume per container | `drugs.volume_per_pack_ml` | CONFIRMED for the legacy formula | Parsed as fixed-point mL; must be non-negative |
| Container label | `drugs.package` | CONFIRMED as display data | Preserved verbatim; blank label makes container semantics unavailable |
| Containers required | `FixNumber(dose / dose_per_pack)` | CONFIRMED | Exact fixed-point ceiling; zero dose requires zero containers |
| Withdrawal volume | `dose * vol_per_pack / dose_per_pack` | CONFIRMED formula; product rounding confirmed 2026-08-27 | Rounded to 1 decimal place using deterministic half-up rounding |
| Unused amount | `(containers * dose_per_pack) - dose` | CONFIRMED mathematically | Labeled unused only, with no waste/reuse interpretation |
| Inventory quantity | Milestone 11 ledger | PARTIALLY_CONFIRMED | Read-only raw container/package projection; shortage never blocks |
| Legacy `noofdrug` | order item field | UNKNOWN | Display-only raw reference; not treated as parity truth |

No `mg`, `mcg`, `g`, `mL`, `IU`, or `unit` conversion is attempted. Punctuation variants are not silently aliased. Missing, malformed, incompatible, or ambiguous relationships return `unavailable` or `unsupported` with a trace.

## Calculation design

The pure `preparation_calc` domain consumes non-identifying strings and returns a versioned explanation:

```text
ordered dose + exact unit relationship + presentation
    -> exact fixed-point ratio
    -> whole-container ceiling
    -> exact withdrawal volume when representable
    -> unused amount (not waste)
    -> optional read-only inventory projection
```

Ruleset: `legacy-cytotoxic-v8+withdrawal-1dp-v1`

Rule: `legacy-cytotoxic-v8:preparation-container-use-withdrawal-1dp`

The implementation reuses the Milestone 7 fixed-point decimal type. Withdrawal division is rounded deterministically to 1 decimal place with midpoint values rounded upward. This product rule was confirmed on 2026-08-27; container count continues to use exact integer ceiling.

Inventory preview states are deterministic and advisory:

- `shortage`: projected balance below zero
- `out`: projected balance equals zero
- `low`: projected balance is non-negative and less than or equal to configured minimum
- `normal`: other tracked balances
- `unknown`: no authoritative balance
- `untracked`: tracking disabled

No preview result changes the ability to save, prepare, or verify.

## Schema decision

No migration 007 is required. Existing drug fields are sufficient for the confirmed compatibility calculation, and unknown presentation/unit relationships must remain unsupported rather than being mass-converted. AppData schema version therefore remains 6.

## Implementation plan

1. Extend the Milestone 7 fixed-point primitive with checked subtraction, integer multiplication, exact terminating division, and exact non-negative ratio ceiling.
2. Add a pure, versioned `preparation_calc` domain with presentation validation, unit compatibility, container count, exact withdrawal, unused amount, trace, warnings, legacy-reference comparison, and inventory projection.
3. Enrich the existing Preparation Workspace repository read with raw decimal text, presentation unit/package, raw `noofdrug`, and ledger balance/minimum without creating any write path.
4. Integrate the result into each eligible workspace item while preserving the Milestone 9 legacy reference display and marker eligibility behavior.
5. Add synthetic Rust fixtures and tests for boundaries, unsupported cases, deterministic arithmetic, parity, projection, shortage, and database non-mutation.
6. Add frontend rendering/tests for supported, partial, unsupported, legacy-reference, and shortage states; ensure shortage never disables workflow actions.
7. Validate the actual AppData database and preservation baselines, then run formatting, strict Clippy, Rust/frontend tests, typecheck, lint, production build, NSIS release build, and a normal application startup check.

Milestone 13 is not included.

## Implemented result

### Rust calculation boundary

`src-tauri/src/preparation_calc/` contains the pure calculation domain:

- `model.rs`: versioned result, presentation, quantity, trace, warning, legacy-reference, and inventory-projection DTOs;
- `presentation.rs`: fixed-point parsing, raw package handling, and conservative unit-label compatibility;
- `container.rs`: exact whole-container ceiling and unused-amount arithmetic;
- `quantity.rs`: orchestration, confirmed withdrawal/concentration behavior, explicit unsupported outcomes, and advisory ledger projection;
- `trace.rs`: non-identifying explanation steps.

The Milestone 7 decimal primitive now supports checked subtraction, integer multiplication, exact terminating division, exact ratio ceiling, and deterministic half-up division to a requested decimal scale. No binary floating-point operation is used by the clinically meaningful calculation.

The engine supports independent partial results. Withdrawal ratios such as `1 / 3` now use the explicitly confirmed 1-decimal product rule, while missing, incompatible, or overflowing inputs remain unavailable or unsupported. The trace records the rounding rule rather than applying it silently.

### Preparation Workspace

The existing typed workspace query now reads, without writing:

- authoritative raw ordered dose and unit;
- amount and volume per container as SQLite decimal text;
- migrated drug unit and raw package label;
- raw legacy `noofdrug` reference;
- Milestone 11 tracking flag, minimum, and ledger-derived current balance.

Every eligible item displays:

- ordered dose separately from calculated values;
- amount/volume per raw package or container label;
- exact withdrawal volume where supported;
- whole-container requirement;
- unused amount, explicitly not called waste or reusable;
- current-to-projected inventory with Normal/Low/Out/Shortage/Unknown/Untracked state;
- ruleset, rule, trace, warnings, and raw legacy-reference provenance.

Shortage is visibly advisory and never participates in order, preparation, or verification button enablement. The calculation path has no inventory repository write and no Tauri mutation command.

The Milestone 9 reference output remains present. It is now produced from the same checked result instead of uncontrolled floating-point formatting. A stored `noofdrug` value is preserved separately and marked not comparable because its semantics remain unknown.

## Automated coverage

The synthetic fixture corpus at `tests/fixtures/preparation_calc/legacy_cytotoxic_v8.json` covers exact, below, above, decimal, zero, unsupported, and inventory-projection cases. It contains no patient data.

Rust tests cover:

- fixed-point parsing and exact terminating division;
- exact one-container, below-container, above-container, exact-multiple, zero, and decimal boundaries;
- confirmed withdrawal and unused-amount calculations;
- terminating and non-terminating withdrawal rounded to the confirmed 1-decimal rule;
- NULL, malformed, incompatible, punctuation-variant, negative, and overflow behavior;
- deterministic repetition and ruleset identity;
- raw legacy-reference preservation without invented meaning;
- positive, zero, and negative projected inventory;
- untracked and unknown inventory;
- calculation/workspace/preparation verification without order, regimen, patient, or inventory movement mutation.

Frontend tests cover supported and unsupported calculation display, ordered/reference separation, container count, raw legacy quantity, trace display, shortage visibility, and a verification action remaining available after safety review even when the preview is negative.

## Completion validation

No migration 007 was added. The optimized application was started normally (hidden only for automated validation) against the actual local AppData `oncoflow.db`, remained running during observation, and was stopped by its exact process ID.

Read-only AppData aggregates were identical before and after startup:

| Check | Before | After |
| --- | ---: | ---: |
| schema version | 6 | 6 |
| integrity check | ok | ok |
| foreign-key violations | 0 | 0 |
| all orders / items | 3 / 4 | 3 / 4 |
| historical orders / items | 1 / 2 | 1 / 2 |
| OncoFlow-created orders / items | 2 / 2 | 2 / 2 |
| preparation tasks | 0 | 0 |
| inventory movements | 48 | 48 |
| inventory movement quantity sum | 546.1061496734619 | 546.1061496734619 |
| safety acknowledgements | 0 | 0 |
| audit events | 5 | 5 |

Validation results:

- `cargo fmt --all -- --check`: passed
- strict `cargo clippy --all-targets --all-features -- -D warnings`: passed
- Rust tests: 118 passed
- frontend tests: 39 passed in 12 files
- frontend typecheck, lint, and production build: passed
- Tauri optimized build: passed
- NSIS installer: built successfully; MSI was intentionally not invoked
- release application startup: passed
- tracked DB/MDB files: zero
- legacy MDB hashes: unchanged from the pre-implementation baselines

NSIS artifact: `src-tauri/target/release/bundle/nsis/OncoFlow_0.1.0_x64-setup.exe`.

## Intentional remaining limitations

- Exact unit aliases and conversions remain unsupported. `mg` and `mg.` are not silently equated.
- Inventory is projected in the unresolved legacy package/container quantity. The UI does not claim a vial, ampoule, or dose unit unless the raw package label provides one.
- `number_of_drug` / `noofdrug` remains an uninterpreted reference rather than a parity oracle.
- Withdrawal values use the confirmed 1-decimal half-up product rule; numeric overflow remains unsupported.
- Unused amount is not classified as waste or reusable, and cross-patient sharing is absent.
- The actual AppData preparation queue currently has no marker-eligible local item; the complete supported/unsupported/shortage UI is verified with synthetic fixtures without contaminating AppData.
