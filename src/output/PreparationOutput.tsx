import { useEffect, useRef, type CSSProperties } from "react";

import { displayDateTime, displayLocalDateTime } from "../shared/dateTime";
import { useLocalLabelFont } from "../hardware/useLocalLabelFont";
import { DEFAULT_LABEL_FONT_SIZES, DEFAULT_LABEL_ROW_STYLES } from "../hardware/printerSettings";
import type { LabelFontSizes, LabelRowStyles, LabelRowKey } from "../types/hardware";
import type { PreparationOutput } from "../types/output";

export interface LabelDimensions {
  id: string;
  label: string;
  widthMm: number;
  heightMm: number;
}

export const LABEL_DIMENSIONS: LabelDimensions[] = [
  { id: "label80", label: "80 × 70 mm", widthMm: 80, heightMm: 70 },
  { id: "compact", label: "Compact · 100 × 70 mm", widthMm: 100, heightMm: 70 },
  { id: "narrow", label: "Narrow · 100 × 50 mm", widthMm: 100, heightMm: 50 },
  { id: "large", label: "Large · 148 × 105 mm", widthMm: 148, heightMm: 105 },
];

export function PreparationOutputView({ output, dimensions, fontSizes = DEFAULT_LABEL_FONT_SIZES, fontName, rowStyles = DEFAULT_LABEL_ROW_STYLES, preprintHeaderSpacingMm = 5, printerName, busy, error, message, onClose, onPrint, onDimensions }: {
  output: PreparationOutput;
  dimensions: LabelDimensions;
  fontSizes?: LabelFontSizes;
  rowStyles?: LabelRowStyles;
  fontName?: string | null;
  preprintHeaderSpacingMm?: number;
  printerName: string | null;
  busy: boolean;
  error: string | null;
  message: string | null;
  onClose: () => void;
  onPrint: () => void;
  onDimensions: (dimensions: LabelDimensions) => void;
}) {
  const { fontFamily, error: fontError } = useLocalLabelFont(fontName);
  const { label, summary } = output;
  const containers = output.containers?.length ? output.containers : [{ containerIndex: 1 }];
  const printButtonLabel = `${output.printRequestCount > 0 ? "Reprint" : "Print"} ${containers.length === 1 ? "label" : `${containers.length} labels`}`;
  const dimensionChoices = LABEL_DIMENSIONS.some((value) => value.id === dimensions.id) ? LABEL_DIMENSIONS : [dimensions, ...LABEL_DIMENSIONS];
  const printStyle = {
    fontFamily,
    "--preparation-label-width": `${dimensions.widthMm}mm`,
    "--preparation-label-height": `${dimensions.heightMm}mm`,
    "--preparation-label-margin": `${dimensions.widthMm / 35}mm`,
    "--preparation-label-preprint-header-spacing": `${preprintHeaderSpacingMm}mm`,
    "--preparation-label-font-header": `${fontSizes.header}px`,
    "--preparation-label-font-patient": `${fontSizes.patient}px`,
    "--preparation-label-font-withdrawal": `${fontSizes.withdrawal}px`,
    "--preparation-label-font-drug": `${fontSizes.drug}px`,
    "--preparation-label-font-route-rate": `${fontSizes.routeRate}px`,
    "--preparation-label-font-storage": `${fontSizes.storage}px`,
    "--preparation-label-font-warning": `${fontSizes.warning}px`,
    "--preparation-label-font-prepared-by": `${fontSizes.preparedBy}px`,
    "--preparation-label-font-expiration": `${fontSizes.expiration}px`,
  } as CSSProperties;
  return <div className="preparation-output-backdrop" role="presentation">
    <section className="preparation-output-dialog" role="dialog" aria-modal="true" aria-labelledby="preparation-output-heading">
      <header className="preparation-output-dialog__header">
        <div><p className="eyebrow">Checked preparation output</p><h2 id="preparation-output-heading">Preparation label</h2><p>Frozen snapshot #{label.snapshotId} · {label.templateVersion}</p></div>
        <button className="button button--secondary" type="button" onClick={onClose} aria-label="Close label preview">Close</button>
      </header>
      {fontError && <div className="form-error-summary" role="alert">{fontError}</div>}
      {error && <div className="form-error-summary" role="alert">{error}</div>}
      {message && <div className="auth-success preparation-output-message" role="status">{message}</div>}
      <div className="preparation-output-toolbar">
        <label>Physical label size<select value={dimensions.id} disabled={busy} onChange={(event) => onDimensions(dimensionChoices.find((value) => value.id === event.target.value) ?? LABEL_DIMENSIONS[0])}>{dimensionChoices.map((value) => <option key={value.id} value={value.id}>{value.label}</option>)}</select></label>
        <p>{printerName ? <>Windows queue: <strong>{printerName}</strong>. Content is fixed; dimensions affect raster layout only.</> : <>Choose a printer in <strong>Settings → Hardware</strong>.</>}</p>
        <button className="button button--primary" type="button" disabled={busy || !printerName} onClick={onPrint}>{busy ? "Sending to Windows…" : printButtonLabel}</button>
      </div>
      <div className="preparation-output-scroll">
        {containers.map((container) => <PreparationLabelPreview key={container.containerIndex} output={output} containerIndex={container.containerIndex} containerCount={containers.length} style={printStyle} fontSizes={fontSizes} rowStyles={rowStyles} />)}

        <article className="preparation-summary surface" aria-labelledby="preparation-summary-heading">
          <header><div><p className="eyebrow">Pharmacist reference</p><h3 id="preparation-summary-heading">Preparation summary</h3></div><span>Generated {displayDateTime(label.generatedAt)}</span></header>
          <div className="preparation-summary__grid">
            <SummaryValue label="Ordered dose" value={join(label.orderedDoseText, label.doseUnitText)} />
            <SummaryValue label="Diluent / volume" value={join(label.diluentName, volume(label.diluentVolumeMl))} />
            <SummaryValue label="Final volume" value={volume(label.finalVolumeMl)} />
            <SummaryValue label="Final containers" value={`${containers.length}`} />
            <SummaryValue label="Route / rate" value={join(label.routeName, label.infusionRateOrDuration)} />
            <SummaryValue label="Containers issued" value={numberValue(summary.containersRequired)} />
            <SummaryValue label="Inventory movement" value={summary.inventoryMovementId === null ? null : `#${summary.inventoryMovementId}`} />
          </div>
          <InventoryOutputSummary output={output} />
          {(summary.preparationInstructions || summary.preparationNotes || summary.storageReference) && <div className="preparation-summary__notes">
            {summary.preparationInstructions && <p><strong>Preparation instructions</strong>{summary.preparationInstructions}</p>}
            {summary.preparationNotes && <p><strong>Preparation notes</strong>{summary.preparationNotes}</p>}
            {summary.storageReference && <p><strong>Legacy storage reference</strong>{summary.storageReference}<small>Expiration is shown only when an expiry duration is configured in Drug master.</small></p>}
          </div>}
          <p className="preparation-summary__boundary">{summary.presentationNotice}</p>
          <footer>Preparation check complete. Label warning and expiry references are frozen from Drug master.</footer>
        </article>
      </div>
    </section>
  </div>;
}

function PreparationLabelPreview({ output, containerIndex, containerCount, style, fontSizes, rowStyles }: {
  output: PreparationOutput;
  containerIndex: number;
  containerCount: number;
  style: CSSProperties;
  fontSizes: LabelFontSizes;
  rowStyles: LabelRowStyles;
}) {
  const rowStyle = (key: LabelRowKey) => ({ "--label-row-weight": rowStyles[key].bold ? 700 : 400, textDecoration: rowStyles[key].underline ? "underline" : "none" }) as CSSProperties;
  const root = useRef<HTMLElement>(null);
  const { label, summary } = output;
  useEffect(() => {
    const element = root.current;
    const content = element?.querySelector<HTMLElement>(".preparation-label__fit");
    if (!element || !content) return;
    const applyScale = (scale: number) => {
      element.style.setProperty("--preparation-label-font-header", `${fontSizes.header * scale}px`);
      element.style.setProperty("--preparation-label-font-patient", `${fontSizes.patient * scale}px`);
      element.style.setProperty("--preparation-label-font-withdrawal", `${fontSizes.withdrawal * scale}px`);
      element.style.setProperty("--preparation-label-font-drug", `${fontSizes.drug * scale}px`);
      element.style.setProperty("--preparation-label-font-route-rate", `${fontSizes.routeRate * scale}px`);
      element.style.setProperty("--preparation-label-font-storage", `${fontSizes.storage * scale}px`);
      element.style.setProperty("--preparation-label-font-warning", `${fontSizes.warning * scale}px`);
      element.style.setProperty("--preparation-label-font-prepared-by", `${fontSizes.preparedBy * scale}px`);
      element.style.setProperty("--preparation-label-font-expiration", `${fontSizes.expiration * scale}px`);
      element.style.setProperty("--preparation-label-line-padding", `${0.35 * scale}mm`);
    };
    const fit = () => {
      content.style.height = "auto";
      const lines = content.querySelector<HTMLElement>(".preparation-label__lines");
      if (lines) lines.style.flex = "";
      applyScale(1);
      const computed = window.getComputedStyle(element);
      const available = element.clientHeight - Number.parseFloat(computed.paddingTop) - Number.parseFloat(computed.paddingBottom);
      let lower = 0.35;
      let upper = 4;
      applyScale(lower);
      for (let index = 0; index < 12; index += 1) {
        const candidate = (lower + upper) / 2;
        applyScale(candidate);
        const rowsFit = Array.from(content.querySelectorAll<HTMLElement>("p, header")).every(
          (row) => row.scrollWidth <= row.clientWidth,
        );
        if (content.scrollHeight <= available && content.scrollWidth <= content.clientWidth && rowsFit) lower = candidate;
        else upper = candidate;
      }
      applyScale(lower);
      content.style.height = `${available}px`;
      if (lines) lines.style.flex = "1";
    };
    fit();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(fit);
    observer?.observe(element);
    document.fonts?.addEventListener("loadingdone", fit);
    return () => {
      observer?.disconnect();
      document.fonts?.removeEventListener("loadingdone", fit);
    };
  }, [fontSizes, rowStyles, label, summary, containerCount, style.fontFamily]);

  return <article ref={root} className="preparation-label-print-root" style={style} aria-label={`Final checked preparation label ${containerIndex}/${containerCount}`}>
    <div className="preparation-label__fit">
      <div className="preparation-label__lines">
      <header data-label-row="header" style={rowStyle("header")} className="preparation-label__header">{`HN ${label.patientIdentifier} | หอผู้ป่วย ${showDash(label.wardName)}`}</header>
        <p data-label-row="patient" style={rowStyle("patient")} className="preparation-label__patient"><strong>{showDash(label.patientName)}</strong></p>
        <p data-label-row="withdrawal" style={rowStyle("withdrawal")} className="preparation-label__withdrawal">{withdrawalVolume(label.withdrawalVolumeMl)}</p>
        <p data-label-row="drug" style={rowStyle("drug")} className="preparation-label__drug-line">ยา: {drugLine(label)}</p>
        <p data-label-row="diluent" style={rowStyle("diluent")} className="preparation-label__drug-line">in {joinPlain(label.diluentName, volume(label.diluentVolumeMl))}</p>
        <p data-label-row="routeRate" style={rowStyle("routeRate")} className="preparation-label__route-rate">วิธีให้ยา: {routeRateLine(label.routeName, label.infusionRateOrDuration)}</p>
        <p data-label-row="storage" style={rowStyle("storage")} className="preparation-label__storage">การเก็บยา: {showDash(summary.storageReference)}</p>
        <p data-label-row="warning" style={rowStyle("warning")} className="preparation-label__warning">คำเตือน: {showDash(label.warningText)}</p>
        <p data-label-row="preparedBy" style={rowStyle("preparedBy")} className="preparation-label__prepared-by">เตรียมเมื่อ {displayDateTime(label.preparedAt)}</p>
        <p data-label-row="expiration" style={rowStyle("expiration")} className="preparation-label__expiration"><strong>หมดอายุ {displayLocalDateTime(label.expirationAt, "—")}</strong><b>({containerIndex}/{containerCount})</b></p>
      </div>
    </div>
  </article>;
}

function InventoryOutputSummary({ output }: { output: PreparationOutput }) {
  const summary = output.summary;
  if (!summary.inventoryPostingStatus) return <div className="output-inventory output-inventory--neutral"><strong>Pre-integration preparation</strong><span>No inventory posting was backfilled.</span></div>;
  if (summary.inventoryPostingStatus === "manual_reconciliation_required") return <div className="output-inventory output-inventory--manual"><strong>Manual inventory reconciliation required</strong><span>Preparation checking is complete; no automatic quantity was guessed.</span></div>;
  if (summary.inventoryPostingStatus !== "posted") return <div className="output-inventory output-inventory--neutral"><strong>Automatic inventory issue not posted</strong><span>{summary.inventoryPostingStatus.replaceAll("_", " ")}</span></div>;
  const shortage = summary.inventoryStockState === "shortage";
  return <div className={`output-inventory ${shortage ? "output-inventory--shortage" : ""}`}><div><strong>Inventory consumption posted</strong><span>{numberValue(summary.inventoryBalanceBefore)} → {numberValue(summary.inventoryBalanceAfter)}</span></div><b>{stockState(summary.inventoryStockState)}</b>{shortage && <p>Shortage is recorded and did not block preparation checking or label output.</p>}</div>;
}

function SummaryValue({ label, value }: { label: string; value: string | null }) { return <div><span>{label}</span><strong>{show(value)}</strong></div>; }
function show(value: string | null | undefined): string { return value?.trim() || "Not recorded"; }
function showDash(value: string | null | undefined): string { return value?.trim() || "—"; }
function join(...values: Array<string | null | undefined>): string | null { return values.map((value) => value?.trim()).filter(Boolean).join(" · ") || null; }
function joinPlain(first: string | null | undefined, second: string | null | undefined, separator = " "): string { return [first, second].map((value) => value?.trim()).filter(Boolean).join(separator) || "—"; }
function drugLine(label: PreparationOutput["label"]): string {
  const drugAndDose = [label.drugName, label.orderedDoseText, label.doseUnitText].map((value) => value?.trim()).filter(Boolean).join(" ");
  return drugAndDose;
}
function withdrawalVolume(value: string | null): string { return `ดูดยา: ${value?.trim() ? `${value.trim()} mL` : "—"}`; }
function routeRateLine(route: string | null, rate: string | null): string {
  const value = rate?.trim();
  if (!value || rateStartsWithZero(value)) return joinPlain(route, "ตามโปรโตคอล");
  return joinPlain(route, `in ${value}`);
}
function rateStartsWithZero(value: string): boolean {
  const numericPrefix = value.match(/^[+-]?(?:\d+(?:[.,]\d*)?|[.,]\d+)/)?.[0];
  return numericPrefix !== undefined
    && Number(numericPrefix.replace(",", ".")) === 0
    && !/\d/.test(value.slice(numericPrefix.length));
}
function volume(value: number | null): string | null { return value === null ? null : `${value} mL`; }
function numberValue(value: number | null): string | null { return value === null ? null : `${value}`; }
function stockState(value: PreparationOutput["summary"]["inventoryStockState"]): string { if (value === "shortage") return "Shortage"; if (value === "out") return "Out"; if (value === "low") return "Low"; if (value === "normal") return "Normal"; return "Not recorded"; }
