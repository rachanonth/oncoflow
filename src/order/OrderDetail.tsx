import { useCallback, useEffect, useState } from "react";
import { addOrderItem, commandError, getOrder, getOrderLookups, recordOrderNoShow, removeOrderItem, reorderOrderItems, rescheduleOrder, updateOrderItem, updateOrderWeight } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import type { CumulativeDoseSummary, OrderDetail as Detail, OrderItemDetail, OrderLookups } from "../types/order";
import { currentBangkokDateTimeValue, displayDate, displayTime } from "../shared/dateTime";
import { displayDateTime } from "./OrderList";
import { OrderItemEditor } from "./OrderItemEditor";
import { PatientOrderHistory } from "./PatientOrderHistory";

type Editor = { kind: "add" } | { kind: "edit"; item: OrderItemDetail } | null;

export function OrderDetail({ orderId, backLabel = "Orders", onBack, onEdit, onOpenOrder, onOpenDrug }: { orderId: number; backLabel?: string; onBack: () => void; onEdit: (order: Detail) => void; onOpenOrder?: (orderId: number) => void; onOpenDrug?: (drugId: number) => void }) {
  const [order, setOrder] = useState<Detail | null>(null); const [lookups, setLookups] = useState<OrderLookups | null>(null); const [error, setError] = useState<string | null>(null); const [loading, setLoading] = useState(true); const [mutating, setMutating] = useState(false); const [editor, setEditor] = useState<Editor>(null);
  const [measurementSaving, setMeasurementSaving] = useState(false);
  const [statusBusy, setStatusBusy] = useState(false);
  const load = useCallback(async () => { setLoading(true); setError(null); try { const [detail, options] = await Promise.all([getOrder(orderId), getOrderLookups()]); setOrder(detail); setLookups(options); } catch (value) { setError(commandError(value).message ?? "Unable to load order."); } finally { setLoading(false); } }, [orderId]);
  useEffect(() => { void load(); }, [load]);
  async function mutate(operation: () => Promise<Detail>) { setMutating(true); setError(null); try { const result = await operation(); setOrder(result); setEditor(null); } catch (value) { setError(commandError(value).message ?? "Unable to update order."); throw value; } finally { setMutating(false); } }
  async function saveOrderWeight(weightKg: number | null) { if (!order) return; setMeasurementSaving(true); setError(null); try { setOrder(await updateOrderWeight(order.id, weightKg)); } catch (value) { setError(commandError(value).message ?? "Unable to update order weight."); throw value; } finally { setMeasurementSaving(false); } }
  async function updateStatus(operation: () => Promise<Detail>) { setStatusBusy(true); setError(null); try { setOrder(await operation()); } catch (value) { setError(commandError(value).message ?? "Unable to update order attendance status."); throw value; } finally { setStatusBusy(false); } }
  async function move(item: OrderItemDetail, direction: -1 | 1) { if (!order) return; const index = order.items.findIndex((value) => value.id === item.id); const target = index + direction; if (target < 0 || target >= order.items.length) return; const ids = order.items.map((value) => value.id); [ids[index], ids[target]] = [ids[target], ids[index]]; await mutate(() => reorderOrderItems(order.id, { itemIds: ids })); }
  if (loading) return <section className="workspace"><button className="back-button" type="button" onClick={onBack}>← {backLabel}</button><div className="detail-loading"><div className="skeleton-block skeleton-block--hero"/><div className="skeleton-block"/></div></section>;
  if (!order || !lookups) return <section className="workspace"><button className="back-button" type="button" onClick={onBack}>← {backLabel}</button><div className="state-panel state-panel--error surface" role="alert"><span className="state-icon">!</span><h1>Order unavailable</h1><p>{error}</p><button className="button button--secondary" type="button" onClick={() => void load()}>Try again</button></div></section>;
  return <section className="workspace regimen-workspace order-workspace" aria-labelledby="order-heading"><button className="back-button" type="button" onClick={onBack}>← {backLabel}</button>
    <OrderDetailHeader order={order} onEdit={onEdit} />
    {error && <div className="form-error-summary" role="alert">{error}</div>}
    <OrderSummary order={order} />
    <OrderMeasurements order={order} saving={measurementSaving} onSave={saveOrderWeight} />
    <div className="regimen-section-heading"><div><p className="eyebrow">Stored drugs</p><h2>Order drugs</h2></div>{order.editable && !editor && <button className="button button--primary button--compact" type="button" onClick={() => setEditor({ kind: "add" })}>Add drug</button>}</div>
    {editor && <OrderItemEditor item={editor.kind === "edit" ? editor.item : undefined} lookups={lookups} onCancel={() => setEditor(null)} onSave={async (input) => { await mutate(() => editor.kind === "edit" ? updateOrderItem(order.id, editor.item.id, input) : addOrderItem(order.id, input)); }} />}
    <div className="regimen-group">{order.items.length === 0 ? <div className="regimen-empty">No drugs recorded.</div> : <OrderDrugsTable order={order} mutating={mutating} onMove={move} onEdit={(item) => setEditor({ kind: "edit", item })} onRemove={(item) => { if (window.confirm("Remove this drug from the order?")) void mutate(() => removeOrderItem(order.id, item.id)); }} />}</div>
    <PatientOrderHistory className="order-history" patientId={order.patientId} currentOrderId={order.id} onOpen={onOpenOrder} />
    <OrderCumulativeDose items={order.cumulativeDoses} onOpenDrug={onOpenDrug} />
    <OrderStatusPanel order={order} busy={statusBusy} onNoShow={(date) => updateStatus(() => recordOrderNoShow(order.id, date))} onReschedule={(missedDate, newDate) => updateStatus(() => rescheduleOrder(order.id, missedDate, newDate))} />
  </section>;
}

export function OrderDetailHeader({ order, onEdit }: { order: Detail; onEdit: (order: Detail) => void }) {
  return <header className="patient-hero order-detail-hero"><div className="patient-avatar order-avatar" aria-hidden="true">Rx</div><div className="patient-hero__identity"><div className="order-detail-hero__primary"><span className="order-detail-hero__hn"><b>HN</b> {order.patientHn}</span><h1 id="order-heading">{order.patientName || "Name not recorded"}</h1></div><p className="order-detail-hero__reference">Order: <strong>{order.orderId}</strong>{!order.editable && <span>Read-only</span>}<span className={`order-workflow-badge order-workflow-badge--${order.workflowStatus}`}>{workflowStatusLabel(order.workflowStatus)}</span></p></div>{order.editable && <button className="order-edit-button" type="button" aria-label={`Edit order ${order.orderId}`} title="Edit order" onClick={() => onEdit(order)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4.2L19 9.2a2.1 2.1 0 0 0 0-3L17.8 5a2.1 2.1 0 0 0-3 0L4 15.8V20Z"/><path d="m13.5 6.5 4 4"/></svg></button>}</header>;
}

export function OrderSummary({ order }: { order: Detail }) {
  return <Section title="Order details" className="order-summary"><Field label="Date / time" value={displayDateTime(order.orderTime)}/><Field label="Regimen" value={order.regimenName}/><Field label="Preparation pharmacist" value={order.assignedPreparerName}/><Field label="Doctor" value={order.doctorName}/><Field label="Ward" value={order.wardName}/><Field label="Notes" value={order.note} wide/></Section>;
}

export function OrderStatusPanel({ order, busy, onNoShow, onReschedule }: { order: Detail; busy: boolean; onNoShow: (date: string) => Promise<void>; onReschedule: (missedDate: string, newDate: string) => Promise<void> }) {
  const [showNoShow, setShowNoShow] = useState(false);
  const [missedDate, setMissedDate] = useState(() => currentBangkokDateTimeValue().slice(0, 10));
  const [newDate, setNewDate] = useState("");
  const outstandingNoShow = order.statusEvents.find((event) => event.eventType === "no_show" && !order.statusEvents.some((candidate) => candidate.eventType === "rescheduled" && candidate.relatedDate === event.effectiveDate));
  const outstandingNoShowDate = outstandingNoShow?.effectiveDate;
  useEffect(() => {
    if (outstandingNoShowDate) setMissedDate(outstandingNoShowDate);
    if (order.workflowStatus === "active") { setShowNoShow(false); setNewDate(""); }
  }, [order.workflowStatus, outstandingNoShowDate]);
  if (!order.editable || order.workflowStatus === "legacy") return null;
  async function submitNoShow(event: React.FormEvent) { event.preventDefault(); if (!missedDate) return; try { await onNoShow(missedDate); } catch { /* Page-level error is already shown. */ } }
  async function submitReschedule(event: React.FormEvent) { event.preventDefault(); if (!outstandingNoShow || !newDate) return; try { await onReschedule(outstandingNoShow.effectiveDate, newDate); } catch { /* Page-level error is already shown. */ } }
  return <section className={`detail-section surface order-attendance order-attendance--${order.workflowStatus}`} aria-labelledby="order-attendance-heading">
    <div className="order-attendance__heading"><div><p className="eyebrow">Attendance exception</p><h2 id="order-attendance-heading">{order.workflowStatus === "on_hold" ? "Order on hold" : "Appointment status"}</h2></div><span className={`order-workflow-badge order-workflow-badge--${order.workflowStatus}`}>{workflowStatusLabel(order.workflowStatus)}</span></div>
    {order.workflowStatus === "active" && !showNoShow && <div className="order-attendance__normal"><button className="button button--secondary" type="button" disabled={busy} onClick={() => setShowNoShow(true)}>ผู้ป่วยไม่มาตามนัด</button></div>}
    {order.workflowStatus === "active" && showNoShow && <form className="order-attendance__form" onSubmit={submitNoShow}><label>Missed appointment date<BuddhistDateInput value={missedDate} onChange={setMissedDate} /></label><p>This records a no-show and removes the order from preparation until a new date is assigned. Original item dates remain unchanged.</p><div><button className="button button--secondary" type="button" disabled={busy} onClick={() => setShowNoShow(false)}>Cancel</button><button className="button button--primary" type="submit" disabled={busy || !missedDate}>{busy ? "Recording…" : "Record no-show and hold"}</button></div></form>}
    {order.workflowStatus === "on_hold" && outstandingNoShow && <form className="order-attendance__form" onSubmit={submitReschedule}><p>Missed appointment: <strong>{displayDate(outstandingNoShow.effectiveDate)}</strong>. The original order and drug start/stop dates have not been changed.</p><label>New appointment date<BuddhistDateInput value={newDate} onChange={setNewDate} /></label><div><button className="button button--primary" type="submit" disabled={busy || !newDate || newDate === outstandingNoShow.effectiveDate}>{busy ? "Saving…" : "Continue order on new date"}</button></div></form>}
    {order.statusEvents.length > 0 && <details className="order-attendance__history"><summary>Attendance history ({order.statusEvents.length})</summary><ol>{order.statusEvents.map((event) => <li key={event.id}><strong>{event.eventType === "no_show" ? "No show" : "Rescheduled"}</strong><span>{event.eventType === "rescheduled" && event.relatedDate ? `${displayDate(event.relatedDate)} → ` : ""}{displayDate(event.effectiveDate)}</span><small>{event.actorDisplayName} · {displayDateTime(event.occurredAt)}</small></li>)}</ol></details>}
  </section>;
}

export function OrderDrugsTable({ order, mutating, onMove, onEdit, onRemove }: { order: Detail; mutating: boolean; onMove: (item: OrderItemDetail, direction: -1 | 1) => void; onEdit: (item: OrderItemDetail) => void; onRemove: (item: OrderItemDetail) => void }) {
  return <div className="table-scroll"><table className="regimen-item-table order-item-table"><thead><tr><th>Seq.</th><th>Drug / dose</th><th>Preparation</th><th>Route / rate</th><th>Schedule</th>{order.editable && <th>Actions</th>}</tr></thead><tbody>{order.items.map((item, index) => <tr key={item.id}>
    <td><strong>{item.orderingNo ?? "—"}</strong>{item.regimenItemGroup && <small>Group {item.regimenItemGroup}</small>}</td>
    <td><strong>{item.drugName}</strong>{item.doseText ? <span className="order-dose">{item.doseText}<small className="order-dose__unit">{item.regimenUnitText?.trim() || "mg"}</small></span> : <span className="muted">No dose recorded</span>}</td>
    <td><span>{item.diluentName ?? "No diluent"}</span><small>{item.diluentVolumeMl !== null ? `${item.diluentVolumeMl} mL` : item.diluentId !== null ? "Uses master volume" : "No volume recorded"}</small></td>
    <td><span>{item.routeName ?? item.regimenRouteText ?? "No route"}</span><small>{item.rate ?? "No rate"}</small></td>
    <td><span>{range(item.startDate, item.stopDate)}</span><small>{displayTime(item.scheduleTime, "No time")}</small>{item.regimenStartDay !== null && <small>Raw regimen start day: {item.regimenStartDay}</small>}{item.regimenDuration && <small>Raw duration: {item.regimenDuration}</small>}</td>
    {order.editable && <td><div className="item-actions"><button type="button" disabled={mutating || index === 0} onClick={() => onMove(item, -1)}>↑</button><button type="button" disabled={mutating || index === order.items.length - 1} onClick={() => onMove(item, 1)}>↓</button><button type="button" onClick={() => onEdit(item)}>Edit</button><button className="danger-link" type="button" onClick={() => onRemove(item)}>Remove</button></div></td>}
  </tr>)}</tbody></table></div>;
}

export function OrderMeasurements({ order, saving, onSave }: { order: Detail; saving: boolean; onSave: (weightKg: number | null) => Promise<void> }) {
  const [editing, setEditing] = useState(false);
  const [weight, setWeight] = useState(order.weightKg?.toString() ?? "");
  const [weightError, setWeightError] = useState("");
  useEffect(() => { setWeight(order.weightKg?.toString() ?? ""); }, [order.weightKg]);
  const bsa = calculateBodySurfaceArea(order.weightKg, order.heightCm);
  function cancel() { setWeight(order.weightKg?.toString() ?? ""); setWeightError(""); setEditing(false); }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const parsedWeight = parsePatientWeight(weight);
    if (parsedWeight.error) { setWeightError(parsedWeight.error); return; }
    try { await onSave(parsedWeight.value); setWeightError(""); setEditing(false); } catch { /* The page-level error contains the command failure. */ }
  }
  return <section className="detail-section surface order-measurements" aria-labelledby="order-measurements-heading"><div className="order-measurements__heading"><div><h2 id="order-measurements-heading">Measurements</h2><p>Order-time snapshot</p></div></div><dl className="measurement-grid">
    <div><dt>Weight</dt><dd>{editing ? <form className="weight-inline-editor" onSubmit={submit} noValidate><div className="input-with-unit"><input autoFocus type="number" min="0" max="500" step="any" value={weight} disabled={saving} aria-label="Order weight" aria-invalid={Boolean(weightError)} onChange={(event) => { setWeight(event.target.value); setWeightError(""); }} /><span>kg</span></div><button className="measurement-icon-button is-save" type="submit" disabled={saving} aria-label={saving ? "Saving order weight" : "Save order weight"} title="Save order weight"><SaveIcon /></button><button className="measurement-icon-button" type="button" disabled={saving} aria-label="Cancel weight edit" title="Cancel" onClick={cancel}><CloseIcon /></button>{weightError && <small className="field-error">{weightError}</small>}</form> : <span className="measurement-value">{measurement(order.weightKg, "kg")}{order.editable && <button className="measurement-icon-button" type="button" aria-label="Edit order weight" title="Edit order weight" onClick={() => setEditing(true)}><PencilIcon /></button>}</span>}</dd></div>
    <div><dt>Height</dt><dd>{measurement(order.heightCm, "cm")}</dd></div>
    <div><dt>Body surface area</dt><dd>{bsa === null ? <span className="muted">Requires weight and height</span> : `${bsa.toFixed(2)} m²`}<small>Mosteller formula</small></dd></div>
  </dl></section>;
}

export function OrderCumulativeDose({ items, onOpenDrug }: { items: CumulativeDoseSummary[]; onOpenDrug?: (drugId: number) => void }) {
  return <section className="detail-section surface order-cumulative" aria-labelledby="order-cumulative-heading">
    <header className="order-cumulative__heading"><div><p className="eyebrow">Patient exposure</p><h2 id="order-cumulative-heading">Cumulative dose</h2></div><p>Reference only · Drugs recorded for this patient with Cumulative alert enabled.</p></header>
    {items.length === 0 ? <p className="order-cumulative__empty">No recorded drugs with Cumulative alert enabled.</p> : <div className="order-cumulative__list">{items.map((item) => { const storedTotal = wholeNumber(item.totalDose); return <article className="order-cumulative__item" key={item.drugId}><div className="order-cumulative__drug">{onOpenDrug ? <button className="order-cumulative__drug-link" type="button" aria-label={`Open ${item.drugName} drug master record`} onClick={() => onOpenDrug(item.drugId)}>{item.drugName}<span aria-hidden="true">↗</span></button> : <strong>{item.drugName}</strong>}</div><div><span>Stored total</span><strong>{storedTotal !== null ? `${storedTotal} mg/m²` : "Not available"}</strong></div><div><span>Threshold</span><strong>{item.threshold ? `${item.threshold} mg/m²` : "Not configured"}</strong></div></article>; })}</div>}
    <footer>Stored total = Σ (recorded dose ÷ order-time BSA snapshot). Display only; no threshold comparison or alert is generated.</footer>
  </section>;
}

export function calculateBodySurfaceArea(weightKg: number | null, heightCm: number | null): number | null {
  if (weightKg === null || heightCm === null || !Number.isFinite(weightKg) || !Number.isFinite(heightCm) || weightKg <= 0 || heightCm <= 0) return null;
  return Math.sqrt((weightKg * heightCm) / 3600);
}

function parsePatientWeight(value: string): { value: number | null; error?: string } {
  if (!value.trim()) return { value: null };
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 && parsed <= 500 ? { value: parsed } : { value: null, error: "Weight must be greater than 0 and no more than 500 kg." };
}

function measurement(value: number | null, unit: string): React.ReactNode { return value === null ? <span className="muted">Not recorded</span> : `${value} ${unit}`; }
function wholeNumber(value: string | null): string | null { const parsed = value === null ? Number.NaN : Number(value); return Number.isFinite(parsed) ? Math.round(parsed).toString() : null; }
function workflowStatusLabel(status: Detail["workflowStatus"]): string { if (status === "on_hold") return "On hold"; if (status === "legacy") return "Historical"; return "Active"; }
function PencilIcon() { return <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4.2L19 9.2a2.1 2.1 0 0 0 0-3L17.8 5a2.1 2.1 0 0 0-3 0L4 15.8V20Z"/><path d="m13.5 6.5 4 4"/></svg>; }
function SaveIcon() { return <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 3h12l2 2v16H5V3Z"/><path d="M8 3v6h8V3"/><path d="M8 21v-7h8v7"/></svg>; }
function CloseIcon() { return <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m7 7 10 10M17 7 7 17"/></svg>; }

function Section({ title, className = "", children }: { title: string; className?: string; children: React.ReactNode }) { return <section className={`detail-section surface ${className}`}><h2>{title}</h2><dl className="detail-fields">{children}</dl></section>; }
function Field({ label, value: content, wide }: { label: string; value: string | null; wide?: boolean }) { return <div className={wide ? "is-wide" : ""}><dt>{label}</dt><dd>{content || <span className="muted">Not recorded</span>}</dd></div>; }
function range(start: string | null, stop: string | null): string { if (!start && !stop) return "No date range"; if (start === stop || !stop) return start ? displayDate(start) : "No start date"; return `${start ? displayDate(start) : "?"} – ${displayDate(stop)}`; }
