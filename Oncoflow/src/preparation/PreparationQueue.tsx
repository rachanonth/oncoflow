import { useEffect, useMemo, useState } from "react";

import { commandError, listPreparationQueue } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { PageDescription } from "../guidance/PageGuidance";
import { displayDateTime, getBangkokQuickDateRange } from "../order/OrderList";
import { currentBangkokDateTimeValue } from "../shared/dateTime";
import type { PreparationQueueItem, PreparationQueueSourceFilter } from "../types/preparation";
import { WorkingFormulaDialog } from "./WorkingFormula";

export function PreparationQueue({ onOpen }: { onOpen: (orderId: number, preparationDate: string) => void }) {
  const [search, setSearch] = useState("");
  const [preparationDate, setPreparationDate] = useState(() => currentBangkokDateTimeValue().slice(0, 10));
  const [sourceFilter, setSourceFilter] = useState<PreparationQueueSourceFilter>("all");
  const [selected, setSelected] = useState<number | null>(null);
  const [formulaOpen, setFormulaOpen] = useState(false);
  const [refreshVersion, setRefreshVersion] = useState(0);
  const [state, setState] = useState<{ loading: boolean; items: PreparationQueueItem[]; total: number; error: string | null }>({ loading: true, items: [], total: 0, error: null });
  const request = useMemo(() => ({ search, preparationDate, sourceFilter, limit: 200 }), [search, preparationDate, sourceFilter]);

  useEffect(() => {
    let active = true;
    setState((value) => ({ ...value, loading: true, error: null }));
    const timeout = window.setTimeout(() => {
      void listPreparationQueue(request).then((response) => {
        if (active) setState({ loading: false, items: response.items, total: response.total, error: null });
      }).catch((error: unknown) => {
        if (active) setState({ loading: false, items: [], total: 0, error: commandError(error).message ?? "Unable to load the preparation queue." });
      });
    }, 180);
    return () => { active = false; window.clearTimeout(timeout); };
  }, [request, refreshVersion]);

  function key(event: React.KeyboardEvent<HTMLTableRowElement>, item: PreparationQueueItem) {
    if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onOpen(item.orderId, item.preparationDate); }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const index = state.items.findIndex((value) => value.orderId === item.orderId);
      const next = Math.max(0, Math.min(state.items.length - 1, index + (event.key === "ArrowDown" ? 1 : -1)));
      setSelected(state.items[next]?.orderId ?? null);
    }
  }

  function applyQuickDate(preset: "today" | "yesterday") {
    const range = getBangkokQuickDateRange(preset);
    setPreparationDate(range.dateFrom);
  }

  const todayRange = getBangkokQuickDateRange("today");
  const yesterdayRange = getBangkokQuickDateRange("yesterday");
  const quickDate = preparationDate === todayRange.dateFrom
    ? "today"
    : preparationDate === yesterdayRange.dateFrom
      ? "yesterday"
      : null;

  return <section className="workspace preparation-workspace" aria-labelledby="preparation-queue-heading">
    <div className="page-heading"><div><p className="eyebrow">Pharmacist workspace</p><h1 id="preparation-queue-heading">Preparation queue</h1><PageDescription pageKey="preparation" /></div><div className="working-formula-action"><span>Uses the current preparation view</span><button className="button button--primary" type="button" disabled={state.loading || state.items.length === 0} onClick={() => setFormulaOpen(true)}>Working formula ({state.items.length})</button></div></div>
    <div className="surface list-card">
      <div className="list-toolbar preparation-queue-toolbar"><label className="search-box"><span aria-hidden="true">⌕</span><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search order, HN, patient, ward, or regimen" aria-label="Search preparation queue" /></label><label className="compact-filter">Preparation date<BuddhistDateInput value={preparationDate} onChange={setPreparationDate} /></label><div className="order-quick-filter" role="group" aria-label="Quick preparation date"><button className={quickDate === "today" ? "is-active" : ""} type="button" aria-pressed={quickDate === "today"} onClick={() => applyQuickDate("today")}>Today</button><button className={quickDate === "yesterday" ? "is-active" : ""} type="button" aria-pressed={quickDate === "yesterday"} onClick={() => applyQuickDate("yesterday")}>Yesterday</button></div><div className="order-quick-filter preparation-source-filter" role="group" aria-label="Preparation source"><button className={sourceFilter === "all" ? "is-active" : ""} type="button" aria-pressed={sourceFilter === "all"} onClick={() => setSourceFilter("all")}>All</button><button className={sourceFilter === "same_day" ? "is-active" : ""} type="button" aria-pressed={sourceFilter === "same_day"} onClick={() => setSourceFilter("same_day")}>Today order</button><button className={sourceFilter === "continuing" ? "is-active" : ""} type="button" aria-pressed={sourceFilter === "continuing"} onClick={() => setSourceFilter("continuing")}>Continuing</button></div></div>
      {state.error ? <div className="state-panel state-panel--error" role="alert"><span className="state-icon">!</span><h2>Preparation queue unavailable</h2><p>{state.error}</p></div> : state.loading ? <div className="list-skeleton" aria-label="Loading preparation queue">{[1, 2, 3].map((value) => <div className="skeleton-row" key={value}><span/><span/><span/><span/></div>)}</div> : state.items.length === 0 ? <div className="state-panel"><span className="state-icon">⌁</span><h2>No eligible orders in the queue</h2><p>Create or edit a local order with a drug enabled for chemotherapy preparation. Historical orders are not converted automatically.</p></div> : <PreparationQueueTable items={state.items} selected={selected} onSelect={setSelected} onOpen={onOpen} onKey={key} />}
      <div className="list-footer"><span>{state.total} eligible order{state.total === 1 ? "" : "s"}</span><span>Local SQLite · no inventory deduction</span></div>
    </div>{formulaOpen && <WorkingFormulaDialog date={preparationDate} items={state.items} onClose={() => { setFormulaOpen(false); setRefreshVersion((value) => value + 1); }} />}
  </section>;
}

export function PreparationQueueTable({ items, selected, onSelect, onOpen, onKey }: { items: PreparationQueueItem[]; selected: number | null; onSelect: (id: number) => void; onOpen: (id: number, preparationDate: string) => void; onKey: (event: React.KeyboardEvent<HTMLTableRowElement>, item: PreparationQueueItem) => void }) {
  return <div className="table-scroll"><table className="patient-table preparation-queue-table"><thead><tr><th>Order</th><th>Order date</th><th>Patient</th><th>Ward</th><th>สถานะพิมพ์ฉลาก</th><th aria-label="Actions" /></tr></thead><tbody>{items.map((item) => <tr key={`${item.orderId}:${item.preparationDate}`} tabIndex={0} className={selected === item.orderId ? "is-selected" : ""} onFocus={() => onSelect(item.orderId)} onDoubleClick={() => onOpen(item.orderId, item.preparationDate)} onKeyDown={(event) => onKey(event, item)}><td><span className="hn-value">{item.orderCode}</span><small className={`preparation-source preparation-source--${item.sourceKind}`}>{sourceLabel(item.sourceKind)}</small><small className="row-subtitle">{item.eligibleItemCount} item{item.eligibleItemCount === 1 ? "" : "s"} due</small></td><td>{displayDateTime(item.treatmentTime)}</td><td><span className="preparation-patient-hn">HN {item.patientHn}</span><strong className="patient-name">{item.patientName || "Name not recorded"}</strong></td><td>{item.wardName ?? <span className="muted">Not recorded</span>}</td><td><LabelPrintStatus item={item} /></td><td><div className="preparation-queue-actions"><button className="row-action" type="button" onClick={() => onOpen(item.orderId, item.preparationDate)} aria-label={`Open preparation for ${item.orderCode}`}>›</button></div></td></tr>)}</tbody></table></div>;
}

export function labelPrintStatus(item: PreparationQueueItem): { label: string; tone: string } {
  if (item.printedLabelCount >= item.eligibleItemCount && item.eligibleItemCount > 0) return { label: "พิมพ์แล้ว", tone: "printed" };
  if (item.printedLabelCount > 0) return { label: `พิมพ์บางส่วน ${item.printedLabelCount}/${item.eligibleItemCount}`, tone: "partial" };
  if (item.verifiedItemCount >= item.eligibleItemCount && item.eligibleItemCount > 0) return { label: "พร้อมพิมพ์", tone: "ready" };
  return { label: "รอตรวจสอบ", tone: "waiting" };
}

function LabelPrintStatus({ item }: { item: PreparationQueueItem }) {
  const status = labelPrintStatus(item);
  return <span className={`label-print-status label-print-status--${status.tone}`}>{status.label}</span>;
}

function sourceLabel(source: PreparationQueueItem["sourceKind"]): string { if (source === "rescheduled") return "Rescheduled order"; if (source === "continuing") return "Continuing order"; return "Today order"; }
