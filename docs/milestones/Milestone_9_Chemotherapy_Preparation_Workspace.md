# Milestone 9: Chemotherapy Preparation Workspace

## Scope and architecture

Milestone 9 adds a local pharmacist preparation queue and workspace. SQLite remains the only runtime data source. Preparation records are derived from local OncoFlow orders and never update the source order, regimen, patient, inventory, or administration data.

This milestone does not reproduce legacy paper forms, implement stock deduction or administration, fetch external data, or add a clinical formula that was not established by legacy evidence.

## Pre-implementation database verification

The normal AppData database was inspected using aggregate queries only:

| Check | Result |
| --- | --- |
| Schema version | 3 |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| Historical orders / items | 1 / 2 |
| OncoFlow-created orders / items | 1 / 1 |
| Historical items with the legacy preparation marker | 2 |
| OncoFlow-created items with the legacy preparation marker | 0 |

The existing queue is therefore expected to be empty until a local editable order contains an eligible preparation item. Historical records are not silently converted into OncoFlow preparation tasks.

## Safely inspected legacy evidence

The Access databases were treated as read-only reference material. Saved query SQL and exported object definitions were inspected from a disposable copy with startup behavior disabled; no legacy form or workflow was executed. The disposable files were removed after inspection.

| Evidence | Confidence | Milestone 9 interpretation |
| --- | --- | --- |
| The order-detail drug selector filters `Tbldrug.marker=True` | CONFIRMED | `drugs.marker` is the deterministic legacy selector for preparation-workspace eligibility. |
| Marked drugs include antineoplastics and protocol-linked products such as IV mesna; unmarked data includes routine supportive/oral products | CONFIRMED from aggregate/configuration inspection | The marker supports the required chemotherapy-preparation boundary without hard-coded drug names. |
| `Print`, `Print Continue`, and `Print WF One Day` use ordered dose, dose/pack, volume/pack, diluent, diluent volume, route, rate, detail, storage, warning, expiry, and sequence | CONFIRMED | These are preparation display inputs. The order values remain authoritative. |
| Legacy `sumvol` is `(dose * volume/pack) / dose/pack`; package-equivalent is `dose / dose/pack` | CONFIRMED | May be displayed as a reference quantity when every operand is present and valid. No rounding, unit conversion, or order mutation is added. |
| `order details.print=True` is set by a legacy confirmation/printing workflow | CONFIRMED | It is a print/workflow flag, not proof of pharmacist verification and is not migrated into the new verification state. |
| `worker` identifies an order recorder/editor | PARTIALLY_CONFIRMED | It is not treated as a compounder/checker identity. |
| Legacy records contain a durable preparation task with preparer/checker and preparation timestamps | UNKNOWN / no supporting record found | Do not claim legacy verification provenance. New tasks apply only to OncoFlow-created orders. |
| Marker values encode the finer roles `antineoplastic`, `protocol_adjunct`, `rescue_protective`, or `preparation_fluid` | UNKNOWN | Do not infer or persist a role from a drug name. The raw legacy marker remains the compatibility rule. |
| `Drug Administration` represents pharmacy preparation | CONTRADICTED by schema/usage | Excluded; administration is outside Milestone 9. |
| Legacy inventory-update queries should run during preparation | CONTRADICTED by scope | Excluded; no inventory mutation is performed. |

## Eligibility decision

An order item is eligible when all of the following are true:

1. it belongs to an OncoFlow-created, editable local order;
2. it references a local drug; and
3. the drug's migrated legacy `marker` is true.

The rule has an explicit identity (`legacy-cytotoxic-v8:preparation-marker`) and is deterministic. It does not inspect or match a drug name. Unmarked items remain visible only through the order workflow and are counted as excluded in the workspace; they do not create preparation tasks.

The conceptual role taxonomy requested by the product scope remains useful documentation, but the legacy data does not safely distinguish those roles. No role column is added in this milestone.

## Order-to-preparation mapping

| Preparation concept | Source |
| --- | --- |
| Source order and item | `orders.id`, `order_items.id` |
| Patient/order identity | Existing patient and order joins; no patient snapshot in the preparation table |
| Regimen / treatment day | Existing order and order-item fields |
| Drug | `order_items.drug_id` and local drug master |
| Ordered dose | Existing raw/numeric order-item dose; snapshotted verbatim at task creation |
| Dose unit | Existing order-item unit; snapshotted verbatim |
| Diluent / volume | Existing local diluent and order-item volume; snapshotted |
| Route / rate | Existing route and raw rate; snapshotted |
| Sequence | Existing order-item sequence; snapshotted |
| Preparation instructions | Existing regimen/drug detail and storage text, displayed without reinterpretation |
| Reference drug quantity | Confirmed legacy formula, calculated for display only when supported |
| Safety | Re-evaluated using the Milestone 8 safety evaluator; findings remain explainable and non-mutating |

## Persistence and workflow decision

Migration 004 is required because the legacy schema has no defensible durable preparation-verification record, while OncoFlow must record new preparation work honestly and transactionally. It adds one `preparation_tasks` row per eligible source order item.

The table stores a source reference plus a compact snapshot of prescribed/preparation inputs. The snapshot is justified by historical integrity: it records exactly what the pharmacist reviewed, permits stale-source detection before verification, and never writes those values back to the order. Patient names and addresses are not duplicated.

Only three states are introduced:

- `pending`: initialized but not marked prepared;
- `prepared`: preparation values reviewed/entered and ready for verification;
- `verified`: immutable verification of the stored snapshot.

No administration, dispensing, billing, inventory, treatment approval, or paper-print status is represented. Preparer/checker identity is deferred because the current application does not yet provide an authenticated local user identity; legacy plaintext credentials are not reused. Timestamps are stored without fabricating an identity.

Safety acknowledgement remains an explicit, transient verification input. Verification re-evaluates current findings inside the transaction, requires acknowledgement of current warnings for the item, rejects changed source-order snapshots, and never changes clinical values. It does not persist a treatment approval.

## Implementation plan

1. Add migration 004 with the smallest preparation task/snapshot table, constraints, and indexes; update migration tests.
2. Add a pure, versioned eligibility layer and reference-quantity helper, then implement repository, service, typed commands, transactions, stale-source checks, and safety propagation.
3. Add a desktop preparation queue and workspace with empty/loading/error states, preparation editing, persistent safety findings, explicit prepare/verify actions, and excluded-item visibility.
4. Add synthetic Rust and frontend tests for eligibility, adjunct inclusion, supportive exclusion, lifecycle, rollback, immutability, safety acknowledgement, UTF-8, and UI behavior.
5. Validate SQLite integrity/foreign keys and unchanged historical counts; run formatting, strict Clippy, Rust/frontend tests, typecheck, lint, production build, NSIS-only Tauri release build, startup check, tracked-database checks, and legacy MDB hash comparison.

## Legacy file integrity baseline

| File | SHA-256 before implementation |
| --- | --- |
| `legacy/AllTable.mdb` | `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3` |
| `legacy/Cytotoxic V8.0.mdb` | `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D` |

## Implemented result

- Added migration 004 and upgraded the runtime schema to version 4.
- Added a dedicated Rust `preparation` domain with typed models, SQLite-only repository access, service validation, eligibility, typed Tauri commands, stale-source detection, and immediate transactions.
- Added a database-backed Preparation Queue and a pharmacist-oriented Preparation Workspace.
- Added display-only legacy preparation reference quantities with formula and provenance; they do not update the order or inventory.
- Integrated active Milestone 8 findings and session-only explicit acknowledgement into verification. Applicable current warnings must be acknowledged before a prepared task can be verified.
- Added synthetic Rust and frontend coverage for marked/unmarked eligibility, a mesna-like protocol adjunct, queue/search, Thai UTF-8, preparation lifecycle, NULL values, safety propagation, acknowledgement, transaction rollback, historical immutability, stale order detection, and non-mutation of order/regimen/patient/inventory data.

## Final validation

The release application was started against its actual AppData database. Startup remained running normally during the check, and migration 004 applied without backfilling historical or local orders.

| Validation | Result |
| --- | --- |
| Actual AppData schema version | 4 |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| Historical orders / items after migration | 1 / 2 (unchanged) |
| OncoFlow-created orders / items after migration | 1 / 1 (unchanged) |
| Preparation tasks after migration | 0 (no automatic backfill) |
| `cargo fmt --all -- --check` | passed |
| `cargo test --all-targets --all-features` | 78 passed |
| strict `cargo clippy --all-targets --all-features -- -D warnings` | passed |
| Frontend tests | 25 passed |
| Frontend typecheck | passed |
| Frontend lint | passed |
| Frontend production build | passed |
| Tauri release build | passed, NSIS only |
| Release application startup | passed |
| Tracked `.db` / `.sqlite` / `.mdb` files | none |
| Legacy MDB SHA-256 values | unchanged from baseline |

NSIS artifact: `src-tauri/target/release/bundle/nsis/OncoFlow_0.1.0_x64-setup.exe`. MSI packaging was intentionally not invoked.

## Remaining compatibility risks

- The confirmed legacy marker is a coarse preparation-workspace selector. It does not encode the finer conceptual product role, so the UI does not claim that distinction. A future reviewed local classification mechanism may be needed if a marked legacy drug is found outside the preparation service.
- No authenticated local identity is available yet. Preparation and verification timestamps are recorded, but the application does not fabricate a preparer/checker identity or reuse legacy plaintext credentials.
- Safety acknowledgement is deliberately session-only and is not represented as treatment approval. Verification records the reviewed task snapshot, not a persisted copy of finding messages.
- The confirmed legacy reference-quantity formula is applied only to compatible numeric source values and performs no unit conversion. Missing or invalid configurations remain explicitly unavailable/unsupported.
- Existing local development data currently contains no marked order item, so the actual AppData preparation queue correctly starts empty. Synthetic tests cover the complete workflow without contaminating that database.
