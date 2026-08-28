import { useEffect, useMemo, useState } from "react";

import { commandError, listDrugs } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type {
  DrugSortField,
  DrugSummary,
  SortDirection,
} from "../types/drug";
import { displayDrugValue } from "./format";

interface DrugListProps {
  onCreate: () => void;
  onOpen: (drugId: number) => void;
}

type InventoryFilter = "all" | "enabled" | "disabled";
type ListState =
  | { kind: "loading" }
  | { kind: "ready"; items: DrugSummary[]; total: number }
  | { kind: "error"; message: string };

export function DrugList({ onCreate, onOpen }: DrugListProps) {
  const pageSize = 100;
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const [inventoryFilter, setInventoryFilter] = useState<InventoryFilter>("all");
  const [sortBy, setSortBy] = useState<DrugSortField>("name");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [page, setPage] = useState(0);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [state, setState] = useState<ListState>({ kind: "loading" });

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setSearch(query.trim());
      setPage(0);
    }, 250);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    void listDrugs({
      search: search || null,
      inventoryEnabled:
        inventoryFilter === "all" ? null : inventoryFilter === "enabled",
      sortBy,
      sortDirection,
      limit: pageSize,
      offset: page * pageSize,
    })
      .then((response) => {
        if (!active) return;
        setState({ kind: "ready", ...response });
        setSelectedId((current) =>
          response.items.some((drug) => drug.id === current) ? current : null,
        );
      })
      .catch((error: unknown) => {
        if (!active) return;
        setState({
          kind: "error",
          message: commandError(error).message ?? "Unable to load drugs.",
        });
      });
    return () => {
      active = false;
    };
  }, [inventoryFilter, page, reloadKey, search, sortBy, sortDirection]);

  const selectedDrug = useMemo(
    () =>
      state.kind === "ready"
        ? state.items.find((drug) => drug.id === selectedId)
        : undefined,
    [selectedId, state],
  );

  function changeSort(field: DrugSortField) {
    if (field === sortBy) {
      setSortDirection((direction) => (direction === "asc" ? "desc" : "asc"));
    } else {
      setSortBy(field);
      setSortDirection("asc");
    }
    setPage(0);
  }

  return (
    <section className="workspace" aria-labelledby="drugs-heading">
      <div className="page-heading">
        <div>
          <p className="eyebrow">Medication configuration</p>
          <h1 id="drugs-heading">Drug master</h1>
          <PageDescription pageKey="drugs" />
        </div>
        <button className="button button--primary" type="button" onClick={onCreate}>
          <span aria-hidden="true">＋</span> New drug
        </button>
      </div>

      <div className="list-card">
        <div className="list-toolbar drug-toolbar">
          <label className="search-field">
            <span className="sr-only">Search drugs</span>
            <span className="search-icon" aria-hidden="true">⌕</span>
            <input
              autoFocus
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search by drug name"
            />
            {query && (
              <button className="clear-search" type="button" onClick={() => setQuery("")} aria-label="Clear search">×</button>
            )}
          </label>
          <div className="compact-filter inventory-filter" role="group" aria-label="Inventory filter">
            <span>Inventory</span>
            <div className="inventory-filter__options">
              {([
                ["all", "All"],
                ["enabled", "Enabled"],
                ["disabled", "Disabled"],
              ] as const).map(([value, label]) => (
                <button
                  className={`inventory-filter__button ${inventoryFilter === value ? "is-active" : ""}`}
                  type="button"
                  key={value}
                  aria-pressed={inventoryFilter === value}
                  onClick={() => {
                    setInventoryFilter(value);
                    setPage(0);
                  }}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          <span className="result-count" aria-live="polite">
            {state.kind === "ready"
              ? `${state.total.toLocaleString()} ${state.total === 1 ? "drug" : "drugs"}`
              : "Loading records…"}
          </span>
        </div>

        {state.kind === "loading" && <DrugListSkeleton />}
        {state.kind === "error" && (
          <div className="state-panel state-panel--error" role="alert">
            <span className="state-icon" aria-hidden="true">!</span>
            <h2>Drug records could not be loaded</h2>
            <p>{state.message}</p>
            <button className="button button--secondary" type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button>
          </div>
        )}
        {state.kind === "ready" && state.items.length === 0 && (
          <div className="state-panel">
            <span className="state-icon" aria-hidden="true">⌕</span>
            <h2>{search || inventoryFilter !== "all" ? "No matching drugs" : "No drug records yet"}</h2>
            <p>{search || inventoryFilter !== "all" ? "Adjust the search or inventory filter." : "Create the first local drug record."}</p>
          </div>
        )}
        {state.kind === "ready" && state.items.length > 0 && (
          <>
            <div className="table-scroll">
              <table className="patient-table drug-table">
                <thead>
                  <tr>
                    <DrugSortHeading field="name" label="Drug name" active={sortBy} direction={sortDirection} onSort={changeSort} />
                    <DrugSortHeading field="unit" label="Unit" active={sortBy} direction={sortDirection} onSort={changeSort} />
                    <th>Package</th>
                    <DrugSortHeading field="inventory" label="Inventory" active={sortBy} direction={sortDirection} onSort={changeSort} />
                    <th>Min / max</th>
                    <th><span className="sr-only">Open</span></th>
                  </tr>
                </thead>
                <tbody>
                  {state.items.map((drug) => (
                    <tr
                      key={drug.id}
                      className={selectedId === drug.id ? "is-selected" : undefined}
                      tabIndex={0}
                      aria-selected={selectedId === drug.id}
                      onClick={() => setSelectedId(drug.id)}
                      onDoubleClick={() => onOpen(drug.id)}
                      onKeyDown={(event) => event.key === "Enter" && onOpen(drug.id)}
                    >
                      <td><span className="patient-name">{drug.name}</span></td>
                      <td>{drug.unit ?? <span className="muted">Not set</span>}</td>
                      <td>{drug.package ?? <span className="muted">Not set</span>}</td>
                      <td>
                        <span className={`inventory-status ${drug.inventoryEnabled ? "is-enabled" : ""}`}>
                          {drug.inventoryEnabled ? "Enabled" : "Disabled"}
                        </span>
                      </td>
                      <td>{displayDrugValue(drug.inventoryMin)} / {displayDrugValue(drug.inventoryMax)}</td>
                      <td>
                        <button className="row-action" type="button" aria-label={`Open drug ${drug.name}`} onClick={(event) => { event.stopPropagation(); onOpen(drug.id); }}>›</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="list-footer">
              <span>Showing {page * pageSize + 1}–{Math.min((page + 1) * pageSize, state.total)} of {state.total}</span>
              <div className="list-footer__actions">
                <button className="button button--secondary button--compact" type="button" disabled={page === 0} onClick={() => setPage((value) => Math.max(0, value - 1))}>Previous</button>
                <button className="button button--secondary button--compact" type="button" disabled={(page + 1) * pageSize >= state.total} onClick={() => setPage((value) => value + 1)}>Next</button>
                <button className="button button--secondary button--compact" type="button" disabled={!selectedDrug} onClick={() => selectedDrug && onOpen(selectedDrug.id)}>Open selected</button>
              </div>
            </div>
          </>
        )}
      </div>
    </section>
  );
}

function DrugSortHeading({ field, label, active, direction, onSort }: { field: DrugSortField; label: string; active: DrugSortField; direction: SortDirection; onSort: (field: DrugSortField) => void }) {
  const selected = field === active;
  return (
    <th aria-sort={selected ? (direction === "asc" ? "ascending" : "descending") : "none"}>
      <button className="sort-button" type="button" onClick={() => onSort(field)}>
        {label}<span aria-hidden="true">{selected ? (direction === "asc" ? "↑" : "↓") : "↕"}</span>
      </button>
    </th>
  );
}

function DrugListSkeleton() {
  return <div className="list-skeleton" aria-label="Loading drug records">{Array.from({ length: 6 }, (_, index) => <div key={index} className="skeleton-row"><span /><span /><span /><span /></div>)}</div>;
}
