import { useCallback, useEffect, useState } from "react";

import {
  checkPreparationTask,
  commandError,
  initializePreparation,
  getPreparationOutput,
  printOrderPreparationLabels,
  printPreparationLabel,
  updatePreparationTask,
} from "../api/commands";
import { displayDateTime, displayLocalDateTime } from "../shared/dateTime";
import { LABEL_DIMENSIONS, PreparationOutputView, type LabelDimensions } from "../output/PreparationOutput";
import { loadLabelPrinterConfig } from "../hardware/printerSettings";
import type { PreparationTask, PreparationTaskInput, PreparationWorkspace as Workspace, PreparationWorkspaceItem } from "../types/preparation";
import type { PreparationOutput } from "../types/output";

type WorkspaceState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; workspace: Workspace };

export function PreparationWorkspace({ orderId, preparationDate, onBack, onOpenOrder }: { orderId: number; preparationDate: string; onBack: () => void; onOpenOrder: () => void }) {
  const [state, setState] = useState<WorkspaceState>({ kind: "loading" });
  const [operationError, setOperationError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [output, setOutput] = useState<PreparationOutput | null>(null);
  const [outputBusy, setOutputBusy] = useState(false);
  const [outputError, setOutputError] = useState<string | null>(null);
  const [outputMessage, setOutputMessage] = useState<string | null>(null);
  const [selectedTaskIds, setSelectedTaskIds] = useState<Set<number>>(new Set());
  const [batchMessage, setBatchMessage] = useState<string | null>(null);
  const [labelDimensions, setLabelDimensions] = useState<LabelDimensions>(() => {
    const saved = loadLabelPrinterConfig();
    if (!saved) return LABEL_DIMENSIONS[0];
    return LABEL_DIMENSIONS.find((value) => value.widthMm === saved.widthMm && value.heightMm === saved.heightMm) ?? { id: "configured", label: `Configured · ${saved.widthMm} × ${saved.heightMm} mm`, widthMm: saved.widthMm, heightMm: saved.heightMm };
  });
  const printerConfig = loadLabelPrinterConfig();

  const load = useCallback(async () => {
    setState({ kind: "loading" });
    setOperationError(null);
    try {
      const workspace = await initializePreparation(orderId, preparationDate);
      setState({ kind: "ready", workspace });
      setSelectedTaskIds(new Set(workspace.items.flatMap((item) => item.task?.state === "verified" ? [item.task.id] : [])));
    }
    catch (error) { setState({ kind: "error", message: commandError(error).message ?? "Unable to open preparation." }); }
  }, [orderId, preparationDate]);

  useEffect(() => { void load(); }, [load]);

  function replaceTask(task: PreparationTask) {
    setState((current) => current.kind === "ready" ? { kind: "ready", workspace: { ...current.workspace, items: current.workspace.items.map((item) => item.orderItemId === task.sourceOrderItemId ? { ...item, task } : item) } } : current);
    if (task.state === "verified") setSelectedTaskIds((current) => new Set(current).add(task.id));
  }

  async function runTask(operation: () => Promise<PreparationTask>) {
    setBusy(true); setOperationError(null);
    try { replaceTask(await operation()); }
    catch (error) { setOperationError(commandError(error).message ?? "Preparation operation failed."); }
    finally { setBusy(false); }
  }

  async function openOutput(taskId: number) {
    setOutputBusy(true); setOutputError(null); setOutputMessage(null); setOperationError(null);
    try { setOutput(await getPreparationOutput(taskId)); }
    catch (error) { setOperationError(commandError(error).message ?? "Preparation label unavailable."); }
    finally { setOutputBusy(false); }
  }

  async function printOutput() {
    if (!output) return;
    if (!printerConfig) { setOutputError("Choose a Windows label-printer queue in Settings → Hardware before printing."); return; }
    setOutputBusy(true); setOutputError(null); setOutputMessage(null);
    try {
      const result = await printPreparationLabel(output.label.preparationId, {
        ...printerConfig,
        widthMm: labelDimensions.widthMm,
        heightMm: labelDimensions.heightMm,
      });
      setOutput(result.output);
      const labelCount = result.output.containers?.length ?? 1;
      setOutputMessage(`Windows accepted print job #${result.job.windowsJobId} for ${labelCount} label(s). Confirm the physical labels printed correctly.`);
    } catch (error) {
      setOutputError(commandError(error).message ?? "Unable to open the local print workflow.");
    } finally { setOutputBusy(false); }
  }

  async function printBatch(selectedOnly: boolean) {
    if (state.kind !== "ready") return;
    if (!printerConfig) { setOperationError("Choose a Windows label-printer queue in Settings → Hardware before printing."); return; }
    const ids = selectedOnly
      ? Array.from(selectedTaskIds)
      : state.workspace.items.flatMap((item) => item.task?.state === "verified" ? [item.task.id] : []);
    if (ids.length === 0) { setOperationError(selectedOnly ? "Select at least one checked preparation item." : "Check at least one preparation item before printing."); return; }
    setBusy(true); setOperationError(null); setBatchMessage(null);
    try {
      const result = await printOrderPreparationLabels(orderId, ids, {
        ...printerConfig,
        widthMm: labelDimensions.widthMm,
        heightMm: labelDimensions.heightMm,
      });
      const labelCount = result.outputs.reduce((sum, item) => sum + (item.containers?.length ?? 1), 0);
      setBatchMessage(`Windows accepted batch job #${result.job.windowsJobId} for ${labelCount} label(s). Confirm the physical labels printed correctly.`);
    } catch (error) {
      setOperationError(commandError(error).message ?? "Unable to print preparation labels.");
    } finally { setBusy(false); }
  }

  if (state.kind === "loading") return <section className="workspace preparation-workspace"><div className="state-panel" aria-busy="true"><span className="state-icon">⌁</span><h2>Opening preparation workspace…</h2></div></section>;
  if (state.kind === "error") return <section className="workspace preparation-workspace"><button className="back-button" type="button" onClick={onBack}>← Preparation queue</button><div className="state-panel state-panel--error" role="alert"><span className="state-icon">!</span><h2>Preparation unavailable</h2><p>{state.message}</p><button className="button button--secondary" type="button" onClick={() => void load()}>Try again</button></div></section>;

  return <><PreparationWorkspaceView
    workspace={state.workspace}
    busy={busy}
    operationError={operationError}
    batchMessage={batchMessage}
    selectedTaskIds={selectedTaskIds}
    onBack={onBack}
    onOpenOrder={onOpenOrder}
    onSave={(taskId, input) => runTask(() => updatePreparationTask(taskId, input))}
    onCheck={(taskId, input) => runTask(async () => {
      await updatePreparationTask(taskId, input);
      return checkPreparationTask(taskId);
    })}
    onOutput={(taskId) => void openOutput(taskId)}
    onToggleSelected={(taskId, selected) => setSelectedTaskIds((current) => { const next = new Set(current); if (selected) next.add(taskId); else next.delete(taskId); return next; })}
    onPrintAll={() => void printBatch(false)}
    onPrintSelected={() => void printBatch(true)}
  />{output && <PreparationOutputView output={output} dimensions={labelDimensions} fontSizes={printerConfig?.fontSizes} preprintHeaderSpacingMm={printerConfig?.preprintHeaderSpacingMm} printerName={printerConfig?.spoolerName ?? null} busy={outputBusy} error={outputError} message={outputMessage} onClose={() => { setOutput(null); setOutputError(null); setOutputMessage(null); }} onPrint={() => void printOutput()} onDimensions={setLabelDimensions} />}</>;
}

export function PreparationWorkspaceView({ workspace, busy, operationError, batchMessage, selectedTaskIds, onBack, onOpenOrder, onSave, onCheck, onOutput, onToggleSelected, onPrintAll, onPrintSelected }: { workspace: Workspace; busy: boolean; operationError: string | null; batchMessage: string | null; selectedTaskIds: ReadonlySet<number>; onBack: () => void; onOpenOrder: () => void; onSave: (taskId: number, input: PreparationTaskInput) => void; onCheck: (taskId: number, input: PreparationTaskInput) => void; onOutput?: (taskId: number) => void; onToggleSelected: (taskId: number, selected: boolean) => void; onPrintAll: () => void; onPrintSelected: () => void }) {
  const checkedTasks = workspace.items.flatMap((item) => item.task?.state === "verified" ? [item.task] : []);
  const dailySequenceByTaskId = preparationTaskSequences(workspace.items);
  return <section className="workspace preparation-workspace" aria-labelledby="preparation-heading">
    <button className="back-button" type="button" onClick={onBack}>← Preparation queue</button>
    <div className="preparation-hero surface"><div><h1 id="preparation-heading">HN {workspace.patientHn}: {workspace.patientName || "Name not recorded"}</h1><p className="preparation-hero__order-number">Order {workspace.orderCode}</p><button className="button button--secondary button--compact preparation-hero__order-action" type="button" onClick={onOpenOrder}>แก้ไขคำสั่งใช้ยา</button></div><div className="preparation-hero__meta"><span><small>Preparation date</small>{displayLocalDateTime(workspace.preparationDate)}</span><span><small>Order date</small>{displayLocalDateTime(workspace.treatmentTime)}</span>{workspace.regimenName && <span><small>Regimen</small>{workspace.regimenName}</span>}<span><small>Preparation pharmacist</small>{workspace.assignedPreparer?.displayName ?? "Not assigned"}</span><span><small>Items due</small>{workspace.items.length}</span></div></div>
    {operationError && <div className="form-error-summary" role="alert">{operationError}</div>}
    {batchMessage && <div className="auth-success" role="status">{batchMessage}</div>}
    {checkedTasks.length > 0 && <div className="surface preparation-batch-actions"><div><p className="eyebrow">Label output</p><h2>Print order labels</h2><p>พิมพ์ทั้งใบสั่งยา หรือพิมพ์เฉพาะรายการที่เลือก</p></div><div><button className="button button--primary" type="button" disabled={busy} onClick={onPrintAll}>Print all labels in order</button><button className="button button--secondary" type="button" disabled={busy || selectedTaskIds.size === 0} onClick={onPrintSelected}>Print selected ({selectedTaskIds.size})</button></div></div>}
    <section className="preparation-items" aria-labelledby="preparation-items-heading"><div className="section-heading"><div><p className="eyebrow">Ordered products</p><h2 id="preparation-items-heading">Preparation items</h2></div>{workspace.excludedItemCount > 0 && <span className="preparation-excluded">{workspace.excludedItemCount} routine/unmarked item{workspace.excludedItemCount === 1 ? "" : "s"} excluded</span>}</div>{workspace.items.length === 0 ? <div className="state-panel surface"><span className="state-icon">⌁</span><h2>ไม่มีรายการยาสำหรับเตรียม</h2><p>ใบสั่งยานี้ไม่มีรายการยาที่ต้องเตรียมในวันที่เลือก</p><button className="button button--secondary" type="button" onClick={onOpenOrder}>แก้ไขคำสั่งใช้ยา</button></div> : <div className="preparation-item-list">{workspace.items.map((item) => <PreparationItemCard key={`${item.orderItemId}:${item.task?.updatedAt ?? "new"}`} item={item} displaySequence={item.task ? dailySequenceByTaskId.get(item.task.id) ?? null : null} assignedPreparer={workspace.assignedPreparer} selected={item.task ? selectedTaskIds.has(item.task.id) : false} busy={busy} onSave={onSave} onCheck={onCheck} onOutput={onOutput} onToggleSelected={onToggleSelected} />)}</div>}</section>
    <footer className="preparation-provenance">Eligibility <code>{workspace.eligibilityRuleId}</code>. Checking posts one inventory issue only for a fully supported container result. Warnings and precautions are deferred to a future release; no administration record is created.</footer>
  </section>;
}

type FinalVolumeMode = "solution_plus_drug" | "manual";

function PreparationItemCard({ item, displaySequence, assignedPreparer, selected, busy, onSave, onCheck, onOutput, onToggleSelected }: { item: PreparationWorkspaceItem; displaySequence: number | null; assignedPreparer: Workspace["assignedPreparer"]; selected: boolean; busy: boolean; onSave: (taskId: number, input: PreparationTaskInput) => void; onCheck: (taskId: number, input: PreparationTaskInput) => void; onOutput?: (taskId: number) => void; onToggleSelected: (taskId: number, selected: boolean) => void }) {
  const task = item.task;
  const automaticVolume = item.defaultPreparationVolumeMl;
  const storedVolume = task?.preparationVolumeMl?.toString() ?? "";
  const [volumeMode, setVolumeMode] = useState<FinalVolumeMode>(() => task?.state === "verified" && storedVolume === "" ? "manual" : storedVolume !== "" && storedVolume !== automaticVolume ? "manual" : "solution_plus_drug");
  const [manualVolume, setManualVolume] = useState(storedVolume || automaticVolume || "");
  const [notes, setNotes] = useState(task?.preparationNotes ?? "");
  const [containerCount, setContainerCount] = useState(task?.finalContainerCount ?? 1);
  const volume = volumeMode === "solution_plus_drug" ? automaticVolume ?? "" : manualVolume;
  const parsedVolume = volume.trim() === "" ? null : Number(volume);
  const volumeInvalid = parsedVolume !== null && (!Number.isFinite(parsedVolume) || parsedVolume < 0);
  const preparationInput = (): PreparationTaskInput => ({ preparationVolumeMl: parsedVolume, preparationNotes: notes.trim() || null, finalContainerCount: containerCount });
  const readOnly = !task || task.state === "verified";
  return <article className="surface preparation-item-card">
    <header><div className="preparation-sequence">{displaySequence ?? "—"}</div><div><h3>{item.drugName}</h3></div><div className="preparation-item-card__header-actions"><span className={`preparation-status preparation-status--${task?.state ?? "neutral"}`}>{task ? stateLabel(task.state) : "Not initialized"}</span>{task?.state === "verified" && <><label className="preparation-label-selection"><input type="checkbox" checked={selected} onChange={(event) => onToggleSelected(task.id, event.target.checked)} />Select for label batch</label><button className="button button--secondary button--compact" type="button" disabled={busy} onClick={() => onOutput?.(task.id)}>Preview / print this label</button></>}</div></header>
    <div className="preparation-values"><Value label="Ordered dose" value={joinValue(item.orderedDoseText, item.doseUnitText)} strong/><Value label="Diluent" value={joinValue(item.diluentName, item.diluentVolumeMl === null ? null : `${item.diluentVolumeMl} mL`)} /><Value label="Route" value={item.routeName}/><Value label="Rate / duration" value={item.rateText}/><Value label="Treatment day" value={item.treatmentDay}/><Value label="Drug solution reference" value={item.referenceQuantity.drugSolutionVolumeMl ? `${item.referenceQuantity.drugSolutionVolumeMl} mL` : statusLabel(item.referenceQuantity.status)} /></div>
    <PreparationCalculationPanel item={item} />
    {item.referenceQuantity.status === "calculated" && <details className="preparation-reference"><summary>Reference quantity provenance</summary><p>{item.referenceQuantity.formula}</p><p>Package equivalent: {item.referenceQuantity.packageEquivalent}</p><p>{item.referenceQuantity.notice}</p></details>}
    {(item.regimenDetails || item.drugDetail || item.drugStorage) && <div className="preparation-instructions"><h4>Preparation information</h4>{item.regimenDetails && <p><strong>Regimen:</strong> {item.regimenDetails}</p>}{item.drugDetail && <p><strong>Drug detail:</strong> {item.drugDetail}</p>}{item.drugStorage && <p><strong>Storage:</strong> {item.drugStorage}</p>}</div>}
    {task && <div className="preparation-entry">
      <fieldset className="preparation-volume-method">
        <legend>วิธีกำหนด Final / preparation volume</legend>
        <label><input type="radio" name={`preparation-volume-method-${task.id}`} value="solution_plus_drug" checked={volumeMode === "solution_plus_drug"} disabled={readOnly || busy} onChange={() => setVolumeMode("solution_plus_drug")} />ปริมาตรสารละลาย + ปริมาตรยา <small>(ค่าเริ่มต้น)</small></label>
        <label><input type="radio" name={`preparation-volume-method-${task.id}`} value="manual" checked={volumeMode === "manual"} disabled={readOnly || busy} onChange={() => { setManualVolume(volume); setVolumeMode("manual"); }} />ระบุปริมาตรสุดท้ายเอง</label>
        {volumeMode === "solution_plus_drug" && <small className="preparation-volume-equation">{automaticVolume === null ? "คำนวณไม่ได้ เนื่องจากไม่มีปริมาตรสารละลายหรือปริมาตรยา" : `${item.diluentVolumeMl} mL + ${item.calculation.withdrawalVolumeMl} mL = ${automaticVolume} mL`}</small>}
      </fieldset>
      <label>Final / preparation volume (mL)<input type="number" min="0" step="any" value={volume} disabled={readOnly || busy} readOnly={volumeMode === "solution_plus_drug"} onChange={(event) => setManualVolume(event.target.value)} aria-invalid={volumeInvalid}/></label>
      <label className="preparation-notes-field">Preparation notes<textarea rows={3} value={notes} disabled={readOnly || busy} onChange={(event) => setNotes(event.target.value)} placeholder="Optional preparation note" /></label>
      <fieldset className="preparation-final-containers">
        <legend>จำนวนฉลากยา</legend>
        <label>จำนวนฉลากสำหรับยารายการนี้<input type="number" min="1" max="20" step="1" value={containerCount} disabled={readOnly || busy} onChange={(event) => setContainerCount(Math.max(1, Math.min(20, Number.parseInt(event.target.value, 10) || 1)))} /></label>
        <small>ฉลากทุกใบแสดง ordered dose และ final volume เหมือนกัน แตกต่างเฉพาะเลขฉลาก 1/{containerCount} ถึง {containerCount}/{containerCount} และไม่เกี่ยวกับจำนวน vial/ampoule ที่เบิกจากคลัง</small>
      </fieldset>
      {volumeInvalid && <p className="field-error">Enter zero or a positive number.</p>}
      {task.state !== "verified" && <div className="preparation-entry__actions"><button className="button button--secondary" type="button" disabled={busy || volumeInvalid} onClick={() => onSave(task.id, preparationInput())}>Save preparation details</button><button className="button button--primary" type="button" disabled={busy || volumeInvalid || (!task.preparedBy && !assignedPreparer)} onClick={() => onCheck(task.id, preparationInput())}>Check preparation</button></div>}
      {task.state !== "verified" && <p className="preparation-output-unavailable">Final label becomes available after preparation checking.</p>}
      {task.state === "verified" && <p className="preparation-verified">✓ Checked by <strong>{task.verifiedBy?.displayName ?? "Unknown prior actor"}</strong> {task.verifiedAt ? displayDateTime(task.verifiedAt) : ""}. The checked snapshot is now read-only.</p>}
    </div>}
  </article>;
}

export function preparationTaskSequences(items: PreparationWorkspaceItem[]): ReadonlyMap<number, number> {
  const sequences = new Map<number, number>();
  for (const item of items) {
    if (item.task) sequences.set(item.task.id, sequences.size + 1);
  }
  return sequences;
}

function Value({ label, value, strong = false }: { label: string; value: string | null; strong?: boolean }) { return <div><span>{label}</span>{strong ? <strong>{value ?? "Not recorded"}</strong> : <b>{value ?? "Not recorded"}</b>}</div>; }
function joinValue(first: string | null, second: string | null): string | null { return [first, second].filter(Boolean).join(" ") || null; }
function stateLabel(state: PreparationTask["state"]): string { if (state === "prepared") return "Ready to check"; if (state === "verified") return "Checked"; return "Pending check"; }
function statusLabel(status: PreparationWorkspaceItem["referenceQuantity"]["status"]): string { return status === "unsupported" ? "Unsupported configuration" : "Not available"; }

function PreparationCalculationPanel({ item }: { item: PreparationWorkspaceItem }) {
  const calculation = item.calculation;
  const presentation = calculation.presentation;
  const container = presentation.containerLabel ?? "container";
  const status = calculationStatusLabel(calculation.status);
  return <section className={`preparation-calculation preparation-calculation--${calculation.status}`} aria-label="Preparation calculation">
    <div className="preparation-calculation__heading"><div><p className="eyebrow">Preparation reference</p><h4>Quantity &amp; container preview</h4></div><span className="preparation-calculation__status">{status}</span></div>
    <div className="preparation-calculation__grid">
      <CalculationValue label="Presentation" value={presentation.amountPerContainer ? `${presentation.amountPerContainer.value} ${presentation.amountPerContainer.unit} / ${container}` : "Not supported"} detail={presentation.volumePerContainerMl ? `${presentation.volumePerContainerMl} mL / ${container}` : "Volume not available"} />
      <CalculationValue label="Withdrawal" value={calculation.withdrawalVolumeMl ? `${calculation.withdrawalVolumeMl} mL` : "Unsupported"} detail={calculation.concentration ?? "No concentration asserted"} />
      <CalculationValue label="Containers required" value={calculation.containersRequired ? `${calculation.containersRequired} × ${container}` : "Unsupported"} detail="Whole-container ceiling where confirmed" />
      <CalculationValue label="Unused amount" value={calculation.unusedAmount ? `${calculation.unusedAmount.value} ${calculation.unusedAmount.unit}` : "Not available"} detail="Not classified as waste or reusable" />
    </div>
    {calculation.legacyReference.storedQuantity && <div className="legacy-reference-row"><strong>Raw legacy quantity</strong><span>{calculation.legacyReference.storedQuantity}</span><small>{calculation.legacyReference.storedQuantitySemantics}; not silently compared.</small></div>}
    {calculation.warnings.length > 0 && <ul className="preparation-calculation__warnings">{calculation.warnings.map((warning) => <li key={warning.code}>{warning.message}</li>)}</ul>}
    <details className="preparation-calculation__trace"><summary>How OncoFlow calculated this</summary><p><code>{calculation.ruleId}</code> · ruleset <code>{calculation.rulesetVersion}</code></p>{calculation.trace.map((trace) => <div key={`${trace.step}:${trace.expression}`}><strong>{trace.step}</strong><span>{trace.expression}</span><b>{trace.result ?? "No value"}</b><small>{trace.confidence}</small></div>)}</details>
  </section>;
}

function CalculationValue({ label, value, detail }: { label: string; value: string; detail: string }) { return <div><span>{label}</span><strong>{value}</strong><small>{detail}</small></div>; }
function calculationStatusLabel(status: PreparationWorkspaceItem["calculation"]["status"]): string { if (status === "calculated") return "Calculated"; if (status === "partially_calculated") return "Partially supported"; if (status === "unsupported") return "Unsupported"; return "Unavailable"; }
