# Thai Page Descriptions and Guidance

Status: implemented as a focused local UI-configuration feature (2026-08-25). It changes explanatory interface copy only and adds no clinical behavior.

## User experience

Every existing OncoFlow page-summary description is now sourced from one fixed Thai catalog. The standard description cannot be edited in the application, so OncoFlow terminology and safety boundaries remain consistent.

Descriptions are intentionally concise and describe only the task performed on each page. Repetitive implementation wording such as “stored in the local OncoFlow database”, “ภายในเครื่อง”, and equivalent phrases is omitted because the local-only architecture is already established elsewhere in the interface.

An optional second line is displayed only when configured:

```text
คำอธิบายมาตรฐานภาษาไทย
Guidance  [optional workstation-specific text]
```

The editable line is labeled exactly **Guidance**. It may contain Thai or English Unicode text and preserves line breaks. It is visually distinct from the standard description and is never interpreted as a clinical rule.

Local administrators manage it from:

```text
Settings
└── Guidance
```

The editor provides a page selector, the read-only Thai standard description, a 500-character Guidance field, Save, and Reset. Reset removes only the optional Guidance and restores the page to its fixed Thai description. Standard users can view configured Guidance but cannot edit it.

Administrators are warned not to enter patient-identifying information or use Guidance to define clinical rules.

## Architecture and persistence

Migration `010_page_guidance.sql` advances the database to schema version 10 and creates only:

```text
page_guidance(
  page_key,
  guidance,
  updated_by_user_id,
  updated_at
)
```

No existing clinical table is altered. The migration is transactional and the normal pre-migration recovery snapshot continues to protect the active database.

Rust exposes typed `list_page_guidance` and `update_page_guidance` commands. Reads require an authenticated OncoFlow session; updates and resets require an administrator. Supported page keys are allow-listed in Rust, and the frontend cannot supply an acting user ID.

Updates are committed atomically with an append-only `page_guidance_updated` or `page_guidance_reset` audit event. Audit metadata contains the page key only and deliberately omits the user-entered Guidance text.

## Clinical and privacy boundary

Guidance does not change calculations, safety findings, orders, preparation, verification, inventory, labels, authentication, or authorization. It is never printed on preparation labels and is not included in clinical records. Existing MDB files are not read or modified.

## Tests

Synthetic coverage verifies:

- schema 9 to 10 migration with existing data preserved;
- Thai Unicode storage, trimming, listing, and reset;
- supported-page and 500-character validation;
- authenticated reads and administrator-only writes;
- Guidance plus audit atomic rollback;
- omission of Guidance content from audit metadata;
- fixed Thai page descriptions; and
- separate rendering of the English `Guidance` label.

## Release validation

Validation completed on 2026-08-25:

- `cargo fmt --all -- --check`: passed;
- strict `cargo clippy --all-targets --all-features -- -D warnings`: passed;
- Rust tests: 177 passed;
- frontend tests: 86 passed across 24 files;
- frontend typecheck, lint, and production build: passed;
- Tauri release build and NSIS-only packaging: passed;
- tracked database/MDB files: zero; and
- both legacy MDB SHA-256 hashes: unchanged.

Release artifact: `OncoFlow_0.1.2_x64-setup.exe` (3,372,480 bytes; SHA-256 `78C077AE0586EEC5BBDEB725C45179E6185EC9B35E1F5B270C35C53072A4C93F`).

The existing installed/running AppData database was not modified during development. Migration 010 applies transactionally, with the established pre-migration recovery snapshot, on the first launch of this build.
