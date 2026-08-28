import { useEffect, useMemo, useState } from "react";

import { commandError, getLowStockItems, listInventory } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type {
  InventoryListRequest,
  InventorySortDirection,
  InventorySortField,
  InventorySummary,
  StockState,
} from "../types/inventory";

type ListState =
  | { kind: "loading" }
  | { kind: "ready"; items: InventorySummary[]; total: number }
  | { kind: "error"; message: string };

export function InventoryList({ onOpen }: { onOpen: (drugId: number) => void }) {
  const pageSize = 100;
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const [trackedOnly, setTrackedOnly] = useState(true);
  const [lowOnly, setLowOnly] = useState(false);
  const [sortBy, setSortBy] = useState<InventorySortField>("state");
  const [sortDirection, setSortDirection] = useState<InventorySortDirection>("asc");
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<number | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [state, setState] = useState<ListState>({ kind: "loading" });

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setSearch(query.trim());
      setPage(0);
    }, 220);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let active = true;
    const request = buildInventoryListRequest(search, trackedOnly, lowOnly, sortBy, sortDirection, page, pageSize);
    setState({ kind: "loading" });
    void (lowOnly ? getLowStockItems(request) : listInventory(request))
      .then((response) => {
        if (!active) return;
        setState({ kind: "ready", ...response });
        setSelected((current) => response.items.some((item) => item.drugId === current) ? current : null);
      })
      .catch((error: unknown) => {
        if (active) setState({ kind: "error", message: commandError(error).message ?? "Unable to load inventory." });
      });
    return () => { active = false; };
  }, [lowOnly, page, reloadKey, search, sortBy, sortDirection, trackedOnly]);

  const selectedItem = useMemo(
    () => state.kind === "ready" ? state.items.find((item) => item.drugId === selected) : undefined,
    [selected, state],
  );

  function changeSort(field: InventorySortField) {
    if (field === sortBy) setSortDirection((value) => value === "asc" ? "desc" : "asc");
    else { setSortBy(field); setSortDirection("asc"); }
    setPage(0);
  }

  return <section className="workspace inventory-workspace" aria-labelledby="inventory-heading">
    <div className="page-heading"><div><p className="eyebrow">Local stock ledger</p><h1 id="inventory-heading">Inventory</h1><PageDescription pageKey="inventory" /></div></div>
    <div className="inventory-boundary-note"><strong>Advisory only.</strong> Negative stock is recorded as a shortage and never blocks an order or preparation. No stock is deducted automatically.</div>
    <div className="surface list-card">
      <div className="list-toolbar inventory-toolbar">
        <label className="search-field"><span className="sr-only">Search inventory</span><span className="search-icon" aria-hidden="true">⌕</span><input autoFocus type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search drug code or name" />{query && <button className="clear-search" type="button" onClick={() => setQuery("")} aria-label="Clear search">×</button>}</label>
        <label className="check-filter"><input type="checkbox" checked={trackedOnly || lowOnly} disabled={lowOnly} onChange={(event) => { setTrackedOnly(event.target.checked); setPage(0); }} /> Tracked only</label>
        <label className="check-filter check-filter--low"><input type="checkbox" checked={lowOnly} onChange={(event) => { setLowOnly(event.target.checked); setPage(0); }} /> Low stock</label>
        <span className="result-count" aria-live="polite">{state.kind === "ready" ? `${state.total.toLocaleString()} item${state.total === 1 ? "" : "s"}` : "Loading stock…"}</span>
      </div>
      {state.kind === "loading" && <div className="list-skeleton" aria-label="Loading inventory">{[1,2,3,4,5].map((value) => <div className="skeleton-row" key={value}><span/><span/><span/><span/></div>)}</div>}
      {state.kind === "error" && <div className="state-panel state-panel--error" role="alert"><span className="state-icon">!</span><h2>Inventory unavailable</h2><p>{state.message}</p><button className="button button--secondary" type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button></div>}
      {state.kind === "ready" && state.items.length === 0 && <div className="state-panel"><span className="state-icon">⌕</span><h2>No matching inventory</h2><p>Adjust the search or stock filters. Drugs without a known balance remain available when tracked-only is cleared.</p></div>}
      {state.kind === "ready" && state.items.length > 0 && <>
        <InventoryTable items={state.items} selected={selected} sortBy={sortBy} sortDirection={sortDirection} onSelect={setSelected} onOpen={onOpen} onSort={changeSort} />
        <div className="list-footer"><span>Showing {page * pageSize + 1}–{Math.min((page + 1) * pageSize, state.total)} of {state.total}</span><div className="list-footer__actions"><button className="button button--secondary button--compact" type="button" disabled={page === 0} onClick={() => setPage((value) => Math.max(0, value - 1))}>Previous</button><button className="button button--secondary button--compact" type="button" disabled={(page + 1) * pageSize >= state.total} onClick={() => setPage((value) => value + 1)}>Next</button><button className="button button--secondary button--compact" type="button" disabled={!selectedItem} onClick={() => selectedItem && onOpen(selectedItem.drugId)}>Open selected</button></div></div>
      </>}
    </div>
  </section>;
}

export function buildInventoryListRequest(search: string, trackedOnly: boolean, lowOnly: boolean, sortBy: InventorySortField, sortDirection: InventorySortDirection, page: number, pageSize: number): InventoryListRequest {
  return {
    search: search.trim() || null,
    trackedOnly: lowOnly || trackedOnly,
    lowStockOnly: lowOnly,
    sortBy,
    sortDirection,
    limit: pageSize,
    offset: page * pageSize,
  };
}

export function InventoryTable({ items, selected, sortBy, sortDirection, onSelect, onOpen, onSort }: { items: InventorySummary[]; selected: number | null; sortBy: InventorySortField; sortDirection: InventorySortDirection; onSelect: (id: number) => void; onOpen: (id: number) => void; onSort: (field: InventorySortField) => void }) {
  return <div className="table-scroll"><table className="patient-table inventory-table"><thead><tr><InventoryHeading field="code" label="Drug code" active={sortBy} direction={sortDirection} onSort={onSort}/><InventoryHeading field="name" label="Drug name" active={sortBy} direction={sortDirection} onSort={onSort}/><InventoryHeading field="currentStock" label="Current stock" active={sortBy} direction={sortDirection} onSort={onSort}/><InventoryHeading field="minimum" label="Minimum" active={sortBy} direction={sortDirection} onSort={onSort}/><InventoryHeading field="maximum" label="Maximum" active={sortBy} direction={sortDirection} onSort={onSort}/><th>Tracking</th><InventoryHeading field="state" label="State" active={sortBy} direction={sortDirection} onSort={onSort}/><th aria-label="Open"/></tr></thead><tbody>{items.map((item) => <tr key={item.drugId} className={selected === item.drugId ? "is-selected" : ""} tabIndex={0} aria-selected={selected === item.drugId} onFocus={() => onSelect(item.drugId)} onDoubleClick={() => onOpen(item.drugId)} onKeyDown={(event) => { if (event.key === "Enter") onOpen(item.drugId); }}><td><span className="hn-value">{item.drugCode}</span></td><td><strong className="patient-name">{item.drugName}</strong>{item.package && <small className="row-subtitle">{item.package}</small>}</td><td className={item.currentStock !== null && item.currentStock < 0 ? "inventory-negative" : ""}>{formatQuantity(item.currentStock)}</td><td>{formatQuantity(item.minimumStock)}</td><td>{formatQuantity(item.maximumStock)}</td><td>{item.trackingEnabled ? "Enabled" : "Disabled"}</td><td><StockStateBadge state={item.stockState}/></td><td><button className="row-action" type="button" aria-label={`Open inventory for ${item.drugCode}`} onClick={() => onOpen(item.drugId)}>›</button></td></tr>)}</tbody></table></div>;
}

function InventoryHeading({ field, label, active, direction, onSort }: { field: InventorySortField; label: string; active: InventorySortField; direction: InventorySortDirection; onSort: (field: InventorySortField) => void }) {
  const selected = field === active;
  return <th aria-sort={selected ? (direction === "asc" ? "ascending" : "descending") : "none"}><button className="sort-button" type="button" onClick={() => onSort(field)}>{label}<span aria-hidden="true">{selected ? direction === "asc" ? "↑" : "↓" : "↕"}</span></button></th>;
}

export function StockStateBadge({ state }: { state: StockState }) {
  return <span className={`stock-state stock-state--${state}`}>{stockStateLabel(state)}</span>;
}

export function stockStateLabel(state: StockState): string {
  return ({ untracked: "Untracked", unknown: "Unknown", shortage: "Shortage", out: "Out", low: "Low", normal: "Normal" })[state];
}

export function formatQuantity(value: number | null): string {
  return value === null ? "Not known" : new Intl.NumberFormat(undefined, { maximumFractionDigits: 6 }).format(value);
}
