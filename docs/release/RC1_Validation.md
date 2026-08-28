# OncoFlow Release Candidate 1 Validation

**Validation date:** 2026-08-23  
**Scope:** feature-frozen Milestones 1–15; defects and regression coverage only  
**Overall status:** **AUTOMATED VALIDATION PASSED — OPERATOR ACCEPTANCE PENDING**

No blocker defect or known data-loss defect remains in the automated scope. RC1 is not yet declared fully accepted because the physical Xprinter label inspection and removable-media backup/preflight are intentionally operator-controlled and have not been performed during this validation run. The current RC installer lifecycle must also be repeated after the operator closes the installed OncoFlow process; that process was not interrupted. No unsolicited print, production restore, or installer action against the running workstation application was performed.

## Independent version inventory

| Version concept | RC1 identity |
| --- | --- |
| Application semantic version | `0.1.0` |
| Database schema version | `8` |
| Clinical ruleset | `legacy-cytotoxic-v8` |
| Preparation label layout | `oncoflow-preparation-label-v1` |
| Windows label renderer | `oncoflow-raw-label-raster-v2` |
| NSIS installer version | `0.1.0` |

These identities are independent. In particular, schema 8 and the legacy clinical ruleset do not imply an application or installer version of 8.

## RC1 findings

### RC1-001 — HIGH — Order create could be submitted twice

- **Subsystem:** Orders / create form
- **Reproduction:** Double-click Create Order before React commits the `saving` state.
- **Expected:** One create command is accepted.
- **Actual:** Two invocations could enter the async handler during the same render frame, risking duplicate new orders with different generated IDs.
- **Fix/status:** **FIXED.** A synchronous `useRef` submission lock now guards the command before the first await and is released in `finally`. Cancel and submit controls remain disabled while saving. A regression test proves that the second acquisition fails until release.

### RC1-002 — MEDIUM — Backup/restore picker failures escaped controlled UI errors

- **Subsystem:** Recovery / Backup & Restore
- **Reproduction:** Make the native file/folder picker fail, or activate the action repeatedly before a selection returns.
- **Expected:** One picker operation is active and any failure appears as a controlled OncoFlow message.
- **Actual:** The dialog call occurred before the guarded block and before busy state, allowing an unhandled rejection and repeated picker activation.
- **Fix/status:** **FIXED.** Busy state begins before opening the native dialog; picker, preflight, and backup errors share the controlled error path; cancel clears the lock. Busy wording now includes the selection phase. Frontend regression coverage was added.

### RC1-003 — MEDIUM — Failed printer refresh could retain stale availability

- **Subsystem:** Settings / Hardware
- **Reproduction:** Load printer queues successfully, then make a later Windows queue refresh fail.
- **Expected:** Availability becomes unknown/unavailable and the UI explains discovery failure.
- **Actual:** The old queue list remained in component state, so a removed or unreachable queue could still appear available.
- **Fix/status:** **FIXED.** A discovery failure now clears the queue list before showing the controlled error. OncoFlow still does not claim that queue presence proves physical paper output.

### RC1-004 — MEDIUM — Sidebar could clip navigation at short desktop heights

- **Subsystem:** Application shell / navigation
- **Reproduction:** Use a short desktop viewport with all Milestone 15 navigation entries visible.
- **Expected:** Every navigation and session action remains reachable.
- **Actual:** The fixed-height sticky sidebar could clip its lower entries.
- **Fix/status:** **FIXED.** The sidebar now scrolls vertically with contained overscroll. Browser viewport checks at 1024×768, 1280×720, and 1920×1080 found no page overflow in the reachable startup/recovery surface.

### RC1-005 — HIGH — Xprinter label raster used reversed physical polarity

- **Subsystem:** Hardware printing / TSPL renderer
- **Reproduction:** Send the RAW TSPL label bitmap to the configured Xprinter queue.
- **Expected:** White label background with black text and rules.
- **Actual:** The printer interpreted the previous payload polarity as a black background with white text.
- **Fix/status:** **FIXED IN SOFTWARE; PHYSICAL RECHECK PENDING.** TSPL payload bytes are now reversed at the printer-language boundary while the immutable label model and preview remain unchanged. ESC/POS polarity is unchanged. The renderer identity is now `oncoflow-raw-label-raster-v2`, and a byte-level regression test verifies white background (`0xff`) and black marked pixels (`0` bits) for this RAW TSPL path.

No LOW or COSMETIC issue was retained after review.

## Synthetic end-to-end workflow

Six Rust integration cases run against isolated temporary SQLite databases. They use synthetic identities and clinical data only and never touch AppData or either legacy MDB.

| Case | Result | Verified behavior |
| --- | --- | --- |
| A — normal stock | Passed | Patient/regimen/order flow, deterministic safety and preparation calculation, prepare/verify, exactly one inventory issue, retry/reload idempotency, immutable label content, print/reprint simulation without spooler, unchanged order values, audit provenance, validated backup |
| B — shortage | Passed | Stock 1, supported requirement 3, verification succeeds, one issue posts, resulting balance `-2`, state `Shortage`, and label output remains available |
| C — unsupported calculation | Passed | Incompatible dose/presentation units stay unsupported, no conversion is guessed, verification succeeds, no automatic movement posts, and manual reconciliation is exposed |
| D — safety warning | Passed | Confirmed concentration warning is deterministic and explained; verification requires review; acknowledgement persists; relevant input changes produce a new fingerprint; order dose is not mutated |
| E — Thai data | Passed | Synthetic Thai patient, drug, regimen, and preparation text survives SQLite, local search, output serialization, and real raster generation with no replacement character (`U+FFFD`) |
| F — backup/restore | Passed | Full workflow backup, active-data mutation, and controlled restore recover users/password hashes, orders, preparations, inventory, acknowledgements, label snapshots, and backed-up audit history; only the intentional restore-completed event is added afterward |

Case A also verifies that label generation/reprint does not create another inventory movement, re-verify a task, or alter patient, regimen, order, safety acknowledgement, or preparation data.

## Legacy parity spot checks

The existing reference corpus and RC integration coverage were rerun without changing clinical logic.

| Legacy target | RC1 result | Evidence/limitations |
| --- | --- | --- |
| `StandardDose` | Passed for confirmed cases | Confirmed fixture behavior only; unknown NULL/locale forms remain unsupported |
| `ANCCal` | Passed | Typical, zero, NULL, decimal, and boundary fixtures; external lookup/unit semantics remain documented limitations |
| `ANCGrade` | Passed | Exact, immediately-below, and immediately-above thresholds covered |
| `Platelet` | Passed for confirmed passthrough behavior | No treatment recommendation or unsupported external lookup semantics added |
| `LabMinMax` | Passed for supported subset | Broader legacy purpose remains explicitly unknown |
| `FixNumber` | Passed | `.5`, negative, zero, small, large, and precision behavior covered deterministically |
| Container ceiling | Passed | Exact container, below/above one container, exact multiple, and decimal presentation fixtures |
| Withdrawal volume | Passed where unit relationship is confirmed | Rounded to 1 decimal place using the confirmed half-up product rule; unknown/incompatible relationships remain unsupported |
| Concentration alert | Passed | Strict configured boundary behavior covered; no automatic value mutation |
| Cumulative-dose alert | Passed for confirmed subset | Unconfirmed historical inclusion/unit semantics remain unsupported |
| Dilution compatibility | Passed for confirmed exact mappings | Free text remains advisory and is not interpreted as a new rule |

The clinical fixture corpus contains 66 reference cases; the preparation corpus covers exact, decimal, zero, positive/zero/negative projected inventory, unsupported units, and missing presentation data. No textbook or external clinical rule was substituted.

## UI and error-path review

Frontend regression tests exercise Patients, Drugs, Regimens, Orders, Preparation, Inventory, Authentication/Account, Hardware, Backup/Restore, Diagnostics, output preview, safety acknowledgement, loading/error/empty states, and Thai text. Source and interaction review found and fixed the four defects above.

The in-app browser can load only the plain Vite surface and has no Tauri IPC bridge; it was therefore used for responsive startup/recovery layout checks, not as evidence for native authenticated command execution. Native workflows are covered through Rust integration tests and component tests. No production credential or identifiable patient data was entered into the browser.

Controlled automated failure paths passed for:

- missing, corrupt, locked/unrecognized, and migration-failed databases;
- corrupt, random, changed-after-preflight, and future-schema restore candidates;
- unavailable/permission-denied backup destinations;
- restored-database failure with active-DB recovery;
- removed/unavailable printer queue without an automatic print;
- unsupported preparation units;
- repeated verification and inventory-issue idempotency;
- negative inventory and shortage continuation;
- anonymous/expired session mutations and wrong-password login.

Errors return controlled non-PHI messages rather than crashing. The browser-only `window.__TAURI__` absence message is not a production Tauri defect.

## Data integrity and immutability

Read-only aggregate inspection of the actual AppData database reported:

| Check | Result |
| --- | ---: |
| Database | `%APPDATA%\com.laste.oncoflow\oncoflow.db` |
| Schema version | `8` |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | `0` rows |
| Orders / order items | `3 / 4` |
| Preparation tasks | `0` |
| Inventory movements / aggregate quantity | `48 / 546.1061496734619` |
| Users | `2` |
| Safety acknowledgements | `0` |
| Preparation label snapshots | `0` |

Nine audit rows were observed after normal installed-application use; only event-type aggregates were inspected (`user_bootstrapped`, `user_login`, `user_logout`, and `database_restore_completed`). The changing audit count is expected append-only operational activity, not a clinical or inventory mutation. Synthetic RC tests use isolated databases.

RC tests also prove database-enforced single automatic preparation issue, immutable historical reads, immutable verified label snapshots, append-only inventory/audit service behavior, and no unexpected patient/order/regimen mutation.

## Privacy and logging review

OncoFlow currently writes no persistent application clinical log directory under the expected Roaming/Local AppData locations. Runtime logging/source inspection found only the development MDB importer’s table/count diagnostics. New recovery, printer, and diagnostic error paths are generic and have redaction tests. No password, password hash, patient name, address, clinical note, complete identifiable order, or label payload was observed in RC output.

Audit metadata remains minimal and identifier-based; it does not contain label payloads or patient-identifying text.

## Printer validation

Read-only Windows discovery found:

```text
Xprinter XP-420B — Normal — USB002
```

No print was sent. Queue presence proves only that Windows exposes the queue; it does not prove media, gap calibration, orientation, Thai raster quality, or physical output.

**Open operator gate — physical label:** In Settings → Hardware, select `Xprinter XP-420B`, confirm language/dimensions/DPI, intentionally run Test Print, then print one synthetic verified preparation label. Inspect Thai rendering, clipping, alignment, orientation, feed/gap, font size, dose readability, and patient-identifier readability. Record any layout defect separately from clinical-content defects.

## Backup and restore validation

Automated backup/restore validation passed with the SQLite online backup API, SHA-256 manifest validation, schema compatibility, pre-restore recovery, complete-domain restoration, restored-user authority, and rollback injection. It never restored over the primary AppData database.

**Open operator gate — real destination:** Choose a removable drive or other real destination, create a manual backup, confirm the `.db` and manifest, independently compare SHA-256, reconnect the destination, and run restore preflight only. Perform a destructive restore solely in a controlled test environment.

## Installer and release artifact

The fresh RC1 NSIS package built successfully:

```text
C:\oncoflow\src-tauri\target\release\bundle\nsis\OncoFlow_0.1.0_x64-setup.exe
Size:   3,254,436 bytes
SHA-256: BB7A218103498A7EF77DF1C01E3CC795317D1644972FB10C4E8EA3E04F0A87A0
```

Milestone 15’s controlled installer evidence covered clean install, two upgrade installs, uninstall, and reinstall, with the AppData database preserved byte-for-byte and discovered again. The current RC run rebuilt the package and release application. It did not rerun an installer/uninstaller over the operator’s currently running installed OncoFlow process. No custom uninstaller action targets `%APPDATA%\com.laste.oncoflow`; clinical data removal remains an explicit manual operation after backup.

**Open release gate — current RC installer lifecycle:** After closing OncoFlow, exercise this exact checksummed package through controlled clean-install/first-run and upgrade/uninstall/reinstall scenarios. Hash or safely snapshot AppData before the sequence, confirm upgrade and uninstall preserve it, and confirm reinstall discovers it. Do not use either legacy MDB or the only production database as a destructive test target.

Git tracks no `.db`, `.sqlite`, `.mdb`, WAL, or SHM file. The installer resource configuration includes no database or MDB payload.

Legacy reference hashes remain unchanged:

- `AllTable.mdb`: `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3`
- `Cytotoxic V8.0.mdb`: `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D`

## Regression suite

| Check | RC1 result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| strict Clippy (`--all-targets --all-features -- -D warnings`) | Passed |
| Rust tests | **156 passed, 0 failed, 0 ignored** |
| Frontend tests | **60 passed across 17 files** |
| Frontend typecheck | Passed |
| Frontend lint | Passed |
| Frontend production build | Passed |
| Tauri release build | Passed |
| NSIS package build | Passed |
| `git diff --check` | Passed; line-ending notices only |

No failing test was skipped or hidden to obtain this result.

## RC1 decision

- **Software defect status:** no open BLOCKER; no known data-loss, clinical-regression, duplicate-deduction, or workflow-blocking shortage defect.
- **Automated RC status:** passed.
- **Release acceptance:** pending three explicit gates: physical Xprinter inspection, real-destination backup/preflight, and the current RC package’s installer lifecycle after the running application is closed.
- **Feature freeze:** maintained. No Milestone 16 or new clinical/product feature was started.

Once all three gates are recorded as successful, the documented RC1 pass criteria are satisfied without further feature work. Any physical layout, media, installer, or recovery defect discovered at that stage should be logged as a concrete RC defect and fixed without changing clinical behavior.
