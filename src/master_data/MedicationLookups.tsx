import { useCallback, useEffect, useRef, useState } from "react";

import {
  commandError,
  createDiluent,
  createRoute,
  listDiluents,
  listRoutes,
  updateDiluent,
  updateRoute,
} from "../api/commands";
import type { DiluentInput, DiluentRecord, RouteInput, RouteRecord } from "../types/masterData";
import { PageDescription } from "../guidance/PageGuidance";
import type { PageKey } from "../guidance/pageDescriptions";

type NameValues = { name: string };
type DiluentValues = NameValues & { volumeMl: string };
type NameErrors = Partial<Record<keyof NameValues, string>>;
type DiluentErrors = Partial<Record<keyof DiluentValues, string>>;

export function RoutesPage() {
  const [records, setRecords] = useState<RouteRecord[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editing, setEditing] = useState<RouteRecord | null>(null);
  const [creating, setCreating] = useState(false);
  const [values, setValues] = useState<NameValues>({ name: "" });
  const [errors, setErrors] = useState<NameErrors>({});
  const [feedback, setFeedback] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);

  const load = useCallback(async (query: string) => {
    setLoading(true); setLoadError(null);
    try { setRecords(await listRoutes({ search: query.trim() || null })); }
    catch (error) { setRecords([]); setLoadError(commandError(error).message ?? "Routes could not be loaded."); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void load(search); }, [load, search]);

  function beginCreate() {
    setEditing(null); setCreating(true); setValues({ name: "" }); setErrors({}); setFeedback(null);
  }

  function beginEdit(record: RouteRecord) {
    setCreating(false); setEditing(record); setValues({ name: record.name }); setErrors({}); setFeedback(null);
  }

  function cancel() {
    if (!busy) { setCreating(false); setEditing(null); setErrors({}); setFeedback(null); }
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateRoute(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || submissionLock.current) return;
    submissionLock.current = true; setBusy(true); setFeedback(null);
    const input: RouteInput = { name: values.name.trim(), legacyCode: editing?.legacyCode ?? null };
    try {
      if (editing) {
        await updateRoute(editing.id, input);
        setFeedback({ kind: "success", text: "Route updated." });
      } else {
        await createRoute(input);
        setFeedback({ kind: "success", text: "Route added." });
      }
      setCreating(false); setEditing(null); setValues({ name: "" });
      await load(search);
    } catch (error) {
      const parsed = commandError(error);
      if (parsed.field === "name") setErrors({ name: parsed.message ?? "Invalid route name." });
      else setFeedback({ kind: "error", text: parsed.message ?? "The route could not be saved." });
    } finally {
      submissionLock.current = false; setBusy(false);
    }
  }

  return <section className="workspace master-data-workspace" aria-labelledby="routes-heading">
    <LookupHeading title="Routes" pageKey="routes" action="Add route" onAction={beginCreate} busy={busy} />
    <p className="master-data-note">Route names are displayed in lookup menus. Existing compatibility identifiers remain preserved but are not shown.</p>
    {feedback && <div className={feedback.kind === "success" ? "auth-success" : "auth-error"} role={feedback.kind === "success" ? "status" : "alert"}>{feedback.text}</div>}
    {creating && <CreateEditor title="Add route" busy={busy} onCancel={cancel} onSubmit={(event) => void submit(event)}>
      <LookupField label="Route name" error={errors.name}><input autoFocus value={values.name} disabled={busy} onChange={(event) => { setValues({ name: event.target.value }); setErrors({}); }} /></LookupField>
    </CreateEditor>}
    <LookupSearch value={search} onChange={setSearch} count={records.length} noun="route" />
    {loadError ? <LookupLoadError message={loadError} onRetry={() => void load(search)} /> : <RouteTable records={records} loading={loading} onEdit={beginEdit} editor={editing ? { recordId: editing.id, values, errors, busy, onChange: (name) => { setValues({ name }); setErrors({}); }, onCancel: cancel, onSubmit: (event) => void submit(event) } : undefined} />}
  </section>;
}

export function DiluentsPage() {
  const [records, setRecords] = useState<DiluentRecord[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editing, setEditing] = useState<DiluentRecord | null>(null);
  const [creating, setCreating] = useState(false);
  const [values, setValues] = useState<DiluentValues>({ name: "", volumeMl: "" });
  const [errors, setErrors] = useState<DiluentErrors>({});
  const [feedback, setFeedback] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);

  const load = useCallback(async (query: string) => {
    setLoading(true); setLoadError(null);
    try { setRecords(await listDiluents({ search: query.trim() || null })); }
    catch (error) { setRecords([]); setLoadError(commandError(error).message ?? "Diluents could not be loaded."); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void load(search); }, [load, search]);

  function beginCreate() {
    setEditing(null); setCreating(true); setValues({ name: "", volumeMl: "" }); setErrors({}); setFeedback(null);
  }

  function beginEdit(record: DiluentRecord) {
    setCreating(false); setEditing(record);
    setValues({ name: record.name, volumeMl: record.volumeMl === null ? "" : String(record.volumeMl) });
    setErrors({}); setFeedback(null);
  }

  function cancel() {
    if (!busy) { setCreating(false); setEditing(null); setErrors({}); setFeedback(null); }
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateDiluent(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || submissionLock.current) return;
    submissionLock.current = true; setBusy(true); setFeedback(null);
    const input: DiluentInput = {
      name: values.name.trim(),
      volumeMl: values.volumeMl.trim() === "" ? null : Number(values.volumeMl),
      legacyCode: editing?.legacyCode ?? null,
    };
    try {
      if (editing) {
        await updateDiluent(editing.id, input);
        setFeedback({ kind: "success", text: "Diluent updated." });
      } else {
        await createDiluent(input);
        setFeedback({ kind: "success", text: "Diluent added." });
      }
      setCreating(false); setEditing(null); setValues({ name: "", volumeMl: "" });
      await load(search);
    } catch (error) {
      const parsed = commandError(error);
      if (parsed.field === "name" || parsed.field === "volumeMl") setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." }));
      else setFeedback({ kind: "error", text: parsed.message ?? "The diluent could not be saved." });
    } finally {
      submissionLock.current = false; setBusy(false);
    }
  }

  return <section className="workspace master-data-workspace" aria-labelledby="diluents-heading">
    <LookupHeading title="Diluents" pageKey="diluents" action="Add diluent" onAction={beginCreate} busy={busy} />
    <p className="master-data-note">Volume is stored as the existing optional mL reference. This page does not add unit conversion or preparation calculations.</p>
    {feedback && <div className={feedback.kind === "success" ? "auth-success" : "auth-error"} role={feedback.kind === "success" ? "status" : "alert"}>{feedback.text}</div>}
    {creating && <CreateEditor title="Add diluent" busy={busy} onCancel={cancel} onSubmit={(event) => void submit(event)}>
      <LookupField label="Diluent name" error={errors.name}><input autoFocus value={values.name} disabled={busy} onChange={(event) => { setValues((current) => ({ ...current, name: event.target.value })); setErrors((current) => ({ ...current, name: undefined })); }} /></LookupField>
      <LookupField label="Volume (mL, optional)" error={errors.volumeMl}><input type="number" min="0" step="any" value={values.volumeMl} disabled={busy} onChange={(event) => { setValues((current) => ({ ...current, volumeMl: event.target.value })); setErrors((current) => ({ ...current, volumeMl: undefined })); }} /></LookupField>
    </CreateEditor>}
    <LookupSearch value={search} onChange={setSearch} count={records.length} noun="diluent" />
    {loadError ? <LookupLoadError message={loadError} onRetry={() => void load(search)} /> : <DiluentTable records={records} loading={loading} onEdit={beginEdit} editor={editing ? { recordId: editing.id, values, errors, busy, onNameChange: (name) => { setValues((current) => ({ ...current, name })); setErrors((current) => ({ ...current, name: undefined })); }, onVolumeChange: (volumeMl) => { setValues((current) => ({ ...current, volumeMl })); setErrors((current) => ({ ...current, volumeMl: undefined })); }, onCancel: cancel, onSubmit: (event) => void submit(event) } : undefined} />}
  </section>;
}

function LookupHeading({ title, pageKey, action, onAction, busy }: { title: string; pageKey: PageKey; action: string; onAction: () => void; busy: boolean }) {
  return <div className="page-heading"><div><p className="eyebrow">Master data</p><h1>{title}</h1><PageDescription pageKey={pageKey} /></div><button className="button button--primary" type="button" disabled={busy} onClick={onAction}>{action}</button></div>;
}

function CreateEditor({ title, busy, onCancel, onSubmit, children }: { title: string; busy: boolean; onCancel: () => void; onSubmit: (event: React.FormEvent) => void; children: React.ReactNode }) {
  return <section className="surface master-data-editor"><div><p className="eyebrow">Local lookup</p><h2>{title}</h2></div><form onSubmit={onSubmit} noValidate>{children}<div className="master-data-editor__actions"><button className="button button--secondary" type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="button button--primary" type="submit" disabled={busy}>{busy ? "Saving…" : "Save"}</button></div></form></section>;
}

function LookupField({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) {
  return <label className="form-field"><span className="field-label">{label}</span>{children}{error && <span className="field-error">{error}</span>}</label>;
}

function LookupSearch({ value, onChange, count, noun }: { value: string; onChange: (value: string) => void; count: number; noun: string }) {
  return <div className="list-card master-data-search"><div className="list-toolbar"><label className="search-field"><span className="search-icon" aria-hidden="true">⌕</span><span className="sr-only">Search {noun}s</span><input value={value} placeholder={`Search ${noun} name`} onChange={(event) => onChange(event.target.value)} />{value && <button className="clear-search" type="button" aria-label="Clear search" onClick={() => onChange("")}>×</button>}</label><span className="result-count">{count} {count === 1 ? noun : `${noun}s`}</span></div></div>;
}

function LookupLoadError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return <div className="form-error-summary" role="alert">{message} <button className="button button--compact button--secondary" type="button" onClick={onRetry}>Retry</button></div>;
}

type RouteEditor = { recordId: number; values: NameValues; errors: NameErrors; busy: boolean; onChange: (name: string) => void; onCancel: () => void; onSubmit: (event: React.FormEvent) => void };
type DiluentEditor = { recordId: number; values: DiluentValues; errors: DiluentErrors; busy: boolean; onNameChange: (name: string) => void; onVolumeChange: (volume: string) => void; onCancel: () => void; onSubmit: (event: React.FormEvent) => void };

export function RouteTable({ records, loading, onEdit, editor }: { records: RouteRecord[]; loading: boolean; onEdit: (record: RouteRecord) => void; editor?: RouteEditor }) {
  if (loading) return <div className="detail-loading" aria-busy="true">Loading routes…</div>;
  if (records.length === 0) return <div className="empty-state"><h2>No routes found</h2><p>Add a route or clear the search.</p></div>;
  return <div className="list-card"><div className="table-scroll"><table className="patient-table master-data-table"><thead><tr><th>Route name</th><th aria-label="Actions" /></tr></thead><tbody>{records.map((record) => editor?.recordId === record.id ? <tr className="master-data-inline-row" key={record.id}><td colSpan={2}><form className="master-data-inline-editor master-data-inline-editor--doctor" onSubmit={editor.onSubmit} noValidate><label className="master-data-inline-field"><span className="sr-only">Route name</span><input autoFocus value={editor.values.name} disabled={editor.busy} aria-invalid={Boolean(editor.errors.name)} onChange={(event) => editor.onChange(event.target.value)} />{editor.errors.name && <span className="field-error">{editor.errors.name}</span>}</label><EditorActions busy={editor.busy} onCancel={editor.onCancel} /></form></td></tr> : <tr key={record.id}><td><strong>{record.name}</strong></td><td><EditLookupButton label={`Edit route ${record.name}`} onClick={() => onEdit(record)} /></td></tr>)}</tbody></table></div></div>;
}

export function DiluentTable({ records, loading, onEdit, editor }: { records: DiluentRecord[]; loading: boolean; onEdit: (record: DiluentRecord) => void; editor?: DiluentEditor }) {
  if (loading) return <div className="detail-loading" aria-busy="true">Loading diluents…</div>;
  if (records.length === 0) return <div className="empty-state"><h2>No diluents found</h2><p>Add a diluent or clear the search.</p></div>;
  return <div className="list-card"><div className="table-scroll"><table className="patient-table master-data-table master-data-table--diluent"><thead><tr><th>Diluent name</th><th>Volume (mL)</th><th aria-label="Actions" /></tr></thead><tbody>{records.map((record) => editor?.recordId === record.id ? <tr className="master-data-inline-row" key={record.id}><td colSpan={3}><form className="master-data-inline-editor master-data-inline-editor--ward" onSubmit={editor.onSubmit} noValidate><label className="master-data-inline-field"><span className="sr-only">Diluent name</span><input autoFocus value={editor.values.name} disabled={editor.busy} aria-invalid={Boolean(editor.errors.name)} onChange={(event) => editor.onNameChange(event.target.value)} />{editor.errors.name && <span className="field-error">{editor.errors.name}</span>}</label><label className="master-data-inline-field"><span className="sr-only">Volume in mL</span><input type="number" min="0" step="any" value={editor.values.volumeMl} disabled={editor.busy} aria-invalid={Boolean(editor.errors.volumeMl)} onChange={(event) => editor.onVolumeChange(event.target.value)} />{editor.errors.volumeMl && <span className="field-error">{editor.errors.volumeMl}</span>}</label><EditorActions busy={editor.busy} onCancel={editor.onCancel} /></form></td></tr> : <tr key={record.id}><td><strong>{record.name}</strong></td><td>{record.volumeMl ?? <span className="muted-value">Not set</span>}</td><td><EditLookupButton label={`Edit diluent ${record.name}`} onClick={() => onEdit(record)} /></td></tr>)}</tbody></table></div></div>;
}

function EditorActions({ busy, onCancel }: { busy: boolean; onCancel: () => void }) {
  return <div className="master-data-inline-actions"><button className="button button--secondary button--compact" type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="button button--primary button--compact" type="submit" disabled={busy}>{busy ? "Saving…" : "Save"}</button></div>;
}

function EditLookupButton({ label, onClick }: { label: string; onClick: () => void }) {
  return <button className="row-action master-data-edit-button" type="button" aria-label={label} title={label} onClick={onClick}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4.2L19 9.2a2.1 2.1 0 0 0 0-3L17.8 5a2.1 2.1 0 0 0-3 0L4 15.8V20Z" /><path d="m13.5 6.5 4 4" /></svg></button>;
}

export function validateRoute(values: NameValues): NameErrors {
  const errors: NameErrors = {};
  if (!values.name.trim() || [...values.name.trim()].length > 200) errors.name = "Route name is required and limited to 200 characters.";
  return errors;
}

export function validateDiluent(values: DiluentValues): DiluentErrors {
  const errors: DiluentErrors = {};
  if (!values.name.trim() || [...values.name.trim()].length > 200) errors.name = "Diluent name is required and limited to 200 characters.";
  const volume = values.volumeMl.trim();
  if (volume && (!/^(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(volume) || !Number.isFinite(Number(volume)) || Number(volume) < 0)) errors.volumeMl = "Volume must be a number greater than or equal to zero.";
  return errors;
}
