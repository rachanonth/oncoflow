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
