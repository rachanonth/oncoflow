import { useCallback, useEffect, useState } from "react";

import { commandError, listSystemPrinters, printTestLabel } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type { LabelPrinterConfig } from "../types/hardware";
import { DEFAULT_LABEL_PRINTER, loadLabelPrinterConfig, saveLabelPrinterConfig } from "./printerSettings";

export function HardwareSettings() {
  const saved = loadLabelPrinterConfig();
  const [config, setConfig] = useState<LabelPrinterConfig>(saved ?? { spoolerName: "", ...DEFAULT_LABEL_PRINTER });
  const [printers, setPrinters] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(saved ? "Saved locally on this workstation." : null);

  const refresh = useCallback(async () => {
    setLoading(true); setError(null);
    try { setPrinters(await listSystemPrinters()); }
    catch (requestError) {
      setPrinters([]);
      setError(commandError(requestError).message ?? "Unable to list Windows printers.");
    }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  function save() {
    const validation = validatePrinterConfig(config);
    if (validation) { setError(validation); setMessage(null); return; }
    try {
      saveLabelPrinterConfig(config);
      setError(null);
      setMessage("Printer queue, label layout, and font sizes saved locally. Run a test print to confirm the device.");
    } catch {
      setError("The local printer setting could not be saved.");
    }
  }

  async function testPrint() {
    const validation = validatePrinterConfig(config);
    if (validation) { setError(validation); return; }
    setBusy(true); setError(null); setMessage(null);
    try {
      const job = await printTestLabel(config);
      setMessage(`Windows accepted test job #${job.windowsJobId} (${job.bytesSubmitted} bytes). Confirm the physical label printed correctly.`);
    } catch (requestError) {
      setError(commandError(requestError).message ?? "The Windows spooler rejected the test label.");
    } finally { setBusy(false); }
  }

  return <HardwareSettingsView config={config} printers={printers} loading={loading} busy={busy} error={error} message={message} onConfig={setConfig} onRefresh={() => void refresh()} onSave={save} onTest={() => void testPrint()} />;
}

export function HardwareSettingsView({ config, printers, loading, busy, error, message, onConfig, onRefresh, onSave, onTest }: {
  config: LabelPrinterConfig;
  printers: string[];
  loading: boolean;
  busy: boolean;
  error: string | null;
  message: string | null;
  onConfig: (config: LabelPrinterConfig) => void;
  onRefresh: () => void;
  onSave: () => void;
  onTest: () => void;
}) {
  const savedQueueMissing = config.spoolerName !== "" && !loading && !printers.includes(config.spoolerName);
  return <section className="workspace hardware-workspace" aria-labelledby="hardware-heading">
    <div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="hardware-heading">Label printer</h1><PageDescription pageKey="hardware" /></div></div>
    <div className="hardware-boundary-note"><strong>Windows manages the connection.</strong> Install the manufacturer driver first. OncoFlow sends rasterized ESC/POS or TSPL bytes to the selected installed queue; it never connects directly to USB or LAN.</div>
    <div className="surface hardware-card">
      <div className="hardware-card__heading"><div><p className="eyebrow">This workstation</p><h2>Preparation label output</h2></div><button className="button button--secondary" type="button" disabled={loading || busy} onClick={onRefresh}>{loading ? "Refreshing…" : "Refresh queues"}</button></div>
      {error && <div className="auth-error" role="alert">{error}</div>}
      {message && <div className="auth-success" role="status">{message}</div>}
      {savedQueueMissing && <div className="hardware-warning" role="status">The saved queue is not currently installed or visible. Reinstall the driver or choose another queue.</div>}
      <div className="hardware-form-grid">
        <label className="is-wide">Windows printer queue<select value={config.spoolerName} disabled={loading || busy} onChange={(event) => onConfig({ ...config, spoolerName: event.target.value })}><option value="">Select an installed printer</option>{savedQueueMissing && <option value={config.spoolerName}>{config.spoolerName} (saved, unavailable)</option>}{printers.map((printer) => <option key={printer} value={printer}>{printer}</option>)}</select><small>Queue names may change when a printer driver is reinstalled.</small></label>
        <label>Printer language<select value={config.language} disabled={busy} onChange={(event) => onConfig({ ...config, language: event.target.value === "escpos" ? "escpos" : "tspl" })}><option value="tspl">TSPL</option><option value="escpos">ESC/POS</option></select><small>Must match the target printer.</small></label>
        <label>Resolution<select value={config.dpi} disabled={busy} onChange={(event) => onConfig({ ...config, dpi: Number(event.target.value) })}><option value={203}>203 dpi</option><option value={300}>300 dpi</option><option value={600}>600 dpi</option></select></label>
        <label>Width (mm)<input type="number" min="25" max="200" step="0.1" value={config.widthMm} disabled={busy} onChange={(event) => onConfig({ ...config, widthMm: Number(event.target.value) })} /></label>
        <label>Height (mm)<input type="number" min="20" max="200" step="0.1" value={config.heightMm} disabled={busy} onChange={(event) => onConfig({ ...config, heightMm: Number(event.target.value) })} /></label>
        <label>Label gap (mm)<input type="number" min="0" max="20" step="0.1" value={config.gapMm} disabled={busy} onChange={(event) => onConfig({ ...config, gapMm: Number(event.target.value) })} /></label>
        <label>Top spacing from preprinted header (mm)<input type="number" min="0" max="50" step="0.5" value={config.preprintHeaderSpacingMm} disabled={busy} onChange={(event) => onConfig({ ...config, preprintHeaderSpacingMm: Number(event.target.value) })} /><small>พื้นที่ว่างก่อนเริ่มพิมพ์ฉลาก ค่าเริ่มต้น 5 mm</small></label>
        <div className="hardware-font-heading is-wide"><strong>Preparation label font sizes</strong><small>กำหนดขนาดตัวอักษรแยกสำหรับแต่ละบรรทัด ทั้งหน้าตัวอย่างและฉลากจริง</small></div>
        {FONT_SIZE_FIELDS.map(([key, label]) => <label key={key}>{label}<input type="number" min="10" max="40" step="1" value={config.fontSizes[key]} disabled={busy} onChange={(event) => onConfig({ ...config, fontSizes: { ...config.fontSizes, [key]: Number(event.target.value) } })} /><small>10–40</small></label>)}
      </div>
      <div className="hardware-actions"><button className="button button--secondary" type="button" disabled={busy} onClick={onSave}>Save / connect</button><button className="button button--primary" type="button" disabled={busy || loading || savedQueueMissing || !config.spoolerName} onClick={onTest}>{busy ? "Sending test…" : "Print test label"}</button></div>
      <footer>Save/connect stores the queue and label layout locally; it does not probe the printer. A successful physical test label is the connection check. Windows accepting a job does not prove paper was printed.</footer>
    </div>
  </section>;
}

export function validatePrinterConfig(config: LabelPrinterConfig): string | null {
  if (!config.spoolerName.trim()) return "Select an installed Windows printer queue.";
  if (!Number.isFinite(config.widthMm) || config.widthMm < 25 || config.widthMm > 200) return "Label width must be between 25 and 200 mm.";
  if (!Number.isFinite(config.heightMm) || config.heightMm < 20 || config.heightMm > 200) return "Label height must be between 20 and 200 mm.";
  if (![203, 300, 600].includes(config.dpi)) return "Choose a supported printer resolution.";
  if (!Number.isFinite(config.gapMm) || config.gapMm < 0 || config.gapMm > 20) return "Label gap must be between 0 and 20 mm.";
  if (!Number.isFinite(config.preprintHeaderSpacingMm) || config.preprintHeaderSpacingMm < 0 || config.preprintHeaderSpacingMm > 50 || config.preprintHeaderSpacingMm > config.heightMm - 5) return "Top spacing must be between 0 and 50 mm and leave at least 5 mm of printable label height.";
  const invalidFontSize = FONT_SIZE_FIELDS.find(([key]) => !Number.isFinite(config.fontSizes[key]) || config.fontSizes[key] < 10 || config.fontSizes[key] > 40);
  if (invalidFontSize) return `${invalidFontSize[1]} font size must be between 10 and 40.`;
  return null;
}

const FONT_SIZE_FIELDS = [
  ["header", "Header"],
  ["patient", "Patient / HN"],
  ["withdrawal", "Withdrawal volume"],
  ["drug", "Drug / diluent"],
  ["routeRate", "Route / rate"],
  ["storage", "Storage"],
  ["warning", "Warning"],
  ["preparedBy", "Prepared by"],
  ["expiration", "Expiration"],
] as const;
