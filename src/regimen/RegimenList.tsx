import { useEffect, useMemo, useState } from "react";

import { commandError, listRegimens } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type { RegimenSortField, RegimenSummary, SortDirection } from "../types/regimen";

interface Props { onCreate: () => void; onOpen: (id: number) => void }
type State = { kind: "loading" } | { kind: "ready"; items: RegimenSummary[]; total: number } | { kind: "error"; message: string };

export function RegimenList({ onCreate, onOpen }: Props) {
  const pageSize = 100;
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const [sortBy, setSortBy] = useState<RegimenSortField>("code");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [page, setPage] = useState(0);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [state, setState] = useState<State>({ kind: "loading" });

  useEffect(() => {
    const timer = window.setTimeout(() => { setSearch(query.trim()); setPage(0); }, 250);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    void listRegimens({ search: search || null, sortBy, sortDirection, limit: pageSize, offset: page * pageSize })
      .then((response) => {
        if (!active) return;
        setState({ kind: "ready", ...response });
        setSelectedId((current) => response.items.some((item) => item.id === current) ? current : null);
      })
      .catch((error: unknown) => active && setState({ kind: "error", message: commandError(error).message ?? "Unable to load regimens." }));
    return () => { active = false; };
  }, [page, reloadKey, search, sortBy, sortDirection]);

  const selected = useMemo(() => state.kind === "ready" ? state.items.find((item) => item.id === selectedId) : undefined, [selectedId, state]);
  function changeSort(field: RegimenSortField) {
    if (field === sortBy) setSortDirection((value) => value === "asc" ? "desc" : "asc");
    else { setSortBy(field); setSortDirection("asc"); }
    setPage(0);
  }

  return <section className="workspace" aria-labelledby="regimens-heading">
    <div className="page-heading"><div><p className="eyebrow">Treatment protocol data</p><h1 id="regimens-heading">Chemotherapy regimens</h1><PageDescription pageKey="regimens" /></div><button className="button button--primary" type="button" onClick={onCreate}>＋ New regimen</button></div>
    <div className="list-card">
      <div className="list-toolbar"><label className="search-field"><span className="sr-only">Search regimens</span><span className="search-icon" aria-hidden="true">⌕</span><input autoFocus type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search by regimen code or name" />{query && <button className="clear-search" type="button" onClick={() => setQuery("")} aria-label="Clear search">×</button>}</label><span className="result-count" aria-live="polite">{state.kind === "ready" ? `${state.total} regimens` : "Loading records…"}</span></div>
      {state.kind === "loading" && <div className="list-skeleton" aria-label="Loading regimen records">{Array.from({ length: 6 }, (_, i) => <div key={i} className="skeleton-row"><span /><span /><span /><span /></div>)}</div>}
      {state.kind === "error" && <div className="state-panel state-panel--error" role="alert"><span className="state-icon">!</span><h2>Regimens could not be loaded</h2><p>{state.message}</p><button className="button button--secondary" type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button></div>}
      {state.kind === "ready" && state.items.length === 0 && <div className="state-panel"><span className="state-icon">⌕</span><h2>{search ? "No matching regimens" : "No regimen records yet"}</h2><p>{search ? "Try a different code or name." : "Create the first local regimen."}</p></div>}
      {state.kind === "ready" && state.items.length > 0 && <><div className="table-scroll"><table className="patient-table regimen-table"><thead><tr><SortHead field="code" label="Code" active={sortBy} direction={sortDirection} onSort={changeSort} /><SortHead field="name" label="Regimen name" active={sortBy} direction={sortDirection} onSort={changeSort} /><th>Treatment groups</th><SortHead field="items" label="Drug steps" active={sortBy} direction={sortDirection} onSort={changeSort} /><th>Legacy marker</th><th><span className="sr-only">Open</span></th></tr></thead><tbody>{state.items.map((item) => <tr key={item.id} className={selectedId === item.id ? "is-selected" : undefined} tabIndex={0} aria-selected={selectedId === item.id} onClick={() => setSelectedId(item.id)} onDoubleClick={() => onOpen(item.id)} onKeyDown={(event) => event.key === "Enter" && onOpen(item.id)}><td><span className="hn-value">{item.code}</span></td><td><span className="patient-name">{item.name}</span></td><td>{item.groupCount}</td><td>{item.itemCount}</td><td><span className={`inventory-status ${item.marker ? "is-enabled" : ""}`}>{item.marker ? "Enabled" : "Disabled"}</span></td><td><button className="row-action" type="button" aria-label={`Open regimen ${item.code}`} onClick={(event) => { event.stopPropagation(); onOpen(item.id); }}>›</button></td></tr>)}</tbody></table></div><div className="list-footer"><span>Showing {page * pageSize + 1}–{Math.min((page + 1) * pageSize, state.total)} of {state.total}</span><div className="list-footer__actions"><button className="button button--secondary button--compact" type="button" disabled={page === 0} onClick={() => setPage((value) => Math.max(0, value - 1))}>Previous</button><button className="button button--secondary button--compact" type="button" disabled={(page + 1) * pageSize >= state.total} onClick={() => setPage((value) => value + 1)}>Next</button><button className="button button--secondary button--compact" type="button" disabled={!selected} onClick={() => selected && onOpen(selected.id)}>Open selected</button></div></div></>}
    </div>
  </section>;
}

function SortHead({ field, label, active, direction, onSort }: { field: RegimenSortField; label: string; active: RegimenSortField; direction: SortDirection; onSort: (field: RegimenSortField) => void }) {
  const selected = field === active;
  return <th aria-sort={selected ? direction === "asc" ? "ascending" : "descending" : "none"}><button className="sort-button" type="button" onClick={() => onSort(field)}>{label}<span aria-hidden="true">{selected ? direction === "asc" ? "↑" : "↓" : "↕"}</span></button></th>;
}
