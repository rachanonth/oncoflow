# Milestone 10 — Local Pharmacist Identity, Authentication and Audit Trail

## Scope and baseline

Milestone 10 adds offline local credentials, one in-process authenticated session, preparation attribution, persistent safety acknowledgement, and a minimal append-only audit trail. SQLite remains the only runtime dependency. This is not hospital IAM, SSO, a remote account system, treatment approval, an electronic signature, or a tamper-proof forensic ledger.

The actual AppData database was inspected with aggregate-only, read-only queries before implementation:

| Check | Result |
| --- | ---: |
| Schema version | 4 |
| Users | 1 |
| Active users | 0 |
| Argon2id credentials | 0 |
| Disabled legacy credential placeholders | 1 |
| Historical orders / items | 1 / 2 |
| OncoFlow-created orders / items | 1 / 1 |
| Inventory events | 1 |
| Preparation tasks | 0 |
| Legacy `audit_log` rows | 0 |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |

The Milestone 2 importer deliberately removed `TblUser.password` before its typed row stream reached Rust. It retained `user` and `name`, stored the role compatibility value `user`, disabled the row, and stored the literal non-credential marker `LEGACY_CREDENTIAL_DISABLED`. The legacy source contained one row. Neither its legacy password nor the placeholder can authenticate in OncoFlow.

## Authentication design

- There are no default credentials. First-run state is determined by the absence of an active user whose `credential_kind` is `argon2id`, not by the total number of legacy metadata or inactive rows. An inactive-only database can establish a new first active administrator without reactivating or impersonating the old identity.
- Bootstrap creates the first local administrator credential. If its normalized username matches the disabled legacy metadata row, that row is claimed in place so its non-secret identity provenance is retained; otherwise a new local row is inserted.
- The database compatibility role `user` is presented as `pharmacist`; `admin` is presented as `admin`. No complex permission tree or clinical competency inference is introduced. This milestone does not add a user-management console.
- Passwords are hashed in Rust with Argon2id v19, an independent random salt, and explicit parameters. Only the PHC password hash is stored. The password and hash are never serialized to the frontend or audit metadata.
- Login verification uses the Argon2 password-hash implementation. Login failure is intentionally generic and does not reveal whether a username exists, is inactive, is legacy-only, or had the wrong password.
- The authenticated user is held behind a Rust process-local lock. Restart requires login. The frontend never supplies an actor user ID for a clinical action.
- Bootstrap, login, logout, current-state lookup, and password change are the only identity commands in this milestone. Remote reset, recovery questions, email recovery, and default credentials are absent.

## Migration 005 decision

Migration 005 preserves migrations 001–004 and adds only these compatibility-safe concepts:

1. `users.credential_kind`, `users.updated_at`, and `users.password_changed_at`;
2. nullable `prepared_by_user_id` and `verified_by_user_id` references on `preparation_tasks`—existing schema-4 tasks remain explicitly unattributed;
3. `safety_acknowledgements`, containing only order/task context, deterministic finding identity, rule/version, actor, timestamp, and whether the source was stale (accepted actions always record false);
4. `audit_events`, containing actor, event/entity identifiers, and minimal structured metadata;
5. SQLite triggers that reject normal `UPDATE` and `DELETE` operations on `audit_events`.

The older compatibility `audit_log` table remains untouched and is not treated as the Milestone 10 event stream.

Because SQLite does not permit adding a non-constant timestamp default to the populated AppData `users` table, migration 005 adds `updated_at` as a compatibility-nullable column, initializes existing rows from `created_at`, and has every modern credential insert/update set it explicitly. A schema-4 upgrade fixture includes a populated disabled legacy identity so this production-shaped migration path is covered.

## Deterministic safety fingerprint

Each evaluated `SafetyFinding` receives a SHA-256 fingerprint over a canonical length-prefixed sequence of:

- fingerprint format version;
- finding ID;
- rule ID;
- ruleset version;
- severity and status;
- optional order-item ID;
- every structured evidence label/value in its established order.

The canonical input excludes patient names, HN, user-facing title/message text, notes, addresses, and medication payloads. Supported threshold findings already expose observed/configured inputs as structured evidence, so a changed ordered dose, preparation/diluent volume, configured threshold, rule outcome, item, or ruleset produces a different fingerprint. A previous acknowledgement therefore does not satisfy the changed finding.

Acknowledgement is performed by a typed backend command that accepts only the local order and current finding ID. Rust re-evaluates safety, locates that current finding, derives its context and fingerprint, verifies preparation snapshots are not stale, derives the actor from the Rust session, then inserts the acknowledgement and audit event in one immediate transaction. The frontend cannot submit rule/version/fingerprint/user values.

Item findings are associated with their preparation task. Order-level findings are associated with the local order. The workspace returns only acknowledgement IDs whose stored fingerprint still matches a current finding. Stale or changed findings therefore require new review without deleting the earlier append-only record.

## Preparation and audit behavior

- Preparation initialization requires authentication and appends one `preparation_created` event per newly created task in the same transaction.
- Marking prepared atomically sets `prepared_by_user_id`, sets the timestamp/state, and appends `preparation_marked_prepared` with `source_stale=false`.
- Verification re-evaluates safety, requires matching persistent acknowledgements, atomically sets `verified_by_user_id`, and appends `preparation_verified` with `source_stale=false`.
- The same authenticated user may prepare and verify. Separation is supported by independent columns but is not imposed as a new clinical rule.
- Existing prepared/verified schema-4 records with NULL actors remain labelled unattributed/previous-workflow. The current user is never backfilled.
- Audit metadata uses internal IDs, state names, rule IDs, ruleset version, fingerprint, and stale-state booleans only. It does not contain passwords, hashes, patient identity, clinical notes, addresses, or full order/product data.
- No normal Tauri command can edit or delete audit events. SQLite administrators can still modify the database outside OncoFlow; the trail is append-only application evidence, not cryptographic non-repudiation.

## Implementation plan

1. Add migration 005, schema upgrade tests, actor/acknowledgement/audit constraints, and immutable-audit triggers.
2. Add a Rust `auth` domain with explicit Argon2id hashing, privacy-safe validation, bootstrap/login/logout/password-change services, and a process-local session.
3. Add deterministic safety fingerprints and repository helpers for current acknowledgement matching.
4. Require the Rust session for all preparation commands; make task initialization, prepare, acknowledgement, and verify changes atomic with audit events.
5. Add first-run/login/account/logout UI, discreet current-user identity, actor display, and persistent acknowledgement refresh behavior.
6. Add synthetic Rust/frontend tests and run the complete schema, integrity, non-mutation, formatting, strict Clippy, test, build, NSIS, startup, Git tracking, and immutable-MDB verification matrix.

## Immutable source baseline

| File | SHA-256 |
| --- | --- |
| `legacy/AllTable.mdb` | `C50849BE079F76E11A28BBF12D5648F41407E491D35A7CB761723FAF807288A3` |
| `legacy/Cytotoxic V8.0.mdb` | `2A6EC0AD301A99BEA7F4BC12D32BCC8B86890778352C503C545826F06058582D` |

## Completed UI behavior

- Startup asks Rust for authentication state before mounting any patient, drug, regimen, order, or preparation workspace.
- With no active Argon2id account, the branded offline first-run screen creates a new local administrator; it never suggests or creates a factory credential.
- Login/logout and password change use typed Tauri commands. The sidebar shows the current local identity and the Account screen describes the record accurately as a local workflow identity, not a digital signature.
- Preparation tasks display the authenticated preparer and verifier. Earlier attributed timestamps with NULL actor references are shown as an unknown prior actor instead of being assigned to the current session.
- The preparation safety panel loads current matching acknowledgements from SQLite, including actor and time. A changed fingerprint is intentionally absent from that current set and restores the review action. The standalone order safety view directs acknowledgement to the preparation workspace, where task context, staleness, and atomic audit behavior can be established.

## Final validation

The corrected release application was started against the actual AppData schema-4 database. It remained running through the startup check, applied migration 005, and was then stopped by the exact launched process ID. No bootstrap credential or clinical workflow record was created during validation.

| Check | Final result |
| --- | --- |
| AppData schema version | 5 |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | 0 violations |
| Historical orders / items | 1 / 2 (unchanged) |
| OncoFlow-created orders / items | 1 / 1 (unchanged) |
| Inventory events | 1 (unchanged) |
| Preparation tasks | 0 (unchanged) |
| Users / active / Argon2id | 1 / 0 / 0; first-run remains required |
| Disabled legacy identities | 1 (unchanged and unusable) |
| Safety acknowledgements / audit events | 0 / 0; no synthetic data written to AppData |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| Rust tests | 98 passed |
| Frontend tests | 32 passed in 10 files |
| Frontend typecheck / lint / production build | pass / pass / pass |
| Tauri release build | pass, NSIS only |
| Release startup | pass |
| Tracked DB/MDB files | 0 |
| Legacy MDB hashes | unchanged from baseline |

NSIS artifact: `src-tauri/target/release/bundle/nsis/OncoFlow_0.1.0_x64-setup.exe` (SHA-256 `C8A601A8C623339FA06CC57FF65AC753408E206AA3CEC48F534EBF4FF8291129`). The project bundle default is NSIS, and MSI packaging was intentionally not run.

## Remaining limitations

- The audit trail is append-only through normal application/database commands, but a person with direct SQLite file access can still tamper with it. It is not a cryptographic ledger or non-repudiation mechanism.
- This milestone intentionally has no user-management console or password recovery. An inactive-only database returns to first-run and permits establishment of a new active local administrator without impersonating an inactive identity.
- Marker-based preparation eligibility and display-only legacy preparation quantity behavior remain unchanged technical debt from Milestone 9.
- No existing AppData preparation tasks were available for live actor backfill validation; preservation of unattributed schema-4 tasks and all actor/acknowledgement transitions are covered with synthetic transactional fixtures.
