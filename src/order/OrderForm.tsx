import { useEffect, useRef, useState } from "react";
import { commandError, createOrder, getOrderLookups, updateOrder } from "../api/commands";
import { BuddhistDateTimeInput } from "../components/BuddhistDateInput";
import { PageDescription } from "../guidance/PageGuidance";
import type { OrderDetail, OrderLookups, PatientOrderLookupOption } from "../types/order";
import { acquireSubmissionLock, emptyOrderValues, orderToValues, toOrderInput, validateOrder, type FormErrors, type OrderFormValues } from "./form";
import { loadLatestOrderContext, saveLatestOrderContext } from "./preferences";

export function OrderForm({ order, initialPatientId, initialPatientHn, onCreatePatient, onCancel, onSaved }: { order?: OrderDetail; initialPatientId?: number; initialPatientHn?: string; onCreatePatient?: (hn: string) => void; onCancel: () => void; onSaved: (order: OrderDetail) => void }) {
  const [values, setValues] = useState<OrderFormValues>(() => {
    if (order) return orderToValues(order);
    return { ...emptyOrderValues(initialPatientId), ...loadLatestOrderContext() };
  });
  const [lookups, setLookups] = useState<OrderLookups | null>(null);
  const [patientHn, setPatientHn] = useState(initialPatientHn?.trim() ?? "");
  const [errors, setErrors] = useState<FormErrors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const submissionLock = useRef(false);
  useEffect(() => {
    let active = true;
    void getOrderLookups().then((value) => {
      if (!active) return;
      setLookups(value);
      if (!order && initialPatientId) {
        setPatientHn(value.patients.find((patient) => patient.id === initialPatientId)?.hn ?? "");
      }
      if (!order) {
        setValues((current) => ({
          ...current,
          doctorId: validRememberedId(current.doctorId, value.doctors),
          wardId: validRememberedId(current.wardId, value.wards),
        }));
      }
    }).catch((error: unknown) => active && setServerError(commandError(error).message ?? "Unable to load local lookups."));
    return () => { active = false; };
  }, [initialPatientId, order]);
  function setField(field: keyof OrderFormValues, value: string | boolean) { setValues((current) => ({ ...current, [field]: value })); setErrors((current) => ({ ...current, [field]: "" })); }
  function setHn(value: string) {
    setPatientHn(value);
    const patient = findPatientByHn(value, lookups?.patients ?? []);
    setField("patientId", patient ? String(patient.id) : "");
  }
  async function submit(event: React.FormEvent) { event.preventDefault(); const next = validateOrder(values); setErrors(next); if (Object.keys(next).length || !acquireSubmissionLock(submissionLock)) return; setSaving(true); setServerError(null); try { const input = toOrderInput(values); const saved = order ? await updateOrder(order.id, input) : await createOrder(input); if (!order) saveLatestOrderContext({ doctorId: values.doctorId, wardId: values.wardId }); onSaved(saved); } catch (error) { const parsed = commandError(error); if (parsed.field) setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." })); setServerError(parsed.message ?? "Unable to save order."); } finally { submissionLock.current = false; setSaving(false); } }
  if (!lookups && !serverError) return <section className="workspace"><div className="detail-loading"><div className="skeleton-block skeleton-block--hero"/><div className="skeleton-block"/></div></section>;
  const selectedPatient = lookups?.patients.find((patient) => String(patient.id) === values.patientId);
  return <section className="workspace order-workspace" aria-labelledby="order-form-heading"><button className="back-button" type="button" onClick={onCancel}>← {order ? "Order" : "Orders"}</button><div className="page-heading form-heading"><div><p className="eyebrow">{order ? "Editable local order" : "New local order"}</p><h1 id="order-form-heading">{order ? `Edit ${order.orderId}` : "Create order"}</h1><PageDescription pageKey="order_form" /></div></div>{serverError && <div className="form-error-summary" role="alert">{serverError}</div>}<form className="patient-form" onSubmit={submit} noValidate><fieldset className="form-section surface"><legend>Order</legend><p className="form-section__description">All options come from the local SQLite database.</p><div className="form-grid">
    {order ? <Field label="Patient" required error={errors.patientId}><select value={values.patientId} onChange={(event) => setField("patientId", event.target.value)}><option value="">Select patient</option>{lookups?.patients.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field> : <><Field label="HN" required error={errors.patientId}><input autoFocus list="order-patient-hns" value={patientHn} disabled={Boolean(initialPatientId)} autoComplete="off" aria-invalid={Boolean(errors.patientId)} placeholder="Enter patient HN" onChange={(event) => setHn(event.target.value)} onBlur={() => selectedPatient && setPatientHn(selectedPatient.hn)} /><datalist id="order-patient-hns">{lookups?.patients.map((patient) => <option key={patient.id} value={patient.hn}>{patient.label}</option>)}</datalist></Field><OrderPatientMatch patientHn={patientHn} selectedPatient={selectedPatient} onCreatePatient={onCreatePatient} /></>}
    <Field label="Order date / time" error={errors.orderTime}><BuddhistDateTimeInput value={values.orderTime} onChange={(value) => setField("orderTime", value)} invalid={Boolean(errors.orderTime)} /></Field>
    <Field label="Regimen" error={errors.regimenId}><select value={values.regimenId} onChange={(event) => setField("regimenId", event.target.value)}><option value="">Not set</option>{lookups?.regimens.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>
    <Field label="Doctor"><select value={values.doctorId} onChange={(event) => setField("doctorId", event.target.value)}><option value="">Not set</option>{lookups?.doctors.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>
    <Field label="Ward"><select value={values.wardId} onChange={(event) => setField("wardId", event.target.value)}><option value="">Not set</option>{lookups?.wards.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>
    <Field label="Preparation pharmacist" required error={errors.assignedPreparerUserId}><select value={values.assignedPreparerUserId} onChange={(event) => setField("assignedPreparerUserId", event.target.value)}><option value="">Select pharmacist</option>{lookups?.preparationPharmacists.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select>{lookups?.preparationPharmacists.length === 0 && <small className="field-error">Create or activate a pharmacist account before ordering.</small>}</Field>
    <Field label="Notes" wide><textarea maxLength={1000} value={values.note} onChange={(event) => setField("note", event.target.value)} /></Field>
    {order && <label className="checkbox-field"><input type="checkbox" checked={values.appointmentFlag} onChange={(event) => setField("appointmentFlag", event.target.checked)} />Legacy appointment flag</label>}
  </div></fieldset><div className="form-actions"><button className="button button--secondary" type="button" onClick={onCancel} disabled={saving}>Cancel</button><button className="button button--primary" type="submit" disabled={saving || !lookups}>{saving ? "Saving…" : order ? "Save order" : "Create order"}</button></div></form></section>;
}

export function OrderPatientMatch({ patientHn, selectedPatient, onCreatePatient }: { patientHn: string; selectedPatient?: PatientOrderLookupOption; onCreatePatient?: (hn: string) => void }) {
  const missing = Boolean(patientHn.trim()) && !selectedPatient;
  return <div className={`read-only-field order-patient-match ${missing ? "is-missing" : ""}`} aria-live="polite"><span>Patient name</span><strong>{selectedPatient?.label ?? (patientHn ? "Patient not found" : "Enter HN to find patient")}</strong>{missing && onCreatePatient && <button className="button button--secondary button--compact order-patient-match__action" type="button" onClick={() => onCreatePatient(patientHn.trim())}>Add new patient</button>}</div>;
}
function Field({ label, hint, required, error, wide, children }: { label: string; hint?: string; required?: boolean; error?: string; wide?: boolean; children: React.ReactNode }) { return <label className={`form-field ${wide ? "is-wide" : ""}`}><span className="field-label">{label}{required && <em>Required</em>}{hint && <small>{hint}</small>}</span>{children}{error && <span className="field-error">{error}</span>}</label>; }

function validRememberedId(value: string, options: Array<{ id: number }>): string {
  return !value || options.some((option) => String(option.id) === value) ? value : "";
}

export function findPatientByHn(hn: string, patients: PatientOrderLookupOption[]): PatientOrderLookupOption | undefined {
  const normalizedHn = hn.trim().toLocaleLowerCase();
  if (!normalizedHn) return undefined;
  return patients.find((patient) => patient.hn.trim().toLocaleLowerCase() === normalizedHn);
}
