import { editFormKeyboard } from "../components/editFormKeyboard";
import { useEffect, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { AuthFrame } from "../auth/AuthScreens";
import { UsernameHistoryScope, usernameHistoryKey } from "../auth/usernameHistory";
import { getConnectionSettings, saveConnectionSettings, setClientMode, type ConnectionSettings } from "../api/transport";
import { getServerStatus, type ServerStatus } from "../api/transport";
import { ServerControls } from "./ServerControls";

function message(error: unknown) {
  return (error as { message?: string } | null)?.message ?? String(error);
}

export function ConnectionGate({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<ConnectionSettings | null>(null);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [attention, setAttention] = useState<string | null>(null);
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [showServer, setShowServer] = useState(false);
  const [requestExit, setRequestExit] = useState(false);
  useEffect(() => {
    getConnectionSettings().then(value => { setClientMode(value.mode !== "standalone"); setSettings(value); }).catch(e => setError(message(e)));
    const listener = (event: Event) => setAttention(message((event as CustomEvent).detail));
    window.addEventListener("oncoflow-connection-attention", listener);
    return () => window.removeEventListener("oncoflow-connection-attention", listener);
  }, []);
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function refresh() {
      try { const next = await getServerStatus(); if (!disposed) setStatus(next); }
      catch (e) { if (!disposed) setStatus(previous => previous ? { ...previous, running: false, databaseReady: false, message: message(e) } : null); }
      if (!disposed) timer = setTimeout(() => void refresh(), 10000);
    }
    void refresh();
    const unlisten = listen("oncoflow-host-close-request", () => { setEditing(false); setRequestExit(true); setShowServer(true); });
    return () => { disposed = true; clearTimeout(timer); void unlisten.then(fn => fn()).catch(() => undefined); };
  }, []);
  if (error) return <AuthFrame title="Connection settings unavailable"><p role="alert">{error}</p><button onClick={() => window.location.reload()}>Try again</button></AuthFrame>;
  if (!settings) return <AuthFrame title="Opening OncoFlow"><p>Loading connection settings…</p></AuthFrame>;
  if (editing) return <ConnectionForm settings={settings} onCancel={() => setEditing(false)} />;
  return <>
    <div className="connection-bar">
      <span>{settings.mode === "host" ? `Hosting server on this PC · port ${settings.port}` : settings.mode === "client" ? `LAN server: ${settings.host}:${settings.port}` : "Standalone · database on this PC"}</span>
      {settings.mode !== "standalone" && <button className={`button button--secondary server-status-button ${status?.databaseReady ? "is-online" : "is-offline"}`} onClick={() => { setRequestExit(false); setShowServer(true); }}>{status ? status.databaseReady ? "● Server online" : status.running ? "● Database unavailable" : "● Server offline" : "Checking server…"}</button>}
      <button className="button button--secondary" onClick={() => setEditing(true)}>Connection settings</button>
    </div>
    <div inert={Boolean(attention) || showServer}><UsernameHistoryScope.Provider key={usernameHistoryKey(settings)} value={usernameHistoryKey(settings)}>{children}</UsernameHistoryScope.Provider></div>
    {showServer && <ServerControls status={status} requestExit={requestExit} onStatus={setStatus} onClose={() => { setShowServer(false); setRequestExit(false); }} />}
    {attention && <div className="connection-attention" role="alertdialog" aria-modal="true" aria-label="Server connection requires attention"><AuthFrame title="Reload the shared workspace" summary={attention}>
      <p>Reloading discards unsaved form changes. After signing in, check the current record before submitting again.</p>
      <button className="button button--primary" onClick={() => window.location.reload()}>Reload and sign in</button>
    </AuthFrame></div>}
  </>;
}

function ConnectionForm({ settings, onCancel }: { settings: ConnectionSettings; onCancel: () => void }) {
  const [mode, setMode] = useState(settings.mode);
  const [host, setHost] = useState(settings.host);
  const [port, setPort] = useState(String(settings.port));
  const [certificate, setCertificate] = useState<string | null>(null);
  const [dataDirectory, setDataDirectory] = useState(settings.dataDirectory);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  async function chooseCertificate() {
    try {
      const path = await open({ multiple: false, directory: false, filters: [{ name: "OncoFlow server public certificate", extensions: ["der"] }] });
      if (typeof path === "string") setCertificate(path);
    } catch (e) { setError(message(e)); }
  }
  async function save(event: React.FormEvent) {
    event.preventDefault(); setBusy(true); setError(null);
    try {
      if (mode !== "standalone" && ((mode === "client" && !host.trim()) || !/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535)) throw new Error("Enter a server IP address and a port from 1 to 65535.");
      await saveConnectionSettings(mode === "client" ? host : "", mode !== "standalone" ? Number(port) : 7443, certificate, mode, mode === "host" ? dataDirectory : null);
      setSaved(true);
    } catch (e) { setError(message(e)); } finally { setBusy(false); }
  }
  return <AuthFrame title="Desktop connection" summary="Choose where this installation stores and reads application data.">
    {saved ? <><p role="status">Connection saved. Close OncoFlow completely and reopen it to apply the change.</p><p>Your existing local database has not been moved or merged.</p></> :
      <form onKeyDownCapture={editFormKeyboard} className="auth-form" onSubmit={e => void save(e)}>
        {error && <p className="auth-error" role="alert">{error}</p>}
        <label className="auth-field">Connection mode<select value={mode} onChange={e => setMode(e.target.value as ConnectionSettings["mode"])}><option value="standalone">Standalone — this PC</option><option value="host">Host server on this PC</option><option value="client">LAN client — central server</option></select></label>
        {mode === "host" && <>
          <p>This PC stores the shared database and runs the server inside OncoFlow. You can work here while the other PCs connect.</p>
          <label className="auth-field">Server data folder<input value={dataDirectory} onChange={e => setDataDirectory(e.target.value)} required /></label>
          <button className="button button--secondary" type="button" onClick={() => void open({ directory: true, multiple: false }).then(path => { if (typeof path === "string") setDataDirectory(path); }).catch(e => setError(message(e)))}>Choose data folder</button>
          <label className="auth-field">Server port<input type="number" min="1" max="65535" value={port} onChange={e => setPort(e.target.value)} required /></label>
          <p>Use the folder containing your existing oncoflow.db, or choose an empty folder for a new database. Data is not moved or merged. A server certificate is created automatically on first start.</p>
        </>}
        {mode === "client" && <>
          <label className="auth-field">Server IP address<input placeholder="192.168.1.10" value={host} onChange={e => setHost(e.target.value)} required /></label>
          <label className="auth-field">Server port<input type="number" min="1" max="65535" value={port} onChange={e => setPort(e.target.value)} required /></label>
          <button className="button button--secondary" type="button" onClick={() => void chooseCertificate()}>Select server-certificate.der</button>
          <small>{certificate ?? (settings.certificateFingerprint ? "Using the saved server certificate" : "Copy this public certificate from the server PC.")}</small>
          {settings.certificateFingerprint && <small className="certificate-fingerprint">Saved certificate SHA-256: {settings.certificateFingerprint}</small>}
          <p>The server must be running. Saving tests its encrypted connection.</p>
        </>}
        <button className="button button--primary" disabled={busy}>{busy ? "Testing and saving…" : "Save connection"}</button>
        <button className="button button--secondary" type="button" disabled={busy} onClick={onCancel}>Cancel</button>
      </form>}
  </AuthFrame>;
}
