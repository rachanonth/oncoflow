import { useState } from "react";
import { commandError } from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import type { OrderItemDetail, OrderItemInput, OrderLookups } from "../types/order";
import { convertRateValue, emptyOrderItemValues, itemToValues, toOrderItemInput, validateOrderItem, type FormErrors, type OrderItemFormValues, type RateUnit } from "./form";

const HOUR_OPTIONS = Array.from({ length: 24 }, (_, hour) => `${String(hour).padStart(2, "0")}:00`);

export function OrderItemEditor({ item, lookups, onCancel, onSave }: { item?: OrderItemDetail; lookups: OrderLookups; onCancel: () => void; onSave: (input: OrderItemInput) => Promise<void> }) {
  const [values, setValues] = useState<OrderItemFormValues>(() => item ? itemToValues(item) : emptyOrderItemValues());
  const [errors, setErrors] = useState<FormErrors>({}); const [serverError, setServerError] = useState<string | null>(null); const [saving, setSaving] = useState(false);
  function setField(field: keyof OrderItemFormValues, value: string | boolean) { setValues((current) => ({ ...current, [field]: value })); setErrors((current) => ({ ...current, [field]: "" })); }
  function selectDiluent(diluentId: string) {
    setValues((current) => ({ ...current, diluentId, diluentVolumeMl: diluentVolumeFromMaster(diluentId, lookups.diluents) }));
    setErrors((current) => ({ ...current, diluentId: "", diluentVolumeMl: "" }));
  }
  function setRateValue(rateValue: string) {
    setValues((current) => ({ ...current, rateValue, rateOriginal: "", rateTouched: true }));
    setErrors((current) => ({ ...current, rateValue: "" }));
  }
  function setRateUnit(rateUnit: RateUnit) {
    setValues((current) => ({ ...current, rateValue: convertRateValue(current.rateValue, current.rateUnit, rateUnit), rateUnit, rateOriginal: "", rateTouched: true }));
    setErrors((current) => ({ ...current, rateValue: "" }));
  }
  async function submit(event: React.FormEvent) { event.preventDefault(); const next = validateOrderItem(values); setErrors(next); if (Object.keys(next).length) return; setSaving(true); setServerError(null); try { await onSave(toOrderItemInput(values)); } catch (error) { const parsed = commandError(error); if (parsed.field) setErrors((current) => ({ ...current, [parsed.field!]: parsed.message ?? "Invalid value." })); setServerError(parsed.message ?? "Unable to save drug."); } finally { setSaving(false); } }
  const existingSchedule = values.scheduleTime && !HOUR_OPTIONS.includes(values.scheduleTime) ? values.scheduleTime : null;
  const unparsedRate = !values.rateTouched && values.rateOriginal && !values.rateValue ? values.rateOriginal : null;
  return <form className="inline-editor" onSubmit={submit} noValidate><div className="inline-editor__heading"><div><p className="eyebrow">Order drug</p><h2>{item ? "Edit drug" : "Add drug"}</h2></div><button className="button button--secondary button--compact" type="button" onClick={onCancel}>Cancel</button></div>{serverError && <div className="form-error-summary" role="alert">{serverError}</div>}<div className="form-grid regimen-item-form-grid">
    <Field label="Drug" required error={errors.drugId}><select value={values.drugId} onChange={(event) => setField("drugId", event.target.value)}><option value="">Select drug</option>{lookups.drugs.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>
    <Field label="Dose" error={errors.doseText}><div className="input-with-unit"><input value={values.doseText} maxLength={100} inputMode="decimal" onChange={(event) => setField("doseText", event.target.value)} placeholder="e.g. 100" /><span>mg</span></div></Field>
    <Field label="Route"><select value={values.routeId} onChange={(event) => setField("routeId", event.target.value)}><option value="">Not set</option>{lookups.routes.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>
    <Field label="Diluent"><select value={values.diluentId} onChange={(event) => selectDiluent(event.target.value)}><option value="">Not set</option>{lookups.diluents.map((option) => <option key={option.id} value={option.id}>{option.label}{option.volumeMl === null ? "" : ` (${option.volumeMl} mL)`}</option>)}</select></Field>
    <Field label="Diluent volume" hint="mL" error={errors.diluentVolumeMl}><input type="number" min="0" step="any" value={values.diluentVolumeMl} onChange={(event) => setField("diluentVolumeMl", event.target.value)} /></Field>
    <Field label="Rate" error={errors.rateValue}><div className="rate-control"><div className="input-with-unit"><input type="number" min="0" step="any" value={values.rateValue} onChange={(event) => setRateValue(event.target.value)} /><span>{values.rateUnit === "minute" ? "min" : "hr"}</span></div><div className="rate-unit-toggle" role="group" aria-label="Rate unit"><button className={values.rateUnit === "minute" ? "is-active" : ""} type="button" aria-pressed={values.rateUnit === "minute"} onClick={() => setRateUnit("minute")}>Minutes</button><button className={values.rateUnit === "hour" ? "is-active" : ""} type="button" aria-pressed={values.rateUnit === "hour"} onClick={() => setRateUnit("hour")}>Hours</button></div>{unparsedRate && <small>Existing value: {unparsedRate}. Enter a number to replace it.</small>}</div></Field>
    <Field label="Schedule time" error={errors.scheduleTime}><select value={values.scheduleTime} onChange={(event) => setField("scheduleTime", event.target.value)}><option value="">Not set</option>{existingSchedule && <option value={existingSchedule}>{existingSchedule} (existing)</option>}{HOUR_OPTIONS.map((time) => <option key={time} value={time}>{time}</option>)}</select></Field>
    <Field label="Start date"><BuddhistDateInput value={values.startDate} onChange={(value) => setField("startDate", value)} /></Field>
    <Field label="Stop date" error={errors.stopDate}><BuddhistDateInput value={values.stopDate} onChange={(value) => setField("stopDate", value)} invalid={Boolean(errors.stopDate)} /></Field>
  </div><div className="inline-editor__actions"><button className="button button--primary drug-save-button" type="submit" disabled={saving} aria-label={saving ? "Saving drug" : "Save drug"} title={saving ? "Saving drug" : "Save drug"}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 3h12l2 2v16H5V3Z" /><path d="M8 3v6h8V3" /><path d="M8 21v-7h8v7" /></svg></button></div></form>;
}
function Field({ label, hint, required, error, children }: { label: string; hint?: string; required?: boolean; error?: string; children: React.ReactNode }) { return <label className="form-field"><span className="field-label">{label}{required && <em>Required</em>}{hint && <small>{hint}</small>}</span>{children}{error && <span className="field-error">{error}</span>}</label>; }

export function diluentVolumeFromMaster(diluentId: string, diluents: OrderLookups["diluents"]): string {
  const volume = diluents.find((option) => String(option.id) === diluentId)?.volumeMl;
  return volume === null || volume === undefined ? "" : String(volume);
}
