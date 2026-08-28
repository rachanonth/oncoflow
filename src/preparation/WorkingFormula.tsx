import { useEffect, useState } from "react";

import { checkPreparationTasks, commandError, getApplicationSettings, initializePreparation, printOrderPreparationLabels } from "../api/commands";
import { loadLabelPrinterConfig } from "../hardware/printerSettings";
import { formatWorkingFormulaAppName } from "../settings/GeneralSettings";
import { displayDateTime, displayLocalDateTime } from "../shared/dateTime";
import type { PreparationQueueItem, PreparationTask, PreparationWorkspace, PreparationWorkspaceItem } from "../types/preparation";
import { loadWorkingFormulaArrangement, saveWorkingFormulaArrangement, type WorkingFormulaArrangement } from "./preferences";

type FormulaState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; workspaces: PreparationWorkspace[] };

interface WorkingFormulaDrugGroup {
  drugId: number;
  drugName: string;
  entries: Array<{
    workspace: PreparationWorkspace;
    item: PreparationWorkspaceItem;
    sourceKind: PreparationQueueItem["sourceKind"];
  }>;
}

export function WorkingFormulaDialog({ date, items, onClose }: { date: string; items: PreparationQueueItem[]; onClose: () => void }) {
  const [state, setState] = useState<FormulaState>({ kind: "loading" });
  const [busy, setBusy] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [batchMessage, setBatchMessage] = useState<string | null>(null);
  const [hospitalName, setHospitalName] = useState<string | null>(null);
  const [arrangement, setArrangement] = useState<WorkingFormulaArrangement>(() => loadWorkingFormulaArrangement());

  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const settings = await getApplicationSettings();
        const workspaces: PreparationWorkspace[] = [];
        for (const item of items) {
          workspaces.push(await initializePreparation(item.orderId, item.preparationDate));
        }
        if (active) {
          setHospitalName(settings.hospitalName);
          setState({ kind: "ready", workspaces });
        }
      } catch (error) {
        if (active) setState({ kind: "error", message: commandError(error).message ?? "Unable to prepare the working formula." });
      }
    }
    void load();
    return () => { active = false; };
  }, [date, items]);

  const uncheckedTasks = state.kind === "ready"
    ? state.workspaces.flatMap((workspace) => workspace.items.flatMap((item) => item.task && item.task.state !== "verified" ? [item.task] : []))
    : [];
  const missingPreparerCount = state.kind === "ready"
    ? state.workspaces.reduce((count, workspace) => count + workspace.items.filter((item) => item.task && item.task.state !== "verified" && !item.task.preparedBy && !workspace.assignedPreparer).length, 0)
    : 0;
  const printableGroups = state.kind === "ready"
    ? state.workspaces.map((workspace) => ({
      orderId: workspace.orderId,
      taskIds: workspace.items.flatMap((item) => item.task?.state === "verified" ? [item.task.id] : []),
    })).filter((group) => group.taskIds.length > 0)
    : [];
  const printableLabelCount = printableGroups.reduce((count, group) => count + group.taskIds.length, 0);

  async function batchCheck() {
    if (state.kind !== "ready" || uncheckedTasks.length === 0) return;
    const confirmed = window.confirm(`Check all ${uncheckedTasks.length} unchecked preparation task(s) shown for ${date}? This will post the normal inventory issue for every supported task.`);
    if (!confirmed) return;
    setBusy(true);
    setOperationError(null);
    setBatchMessage(null);
    try {
      const checked = await checkPreparationTasks(uncheckedTasks.map((task) => task.id));
      const tasksById = new Map<number, PreparationTask>(checked.map((task) => [task.id, task]));
      setState((current) => current.kind === "ready" ? {
        kind: "ready",
        workspaces: current.workspaces.map((workspace) => ({
          ...workspace,
          items: workspace.items.map((item) => item.task && tasksById.has(item.task.id) ? { ...item, task: tasksById.get(item.task.id)! } : item),
        })),
      } : current);
      setBatchMessage(`${checked.length} preparation task(s) checked successfully.`);
    } catch (error) {
      const message = commandError(error).message ?? "Unable to batch check preparation tasks.";
      setOperationError(`${message} No tasks in this batch were changed.`);
    } finally {
      setBusy(false);
    }
  }

  async function printAllLabels() {
    if (printableLabelCount === 0) return;
    const printerConfig = loadLabelPrinterConfig();
    if (!printerConfig) {
      setOperationError("Choose a Windows label-printer queue in Settings → Hardware before printing.");
      return;
    }
    const confirmed = window.confirm(`Print ${printableLabelCount} checked label(s) from ${printableGroups.length} order(s) in the current view?`);
    if (!confirmed) return;
    setBusy(true);
    setOperationError(null);
    setBatchMessage(null);
    let submitted = 0;
    const jobIds: number[] = [];
    try {
      for (const group of printableGroups) {
        const result = await printOrderPreparationLabels(group.orderId, group.taskIds, printerConfig);
        submitted += result.outputs.length;
        jobIds.push(result.job.windowsJobId);
      }
      setBatchMessage(`${submitted} label(s) sent to Windows in ${jobIds.length} job(s): ${jobIds.map((id) => `#${id}`).join(", ")}. Confirm the physical labels printed correctly.`);
    } catch (error) {
      const message = commandError(error).message ?? "Unable to print all labels.";
      setOperationError(submitted > 0 ? `${message} ${submitted} label(s) were already submitted to Windows; check the printer queue before retrying.` : message);
    } finally {
      setBusy(false);
    }
  }

  function selectArrangement(next: WorkingFormulaArrangement) {
    setArrangement(next);
    saveWorkingFormulaArrangement(next);
  }

  return <div className="working-formula-backdrop" role="presentation">
    <section className="working-formula-dialog" role="dialog" aria-modal="true" aria-labelledby="working-formula-heading">
      <header className="working-formula-toolbar">
        <div className="working-formula-toolbar__heading">
          <div><p className="eyebrow">Daily preparation output</p><h1 id="working-formula-heading">Working formula · {date}</h1>{state.kind === "ready" && <p>{state.workspaces.length} order(s) · {uncheckedTasks.length} awaiting check</p>}</div>
          <button className="working-formula-close" type="button" aria-label="Close working formula" title="Close" onClick={onClose}>×</button>
        </div>
        <div className="working-formula-toolbar__controls">
          <div className="working-formula-arrangement"><span>Sort by</span><div className="order-quick-filter" role="group" aria-label="Working formula sort"><button className={arrangement === "order" ? "is-active" : ""} type="button" aria-pressed={arrangement === "order"} onClick={() => selectArrangement("order")}>Order</button><button className={arrangement === "drug" ? "is-active" : ""} type="button" aria-pressed={arrangement === "drug"} onClick={() => selectArrangement("drug")}>Drug</button></div></div>
          <div className="working-formula-toolbar__actions"><button className="button button--secondary button--compact" type="button" disabled={state.kind !== "ready" || state.workspaces.length === 0} onClick={() => window.print()}>Print working formula</button><button className="button button--secondary button--compact" type="button" disabled={busy || printableLabelCount === 0} onClick={() => void printAllLabels()}>Print all labels ({printableLabelCount})</button><button className="button button--primary button--compact" type="button" disabled={busy || uncheckedTasks.length === 0 || missingPreparerCount > 0} onClick={() => void batchCheck()}>{busy ? "Checking…" : `Batch check all (${uncheckedTasks.length})`}</button></div>
        </div>
      </header>
      {operationError && <div className="working-formula-feedback auth-error" role="alert">{operationError}</div>}
      {batchMessage && <div className="working-formula-feedback auth-success" role="status">{batchMessage}</div>}
      {missingPreparerCount > 0 && <div className="working-formula-feedback hardware-warning" role="status">Assign an active preparation pharmacist to {missingPreparerCount} task(s) before batch checking this view.</div>}
      <div className="working-formula-print-root">
        <WorkingFormulaPrintHeader date={date} hospitalName={hospitalName} />
        {state.kind === "loading" && <div className="state-panel" aria-busy="true">Preparing daily working formula…</div>}
        {state.kind === "error" && <div className="state-panel state-panel--error" role="alert"><h2>Working formula unavailable</h2><p>{state.message}</p></div>}
        {state.kind === "ready" && state.workspaces.length === 0 && <div className="state-panel"><h2>No preparation orders for this date</h2><p>Choose another treatment date.</p></div>}
        {state.kind === "ready" && arrangement === "order" && state.workspaces.map((workspace) => <WorkingFormulaOrder key={workspace.orderId} workspace={workspace} sourceKind={items.find((item) => item.orderId === workspace.orderId)?.sourceKind ?? "same_day"} />)}
        {state.kind === "ready" && arrangement === "drug" && <WorkingFormulaDrugGroups workspaces={state.workspaces} queueItems={items} />}
      </div>
    </section>
  </div>;
}

export function WorkingFormulaPrintHeader({ date, hospitalName }: { date: string; hospitalName: string | null }) {
  return <header><strong>{formatWorkingFormulaAppName(hospitalName)}</strong><h1>Chemotherapy preparation working formula</h1><p>Treatment date {date} · Printed {displayDateTime(new Date().toISOString())}</p></header>;
}

export function WorkingFormulaOrder({ workspace, sourceKind }: { workspace: PreparationWorkspace; sourceKind: PreparationQueueItem["sourceKind"] }) {
  return <article className="working-formula-order">
    <header><div><h2>HN {workspace.patientHn} - {workspace.patientName || "Name not recorded"}</h2><p className="working-formula-order__number">Order {workspace.orderCode}</p></div><div>{workspace.regimenName && <strong>{workspace.regimenName}</strong>}<span>Ward: {workspace.wardName ?? "Not recorded"}</span><span>Order time: {displayLocalDateTime(workspace.treatmentTime)}</span><span>Prepared by: {workspace.assignedPreparer?.displayName ?? "Not assigned"}</span></div></header>
    <table><thead><tr><th>Seq.</th><th>Drug / ordered dose</th><th>Presentation / quantity</th><th>Preparation</th><th>Instructions</th></tr></thead><tbody>{workspace.items.map((item) => <tr key={item.orderItemId}><td>{item.sequenceNo ?? "—"}</td><td><strong>{item.drugName}</strong><span>{join(item.orderedDoseText, item.doseUnitText)}</span><span className={`working-formula-order-type working-formula-order-type--${sourceKind}`}>{sourceKind === "continuing" ? "Continuing order" : "Today order"}</span></td><td><span>{item.calculation.presentation.amountPerContainer ? `${item.calculation.presentation.amountPerContainer.value} ${item.calculation.presentation.amountPerContainer.unit} / ${item.calculation.presentation.containerLabel ?? "container"}` : "Unsupported presentation"}</span><span>Withdrawal: {item.calculation.withdrawalVolumeMl ? `${item.calculation.withdrawalVolumeMl} mL` : "Unsupported"}</span><span>Containers: {item.calculation.containersRequired ?? "Unsupported"}</span></td><td><span>{join(item.diluentName, item.diluentVolumeMl === null ? null : `${item.diluentVolumeMl} mL`)}</span><span>{join(item.routeName, item.rateText)}</span><span>Final volume: {item.task?.preparationVolumeMl === null || item.task?.preparationVolumeMl === undefined ? "Not recorded" : `${item.task.preparationVolumeMl} mL`}</span></td><td>{item.regimenDetails || item.drugDetail || item.task?.preparationNotes || "—"}</td></tr>)}</tbody></table>
  </article>;
}

export function buildWorkingFormulaDrugGroups(workspaces: PreparationWorkspace[], queueItems: PreparationQueueItem[]): WorkingFormulaDrugGroup[] {
  const sourceKinds = new Map(queueItems.map((item) => [item.orderId, item.sourceKind]));
  const groups = new Map<number, WorkingFormulaDrugGroup>();

  for (const workspace of workspaces) {
    for (const item of workspace.items) {
      const group = groups.get(item.drugId) ?? { drugId: item.drugId, drugName: item.drugName, entries: [] };
      group.entries.push({ workspace, item, sourceKind: sourceKinds.get(workspace.orderId) ?? "same_day" });
      groups.set(item.drugId, group);
    }
  }

  return [...groups.values()].sort((left, right) => left.drugName.localeCompare(right.drugName, ["en", "th"], { numeric: true, sensitivity: "base" }) || left.drugId - right.drugId);
}

export function WorkingFormulaDrugGroups({ workspaces, queueItems }: { workspaces: PreparationWorkspace[]; queueItems: PreparationQueueItem[] }) {
  const groups = buildWorkingFormulaDrugGroups(workspaces, queueItems);
  return <>{groups.map((group) => <article className="working-formula-drug-group" key={group.drugId}>
    <header><div><p className="eyebrow">Drug</p><h2>{group.drugName}</h2></div><span>{group.entries.length} preparation task(s)</span></header>
    <table><thead><tr><th>Seq.</th><th>Patient / order</th><th>Ordered dose</th><th>Presentation / quantity</th><th>Preparation</th><th>Instructions</th></tr></thead><tbody>{group.entries.map(({ workspace, item, sourceKind }) => <tr key={`${workspace.orderId}-${item.orderItemId}`}><td>{item.sequenceNo ?? "—"}</td><td><strong>HN {workspace.patientHn} - {workspace.patientName || "Name not recorded"}</strong><span>Order {workspace.orderCode}</span><span>Ward: {workspace.wardName ?? "Not recorded"}</span><span>Order time: {displayLocalDateTime(workspace.treatmentTime)}</span>{workspace.regimenName && <span>Regimen: {workspace.regimenName}</span>}<span>Prepared by: {workspace.assignedPreparer?.displayName ?? "Not assigned"}</span></td><td><strong>{join(item.orderedDoseText, item.doseUnitText)}</strong><span className={`working-formula-order-type working-formula-order-type--${sourceKind}`}>{sourceKind === "continuing" ? "Continuing order" : "Today order"}</span></td><td><span>{item.calculation.presentation.amountPerContainer ? `${item.calculation.presentation.amountPerContainer.value} ${item.calculation.presentation.amountPerContainer.unit} / ${item.calculation.presentation.containerLabel ?? "container"}` : "Unsupported presentation"}</span><span>Withdrawal: {item.calculation.withdrawalVolumeMl ? `${item.calculation.withdrawalVolumeMl} mL` : "Unsupported"}</span><span>Containers: {item.calculation.containersRequired ?? "Unsupported"}</span></td><td><span>{join(item.diluentName, item.diluentVolumeMl === null ? null : `${item.diluentVolumeMl} mL`)}</span><span>{join(item.routeName, item.rateText)}</span><span>Final volume: {item.task?.preparationVolumeMl === null || item.task?.preparationVolumeMl === undefined ? "Not recorded" : `${item.task.preparationVolumeMl} mL`}</span></td><td>{item.regimenDetails || item.drugDetail || item.task?.preparationNotes || "—"}</td></tr>)}</tbody></table>
  </article>)}</>;
}

function join(...values: Array<string | null | undefined>): string {
  return values.map((value) => value?.trim()).filter(Boolean).join(" · ") || "Not recorded";
}
