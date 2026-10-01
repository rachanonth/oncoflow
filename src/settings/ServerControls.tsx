import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { exportServerCertificate, getServerStatus, startHostServer, stopHostServer, type ServerStatus } from "../api/transport";

function errorMessage(error: unknown) { return (error as { message?: string } | null)?.message ?? String(error); }

export function ServerControls({ status, requestExit, onClose, onStatus }: { status: ServerStatus | null; requestExit: boolean; onClose: () => void; onStatus: (status: ServerStatus) => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirmStop, setConfirmStop] = useState(requestExit);
  useEffect(() => { if (requestExit) setConfirmStop(true); }, [requestExit]);
  async function start() {
    setBusy(true); setError(null);
    try { await startHostServer(); onStatus(await getServerStatus()); setNotice("Server started. Reload the workspace to sign in."); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  async function stop() {
    setBusy(true); setError(null);
    try { await stopHostServer(requestExit); setConfirmStop(false); onStatus(await getServerStatus()); setNotice("Server stopped. Every user will need to sign in again after it starts."); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  async function exportCertificate() {
    setError(null);
    try {
      const destination = await save({ defaultPath: "server-certificate.der", filters: [{ name: "Public server certificate", extensions: ["der"] }] });
      if (destination) { await exportServerCertificate(destination); setNotice("Public certificate saved. Copy it to the other PCs and select it in their connection settings."); }
    } catch (e) { setError(errorMessage(e)); }
  }
  return <div className="server-controls-overlay" role="dialog" aria-modal="true" aria-labelledby="server-controls-title">
    <section className="server-controls-card">
      <div className="page-heading"><h1 id="server-controls-title">Server status</h1><button className="button button--secondary" disabled={busy} onClick={onClose}>Close</button></div>
      <ServerStatusDetails status={status} />
      {error && <p className="auth-error" role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      {status?.mode === "host" && <>
        <p>Keep OncoFlow open on this PC while other people work. The server starts automatically when you open OncoFlow in host mode.</p>
        {confirmStop ? <div className="server-stop-confirm" role="alert">
          <h2>{requestExit ? "Stop the server and close OncoFlow?" : "Stop the shared server?"}</h2>
          <p>This disconnects all users, including this PC. Ask everyone to save their work first. In-progress requests finish before the database is released.</p>
          <button className="button button--primary" disabled={busy} onClick={() => void stop()}>{busy ? "Stopping…" : requestExit ? "Stop server and exit" : "Stop server"}</button>
          <button className="button button--secondary" disabled={busy} onClick={() => { setConfirmStop(false); if (requestExit) onClose(); }}>Keep running</button>
        </div> : <div className="server-control-actions">
          {status.running ? <button className="button button--secondary" disabled={busy} onClick={() => setConfirmStop(true)}>Stop server…</button> : <button className="button button--primary" disabled={busy} onClick={() => void start()}>{busy ? "Starting…" : "Start server"}</button>}
          <button className="button button--secondary" disabled={busy} onClick={() => void exportCertificate()}>Export certificate for clients</button>
          {status.running && <button className="button button--secondary" disabled={busy} onClick={() => window.location.reload()}>Reload workspace / sign in</button>}
        </div>}
        <p className="privacy-note">Other PCs also need access through Windows Firewall on your private network. “Online” confirms this PC's server and database; check that another PC can connect.</p>
      </>}
    </section>
  </div>;
}

export function ServerStatusDetails({ status }: { status: ServerStatus | null }) {
  if (!status) return <p role="status">Checking server status…</p>;
  return <>
    <p className={status.databaseReady ? "server-health is-online" : "server-health is-offline"} role="status">{status.message}</p>
    {status.mode === "host" && <dl className="server-status-details">
      <dt>Clients connect to</dt><dd>{status.addresses.length ? status.addresses.map(ip => `${ip}:${status.port}`).join(" · ") : `This PC's LAN IPv4 address, port ${status.port}`}</dd>
      <dt>Signed-in connections</dt><dd>{status.signedInConnections} (includes this PC when signed in)</dd>
      <dt>Database folder</dt><dd>{status.dataDirectory}</dd>
      <dt>Running time</dt><dd>{status.running && status.uptimeSeconds !== null ? `${Math.floor(status.uptimeSeconds / 60)} minutes` : "Stopped"}</dd>
    </dl>}
  </>;
}
