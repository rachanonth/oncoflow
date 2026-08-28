import { useState } from "react";

import { commandError, createRegimen, updateRegimen } from "../api/commands";
import { PageDescription } from "../guidance/PageGuidance";
import type { RegimenDetail } from "../types/regimen";
import { emptyRegimenValues, regimenToValues, toRegimenInput, validateRegimen, type FormErrors, type RegimenFormValues } from "./form";

interface Props { regimen?: RegimenDetail; onCancel: () => void; onSaved: (regimen: RegimenDetail) => void }

export function RegimenForm({ regimen, onCancel, onSaved }: Props) {
  const [values, setValues] = useState<RegimenFormValues>(() => regimen ? regimenToValues(regimen) : emptyRegimenValues);
  const [errors, setErrors] = useState<FormErrors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  function setField<K extends keyof RegimenFormValues>(field: K, value: RegimenFormValues[K]) { setValues((current) => ({ ...current, [field]: value })); setErrors((current) => ({ ...current, [field]: "" })); }
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const nextErrors = validateRegimen(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length) return;
    setSaving(true); setServerError(null);
    try { onSaved(regimen ? await updateRegimen(regimen.id, toRegimenInput(values)) : await createRegimen(toRegimenInput(values))); }
    catch (error) { const parsed = commandError(error); if (parsed.field) setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." })); setServerError(parsed.message ?? "Unable to save regimen."); }
    finally { setSaving(false); }
  }
  const flags: Array<[keyof RegimenFormValues, string]> = [["marker", "Marker"], ["flag", "Flag"], ["cycleCheck", "Cycle check"], ["autoMode", "Automatic mode"], ["drugAlert", "Drug alert"], ["appointmentAlert", "Appointment alert"], ["counselAlert", "Counselling alert"]];
  return <section className="workspace"><button className="back-button" type="button" onClick={onCancel}>← Cancel</button><div className="page-heading form-heading"><div><p className="eyebrow">Regimen master</p><h1>{regimen ? "Edit regimen" : "Create regimen"}</h1><PageDescription pageKey="regimen_form" /></div></div><form className="patient-form" onSubmit={submit} noValidate>{serverError && <div className="form-error-summary" role="alert">{serverError}</div>}<fieldset className="form-section"><legend>Identity</legend><div className="form-grid"><Field label="Regimen code" required error={errors.code}><input autoFocus maxLength={64} value={values.code} onChange={(event) => setField("code", event.target.value)} aria-invalid={Boolean(errors.code)} /></Field><Field label="Regimen name" required error={errors.name}><input maxLength={255} value={values.name} onChange={(event) => setField("name", event.target.value)} aria-invalid={Boolean(errors.name)} /></Field></div></fieldset><fieldset className="form-section"><legend>Legacy behavior flags</legend><p className="form-section__description">Stored configuration only. These switches do not execute clinical logic in this milestone.</p><div className="form-grid flag-grid">{flags.map(([field, label]) => <label className="checkbox-field" key={field}><input type="checkbox" checked={Boolean(values[field])} onChange={(event) => setField(field, event.target.checked)} />{label}</label>)}</div></fieldset><div className="form-actions"><button className="button button--secondary" type="button" onClick={onCancel} disabled={saving}>Cancel</button><button className="button button--primary" type="submit" disabled={saving}>{saving ? "Saving…" : regimen ? "Save regimen" : "Create regimen"}</button></div></form></section>;
}

function Field({ label, required, error, children }: { label: string; required?: boolean; error?: string; children: React.ReactNode }) { return <label className="form-field"><span className="field-label">{label}{required && <em>Required</em>}</span>{children}{error && <span className="field-error">{error}</span>}</label>; }
