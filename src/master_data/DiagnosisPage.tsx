import { useCallback, useEffect, useRef, useState } from "react";

import { commandError, createDiagnosis, listDiagnoses, updateDiagnosis } from "../api/commands";
import type { DiagnosisRecord } from "../types/masterData";
import { PageDescription } from "../guidance/PageGuidance";

type DiagnosisValues = { name: string };
type DiagnosisErrors = Partial<Record<keyof DiagnosisValues, string>>;
type InlineEditor = {
  recordId: number;
  values: DiagnosisValues;
  errors: DiagnosisErrors;
  busy: boolean;
  onChange: (name: string) => void;
  onCancel: () => void;
  onSubmit: (event: React.FormEvent) => void;
};

export function DiagnosisPage() {
  const [records, setRecords] = useState<DiagnosisRecord[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editing, setEditing] = useState<DiagnosisRecord | null>(null);
  const [creating, setCreating] = useState(false);
  const [values, setValues] = useState<DiagnosisValues>({ name: "" });
  const [errors, setErrors] = useState<DiagnosisErrors>({});
  const [feedback, setFeedback] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);

  const load = useCallback(async (query: string) => {
    setLoading(true); setLoadError(null);
    try { setRecords(await listDiagnoses({ search: query.trim() || null })); }
    catch (error) { setRecords([]); setLoadError(commandError(error).message ?? "Diagnoses could not be loaded."); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void load(search); }, [load, search]);

  function beginCreate() {
    setEditing(null); setCreating(true); setValues({ name: "" }); setErrors({}); setFeedback(null);
  }

  function beginEdit(record: DiagnosisRecord) {
    setCreating(false); setEditing(record); setValues({ name: record.name }); setErrors({}); setFeedback(null);
  }

  function cancel() {
    if (!busy) { setCreating(false); setEditing(null); setErrors({}); setFeedback(null); }
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateDiagnosis(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || submissionLock.current) return;
    submissionLock.current = true; setBusy(true); setFeedback(null);
    try {
      if (editing) {
        await updateDiagnosis(editing.id, { name: values.name.trim() });
        setFeedback({ kind: "success", text: "Diagnosis updated." });
      } else {
        await createDiagnosis({ name: values.name.trim() });
        setFeedback({ kind: "success", text: "Diagnosis added." });
      }
      setCreating(false); setEditing(null); setValues({ name: "" });
      await load(search);
    } catch (error) {
      const parsed = commandError(error);
      if (parsed.field === "name") setErrors({ name: parsed.message ?? "Invalid diagnosis name." });
      else setFeedback({ kind: "error", text: parsed.message ?? "The diagnosis could not be saved." });
    } finally {
      submissionLock.current = false; setBusy(false);
    }
  }

  const editor: InlineEditor | undefined = editing ? {
    recordId: editing.id,
    values,
    errors,
    busy,
    onChange: (name) => { setValues({ name }); setErrors({}); },
    onCancel: cancel,
    onSubmit: (event) => void submit(event),
  } : undefined;

  return <section className="workspace master-data-workspace" aria-labelledby="diagnoses-heading">
    <div className="page-heading"><div><p className="eyebrow">Master data</p><h1 id="diagnoses-heading">Diagnosis</h1><PageDescription pageKey="diagnoses" /></div><button className="button button--primary" type="button" disabled={busy} onClick={beginCreate}>Add diagnosis</button></div>
    <p className="master-data-note">Only the diagnosis name is managed here. Legacy compatibility and warning fields remain hidden and unchanged.</p>
    {feedback && <div className={feedback.kind === "success" ? "auth-success" : "auth-error"} role={feedback.kind === "success" ? "status" : "alert"}>{feedback.text}</div>}
    {creating && <section className="surface master-data-editor"><div><p className="eyebrow">Local lookup</p><h2>Add diagnosis</h2></div><form onSubmit={(event) => void submit(event)} noValidate><label className="form-field"><span className="field-label">Diagnosis name</span><input autoFocus value={values.name} disabled={busy} aria-invalid={Boolean(errors.name)} onChange={(event) => { setValues({ name: event.target.value }); setErrors({}); }} />{errors.name && <span className="field-error">{errors.name}</span>}</label><div className="master-data-editor__actions"><button className="button button--secondary" type="button" disabled={busy} onClick={cancel}>Cancel</button><button className="button button--primary" type="submit" disabled={busy}>{busy ? "Saving…" : "Save"}</button></div></form></section>}
    <div className="list-card master-data-search"><div className="list-toolbar"><label className="search-field"><span className="search-icon" aria-hidden="true">⌕</span><span className="sr-only">Search diagnoses</span><input value={search} placeholder="Search diagnosis name" onChange={(event) => setSearch(event.target.value)} />{search && <button className="clear-search" type="button" aria-label="Clear search" onClick={() => setSearch("")}>×</button>}</label><span className="result-count">{records.length} {records.length === 1 ? "diagnosis" : "diagnoses"}</span></div></div>
    {loadError ? <div className="form-error-summary" role="alert">{loadError} <button className="button button--compact button--secondary" type="button" onClick={() => void load(search)}>Retry</button></div> : <DiagnosisTable records={records} loading={loading} onEdit={beginEdit} editor={editor} />}
  </section>;
}

export function DiagnosisTable({ records, loading, onEdit, editor }: { records: DiagnosisRecord[]; loading: boolean; onEdit: (record: DiagnosisRecord) => void; editor?: InlineEditor }) {
  if (loading) return <div className="detail-loading" aria-busy="true">Loading diagnoses…</div>;
  if (records.length === 0) return <div className="empty-state"><h2>No diagnoses found</h2><p>Add a diagnosis or clear the search.</p></div>;
  return <div className="list-card"><div className="table-scroll"><table className="patient-table master-data-table"><thead><tr><th>Diagnosis name</th><th aria-label="Actions" /></tr></thead><tbody>{records.map((record) => editor?.recordId === record.id ? <tr className="master-data-inline-row" key={record.id}><td colSpan={2}><form className="master-data-inline-editor master-data-inline-editor--doctor" onSubmit={editor.onSubmit} noValidate><label className="master-data-inline-field"><span className="sr-only">Diagnosis name</span><input autoFocus value={editor.values.name} disabled={editor.busy} aria-invalid={Boolean(editor.errors.name)} onChange={(event) => editor.onChange(event.target.value)} />{editor.errors.name && <span className="field-error">{editor.errors.name}</span>}</label><div className="master-data-inline-actions"><button className="button button--secondary button--compact" type="button" disabled={editor.busy} onClick={editor.onCancel}>Cancel</button><button className="button button--primary button--compact" type="submit" disabled={editor.busy}>{editor.busy ? "Saving…" : "Save"}</button></div></form></td></tr> : <tr key={record.id}><td><strong>{record.name}</strong></td><td><button className="row-action master-data-edit-button" type="button" aria-label={`Edit diagnosis ${record.name}`} title={`Edit diagnosis ${record.name}`} onClick={() => onEdit(record)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4.2L19 9.2a2.1 2.1 0 0 0 0-3L17.8 5a2.1 2.1 0 0 0-3 0L4 15.8V20Z" /><path d="m13.5 6.5 4 4" /></svg></button></td></tr>)}</tbody></table></div></div>;
}

export function validateDiagnosis(values: DiagnosisValues): DiagnosisErrors {
  const errors: DiagnosisErrors = {};
  if (!values.name.trim() || [...values.name.trim()].length > 200) errors.name = "Diagnosis name is required and limited to 200 characters.";
  return errors;
}
