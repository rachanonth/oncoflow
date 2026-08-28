import { useCallback, useEffect, useRef, useState } from "react";

import {
  commandError,
  createDoctor,
  createWard,
  listDoctors,
  listWards,
  updateDoctor,
  updateWard,
} from "../api/commands";
import type { DoctorInput, DoctorRecord, WardInput, WardRecord } from "../types/masterData";
import { PageDescription } from "../guidance/PageGuidance";
import type { PageKey } from "../guidance/pageDescriptions";

type DoctorValues = { name: string };
type WardValues = DoctorValues & { telephone: string };
type DoctorErrors = Partial<Record<keyof DoctorValues, string>>;
type WardErrors = Partial<Record<keyof WardValues, string>>;

const EMPTY_DOCTOR: DoctorValues = { name: "" };
const EMPTY_WARD: WardValues = { name: "", telephone: "" };

export function DoctorsPage() {
  const [records, setRecords] = useState<DoctorRecord[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editing, setEditing] = useState<DoctorRecord | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [values, setValues] = useState<DoctorValues>(EMPTY_DOCTOR);
  const [errors, setErrors] = useState<DoctorErrors>({});
  const [message, setMessage] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);

  const load = useCallback(async (query: string) => {
    setLoading(true);
    setLoadError(null);
    try {
      setRecords(await listDoctors({ search: query.trim() || null }));
    } catch (error) {
      setRecords([]);
      setLoadError(commandError(error).message ?? "Doctors could not be loaded.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(search); }, [load, search]);

  function beginCreate() {
    setEditing(null); setValues(EMPTY_DOCTOR); setErrors({}); setMessage(null); setSaveError(null); setFormOpen(true);
  }

  function beginEdit(record: DoctorRecord) {
    setEditing(record);
    setValues({ name: record.name });
    setErrors({}); setMessage(null); setSaveError(null); setFormOpen(false);
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateDoctor(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || submissionLock.current) return;
    submissionLock.current = true; setBusy(true); setMessage(null); setSaveError(null);
    const input: DoctorInput = { name: values.name.trim(), legacyCode: editing?.legacyCode ?? null };
    try {
      if (editing) {
        await updateDoctor(editing.id, input);
        setMessage("Doctor updated.");
      } else {
        await createDoctor(input);
        setMessage("Doctor added.");
      }
      setFormOpen(false); setEditing(null); setValues(EMPTY_DOCTOR);
      await load(search);
    } catch (error) {
      const parsed = commandError(error);
      if (parsed.field === "name") {
        setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." }));
      } else {
        setSaveError(parsed.message ?? "The doctor could not be saved.");
      }
    } finally {
      submissionLock.current = false; setBusy(false);
    }
  }

  return <section className="workspace master-data-workspace" aria-labelledby="doctors-heading">
    <MasterDataHeading eyebrow="Master data" title="Doctors" pageKey="doctors" action="Add doctor" onAction={beginCreate} busy={busy} />
    <p className="master-data-note">Doctor names are available in local order forms. Records are edited in place and are not deleted.</p>
    {message && <div className="auth-success" role="status">{message}</div>}
    {saveError && <div className="auth-error" role="alert">{saveError}</div>}
    {formOpen && <MasterDataEditor title="Add doctor" busy={busy} onCancel={() => { if (!busy) { setFormOpen(false); setEditing(null); setErrors({}); } }} onSubmit={(event) => void submit(event)}>
      <MasterDataField label="Doctor name" error={errors.name}><input autoFocus value={values.name} disabled={busy} onChange={(event) => { setValues((current) => ({ ...current, name: event.target.value })); setErrors((current) => ({ ...current, name: undefined })); }} /></MasterDataField>
    </MasterDataEditor>}
    <SearchPanel value={search} onChange={setSearch} count={records.length} noun="doctor" />
    {loadError && <LoadError message={loadError} onRetry={() => void load(search)} />}
    {!loadError && <DoctorTable records={records} loading={loading} onEdit={beginEdit} editor={editing ? {
      recordId: editing.id,
      values,
      errors,
      busy,
      onChange: (name) => { setValues({ name }); setErrors((current) => ({ ...current, name: undefined })); },
      onCancel: () => { if (!busy) { setEditing(null); setErrors({}); setSaveError(null); } },
      onSubmit: (event) => void submit(event),
    } : undefined} />}
  </section>;
}

export function WardsPage() {
  const [records, setRecords] = useState<WardRecord[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editing, setEditing] = useState<WardRecord | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [values, setValues] = useState<WardValues>(EMPTY_WARD);
  const [errors, setErrors] = useState<WardErrors>({});
  const [message, setMessage] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submissionLock = useRef(false);

  const load = useCallback(async (query: string) => {
    setLoading(true); setLoadError(null);
    try { setRecords(await listWards({ search: query.trim() || null })); }
    catch (error) { setRecords([]); setLoadError(commandError(error).message ?? "Wards could not be loaded."); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void load(search); }, [load, search]);

  function beginCreate() {
    setEditing(null); setValues(EMPTY_WARD); setErrors({}); setMessage(null); setSaveError(null); setFormOpen(true);
  }

  function beginEdit(record: WardRecord) {
    setEditing(record);
    setValues({ name: record.name, telephone: record.telephone ?? "" });
    setErrors({}); setMessage(null); setSaveError(null); setFormOpen(false);
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateWard(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || submissionLock.current) return;
    submissionLock.current = true; setBusy(true); setMessage(null); setSaveError(null);
    const input: WardInput = { name: values.name.trim(), legacyCode: editing?.legacyCode ?? null, telephone: values.telephone.trim() || null };
    try {
      if (editing) {
        await updateWard(editing.id, input);
        setMessage("Ward updated.");
      } else {
        await createWard(input);
        setMessage("Ward added.");
      }
      setFormOpen(false); setEditing(null); setValues(EMPTY_WARD);
      await load(search);
    } catch (error) {
      const parsed = commandError(error);
      if (parsed.field === "name" || parsed.field === "telephone") {
        setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." }));
      } else {
        setSaveError(parsed.message ?? "The ward could not be saved.");
      }
    } finally {
      submissionLock.current = false; setBusy(false);
    }
  }

  return <section className="workspace master-data-workspace" aria-labelledby="wards-heading">
    <MasterDataHeading eyebrow="Master data" title="Wards" pageKey="wards" action="Add ward" onAction={beginCreate} busy={busy} />
    <p className="master-data-note">Ward names and telephone references remain local to this workstation. Records are edited in place and are not deleted.</p>
    {message && <div className="auth-success" role="status">{message}</div>}
    {saveError && <div className="auth-error" role="alert">{saveError}</div>}
    {formOpen && <MasterDataEditor title="Add ward" busy={busy} onCancel={() => { if (!busy) { setFormOpen(false); setEditing(null); setErrors({}); } }} onSubmit={(event) => void submit(event)}>
      <MasterDataField label="Ward name" error={errors.name}><input autoFocus value={values.name} disabled={busy} onChange={(event) => { setValues((current) => ({ ...current, name: event.target.value })); setErrors((current) => ({ ...current, name: undefined })); }} /></MasterDataField>
      <MasterDataField label="Telephone (optional)" error={errors.telephone}><input value={values.telephone} disabled={busy} onChange={(event) => { setValues((current) => ({ ...current, telephone: event.target.value })); setErrors((current) => ({ ...current, telephone: undefined })); }} /></MasterDataField>
    </MasterDataEditor>}
    <SearchPanel value={search} onChange={setSearch} count={records.length} noun="ward" />
    {loadError && <LoadError message={loadError} onRetry={() => void load(search)} />}
    {!loadError && <WardTable records={records} loading={loading} onEdit={beginEdit} editor={editing ? {
      recordId: editing.id,
      values,
      errors,
      busy,
      onNameChange: (name) => { setValues((current) => ({ ...current, name })); setErrors((current) => ({ ...current, name: undefined })); },
      onTelephoneChange: (telephone) => { setValues((current) => ({ ...current, telephone })); setErrors((current) => ({ ...current, telephone: undefined })); },
      onCancel: () => { if (!busy) { setEditing(null); setErrors({}); setSaveError(null); } },
      onSubmit: (event) => void submit(event),
    } : undefined} />}
  </section>;
}

function MasterDataHeading({ eyebrow, title, pageKey, action, onAction, busy }: { eyebrow: string; title: string; pageKey: PageKey; action: string; onAction: () => void; busy: boolean }) {
  return <div className="page-heading"><div><p className="eyebrow">{eyebrow}</p><h1>{title}</h1><PageDescription pageKey={pageKey} /></div><button className="button button--primary" type="button" disabled={busy} onClick={onAction}>{action}</button></div>;
}

function MasterDataEditor({ title, busy, onCancel, onSubmit, children }: { title: string; busy: boolean; onCancel: () => void; onSubmit: (event: React.FormEvent) => void; children: React.ReactNode }) {
  return <section className="surface master-data-editor" aria-labelledby="master-data-editor-heading"><div><p className="eyebrow">Local lookup</p><h2 id="master-data-editor-heading">{title}</h2></div><form onSubmit={onSubmit} noValidate>{children}<div className="master-data-editor__actions"><button className="button button--secondary" type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="button button--primary" type="submit" disabled={busy}>{busy ? "Saving…" : "Save"}</button></div></form></section>;
}

function MasterDataField({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) {
  return <label className="form-field"><span className="field-label">{label}</span>{children}{error && <span className="field-error">{error}</span>}</label>;
}

function SearchPanel({ value, onChange, count, noun }: { value: string; onChange: (value: string) => void; count: number; noun: string }) {
  return <div className="list-card master-data-search"><div className="list-toolbar"><label className="search-field"><span className="search-icon" aria-hidden="true">⌕</span><span className="sr-only">Search {noun}s</span><input value={value} placeholder={`Search ${noun} name${noun === "ward" ? " or telephone" : ""}`} onChange={(event) => onChange(event.target.value)} />{value && <button className="clear-search" type="button" aria-label="Clear search" onClick={() => onChange("")}>×</button>}</label><span className="result-count">{count} {count === 1 ? noun : `${noun}s`}</span></div></div>;
}

function LoadError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return <div className="form-error-summary" role="alert">{message} <button className="button button--compact button--secondary" type="button" onClick={onRetry}>Retry</button></div>;
}

type DoctorInlineEditor = {
  recordId: number;
  values: DoctorValues;
  errors: DoctorErrors;
  busy: boolean;
  onChange: (name: string) => void;
  onCancel: () => void;
  onSubmit: (event: React.FormEvent) => void;
};

type WardInlineEditor = {
  recordId: number;
  values: WardValues;
  errors: WardErrors;
  busy: boolean;
  onNameChange: (name: string) => void;
  onTelephoneChange: (telephone: string) => void;
  onCancel: () => void;
  onSubmit: (event: React.FormEvent) => void;
};

export function DoctorTable({ records, loading, onEdit, editor }: { records: DoctorRecord[]; loading: boolean; onEdit: (record: DoctorRecord) => void; editor?: DoctorInlineEditor }) {
  if (loading) return <div className="detail-loading" aria-busy="true">Loading doctors…</div>;
  if (records.length === 0) return <div className="empty-state"><h2>No doctors found</h2><p>Add a doctor or clear the search.</p></div>;
  return <div className="list-card"><div className="table-scroll"><table className="patient-table master-data-table"><thead><tr><th>Doctor name</th><th aria-label="Actions" /></tr></thead><tbody>{records.map((record) => editor?.recordId === record.id
    ? <DoctorEditRow key={record.id} editor={editor} />
    : <tr key={record.id}><td><strong>{record.name}</strong></td><td><EditButton label={`Edit doctor ${record.name}`} onClick={() => onEdit(record)} /></td></tr>)}</tbody></table></div></div>;
}

export function WardTable({ records, loading, onEdit, editor }: { records: WardRecord[]; loading: boolean; onEdit: (record: WardRecord) => void; editor?: WardInlineEditor }) {
  if (loading) return <div className="detail-loading" aria-busy="true">Loading wards…</div>;
  if (records.length === 0) return <div className="empty-state"><h2>No wards found</h2><p>Add a ward or clear the search.</p></div>;
  return <div className="list-card"><div className="table-scroll"><table className="patient-table master-data-table master-data-table--ward"><thead><tr><th>Ward name</th><th>Telephone</th><th aria-label="Actions" /></tr></thead><tbody>{records.map((record) => editor?.recordId === record.id
    ? <WardEditRow key={record.id} editor={editor} />
    : <tr key={record.id}><td><strong>{record.name}</strong></td><td>{record.telephone ?? <span className="muted-value">Not set</span>}</td><td><EditButton label={`Edit ward ${record.name}`} onClick={() => onEdit(record)} /></td></tr>)}</tbody></table></div></div>;
}

function DoctorEditRow({ editor }: { editor: DoctorInlineEditor }) {
  return <tr className="master-data-inline-row"><td colSpan={2}><form className="master-data-inline-editor master-data-inline-editor--doctor" onSubmit={editor.onSubmit} noValidate>
    <label className="master-data-inline-field"><span className="sr-only">Doctor name</span><input autoFocus value={editor.values.name} disabled={editor.busy} aria-invalid={Boolean(editor.errors.name)} onChange={(event) => editor.onChange(event.target.value)} />{editor.errors.name && <span className="field-error">{editor.errors.name}</span>}</label>
    <InlineEditorActions busy={editor.busy} onCancel={editor.onCancel} />
  </form></td></tr>;
}

function WardEditRow({ editor }: { editor: WardInlineEditor }) {
  return <tr className="master-data-inline-row"><td colSpan={3}><form className="master-data-inline-editor master-data-inline-editor--ward" onSubmit={editor.onSubmit} noValidate>
    <label className="master-data-inline-field"><span className="sr-only">Ward name</span><input autoFocus value={editor.values.name} disabled={editor.busy} aria-invalid={Boolean(editor.errors.name)} onChange={(event) => editor.onNameChange(event.target.value)} />{editor.errors.name && <span className="field-error">{editor.errors.name}</span>}</label>
    <label className="master-data-inline-field"><span className="sr-only">Telephone (optional)</span><input value={editor.values.telephone} disabled={editor.busy} aria-invalid={Boolean(editor.errors.telephone)} onChange={(event) => editor.onTelephoneChange(event.target.value)} />{editor.errors.telephone && <span className="field-error">{editor.errors.telephone}</span>}</label>
    <InlineEditorActions busy={editor.busy} onCancel={editor.onCancel} />
  </form></td></tr>;
}

function InlineEditorActions({ busy, onCancel }: { busy: boolean; onCancel: () => void }) {
  return <div className="master-data-inline-actions"><button className="button button--secondary button--compact" type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="button button--primary button--compact" type="submit" disabled={busy}>{busy ? "Saving…" : "Save"}</button></div>;
}

function EditButton({ label, onClick }: { label: string; onClick: () => void }) {
  return <button className="row-action master-data-edit-button" type="button" aria-label={label} title={label} onClick={onClick}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4.2L19 9.2a2.1 2.1 0 0 0 0-3L17.8 5a2.1 2.1 0 0 0-3 0L4 15.8V20Z" /><path d="m13.5 6.5 4 4" /></svg></button>;
}

export function validateDoctor(values: DoctorValues): DoctorErrors {
  const errors: DoctorErrors = {};
  if (!values.name.trim() || [...values.name.trim()].length > 200) errors.name = "Doctor name is required and limited to 200 characters.";
  return errors;
}

export function validateWard(values: WardValues): WardErrors {
  const errors: WardErrors = validateDoctor(values);
  if ([...values.telephone.trim()].length > 100) errors.telephone = "Telephone is limited to 100 characters.";
  return errors;
}
