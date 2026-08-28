import { useEffect, useRef, type CSSProperties } from "react";

import { displayDateTime, displayLocalDateTime } from "../shared/dateTime";
import { DEFAULT_LABEL_FONT_SIZES } from "../hardware/printerSettings";
import type { LabelFontSizes } from "../types/hardware";
import type { PreparationOutput } from "../types/output";

export interface LabelDimensions {
  id: string;
  label: string;
  widthMm: number;
  heightMm: number;
}

export const LABEL_DIMENSIONS: LabelDimensions[] = [
  { id: "compact", label: "Compact · 100 × 70 mm", widthMm: 100, heightMm: 70 },
  { id: "narrow", label: "Narrow · 100 × 50 mm", widthMm: 100, heightMm: 50 },
  { id: "large", label: "Large · 148 × 105 mm", widthMm: 148, heightMm: 105 },
];

export function PreparationOutputView({ output, dimensions, fontSizes = DEFAULT_LABEL_FONT_SIZES, preprintHeaderSpacingMm = 5, printerName, busy, error, message, onClose, onPrint, onDimensions }: {
  output: PreparationOutput;
  dimensions: LabelDimensions;
  fontSizes?: LabelFontSizes;
  preprintHeaderSpacingMm?: number;
  printerName: string | null;
  busy: boolean;
  error: string | null;
  message: string | null;
  onClose: () => void;
  onPrint: () => void;
  onDimensions: (dimensions: LabelDimensions) => void;
}) {
  const { label, summary } = output;
  const containers = output.containers?.length ? output.containers : [{ containerIndex: 1 }];
  const printButtonLabel = `${output.printRequestCount > 0 ? "Reprint" : "Print"} ${containers.length === 1 ? "label" : `${containers.length} labels`}`;
  const dimensionChoices = LABEL_DIMENSIONS.some((value) => value.id === dimensions.id) ? LABEL_DIMENSIONS : [dimensions, ...LABEL_DIMENSIONS];
  const printStyle = {
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
      {error && <div className="form-error-summary" role="alert">{error}</div>}
      {message && <div className="auth-success preparation-output-message" role="status">{message}</div>}
      <div className="preparation-output-toolbar">
        <label>Physical label size<select value={dimensions.id} disabled={busy} onChange={(event) => onDimensions(dimensionChoices.find((value) => value.id === event.target.value) ?? LABEL_DIMENSIONS[0])}>{dimensionChoices.map((value) => <option key={value.id} value={value.id}>{value.label}</option>)}</select></label>
        <p>{printerName ? <>Windows queue: <strong>{printerName}</strong>. Content is fixed; dimensions affect raster layout only.</> : <>Choose a printer in <strong>Settings → Hardware</strong>.</>}</p>
        <button className="button button--primary" type="button" disabled={busy || !printerName} onClick={onPrint}>{busy ? "Sending to Windows…" : printButtonLabel}</button>
      </div>
      <div className="preparation-output-scroll">
        {containers.map((container) => <PreparationLabelPreview key={container.containerIndex} output={output} containerIndex={container.containerIndex} containerCount={containers.length} style={printStyle} fontSizes={fontSizes} />)}

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

function PreparationLabelPreview({ output, containerIndex, containerCount, style, fontSizes }: {
  output: PreparationOutput;
  containerIndex: number;
  containerCount: number;
  style: CSSProperties;
  fontSizes: LabelFontSizes;
}) {
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
      element.style.setProperty("--preparation-label-header-gap", `${0.75 * scale}mm`);
    };
    const fit = () => {
      applyScale(1);
      const computed = window.getComputedStyle(element);
      const available = element.clientHeight - Number.parseFloat(computed.paddingTop) - Number.parseFloat(computed.paddingBottom);
      if (content.scrollHeight <= available) return;
      let lower = 0.35;
      let upper = 1;
      applyScale(lower);
      for (let index = 0; index < 12; index += 1) {
        const candidate = (lower + upper) / 2;
        applyScale(candidate);
        if (content.scrollHeight <= available) lower = candidate;
        else upper = candidate;
      }
      applyScale(lower);
    };
    fit();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(fit);
    observer?.observe(element);
    return () => observer?.disconnect();
  }, [fontSizes, label, summary, containerCount]);

  return <article ref={root} className="preparation-label-print-root" style={style} aria-label={`Final checked preparation label ${containerIndex}/${containerCount}`}>
    <div className="preparation-label__fit">
      <header className="preparation-label__header">OncoFlow{label.hospitalName ? ` - ${label.hospitalName}` : ""}</header>
      <div className="preparation-label__lines">
        <p className="preparation-label__patient"><strong>{showDash(label.patientName)}</strong><span>| HN {label.patientIdentifier}</span></p>
        <p className="preparation-label__withdrawal">{withdrawalVolume(label.withdrawalVolumeMl)}</p>
        <p className="preparation-label__drug-line">{drugLine(label)}</p>
        <p className="preparation-label__route-rate">{routeRateLine(label.routeName, label.infusionRateOrDuration)}</p>
        <p className="preparation-label__storage">{showDash(summary.storageReference)}</p>
        <p className="preparation-label__warning">{showDash(label.warningText)}</p>
        <p className="preparation-label__prepared-by">{joinPlain(`Prepared by ${showDash(label.preparedBy)}`, displayDateTime(label.preparedAt), " | ")}</p>
        <p className="preparation-label__expiration"><strong>หมดอายุ {displayLocalDateTime(label.expirationAt, "—")}</strong><b>({containerIndex}/{containerCount})</b></p>
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
  const diluent = joinPlain(label.diluentName, volume(label.diluentVolumeMl));
  return diluent === "—" ? drugAndDose : `${drugAndDose} in ${diluent}`;
}
function withdrawalVolume(value: string | null): string { return `Withdrawal: ${value?.trim() ? `${value.trim()} mL` : "—"}`; }
function routeRateLine(route: string | null, rate: string | null): string {
  const value = rate?.trim();
  if (!value || rateStartsWithZero(value)) return showDash(route);
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
