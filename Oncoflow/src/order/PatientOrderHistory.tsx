import { useEffect, useState } from "react";
import { commandError, listPatientOrders } from "../api/commands";
import type { OrderSummary } from "../types/order";
import { displayDateTime } from "./OrderList";

export function PatientOrderHistory({ patientId, currentOrderId, onCreate, onOpen, className = "" }: { patientId: number; currentOrderId?: number; onCreate?: () => void; onOpen?: (id: number) => void; className?: string }) {
  const [state, setState] = useState<{ loading: boolean; items: OrderSummary[]; error?: string }>({ loading: true, items: [] });
  useEffect(() => { let active = true; void listPatientOrders(patientId).then((response) => active && setState({ loading: false, items: response.items })).catch((error: unknown) => active && setState({ loading: false, items: [], error: commandError(error).message ?? "Unable to load order history." })); return () => { active = false; }; }, [patientId]);
  return <section className={`patient-history surface ${className}`.trim()} aria-labelledby="patient-orders-heading">
    <div className="patient-history__heading"><div><p className="eyebrow">Local records</p><h2 id="patient-orders-heading">Order history</h2></div>{onCreate && <button className="button button--primary button--compact" type="button" onClick={onCreate}>New order</button>}</div>
    {state.loading ? <p className="history-state">Loading order history…</p> : state.error ? <p className="history-state history-state--error" role="alert">{state.error}</p> : state.items.length === 0 ? <p className="history-state">No chemotherapy orders recorded for this patient.</p> : <PatientOrderHistoryTable items={state.items} currentOrderId={currentOrderId} onOpen={onOpen} />}
  </section>;
}

export function PatientOrderHistoryTable({ items, currentOrderId, onOpen }: { items: OrderSummary[]; currentOrderId?: number; onOpen?: (id: number) => void }) {
  return <div className="table-scroll"><table className="history-table"><thead><tr><th>Date / time</th><th>Order</th><th>Drugs / dose</th><th>Regimen</th><th>Doctor / ward</th><th>Lines</th></tr></thead><tbody>{items.map((item) => <tr className={item.id === currentOrderId ? "is-current" : undefined} key={item.id}><td>{displayDateTime(item.orderTime)}</td><td>{onOpen ? <button className="history-order-link" type="button" onClick={() => onOpen(item.id)} aria-label={`Open order ${item.orderId}`}>{item.orderId}</button> : <strong>{item.orderId}</strong>}{item.id === currentOrderId && <small className="history-current">Current order</small>}{item.workflowStatus === "on_hold" && <small>On hold</small>}{!item.editable && <small>Historical</small>}</td><td>{item.drugs.length > 0 ? <span className="history-drug-list">{uniqueDrugLabels(item.drugs).join(" · ")}</span> : <span className="muted">No drugs recorded</span>}</td><td>{item.regimenName ?? "—"}</td><td>{item.doctorName ?? "—"}<small>{item.wardName ?? "No ward"}</small></td><td>{item.itemCount}</td></tr>)}</tbody></table></div>;
}

function uniqueDrugLabels(drugs: OrderSummary["drugs"]): string[] {
  const labels = drugs.map((drug) => drug.doseText?.trim()
    ? `${drug.drugName} ${drug.doseText.trim()} ${drug.unitText?.trim() || "mg"}`
    : drug.drugName);
  return labels.filter((label, index) => labels.indexOf(label) === index);
}
