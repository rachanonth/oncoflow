import { editFormKeyboard } from "../components/editFormKeyboard";
import { useCallback, useEffect, useState } from "react";

import { commandError, listSystemLabelFonts, listSystemPrinters, printTestLabel } from "../api/commands";
import { useLocalLabelFont } from "./useLocalLabelFont";
import { PageDescription } from "../guidance/PageGuidance";
import type { LabelPrinterConfig } from "../types/hardware";
import { DEFAULT_LABEL_ROW_STYLES, DEFAULT_LABEL_PRINTER, inferPrinterLanguageFromQueue, loadLabelPrinterConfig, saveLabelPrinterConfig } from "./printerSettings";

export function HardwareSettings() {
  const saved = loadLabelPrinterConfig();
  const [config, setConfig] = useState<LabelPrinterConfig>(saved ?? { spoolerName: "", ...DEFAULT_LABEL_PRINTER });
  const [fonts, setFonts] = useState<string[]>([]);
  const [printers, setPrinters] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(saved ? "Saved locally on this workstation." : null);

  const refresh = useCallback(async () => {
    setLoading(true); setError(null);
    try {
      const [queues, fontNames] = await Promise.all([listSystemPrinters(), listSystemLabelFonts()]);
      setPrinters(queues); setFonts(fontNames);
    }
    catch (requestError) {
      setPrinters([]); setFonts([]);
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
      setMessage("Printer queue, label layout, font, and font sizes saved locally. Run a test print to confirm the device.");
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

  return <HardwareSettingsView config={config} printers={printers} fonts={fonts} loading={loading} busy={busy} error={error} message={message} onConfig={setConfig} onRefresh={() => void refresh()} onSave={save} onTest={() => void testPrint()} />;
}

export function HardwareSettingsView({ config, printers, fonts = [], loading, busy, error, message, onConfig, onRefresh, onSave, onTest }: {
  config: LabelPrinterConfig;
  printers: string[];
  fonts?: string[];
  loading: boolean;
  busy: boolean;
  error: string | null;
  message: string | null;
  onConfig: (config: LabelPrinterConfig) => void;
  onRefresh: () => void;
  onSave: () => void;
  onTest: () => void;
}) {
  const { fontFamily, error: fontError } = useLocalLabelFont(config.fontName);
  const savedFontMissing = !!config.fontName && !loading && !fonts.includes(config.fontName);
  const savedQueueMissing = config.spoolerName !== "" && !loading && !printers.includes(config.spoolerName);
  return <section className="workspace hardware-workspace" aria-labelledby="hardware-heading">
    <div className="page-heading"><div><p className="eyebrow">Settings</p><h1 id="hardware-heading">Label printer</h1><PageDescription pageKey="hardware" /></div></div>
    <div className="hardware-boundary-note"><strong>Windows manages the connection.</strong> Install the manufacturer driver first. OncoFlow sends rasterized ZPL, TSPL, or ESC/POS bytes to the selected installed queue; it never connects directly to USB or LAN.</div>
    <div onKeyDownCapture={editFormKeyboard} className="surface hardware-card">
      <div className="hardware-card__heading"><div><p className="eyebrow">This workstation</p><h2>Preparation label output</h2></div><button className="button button--secondary" type="button" disabled={loading || busy} onClick={onRefresh}>{loading ? "Refreshing…" : "Refresh printers / fonts"}</button></div>
      {error && <div className="auth-error" role="alert">{error}</div>}
      {message && <div className="auth-success" role="status">{message}</div>}
      {savedQueueMissing && <div className="hardware-warning" role="status">The saved queue is not currently installed or visible. Reinstall the driver or choose another queue.</div>}
      <div className="hardware-form-grid">
        <label className="is-wide">Windows printer queue<select value={config.spoolerName} disabled={loading || busy} onChange={(event) => { const spoolerName = event.target.value; onConfig({ ...config, spoolerName, language: inferPrinterLanguageFromQueue(spoolerName) ?? config.language }); }}><option value="">Select an installed printer</option>{savedQueueMissing && <option value={config.spoolerName}>{config.spoolerName} (saved, unavailable)</option>}{printers.map((printer) => <option key={printer} value={printer}>{printer}</option>)}</select><small>Queue names may change when a printer driver is reinstalled.</small></label>
        <label>Printer language<select value={config.language} disabled={busy} onChange={(event) => { const language = event.target.value; onConfig({ ...config, language: language === "zpl" ? "zpl" : language === "escpos" ? "escpos" : "tspl" }); }}><option value="zpl">ZPL (Zebra)</option><option value="tspl">TSPL</option><option value="escpos">ESC/POS</option></select><small>Must match the target printer. Zebra ZDesigner ZPL queues use ZPL.</small></label>
        <label>Resolution<select value={config.dpi} disabled={busy} onChange={(event) => onConfig({ ...config, dpi: Number(event.target.value) })}><option value={203}>203 dpi</option><option value={300}>300 dpi</option><option value={600}>600 dpi</option></select></label>
        <label>Width (mm)<input type="number" min="25" max="200" step="0.1" value={config.widthMm} disabled={busy} onChange={(event) => onConfig({ ...config, widthMm: Number(event.target.value) })} /></label>
        <label>Height (mm)<input type="number" min="20" max="200" step="0.1" value={config.heightMm} disabled={busy} onChange={(event) => onConfig({ ...config, heightMm: Number(event.target.value) })} /></label>
        <label>Label gap (mm)<input type="number" min="0" max="20" step="0.1" value={config.gapMm} disabled={busy} onChange={(event) => onConfig({ ...config, gapMm: Number(event.target.value) })} /></label>
        <label>Top margin / ระยะขอบบน (mm)<input type="number" min="0" max={Math.max(0, config.heightMm - 5)} step="0.1" value={config.preprintHeaderSpacingMm} disabled={busy} onChange={(event) => onConfig({ ...config, preprintHeaderSpacingMm: Number(event.target.value) })} /><small>ระยะจากขอบบนกระดาษถึงบรรทัด HN เช่น หัวฉลากสูง 12 mm และต้องการเว้นเพิ่ม 2 mm ให้ใส่ 14 mm ใช้กับทั้งพรีวิวและฉลากพิมพ์ บันทึกแยกแต่ละเครื่อง</small></label>
        <label className="is-wide">ฟอนต์ฉลากยา<select value={config.fontName ?? ""} disabled={loading || busy} onChange={(event) => onConfig({ ...config, fontName: event.target.value || null })}><option value="">ค่าเริ่มต้น (อัตโนมัติ)</option>{savedFontMissing && <option value={config.fontName!}>{config.fontName} (ไม่พบในเครื่องนี้)</option>}{fonts.map((font) => <option key={font} value={font}>{font}</option>)}</select><small>ฟอนต์ในเครื่องที่รองรับภาษาไทย บันทึกแยกตามเครื่องที่สั่งพิมพ์</small>{savedFontMissing && <small role="alert">กรุณาติดตั้งฟอนต์เดิมหรือเลือกฟอนต์อื่นก่อนพิมพ์</small>}</label>
        <div className="is-wide"><p style={{ fontFamily }}>ตัวอย่างฉลาก: ดูดยา 20 mL · วิธีให้ยา: IV · HN 123456</p>{fontError && <small role="alert">{fontError}</small>}</div>
        <div className="hardware-font-heading is-wide"><strong>Preparation label font sizes</strong><small>กำหนดขนาดตัวอักษรแยกสำหรับแต่ละบรรทัด ทั้งหน้าตัวอย่างและฉลากจริง</small></div>
        {FONT_SIZE_FIELDS.map(([key, label]) => <label key={key}>{label}<input type="number" min="10" step="1" value={config.fontSizes[key]} disabled={busy} onChange={(event) => onConfig({ ...config, fontSizes: { ...config.fontSizes, [key]: Number(event.target.value) } })} /><small>ตั้งแต่ 10 px ขึ้นไป ไม่จำกัดค่าสูงสุด</small></label>)}
      </div>
      <fieldset className="hardware-row-styles"><legend>รูปแบบตัวอักษรแต่ละแถว</legend>
        {ROW_STYLE_FIELDS.map(([key, label]) => <div key={key} className="hardware-row-style"><strong>{label}</strong>{(["bold", "underline"] as const).map(kind => <label key={kind}><input type="checkbox" checked={(config.rowStyles ?? DEFAULT_LABEL_ROW_STYLES)[key][kind]} disabled={busy} onChange={event => { const styles = config.rowStyles ?? DEFAULT_LABEL_ROW_STYLES; onConfig({ ...config, rowStyles: { ...styles, [key]: { ...styles[key], [kind]: event.target.checked } } }); }} />{kind === "bold" ? "ตัวหนา" : "ขีดเส้นใต้"}<span className="sr-only"> {label}</span></label>)}</div>)}
      </fieldset>
      <div className="hardware-actions"><button className="button button--secondary" type="button" disabled={busy || loading || savedFontMissing} data-editor-save onClick={onSave}>Save / connect</button><button className="button button--primary" type="button" disabled={busy || loading || savedQueueMissing || savedFontMissing || !config.spoolerName} onClick={onTest}>{busy ? "Sending test…" : "Print test label"}</button></div>
      <footer>Save/connect stores the queue and label layout locally; it does not probe the printer. A successful physical test label is the connection check. Windows accepting a job does not prove paper was printed.</footer>
    </div>
  </section>;
}

export function validatePrinterConfig(config: LabelPrinterConfig): string | null {
  if (!config.spoolerName.trim()) return "Select an installed Windows printer queue.";
  const inferredLanguage = inferPrinterLanguageFromQueue(config.spoolerName);
  if (inferredLanguage && inferredLanguage !== config.language) {
    return `The selected Windows queue indicates ${inferredLanguage.toUpperCase()}. Set Printer language to ${inferredLanguage.toUpperCase()}.`;
  }
  if (!Number.isFinite(config.widthMm) || config.widthMm < 25 || config.widthMm > 200) return "Label width must be between 25 and 200 mm.";
  if (!Number.isFinite(config.heightMm) || config.heightMm < 20 || config.heightMm > 200) return "Label height must be between 20 and 200 mm.";
  if (![203, 300, 600].includes(config.dpi)) return "Choose a supported printer resolution.";
  if (!Number.isFinite(config.gapMm) || config.gapMm < 0 || config.gapMm > 20) return "Label gap must be between 0 and 20 mm.";
  if (!Number.isFinite(config.preprintHeaderSpacingMm) || config.preprintHeaderSpacingMm < 0 || config.preprintHeaderSpacingMm > config.heightMm - 5) return "Top margin must be zero or greater and leave at least 5 mm of printable label height.";
  const invalidFontSize = FONT_SIZE_FIELDS.find(([key]) => !Number.isFinite(config.fontSizes[key]) || config.fontSizes[key] < 10);
  if (invalidFontSize) return `${invalidFontSize[1]} font size must be a finite number of at least 10 px.`;
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
  ["preparedBy", "Prepared at"],
  ["expiration", "Expiration"],
] as const;

const ROW_STYLE_FIELDS = [
  ["header", "HN / หอผู้ป่วย"],
  ["patient", "ชื่อผู้ป่วย"],
  ["withdrawal", "ดูดยา"],
  ["drug", "ชื่อยา / ขนาดยา"],
  ["diluent", "สารละลาย (in diluent)"],
  ["routeRate", "วิธีให้ยา"],
  ["storage", "การเก็บยา"],
  ["warning", "คำเตือน"],
  ["preparedBy", "เตรียมเมื่อ"],
  ["expiration", "หมดอายุ / ลำดับฉลาก"],
] as const;
