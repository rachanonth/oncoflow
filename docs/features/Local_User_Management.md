# Local User Management

Status: implemented after RC1 validation request (2026-08-25). This is a focused local-account addition, not Milestone 16 and not a clinical-rule change.

## Model

OncoFlow now separates two concepts:

| Concept | Values | Purpose |
| --- | --- | --- |
| Administrative role | existing `admin` / normal local user | Controls access to Settings → Users |
| User type | `pharmacist` / `non_pharmacist` | Records local identity classification |

User type is deliberately metadata only. It does not represent clinical competency, approve treatment, or automatically change access to order/preparation workflows. Any future permission distinction requires an explicit reviewed product rule.

## Migration 009

Migration 009 adds `users.user_type` with a database `CHECK` constraint limiting it to `pharmacist` or `non_pharmacist`, plus an index for the management list. Schema version becomes 9.

Compatibility behavior:

- existing usable Argon2id OncoFlow accounts become `pharmacist`, preserving their established pharmacist-oriented workflow provenance;
- disabled legacy identities remain `non_pharmacist` and unusable;
- no legacy plaintext credential is activated or migrated;
- no patient, order, preparation, inventory, safety, output, or audit record is rewritten.

## Security and workflow

Only an authenticated local administrator may list, create, or update managed accounts. The frontend never supplies the acting administrator ID; Rust derives it from the process-local authenticated session.

New users receive:

- an administrator-entered unique username;
- a display name;
- exactly one supported user type;
- a new password hashed with the existing Argon2id configuration and random salt;
- normal (non-administrator) access; and
- active state by default.

Administrators may edit username, display name, user type, access level, and active state for other modern local accounts. Access level is deliberately separate from user type: a pharmacist or non-pharmacist identity may have either Standard or Administrator application access. The signed-in administrator cannot deactivate or demote their own account, ensuring the active administrative session cannot remove its own management access. Current-account password changes remain under Settings → Account. Passwords and hashes are never returned by list/create/update commands.

Successful account creation and updates append `user_created` or `user_updated` audit events in the same SQLite transaction. Update metadata contains only target ID-independent state (`user_type`, database access role, and `active`), not username, display name, or credentials.

## UI

Settings → Users is visible only to local administrators. It provides:

- a local-user list;
- pharmacist/non-pharmacist labels;
- active/inactive state;
- account creation with password confirmation;
- editing and activation/deactivation for other accounts;
- promotion from Standard to Administrator, or demotion back to Standard, for other accounts; and
- clear wording that user type does not establish clinical competency.

Everything remains offline and stored in `oncoflow.db`. No network identity provider, hospital directory, email reset, default password, or external account service is introduced.

## Tests

Synthetic coverage verifies migration provenance, both user types, Argon2id storage without hash exposure, administrator-only access, Thai display names, activation changes, promotion/demotion, current-admin self-demotion and deactivation protection, transactional audit rollback, UI rendering, and create/edit validation.

## Validation

Validation completed on 2026-08-25:

- schema-8 → schema-9 migration test: passed with `integrity_check=ok` and zero foreign-key violations;
- `cargo fmt --all -- --check`: passed;
- strict Clippy: passed;
- Rust tests: 170 passed, 0 failed, 0 ignored;
- frontend tests: 71 passed across 20 files;
- frontend typecheck, lint, and production build: passed;
- Tauri release and NSIS-only package build: passed;
- tracked DB/MDB files: zero; and
- both legacy MDB hashes: unchanged.

Release artifact:

```text
OncoFlow_0.1.2_x64-setup.exe
Size: 3,349,704 bytes
SHA-256: 37250CAB607DBFFFF35F992370B0BDAB104FB369E6A2526BDA8522ADC280DB84
```

The previously installed OncoFlow process was left running and was not interrupted. Its AppData database is intentionally not migrated out-of-band; migration 009 applies through normal initialization when the updated application is intentionally installed/launched.

## Administrator password reset (2026-09-29)

Settings → Users → Reset password lets an authenticated administrator set and confirm a new 12–128 character password for another modern OncoFlow account without the old password. Current-account changes remain under Account. Inactive accounts stay inactive. Legacy disabled identities are excluded.

This is a new OncoFlow account-management rule, not a replacement of an Access clinical rule. The prior identity migration documentation records that legacy TblUser passwords were discarded; legacy MDBs are not modified or used for recovery.

Rust checks the acting administrator again in the write transaction, validates and hashes the replacement with Argon2id and a random salt, and appends password_reset_by_admin atomically. Audit contains actor and target IDs and credential kind only. A failed audit insert rolls back the credential change.

Sessions retain a private SHA-256 fingerprint of the credential hash captured at login. Authentication refresh rejects a changed credential, ending existing sessions on their next request, including LAN requests. Normal self-service password changes retain the current session and revoke other sessions. Reset is registered as a LAN write and uses existing stale-write protection. No schema change, email service, temporary/default password, or passwordless login is introduced.

## Remembered login usernames (2026-09-29)

The login screen remembers up to 20 successfully authenticated usernames in this installation's WebView local storage, newest first. It uses the canonical username returned by Rust, not failed login input. The latest username is prefilled; Saved usernames lets the user choose another entry or enter a different username. Switching usernames clears the password field. Clear saved usernames removes this connection's list without deleting accounts.

History is scoped by local database directory (shared by standalone and host mode for that directory) or by LAN server host, port, and pinned certificate fingerprint. Only username strings are persisted: no passwords, password hashes, display names, roles, or session tokens. Missing, corrupt, or unavailable storage never blocks login. This is a new UI convenience, with no Access parity rule or database migration; legacy MDBs remain untouched. History begins with successful logins after this feature is installed and does not enumerate server accounts.
