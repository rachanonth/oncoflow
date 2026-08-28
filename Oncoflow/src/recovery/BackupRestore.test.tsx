import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { BackupRestoreView } from "./BackupRestore";

const handlers = {
  onBackup: () => undefined,
  onSelectRestore: () => undefined,
  onConfirmed: () => undefined,
  onRestore: () => undefined,
  onOpenDataFolder: () => undefined,
  onRestart: () => undefined,
};

const empty = { busy: null, error: null, backup: null, preflight: null, restored: null, confirmed: false } as const;

describe("BackupRestore", () => {
  it("shows manual whole-database backup and conservative restore actions", () => {
    const html = renderToStaticMarkup(<BackupRestoreView recoveryMode={false} state={empty} {...handlers}/>);
    expect(html).toContain("Create validated backup");
    expect(html).toContain("SQLite&#x27;s online backup mechanism");
    expect(html).toContain("Select backup");
    expect(html).toContain("Open data folder");
    expect(html).toContain("never creates default credentials");
  });

  it("shows validation/checksum and requires explicit confirmation", () => {
    const state = {
      ...empty,
      preflight: {
        location: "D:\\safe\\OncoFlow_Backup.db",
        fileName: "OncoFlow_Backup.db",
        schemaVersion: 7,
        supportedSchemaVersion: 10,
        requiresMigration: true,
        createdAt: "2026-08-23T15:37:00Z",
        backupApplicationVersion: "0.1.0",
        integrityCheck: "ok",
        foreignKeyViolations: 0,
        sha256: "abc123",
        sizeBytes: 4096,
        confirmationToken: "token",
      },
    };
    const html = renderToStaticMarkup(<BackupRestoreView recoveryMode={false} state={state} {...handlers}/>);
    expect(html).toContain("Backup validated");
    expect(html).toContain("7 → 10 after restore");
    expect(html).toContain("abc123");
    expect(html).toContain("including users and passwords");
    expect(html).toContain("disabled=\"\"");
  });

  it("shows successful backup metadata without clinical contents", () => {
    const state = {
      ...empty,
      backup: {
        location: "E:\\Backups\\OncoFlow_Backup_2026-08-23_153700.db",
        manifestLocation: "E:\\Backups\\OncoFlow_Backup_2026-08-23_153700.db.manifest.json",
        fileName: "OncoFlow_Backup_2026-08-23_153700.db",
        createdAt: "2026-08-23T08:37:00Z",
        schemaVersion: 10,
        applicationVersion: "0.1.0",
        integrityCheck: "ok",
        foreignKeyViolations: 0,
        sha256: "synthetic-checksum",
        sizeBytes: 8192,
      },
    };
    const html = renderToStaticMarkup(<BackupRestoreView recoveryMode={false} state={state} {...handlers}/>);
    expect(html).toContain("Backup successful");
    expect(html).toContain("Schema");
    expect(html).toContain("0 FK violations");
    expect(html).toContain("synthetic-checksum");
  });

  it("shows recovery mode without offering backup of an unavailable database", () => {
    const html = renderToStaticMarkup(<BackupRestoreView recoveryMode state={empty} {...handlers}/>);
    expect(html).toContain("Restore a backup");
    expect(html).not.toContain("Create validated backup");
  });

  it("locks competing actions while a native picker or validation is active", () => {
    const backupBusy = renderToStaticMarkup(<BackupRestoreView recoveryMode={false} state={{ ...empty, busy: "backup" }} {...handlers}/>);
    expect(backupBusy).toContain("Selecting or backing up…");
    expect((backupBusy.match(/disabled=""/g) ?? []).length).toBeGreaterThanOrEqual(3);

    const restoreBusy = renderToStaticMarkup(<BackupRestoreView recoveryMode={false} state={{ ...empty, busy: "preflight" }} {...handlers}/>);
    expect(restoreBusy).toContain("Selecting or validating…");
    expect((restoreBusy.match(/disabled=""/g) ?? []).length).toBeGreaterThanOrEqual(3);
  });
});
