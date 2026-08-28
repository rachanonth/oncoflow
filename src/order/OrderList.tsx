import { useEffect, useMemo, useState } from "react";
import { commandError, listOrders } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { PageDescription } from "../guidance/PageGuidance";
import { currentBangkokDateTimeValue, displayLocalDateTime } from "../shared/dateTime";
import type { OrderSortField, OrderSummary, SortDirection } from "../types/order";

export function OrderList({ onCreate, onOpen }: { onCreate: () => void; onOpen: (id: number) => void }) {
  const [search, setSearch] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [sortBy, setSortBy] = useState<OrderSortField>("date");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  const [selected, setSelected] = useState<number | null>(null);
  const [state, setState] = useState<{ loading: boolean; items: OrderSummary[]; total: number; error?: string }>({ loading: true, items: [], total: 0 });
  const request = useMemo(() => ({ search, dateFrom: dateFrom || null, dateTo: dateTo || null, sortBy, sortDirection, limit: 200 }), [search, dateFrom, dateTo, sortBy, sortDirection]);

  useEffect(() => {
    let active = true;
    const timeout = window.setTimeout(() => {
      setState((value) => ({ ...value, loading: true, error: undefined }));
      void listOrders(request).then((response) => {
        if (active) setState({ loading: false, items: response.items, total: response.total });
      }).catch((error: unknown) => {
        if (active) setState({ loading: false, items: [], total: 0, error: commandError(error).message ?? "Unable to load orders." });
      });
    }, 180);
    return () => { active = false; window.clearTimeout(timeout); };
  }, [request]);

  function sort(field: OrderSortField) {
    if (sortBy === field) setSortDirection((value) => value === "asc" ? "desc" : "asc");
    else { setSortBy(field); setSortDirection(field === "date" ? "desc" : "asc"); }
  }
  function key(event: React.KeyboardEvent<HTMLTableRowElement>, item: OrderSummary) {
    if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onOpen(item.id); }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault(); const index = state.items.findIndex((value) => value.id === item.id); const next = Math.max(0, Math.min(state.items.length - 1, index + (event.key === "ArrowDown" ? 1 : -1))); setSelected(state.items[next]?.id ?? null);
    }
  }
  function applyQuickDate(preset: QuickDatePreset) {
    const range = getBangkokQuickDateRange(preset);
    setDateFrom(range.dateFrom);
    setDateTo(range.dateTo);
  }

  const todayRange = getBangkokQuickDateRange("today");
  const yesterdayRange = getBangkokQuickDateRange("yesterday");
  const quickDate = dateFrom === todayRange.dateFrom && dateTo === todayRange.dateTo
    ? "today"
    : dateFrom === yesterdayRange.dateFrom && dateTo === yesterdayRange.dateTo
      ? "yesterday"
      : null;

  return <section className="workspace order-workspace" aria-labelledby="orders-heading">
    <div className="page-heading"><div><p className="eyebrow">Local chemotherapy records</p><h1 id="orders-heading">Orders</h1><PageDescription pageKey="orders" /></div><button className="button button--primary" type="button" onClick={onCreate}>New order</button></div>
    <div className="surface list-card">
      <div className="list-toolbar order-toolbar order-list-toolbar">
        <label className="search-box"><span aria-hidden="true">⌕</span><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search HN or patient name" aria-label="Search orders" /></label>
        <label className="compact-filter">From<BuddhistDateInput value={dateFrom} onChange={setDateFrom} /></label>
        <label className="compact-filter">To<BuddhistDateInput value={dateTo} onChange={setDateTo} /></label>
        <div className="order-quick-filter" role="group" aria-label="Quick date filter">
          <button className={quickDate === "today" ? "is-active" : ""} type="button" aria-pressed={quickDate === "today"} onClick={() => applyQuickDate("today")}>Today</button>
          <button className={quickDate === "yesterday" ? "is-active" : ""} type="button" aria-pressed={quickDate === "yesterday"} onClick={() => applyQuickDate("yesterday")}>Yesterday</button>
        </div>
      </div>
      {state.error ? <div className="state-panel state-panel--error" role="alert"><span className="state-icon">!</span><h2>Orders unavailable</h2><p>{state.error}</p></div> : state.loading ? <div className="list-skeleton" aria-label="Loading orders">{[1,2,3,4].map((value) => <div className="skeleton-row" key={value}><span/><span/><span/><span/></div>)}</div> : state.items.length === 0 ? <div className="state-panel"><span className="state-icon">Rx</span><h2>No orders found</h2><p>Adjust the local search or create an order for an existing patient.</p></div> : <OrderTable items={state.items} selected={selected} sortBy={sortBy} sortDirection={sortDirection} onSort={sort} onSelect={setSelected} onOpen={onOpen} onKey={key} />}
      <div className="list-footer"><span>{state.total} order{state.total === 1 ? "" : "s"}</span><span>Local SQLite</span></div>
    </div>
  </section>;
}

export function OrderTable({ items, selected, sortBy, sortDirection, onSort, onSelect, onOpen, onKey }: { items: OrderSummary[]; selected: number | null; sortBy: OrderSortField; sortDirection: SortDirection; onSort: (field: OrderSortField) => void; onSelect: (id: number) => void; onOpen: (id: number) => void; onKey: (event: React.KeyboardEvent<HTMLTableRowElement>, item: OrderSummary) => void }) {
  return <div className="table-scroll"><table className="patient-table order-table"><thead><tr><Sort label="Date / time" field="date" current={sortBy} direction={sortDirection} onSort={onSort}/><th>HN</th><Sort label="Name" field="patient" current={sortBy} direction={sortDirection} onSort={onSort}/><th>Doctor / ward</th><th className="order-drug-count">No. of drugs</th><th aria-label="Open" /></tr></thead><tbody>{items.map((item) => <tr key={item.id} tabIndex={0} className={selected === item.id ? "is-selected" : ""} onFocus={() => onSelect(item.id)} onDoubleClick={() => onOpen(item.id)} onKeyDown={(event) => onKey(event, item)}><td>{displayLocalDateTime(item.orderTime)}{item.workflowStatus === "on_hold" && <small className="order-workflow-badge order-workflow-badge--on_hold">On hold</small>}{!item.editable && <small className="row-subtitle">Historical</small>}</td><td><span className="hn-value">{item.patientHn}</span></td><td><strong className="patient-name">{item.patientName || "Name not recorded"}</strong></td><td><span>{item.doctorName ?? "—"}</span><small className="row-subtitle">{item.wardName ?? "No ward"}</small></td><td className="order-drug-count"><strong>{item.itemCount}</strong></td><td><button className="row-action" type="button" onClick={() => onOpen(item.id)} aria-label={`Open order ${item.orderId}`}>›</button></td></tr>)}</tbody></table></div>;
}

function Sort({ label, field, current, direction, onSort }: { label: string; field: OrderSortField; current: OrderSortField; direction: SortDirection; onSort: (field: OrderSortField) => void }) { return <th><button className="sort-button" type="button" onClick={() => onSort(field)}>{label}<span>{current === field ? direction === "asc" ? "↑" : "↓" : "↕"}</span></button></th>; }

type QuickDatePreset = "today" | "yesterday";

export function getBangkokQuickDateRange(preset: QuickDatePreset, now = new Date()): { dateFrom: string; dateTo: string } {
  const [year, month, day] = currentBangkokDateTimeValue(now).slice(0, 10).split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day + (preset === "yesterday" ? -1 : 0)));
  const value = `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}-${String(date.getUTCDate()).padStart(2, "0")}`;
  return { dateFrom: value, dateTo: value };
}

export { displayLocalDateTime as displayDateTime } from "../shared/dateTime";
