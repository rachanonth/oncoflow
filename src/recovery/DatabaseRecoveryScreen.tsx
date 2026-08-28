import { useState } from "react";

import { commandError, openDataFolder, retryDatabaseInitialization } from "../api/commands";
import { AuthFrame } from "../auth/AuthScreens";
import type { StartupStatus } from "../types/recovery";
import { BackupRestore } from "./BackupRestore";

export function DatabaseRecoveryScreen({ status, onReady }: { status: StartupStatus; onReady: (status: StartupStatus) => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function retry() {
    setBusy(true); setError(null);
    try {
      const next = await retryDatabaseInitialization();
      if (next.databaseReady) onReady(next);
      else setError(next.issue?.message ?? "The database is still unavailable.");
    } catch (cause) { setError(commandError(cause).message ?? "The database retry failed safely."); }
    finally { setBusy(false); }
  }
  async function reveal() {
    try { await openDataFolder(); }
    catch (cause) { setError(commandError(cause).message ?? "The data folder could not be opened."); }
  }
  const issue = status.issue;
  return <AuthFrame eyebrow="Local database recovery" title={issue?.title ?? "Database problem detected"} summary={issue?.message ?? "OncoFlow paused before opening clinical data."}>
    {error && <div className="auth-error" role="alert">{error}</div>}
    <div className="recovery-screen-actions"><button className="button button--secondary" type="button" disabled={busy} onClick={() => void retry()}>{busy ? "Retrying…" : "Retry database"}</button><button className="button button--ghost" type="button" onClick={() => void reveal()}>Open data folder</button></div>
    <p className="auth-privacy">The existing database was not replaced with an empty file. Restore requires a validated current-database recovery copy before replacement.</p>
    <BackupRestore recoveryMode />
  </AuthFrame>;
}
