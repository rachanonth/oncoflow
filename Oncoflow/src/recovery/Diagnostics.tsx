import { useCallback, useEffect, useState } from "react";

import { commandError, getDiagnostics, listSystemPrinters, openDataFolder } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import { loadLabelPrinterConfig } from "../hardware/printerSettings";
import { displayDateTime } from "../shared/dateTime";
import type { Diagnostics as DiagnosticsModel } from "../types/recovery";

type DiagnosticsState = { kind: "loading" } | { kind: "error"; message: string } | { kind: "ready"; diagnostics: DiagnosticsModel; printers: string[]; printerError: string | null };

export function Diagnostics() {
  const [state, setState] = useState<DiagnosticsState>({ kind: "loading" });
  const load = useCallback(async () => {
    setState({ kind: "loading" });
    try {
      const diagnostics = await getDiagnostics();
      try { setState({ kind: "ready", diagnostics, printers: await listSystemPrinters(), printerError: null }); }
      catch { setState({ kind: "ready", diagnostics, printers: [], printerError: "Windows printer queues could not be inspected." }); }
    } catch (error) { setState({ kind: "error", message: commandError(error).message ?? "Diagnostics are unavailable." }); }
  }, []);
  useEffect(() => { void load(); }, [load]);
  const printer = loadLabelPrinterConfig();
  const printerAvailable = Boolean(printer && state.kind === "ready" && state.printers.includes(printer.spoolerName));
  return <DiagnosticsView state={state} printerName={printer?.spoolerName ?? null} printerLanguage={printer?.language ?? null} printerAvailable={printerAvailable} onRetry={() => void load()} onOpenDataFolder={() => void openDataFolder()} />;
}

export function DiagnosticsView({ state, printerName, printerLanguage, printerAvailable, onRetry, onOpenDataFolder }: {
  state: DiagnosticsState;
  printerName: string | null;
  printerLanguage: string | null;
  printerAvailable: boolean;
  onRetry: () => void;
  onOpenDataFolder: () => void;
}) {
  return <section className="workspace diagnostics-workspace" aria-labelledby="diagnostics-heading"><div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="diagnostics-heading">Diagnostics</h1><PageDescription pageKey="diagnostics" /></div><button className="button button--secondary" type="button" onClick={onOpenDataFolder}>Open data folder</button></div>
    {state.kind === "loading" && <div className="state-panel" aria-busy="true">Checking local system health…</div>}
    {state.kind === "error" && <div className="state-panel state-panel--error" role="alert"><h2>Diagnostics unavailable</h2><p>{state.message}</p><button className="button button--secondary" type="button" onClick={onRetry}>Try again</button></div>}
    {state.kind === "ready" && <div className="diagnostics-grid">
      <section className="surface diagnostics-card"><p className="eyebrow">Versions</p><h2>OncoFlow runtime</h2><dl><DiagnosticRow label="Application" value={`${state.diagnostics.applicationName} ${state.diagnostics.applicationVersion}`}/><DiagnosticRow label="Database schema" value={`${state.diagnostics.schemaVersion}`}/><DiagnosticRow label="Clinical ruleset" value={state.diagnostics.clinicalRulesetVersion}/><DiagnosticRow label="Label layout" value={state.diagnostics.labelTemplateVersion}/><DiagnosticRow label="Label renderer" value={state.diagnostics.labelRendererVersion}/></dl></section>
      <section className="surface diagnostics-card"><p className="eyebrow">Database</p><h2>Local data health</h2><dl><DiagnosticRow label="Location" value={state.diagnostics.databaseLocation}/><DiagnosticRow label="Size" value={formatBytes(state.diagnostics.databaseSizeBytes)}/><DiagnosticRow label="Integrity" value={state.diagnostics.integrityCheck}/><DiagnosticRow label="Foreign keys" value={`${state.diagnostics.foreignKeyViolations} violations`}/><DiagnosticRow label="Last manual backup" value={displayDateTime(state.diagnostics.lastBackupAt, "No backup audit in this database")}/></dl><p>{state.diagnostics.automaticBackupPolicy}</p></section>
      <section className="surface diagnostics-card"><p className="eyebrow">Hardware</p><h2>Label printer queue</h2><dl><DiagnosticRow label="Configured queue" value={printerName ?? "Not configured"}/><DiagnosticRow label="Queue status" value={printerName ? (printerAvailable ? "Available in Windows" : "Unavailable / disconnected") : "Not configured"}/><DiagnosticRow label="Printer language" value={printerLanguage?.toUpperCase() ?? "Not configured"}/><DiagnosticRow label="Installed queues" value={`${state.printers.length}`}/></dl>{state.printerError && <div className="hardware-warning">{state.printerError}</div>}<p>Queue availability is not proof of paper output. Use the operator-controlled test label in Hardware.</p></section>
    </div>}
    <p className="privacy-note">Diagnostics never display passwords, password hashes, patient names, orders, or clinical payloads.</p>
  </section>;
}

function DiagnosticRow({ label, value }: { label: string; value: string }) { return <div><dt>{label}</dt><dd>{value}</dd></div>; }
function formatBytes(value: number): string { return value < 1024 * 1024 ? `${Math.round(value / 1024)} KB` : `${(value / 1024 / 1024).toFixed(1)} MB`; }
