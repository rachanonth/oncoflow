import { useEffect, useMemo, useState } from "react";

import { commandError, listPatients } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type {
  PatientSortField,
  PatientSummary,
  SortDirection,
} from "../types/patient";
import { displayDateTime, patientName } from "./format";

interface PatientListProps {
  onCreate: () => void;
  onOpen: (patientId: number) => void;
}

type ListState =
  | { kind: "loading" }
  | { kind: "ready"; items: PatientSummary[]; total: number }
  | { kind: "error"; message: string };

export function PatientList({ onCreate, onOpen }: PatientListProps) {
  const pageSize = 100;
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [sortBy, setSortBy] = useState<PatientSortField>("hn");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
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
    void listPatients({
      search: search || null,
      sortBy,
      sortDirection,
      limit: pageSize,
      offset: page * pageSize,
    })
      .then((response) => {
        if (!active) return;
        setState({ kind: "ready", ...response });
        setSelectedId((current) =>
          response.items.some((patient) => patient.id === current) ? current : null,
        );
      })
      .catch((error: unknown) => {
        if (!active) return;
        setState({
          kind: "error",
          message: commandError(error).message ?? "Unable to load patients.",
        });
      });
    return () => {
      active = false;
    };
  }, [page, reloadKey, search, sortBy, sortDirection]);

  const selectedPatient = useMemo(
    () =>
      state.kind === "ready"
        ? state.items.find((patient) => patient.id === selectedId)
        : undefined,
    [selectedId, state],
  );

  function changeSort(field: PatientSortField) {
    if (field === sortBy) {
      setSortDirection((direction) => (direction === "asc" ? "desc" : "asc"));
    } else {
      setSortBy(field);
      setSortDirection("asc");
    }
    setPage(0);
  }

  return (
    <section className="workspace" aria-labelledby="patients-heading">
      <div className="page-heading">
        <div>
          <p className="eyebrow">Clinical records</p>
          <h1 id="patients-heading">Patients</h1>
          <PageDescription pageKey="patients" />
        </div>
        <button className="button button--primary" type="button" onClick={onCreate}>
          <span aria-hidden="true">＋</span> New patient
        </button>
      </div>

      <div className="list-card">
        <div className="list-toolbar">
          <label className="search-field">
            <span className="sr-only">Search patients</span>
            <span className="search-icon" aria-hidden="true">⌕</span>
            <input
              autoFocus
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search by HN, first name, or last name"
            />
            {query && (
              <button
                className="clear-search"
                type="button"
                onClick={() => setQuery("")}
                aria-label="Clear search"
              >
                ×
              </button>
            )}
          </label>
          <span className="result-count" aria-live="polite">
            {state.kind === "ready"
              ? `${state.total.toLocaleString()} ${state.total === 1 ? "patient" : "patients"}`
              : "Loading records…"}
          </span>
        </div>

        {state.kind === "loading" && <PatientListSkeleton />}

        {state.kind === "error" && (
          <div className="state-panel state-panel--error" role="alert">
            <span className="state-icon" aria-hidden="true">!</span>
            <h2>Patient records could not be loaded</h2>
            <p>{state.message}</p>
            <button
              className="button button--secondary"
              type="button"
              onClick={() => setReloadKey((value) => value + 1)}
            >
              Try again
            </button>
          </div>
        )}

        {state.kind === "ready" && state.items.length === 0 && (
          <div className="state-panel">
            <span className="state-icon" aria-hidden="true">⌕</span>
            <h2>{search ? "No matching patients" : "No patient records yet"}</h2>
            <p>
              {search
                ? "Check the HN or name and try another search."
                : "Create the first local patient record to get started."}
            </p>
            {!search && (
              <button className="button button--primary" type="button" onClick={onCreate}>
                New patient
              </button>
            )}
          </div>
        )}

        {state.kind === "ready" && state.items.length > 0 && (
          <>
            <div className="table-scroll">
              <table className="patient-table">
                <thead>
                  <tr>
                    <SortableHeading
                      field="hn"
                      label="HN"
                      activeField={sortBy}
                      direction={sortDirection}
                      onSort={changeSort}
                    />
                    <SortableHeading
                      field="name"
                      label="Patient"
                      activeField={sortBy}
                      direction={sortDirection}
                      onSort={changeSort}
                    />
                    <th>Diagnosis</th>
                    <th>Regimen</th>
                    <SortableHeading
                      field="lastUpdated"
                      label="Updated"
                      activeField={sortBy}
                      direction={sortDirection}
                      onSort={changeSort}
                    />
                    <th><span className="sr-only">Open</span></th>
                  </tr>
                </thead>
                <tbody>
                  {state.items.map((patient) => (
                    <tr
                      key={patient.id}
                      className={selectedId === patient.id ? "is-selected" : undefined}
                      tabIndex={0}
                      aria-selected={selectedId === patient.id}
                      onClick={() => setSelectedId(patient.id)}
                      onDoubleClick={() => onOpen(patient.id)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") onOpen(patient.id);
                      }}
                    >
                      <td><span className="hn-value">{patient.hn}</span></td>
                      <td>
                        <span className="patient-name">{patientName(patient)}</span>
                      </td>
                      <td>{patient.diagnosis ?? <span className="muted">Not recorded</span>}</td>
                      <td>{patient.regimen ?? <span className="muted">Not recorded</span>}</td>
                      <td className="date-cell">{displayDateTime(patient.lastUpdated)}</td>
                      <td>
                        <button
                          className="row-action"
                          type="button"
                          aria-label={`Open patient ${patient.hn}`}
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpen(patient.id);
                          }}
                        >
                          ›
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="list-footer">
              <span>
                Showing {page * pageSize + 1}–
                {Math.min((page + 1) * pageSize, state.total)} of {state.total}
              </span>
              <div className="list-footer__actions">
                <button
                  className="button button--secondary button--compact"
                  type="button"
                  disabled={page === 0}
                  onClick={() => setPage((value) => Math.max(0, value - 1))}
                >
                  Previous
                </button>
                <button
                  className="button button--secondary button--compact"
                  type="button"
                  disabled={(page + 1) * pageSize >= state.total}
                  onClick={() => setPage((value) => value + 1)}
                >
                  Next
                </button>
                <button
                  className="button button--secondary button--compact"
                  type="button"
                  disabled={!selectedPatient}
                  onClick={() => selectedPatient && onOpen(selectedPatient.id)}
                >
                  Open selected
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    </section>
  );
}

function SortableHeading({
  field,
  label,
  activeField,
  direction,
  onSort,
}: {
  field: PatientSortField;
  label: string;
  activeField: PatientSortField;
  direction: SortDirection;
  onSort: (field: PatientSortField) => void;
}) {
  const active = activeField === field;
  return (
    <th aria-sort={active ? (direction === "asc" ? "ascending" : "descending") : "none"}>
      <button className="sort-button" type="button" onClick={() => onSort(field)}>
        {label}
        <span aria-hidden="true">{active ? (direction === "asc" ? "↑" : "↓") : "↕"}</span>
      </button>
    </th>
  );
}

function PatientListSkeleton() {
  return (
    <div className="list-skeleton" aria-label="Loading patient records">
      {Array.from({ length: 5 }, (_, index) => (
        <div key={index} className="skeleton-row">
          <span /><span /><span /><span />
        </div>
      ))}
    </div>
  );
}
