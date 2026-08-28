import { useEffect, useState } from "react";

import {
  commandError,
  getInventoryItem,
  listInventoryMovements,
  recordInventoryAdjustment,
  recordInventoryManualIssue,
  recordInventoryReceipt,
} from "../api/commands";
import { BuddhistDateTimeInput } from "../components/BuddhistDateInput";
import type { InventoryDetail as InventoryDetailType, InventoryMovement } from "../types/inventory";
import { bangkokLocalDateTimeToUtc, displayDateTime } from "../shared/dateTime";
import { formatQuantity, StockStateBadge } from "./InventoryList";
import {
  emptyInventoryMovementDraft,
  type InventoryFormErrors,
  type InventoryMovementDraft,
  validateInventoryMovement,
} from "./validation";

type DetailState =
  | { kind: "loading" }
  | { kind: "ready"; inventory: InventoryDetailType; movements: InventoryMovement[] }
  | { kind: "error"; message: string };

export function InventoryDetail({ drugId, onBack }: { drugId: number; onBack: () => void }) {
  const [reloadKey, setReloadKey] = useState(0);
  const [state, setState] = useState<DetailState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    void Promise.all([getInventoryItem(drugId), listInventoryMovements(drugId)])
      .then(([inventory, history]) => active && setState({ kind: "ready", inventory, movements: history.items }))
      .catch((error: unknown) => active && setState({ kind: "error", message: commandError(error).message ?? "Unable to load inventory details." }));
    return () => { active = false; };
  }, [drugId, reloadKey]);

  if (state.kind === "loading") return <section className="workspace inventory-workspace"><button className="back-button" type="button" onClick={onBack}>← Inventory</button><div className="detail-loading" aria-label="Loading inventory details"><div className="skeleton-block skeleton-block--hero"/><div className="detail-grid"><div className="skeleton-block"/><div className="skeleton-block"/></div></div></section>;
  if (state.kind === "error") return <section className="workspace inventory-workspace"><button className="back-button" type="button" onClick={onBack}>← Inventory</button><div className="state-panel state-panel--error surface" role="alert"><span className="state-icon">!</span><h1>Inventory details unavailable</h1><p>{state.message}</p><button className="button button--secondary" type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button></div></section>;

  async function record(draft: InventoryMovementDraft) {
    if (state.kind !== "ready") return;
    const common = {
      drugId,
      quantity: Number(draft.quantity),
      occurredAt: draft.occurredAt ? bangkokLocalDateTimeToUtc(draft.occurredAt) : null,
      reference: draft.reference.trim() || null,
    };
    if (draft.operation === "receipt") await recordInventoryReceipt({ ...common, note: draft.note.trim() || null });
    else if (draft.operation === "adjustment") await recordInventoryAdjustment({ ...common, direction: draft.direction, note: draft.note.trim() });
    else await recordInventoryManualIssue({ ...common, note: draft.note.trim() });
    const [inventory, history] = await Promise.all([getInventoryItem(drugId), listInventoryMovements(drugId)]);
    setState({ kind: "ready", inventory, movements: history.items });
  }

  return <InventoryDetailView inventory={state.inventory} movements={state.movements} onBack={onBack} onRecord={record}/>;
}

export function InventoryDetailView({ inventory, movements, onBack, onRecord }: { inventory: InventoryDetailType; movements: InventoryMovement[]; onBack: () => void; onRecord: (draft: InventoryMovementDraft) => Promise<void> | void }) {
  return <section className="workspace inventory-workspace" aria-labelledby="inventory-detail-heading">
    <button className="back-button" type="button" onClick={onBack}>← Inventory</button>
    <header className="surface inventory-hero"><div><p className="eyebrow">Inventory ledger</p><h1 id="inventory-detail-heading">{inventory.drugName}</h1><p>{inventory.drugCode}{inventory.package ? ` · ${inventory.package}` : ""}</p></div><div className="inventory-hero__balance"><span>Current stock</span><strong className={inventory.currentStock !== null && inventory.currentStock < 0 ? "inventory-negative" : ""}>{formatQuantity(inventory.currentStock)}</strong><StockStateBadge state={inventory.stockState}/></div></header>
    <div className="inventory-boundary-note"><strong>Quantity semantics remain explicit.</strong> {inventory.legacyDrugUnit ? `Legacy drug unit: ${inventory.legacyDrugUnit}. ` : "No legacy drug unit is recorded. "}OncoFlow does not infer vial, ampoule, mg, or mL conversions. A verified preparation posts stock only from a fully supported container calculation.</div>
    <div className="inventory-detail-grid">
      <section className="surface inventory-summary-card"><div className="section-heading"><div><p className="eyebrow">Stock configuration</p><h2>Current position</h2></div></div><dl className="inventory-summary-grid"><InventoryValue label="Current stock" value={formatQuantity(inventory.currentStock)} alert={inventory.currentStock !== null && inventory.currentStock < 0}/><InventoryValue label="Minimum" value={formatQuantity(inventory.minimumStock)}/><InventoryValue label="Maximum" value={formatQuantity(inventory.maximumStock)}/><InventoryValue label="Tracking" value={inventory.trackingEnabled ? "Enabled" : "Disabled"}/><InventoryValue label="Legacy Inv snapshot" value={formatQuantity(inventory.legacyInventorySnapshot)}/><InventoryValue label="Legacy InvCut" value={inventory.legacyInventoryCutoff === null ? "Not recorded" : inventory.legacyInventoryCutoff ? "Enabled (display only)" : "Disabled (display only)"}/></dl><p className="inventory-provenance">The legacy snapshot is preserved unchanged. Current stock is the sum of append-only ledger movements. {inventory.legacyInventoryEventCount} migrated InvIN record{inventory.legacyInventoryEventCount === 1 ? "" : "s"} remain compatibility provenance and are not counted again.</p></section>
      <InventoryMovementForm onSubmit={onRecord}/>
    </div>
    <section className="surface inventory-history"><div className="section-heading"><div><p className="eyebrow">Append-only history</p><h2>Inventory movements</h2></div><span>{movements.length} movement{movements.length === 1 ? "" : "s"}</span></div>{movements.length === 0 ? <div className="state-panel"><span className="state-icon">≡</span><h3>No ledger movements</h3><p>This drug has no known opening balance or later movement.</p></div> : <InventoryMovementTable movements={movements}/>}</section>
  </section>;
}

function InventoryValue({ label, value, alert = false }: { label: string; value: string; alert?: boolean }) {
  return <div><dt>{label}</dt><dd className={alert ? "inventory-negative" : ""}>{value}</dd></div>;
}

export function InventoryMovementForm({ onSubmit }: { onSubmit: (draft: InventoryMovementDraft) => Promise<void> | void }) {
  const [draft, setDraft] = useState(emptyInventoryMovementDraft);
  const [errors, setErrors] = useState<InventoryFormErrors>({});
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  function update<K extends keyof InventoryMovementDraft>(key: K, value: InventoryMovementDraft[K]) { setDraft((current) => ({ ...current, [key]: value })); setErrors((current) => ({ ...current, [key]: undefined })); setSaved(false); }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateInventoryMovement(draft);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;
    setBusy(true); setFailure(null); setSaved(false);
    try { await onSubmit(draft); setDraft(emptyInventoryMovementDraft()); setSaved(true); }
    catch (error) { const detail = commandError(error); setFailure(detail.message ?? "Inventory movement could not be recorded."); if (detail.field) setErrors((current) => ({ ...current, [detail.field as keyof InventoryFormErrors]: detail.message })); }
    finally { setBusy(false); }
  }
  return <section className="surface inventory-entry"><div className="section-heading"><div><p className="eyebrow">Authenticated movement</p><h2>Record stock change</h2></div></div><form onSubmit={(event) => void submit(event)}>
    <label>Movement<select value={draft.operation} onChange={(event) => update("operation", event.target.value as InventoryMovementDraft["operation"])}><option value="receipt">Receipt</option><option value="adjustment">Adjustment</option><option value="manualIssue">Manual issue</option></select></label>
    {draft.operation === "adjustment" && <label>Direction<select value={draft.direction} onChange={(event) => update("direction", event.target.value as InventoryMovementDraft["direction"])}><option value="increase">Increase</option><option value="decrease">Decrease</option></select></label>}
    <label>Quantity {draft.operation === "manualIssue" && <small>Whole units only</small>}<input type="number" min={draft.operation === "manualIssue" ? "1" : "0"} step={draft.operation === "manualIssue" ? "1" : "any"} value={draft.quantity} onChange={(event) => update("quantity", event.target.value)} placeholder="0"/>{errors.quantity && <span className="field-error">{errors.quantity}</span>}</label>
    <label>Date and time <small>Blank uses current Bangkok time</small><BuddhistDateTimeInput value={draft.occurredAt} onChange={(value) => update("occurredAt", value)} invalid={Boolean(errors.occurredAt)}/>{errors.occurredAt && <span className="field-error">{errors.occurredAt}</span>}</label>
    <label className="is-wide">Reference <small>Optional local document/reference</small><input value={draft.reference} onChange={(event) => update("reference", event.target.value)} maxLength={120}/>{errors.reference && <span className="field-error">{errors.reference}</span>}</label>
    <label className="is-wide">{draft.operation === "receipt" ? "Note (optional)" : "Reason (required)"}<textarea rows={3} value={draft.note} onChange={(event) => update("note", event.target.value)} maxLength={1000}/>{errors.note && <span className="field-error">{errors.note}</span>}</label>
    {draft.operation === "manualIssue" && <p className="inventory-shortage-note">The issue will commit even if the resulting balance is negative. A shortage is advisory and will not alter clinical records.</p>}
    {failure && <p className="auth-error is-wide" role="alert">{failure}</p>}{saved && <p className="auth-success is-wide" role="status">Inventory movement recorded with the authenticated actor.</p>}
    <div className="inventory-entry__actions"><button className="button button--primary" type="submit" disabled={busy}>{busy ? "Recording…" : "Record movement"}</button></div>
  </form></section>;
}

export function InventoryMovementTable({ movements }: { movements: InventoryMovement[] }) {
  return <div className="table-scroll"><table className="patient-table inventory-movement-table"><thead><tr><th>Ledger</th><th>Occurred</th><th>Movement</th><th>Change</th><th>Balance after</th><th>Actor</th><th>Reference / note</th></tr></thead><tbody>{movements.map((movement) => <tr key={movement.id}><td>#{movement.id}</td><td>{movement.occurredAt ? displayDateTime(movement.occurredAt) : <span className="muted">Legacy time unknown</span>}</td><td>{movementLabel(movement.movementType)}</td><td className={movement.quantityDelta < 0 ? "inventory-negative" : "inventory-positive"}>{movement.quantityDelta > 0 ? "+" : ""}{formatQuantity(movement.quantityDelta)}</td><td className={movement.resultingBalance < 0 ? "inventory-negative" : ""}>{formatQuantity(movement.resultingBalance)}</td><td>{movement.actorDisplayName ?? <span className="muted">Legacy migration</span>}</td><td>{movement.referenceId && <strong>{movement.referenceId}</strong>}{movement.note && <small className="row-subtitle">{movement.note}</small>}{!movement.referenceId && !movement.note && <span className="muted">—</span>}</td></tr>)}</tbody></table></div>;
}

export function movementLabel(type: InventoryMovement["movementType"]): string {
  return ({ opening_balance: "Opening balance", receipt: "Receipt", manual_issue: "Manual issue", preparation_issue: "Preparation issue", adjustment_increase: "Adjustment increase", adjustment_decrease: "Adjustment decrease" })[type];
}
