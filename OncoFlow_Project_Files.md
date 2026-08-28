# OncoFlow Codex Project Files

## Recommended repository layout

```text
oncoflow/
├── AGENTS.md
├── .gitignore
├── README.md
├── docs/
│   └── legacy/
│       ├── OncoFlow_Migration_Blueprint.md
│       └── access_object_inventory.csv
├── legacy/
│   ├── Cytotoxic V8.0.mdb
│   └── AllTable.mdb
├── migrations/
│   └── 001_initial.sql
├── migration/
│   ├── raw/
│   ├── transform/
│   └── reports/
├── src/
└── src-tauri/
```

## Naming rules

- Product/repository: `OncoFlow` / `oncoflow`
- Access backend migration source: `AllTable.mdb`
- Runtime SQLite database: `oncoflow.db`
- Do not use `All Table(1).mdb` in new code or scripts.
- `C:\Ctx\Tbl\All Table.mdb` is a legacy embedded link and may remain in reverse-engineering documentation.
- `PWD=table` is legacy metadata only. `AllTable.mdb` is now an unlocked working copy.

## First Codex task

Read `AGENTS.md` and `docs/legacy/OncoFlow_Migration_Blueprint.md` first. Implement only Milestone 1: Tauri boot, Rust-owned SQLite initialization/migrations, a backend/database status command, a minimal React status shell, and database migration tests. Do not implement clinical calculations or modify the legacy MDB files.
