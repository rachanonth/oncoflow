import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import {
  commandError,
  createDatabaseBackup,
  openDataFolder,
  preflightDatabaseRestore,
  restoreDatabase,
} from "../api/commands";
import type { BackupResult, RestorePreflight, RestoreResult } from "../types/recovery";
import { PageDescription } from "../guidance/PageGuidance";
import { displayDateTime } from "../shared/dateTime";

type PanelState = {
  busy: "backup" | "preflight" | "restore" | null;
  error: string | null;
  backup: BackupResult | null;
  preflight: RestorePreflight | null;
  restored: RestoreResult | null;
  confirmed: boolean;
};

const EMPTY_STATE: PanelState = {
  busy: null,
  error: null,
  backup: null,
  preflight: null,
  restored: null,
  confirmed: false,
};

export function BackupRestore({ recoveryMode = false }: { recoveryMode?: boolean }) {
  const [state, setState] = useState<PanelState>(EMPTY_STATE);

  async function chooseBackupFolder() {
    setState((current) => ({ ...current, busy: "backup", error: null, backup: null }));
    try {
      const directory = await open({ directory: true, multiple: false, title: "Choose OncoFlow backup folder" });
      if (!directory) {
        setState((current) => ({ ...current, busy: null }));
        return;
      }
      const backup = await createDatabaseBackup(directory);
      setState((current) => ({ ...current, busy: null, backup }));
    } catch (error) {
      setState((current) => ({ ...current, busy: null, error: commandError(error).message ?? "Backup could not be created." }));
    }
  }

  async function chooseRestoreFile() {
    setState((current) => ({ ...current, busy: "preflight", error: null, preflight: null, restored: null, confirmed: false }));
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: "Select an OncoFlow database backup",
        filters: [{ name: "OncoFlow SQLite backup", extensions: ["db", "sqlite", "sqlite3"] }],
      });
      if (!selected) {
        setState((current) => ({ ...current, busy: null }));
        return;
      }
      const preflight = await preflightDatabaseRestore(selected);
      setState((current) => ({ ...current, busy: null, preflight }));
    } catch (error) {
      setState((current) => ({ ...current, busy: null, error: commandError(error).message ?? "The selected backup was rejected." }));
    }
  }

  async function restore() {
    if (!state.preflight || !state.confirmed) return;
    setState((current) => ({ ...current, busy: "restore", error: null, restored: null }));
    try {
      const restored = await restoreDatabase({
        backupPath: state.preflight.location,
        expectedSha256: state.preflight.sha256,
        confirmationToken: state.preflight.confirmationToken,
        confirmed: true,
      });
      setState((current) => ({ ...current, busy: null, restored }));
    } catch (error) {
      setState((current) => ({ ...current, busy: null, error: commandError(error).message ?? "Restore did not complete." }));
    }
  }

  async function revealDataFolder() {
    try { await openDataFolder(); }
    catch (error) { setState((current) => ({ ...current, error: commandError(error).message ?? "The data folder could not be opened." })); }
  }

  const view = <BackupRestoreView
    recoveryMode={recoveryMode}
    state={state}
    onBackup={() => void chooseBackupFolder()}
    onSelectRestore={() => void chooseRestoreFile()}
    onConfirmed={(confirmed) => setState((current) => ({ ...current, confirmed }))}
    onRestore={() => void restore()}
    onOpenDataFolder={() => void revealDataFolder()}
    onRestart={() => window.location.reload()}
  />;
  return recoveryMode ? view : <section className="workspace recovery-workspace" aria-labelledby="backup-heading"><div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="backup-heading">Backup &amp; restore</h1><PageDescription pageKey="backup_restore" /></div></div>{view}</section>;
}

export function BackupRestoreView({ recoveryMode, state, onBackup, onSelectRestore, onConfirmed, onRestore, onOpenDataFolder, onRestart }: {
  recoveryMode: boolean;
  state: PanelState;
  onBackup: () => void;
  onSelectRestore: () => void;
  onConfirmed: (confirmed: boolean) => void;
  onRestore: () => void;
  onOpenDataFolder: () => void;
  onRestart: () => void;
}) {
  return <div className="recovery-grid">
    {!recoveryMode && <section className="surface recovery-card" aria-labelledby="manual-backup-heading">
      <p className="eyebrow">Protect current data</p><h2 id="manual-backup-heading">Create validated backup</h2>
      <p>OncoFlow uses SQLite&apos;s online backup mechanism, then verifies database integrity, foreign keys, schema version, and SHA-256 checksum.</p>
      <button className="button button--primary" type="button" disabled={state.busy !== null} onClick={onBackup}>{state.busy === "backup" ? "Selecting or backing up…" : "Choose destination and back up"}</button>
      {state.backup && <div className="recovery-result" role="status"><strong>Backup successful</strong><dl><Row label="Location" value={state.backup.location}/><Row label="Timestamp" value={displayDateTime(state.backup.createdAt)}/><Row label="Schema" value={`${state.backup.schemaVersion}`}/><Row label="Integrity" value={`${state.backup.integrityCheck} · ${state.backup.foreignKeyViolations} FK violations`}/><Row label="SHA-256" value={state.backup.sha256}/></dl></div>}
    </section>}

    <section className="surface recovery-card" aria-labelledby="restore-heading">
      <p className="eyebrow">Whole-database recovery</p><h2 id="restore-heading">Restore a backup</h2>
      <p>The selected file is checked before confirmation. OncoFlow then creates and validates a recovery copy of the current database before changing it.</p>
      <div className="recovery-actions"><button className="button button--secondary" type="button" disabled={state.busy !== null || Boolean(state.restored)} onClick={onSelectRestore}>{state.busy === "preflight" ? "Selecting or validating…" : "Select backup"}</button><button className="button button--ghost" type="button" disabled={state.busy !== null} onClick={onOpenDataFolder}>Open data folder</button></div>
      {state.preflight && !state.restored && <div className="restore-preflight"><strong>Backup validated</strong><dl><Row label="File" value={state.preflight.fileName}/><Row label="Schema" value={`${state.preflight.schemaVersion}${state.preflight.requiresMigration ? ` → ${state.preflight.supportedSchemaVersion} after restore` : ""}`}/><Row label="Created" value={displayDateTime(state.preflight.createdAt, "Manifest timestamp unavailable")}/><Row label="Integrity" value={`${state.preflight.integrityCheck} · ${state.preflight.foreignKeyViolations} FK violations`}/><Row label="SHA-256" value={state.preflight.sha256}/></dl><label className="restore-confirm"><input type="checkbox" checked={state.confirmed} onChange={(event) => onConfirmed(event.target.checked)}/><span>I understand that this replaces the complete local database, including users and passwords, after a recovery backup is created.</span></label><button className="button button--danger" type="button" disabled={!state.confirmed || state.busy !== null} onClick={onRestore}>{state.busy === "restore" ? "Protecting current DB and restoring…" : "Confirm whole-database restore"}</button></div>}
      {state.restored && <div className="recovery-result recovery-result--restore" role="status"><strong>Restore completed safely</strong><p>Schema {state.restored.restoredSchemaVersion} is ready. The prior authenticated session was cleared because identities from the restored database are now authoritative.</p><dl><Row label="Pre-restore recovery copy" value={state.restored.recoveryBackupLocation}/><Row label="Recovery SHA-256" value={state.restored.recoveryBackupSha256}/></dl><button className="button button--primary" type="button" onClick={onRestart}>Restart OncoFlow workspace</button></div>}
      {state.error && <div className="auth-error" role="alert">{state.error}</div>}
      <footer>Restore never merges individual tables and never creates default credentials.</footer>
    </section>
  </div>;
}

function Row({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}
