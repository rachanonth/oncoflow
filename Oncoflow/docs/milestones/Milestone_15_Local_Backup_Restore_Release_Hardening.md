# Milestone 15 — Local Backup, Restore & Release Hardening

## Scope and baseline

Milestone 15 stabilizes the completed local chemotherapy-preparation workflow. It adds no clinical calculation, safety threshold, eligibility rule, preparation behavior, inventory algorithm, barcode, external connection, or network dependency.

The aggregate-only AppData baseline inspected before implementation was:

| Check | Baseline |
| --- | ---: |
| Schema version | 8 |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| Orders / items | 3 / 4 |
| Historical orders / items | 1 / 2 |
| Preparation tasks | 0 |
| Inventory movements / quantity sum | 48 / 546.1061496734619 |
| Users / audit events | 2 / 6 |
| Preparation output snapshots | 0 |

No row-level patient, medication, note, password, or hash value was selected or printed.

## Backup design

Manual backup uses SQLite's online backup API. It does not copy the live `oncoflow.db` file with filesystem copy semantics, so WAL activity and concurrent SQLite readers do not produce an inconsistent snapshot.

The authenticated workflow is:

```text
choose local folder
  -> create a uniquely named full SQLite snapshot
  -> open the snapshot independently
  -> integrity_check
  -> foreign_key_check
  -> confirm OncoFlow schema/version
  -> SHA-256 the completed file
  -> write a non-clinical JSON manifest
  -> append database_backup_created audit event
```

Filename format:

```text
OncoFlow_Backup_YYYY-MM-DD_HHMMSS.db
OncoFlow_Backup_YYYY-MM-DD_HHMMSS.db.manifest.json
```

The whole SQLite database is included. No table is selected or excluded, so users/password hashes, clinical records, preparation/output snapshots, inventory, acknowledgements, audit events, and `app_meta` are all preserved. The manifest contains only application/schema versions, timestamp, basename, byte size, integrity/FK results, and checksum. It contains no patient or credential data. SHA-256 detects accidental corruption; it is not encryption or tamper-proof security.

Manual backups are written only to the folder selected by the operator. A local disk, removable drive, or an operator-chosen Windows-mounted folder works; OncoFlow does not upload or require a network destination. No scheduler or unlimited automatic-backup set was introduced.

## Restore safety and schema compatibility

Restore is a whole-database operation. It never merges tables.

Preflight opens the selected file read-only and requires:

- a valid SQLite database;
- `app_meta.schema_version`;
- the expected OncoFlow core tables (`users`, `patients`, `drugs`, and `orders`);
- `integrity_check=ok`;
- zero foreign-key violations;
- schema version 1–8; and
- a matching sidecar checksum/metadata when a manifest is present.

Random SQLite files, corrupt files, unsupported pre-schema files, the active database itself, and future schemas above version 8 are rejected. A preflight confirmation token incorporates the candidate checksum, size, and schema; changing the file after preflight invalidates confirmation.

The restore sequence is:

```text
validated candidate
  -> copy candidate to an AppData staging DB with SQLite backup API
  -> run normal supported migrations on staging only
  -> validate staged schema 8 DB
  -> append restore-started event to current DB
  -> create and validate a full pre-restore recovery DB
  -> SQLite online restore into the active DB
  -> append restore-completed event to restored DB
  -> final integrity/FK validation
  -> invalidate the Rust session
  -> reload workspace and use restored identities
```

SQLite's backup transaction replaces the active database atomically at page level. A failure before replacement leaves the active database untouched apart from the explicit restore-start audit. A failure after replacement triggers automatic restoration of the validated pre-restore copy; tests inject a restored-database audit failure to prove that behavior. If recovery itself fails, OncoFlow reports a stop/exit preservation error rather than continuing on uncertain data.

The current database must be snapshotted and validated before restore. A corrupt current database that cannot produce a validated recovery copy is therefore not overwritten automatically; the recovery screen provides retry and data-folder access so the damaged file can be preserved for controlled recovery.

Backups at schema 8 restore directly. Schemas 1–7 migrate in staging before the live DB changes. Schema above 8 is rejected without downgrade. Restored `users` and password hashes are authoritative; the old process session is cleared and no default credential is created. A restored DB with no active Argon2id user follows the established first-run bootstrap behavior.

Audit provenance crosses a database-replacement boundary honestly: `database_restore_started` remains in the pre-restore recovery copy, while `database_restore_completed` is appended to the restored database with no fabricated user foreign key. The initiating process required an authenticated user during normal operation. Recovery-mode restore is available only when startup has already classified the database unavailable. This is operational evidence, not a tamper-proof ledger.

## Startup and migration hardening

Tauri setup now always manages a database path plus an explicit startup state. If an existing DB is corrupt, locked, unrecognized, migration-failed, or uses an unsupported schema, the application opens a recovery-oriented screen instead of failing setup or mounting the clinical UI.

An absent database on a genuine first install initializes normally. A non-empty existing random or damaged file is never converted into a new empty OncoFlow database. Retry, validated restore, data-folder access, and exit through the window controls remain available.

Before applying a supported migration to an existing database, initialization creates and validates a SQLite snapshot under `backups/migration`. If that snapshot cannot be created and validated, migration does not begin. Migration SQL remains transactional. OncoFlow retains only its newest seven `pre_migration_schema_*.db` files; it never rotates manually selected backups.

## Diagnostics, printing, and privacy hardening

Settings now contains Backup & Restore and Diagnostics. Diagnostics reports only:

- application, DB schema, clinical ruleset, label layout, and label renderer versions;
- database location/size, integrity, FK status, and last in-database backup audit timestamp;
- workstation platform; and
- configured local printer queue/language and whether Windows currently exposes that queue.

Printer discovery remains read-only. Availability is not reported as physical output. The only physical connectivity check remains the operator-controlled test label under Settings > Hardware. No print occurs on startup or in automated tests; Windows accepting `WritePrinter` still means only that the spooler accepted the job.

The new runtime command errors are allow-listed, generic, and tested not to serialize filesystem source errors containing synthetic passwords, patient names, or sensitive paths. The implementation adds no persistent production logger. Existing user-triggered recovery paths return controlled errors rather than `unwrap`, `expect`, or panic. Test-only invariants continue using those helpers.

## Windows data locations and retention

Default locations for identifier `com.laste.oncoflow` are:

| Data | Windows location / behavior |
| --- | --- |
| Runtime database | `%APPDATA%\com.laste.oncoflow\oncoflow.db` |
| SQLite WAL/SHM while active | same AppData folder; never copy these manually as a backup |
| Pre-migration recovery | `%APPDATA%\com.laste.oncoflow\backups\migration` (newest seven managed by OncoFlow) |
| Pre-restore recovery | `%APPDATA%\com.laste.oncoflow\backups\restore` |
| Temporary restore staging | `%APPDATA%\com.laste.oncoflow\backups\staging` (removed after each operation) |
| Manual backups | operator-selected folder |
| Printer preferences | WebView2 local storage under `%LOCALAPPDATA%\com.laste.oncoflow\EBWebView` |
| Application logs | no OncoFlow persistent clinical log file is currently written |

The Diagnostics and recovery screens provide an explicit Open data folder action. The supported installer target remains NSIS only. Application binaries install outside Roaming AppData, and no custom installer/uninstaller hook targets the clinical data directory. Upgrade and uninstall must not be treated as backup operations: operators should create a validated manual backup first. Manual removal of OncoFlow clinical data requires intentionally removing `%APPDATA%\com.laste.oncoflow` after the application is closed; uninstall does not authorize that removal.

## Separate version identities

These identifiers evolve independently:

| Concept | Current identity |
| --- | --- |
| Application semantic version | `0.1.0` development line |
| SQLite schema version | `8` |
| Legacy clinical ruleset | `legacy-cytotoxic-v8` |
| Preparation label content layout | `oncoflow-preparation-label-v1` |
| Windows raster renderer | `oncoflow-raw-label-raster-v2` (RC1 TSPL polarity correction) |

Future releases should increment the application SemVer for distributable changes, schema version only for migrations, the clinical ruleset only after reviewed parity/rule evolution, and the label version only for output-content layout changes.

## Automated coverage

Synthetic Rust tests cover full backup creation, schema/integrity/FK preservation, SHA-256 and manifest generation, clinical non-mutation, authentication, corrupt/random/future rejection, changed-candidate invalidation, schema-7 staged migration, pre-restore recovery creation, restored-user authority, session invalidation, post-replacement rollback, missing/unrecognized/corrupt startup behavior, migration-failure safety backup, destination failure, privacy-safe diagnostics/errors, and read-only missing-printer detection.

Frontend tests cover backup success metadata, restore preflight, explicit confirmation, schema migration notice, recovery mode, database recovery actions, diagnostics/version display, and missing-printer language. Fixtures contain synthetic values only.

## Deliberate limitations

- Backups are not encrypted. Operators control physical/removable-drive security.
- Direct SQLite/file administrators can replace or modify data and audit files; no cryptographic non-repudiation is claimed.
- There is no cloud backup, scheduler, remote restore, email recovery, auto-update, barcode, or new clinical behavior.
- A corrupt active database is preserved rather than overwritten if a validated pre-restore recovery copy cannot be made.
- Printer queue presence does not prove driver/device/media readiness.

## Completion validation

Milestone 15 completed without a schema migration. The production schema remains version 8.

Automated release validation passed on 2026-08-23:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | passed |
| strict Clippy (`--all-targets --all-features -- -D warnings`) | passed |
| Rust tests | 149 passed |
| Frontend tests | 58 passed across 17 files |
| Frontend typecheck | passed |
| Frontend lint | passed |
| Frontend production build | passed |
| Tauri release build | passed |
| NSIS packaging | passed |
| Release and installed-app startup | passed; exact launched processes were stopped after the check |

The actual AppData database was checked before and after release startup and installer exercises. It remained at schema 8 with `integrity_check=ok` and zero `foreign_key_check` rows. The observed invariants remained unchanged: 3 orders, 4 order items, 0 preparation tasks, 48 inventory movements with aggregate quantity `546.1061496734619`, 2 users, 6 audit events, and 0 preparation label snapshots. No clinical or inventory mutation was introduced by validation.

Controlled backup and restore coverage used isolated synthetic SQLite databases. It verified full-database backup, SHA-256/manifest validation, a schema-7 migration path, restored-user authority, pre-restore recovery creation, changed/corrupt/random/future candidate rejection, and rollback after an injected post-replacement failure. It did not use the production AppData database or either legacy MDB as a restore target. First-run account creation remains covered by the existing isolated authentication tests; no default credentials were introduced.

NSIS was installed twice to exercise upgrade behavior, then uninstalled and reinstalled. In every case the AppData database hash was preserved byte-for-byte. Uninstall removed the application binaries while leaving `%APPDATA%\com.laste.oncoflow` and its clinical database intact. The generated installer is `OncoFlow_0.1.0_x64-setup.exe` (3,258,722 bytes), SHA-256 `B85505E558FD177D44238B45CC874F8586D6A6A5FEA5215AD93F2BB020BF2BB0`.

Read-only Windows queue discovery found `Xprinter XP-420B` available. No automated or unsolicited physical print was sent. An operator-controlled test label remains the required hardware/media check.

Git tracks no `.db`, `.mdb`, `.accdb`, WAL, or SHM file. The legacy MDB hashes remained unchanged:

- `AllTable.mdb`: `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3`
- `Cytotoxic V8.0.mdb`: `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D`

Manual operator acceptance still includes choosing a real backup destination (for example a USB drive) and intentionally sending an Xprinter test label. Those actions were not automated because they affect operator-selected storage and physical media.
