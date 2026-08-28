import { useEffect, useMemo, useState } from "react";

import {
  commandError,
  createDrug,
  getDrugFormOptions,
  updateDrug,
} from "../api/commands";
import type { DrugDetail, DrugFormOptions } from "../types/drug";
import { PageDescription } from "../guidance/PageGuidance";
import {
  convertDurationValue,
  parseDuration,
  serializeDuration,
  type DurationUnit,
} from "../shared/duration";
import {
  drugToForm,
  emptyDrugForm,
  formToDrugInput,
  type DrugFormErrors,
  type DrugFormValues,
  type TriState,
  validateDrugForm,
  withSuggestedDrugCode,
} from "./form";

interface DrugFormProps {
  drug?: DrugDetail;
  onCancel: () => void;
  onSaved: (drug: DrugDetail) => void;
}

export function DrugForm({ drug, onCancel, onSaved }: DrugFormProps) {
  const isEditing = Boolean(drug);
  const [values, setValues] = useState<DrugFormValues>(() =>
    drug ? drugToForm(drug) : { ...emptyDrugForm },
  );
  const [errors, setErrors] = useState<DrugFormErrors>({});
  const [options, setOptions] = useState<DrugFormOptions | null>(null);
  const [optionsError, setOptionsError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void getDrugFormOptions()
      .then((response) => {
        if (!active) return;
        setOptions(response);
        if (!isEditing) {
          setValues((current) => withSuggestedDrugCode(current, response.suggestedCode));
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setOptionsError(
            commandError(error).message ?? "Unable to load local drug lookups.",
          );
        }
      });
    return () => {
      active = false;
    };
  }, [isEditing]);

  const hasErrors = useMemo(() => Object.keys(errors).length > 0, [errors]);

  function setField<K extends keyof DrugFormValues>(field: K, value: DrugFormValues[K]) {
    setValues((current) => ({ ...current, [field]: value }));
    setErrors((current) => {
      if (!current[field]) return current;
      const next = { ...current };
      delete next[field];
      return next;
    });
    setSaveError(null);
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextErrors = validateDrugForm(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) {
      window.requestAnimationFrame(() =>
        document.querySelector<HTMLElement>("[aria-invalid='true']")?.focus(),
      );
      return;
    }

    setSaving(true);
    setSaveError(null);
    try {
      const input = formToDrugInput(values);
      const saved = drug
        ? await updateDrug(drug.id, input)
        : await createDrug(input);
      onSaved(saved);
    } catch (error) {
      const failure = commandError(error);
      if (failure.field && failure.field in values) {
        setErrors((current) => ({
          ...current,
          [failure.field as keyof DrugFormValues]:
            failure.message ?? "Invalid value.",
        }));
      }
      setSaveError(failure.message ?? "The drug could not be saved.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="workspace form-workspace" aria-labelledby="drug-form-heading">
      <button className="back-button" type="button" onClick={onCancel}>← Cancel</button>
      <div className="page-heading form-heading">
        <div>
          <p className="eyebrow">Drug master record</p>
          <h1 id="drug-form-heading">{drug ? "Edit drug" : "New drug"}</h1>
          <PageDescription pageKey="drug_form" />
        </div>
      </div>

      {optionsError && <div className="inline-alert" role="alert">{optionsError} Lookup fields are temporarily unavailable.</div>}
      {saveError && <div className="inline-alert inline-alert--error" role="alert">{saveError}</div>}

      <form className="patient-form drug-record-form" onSubmit={(event) => void submit(event)} noValidate>
        <DrugFormSection title="Identity" description="Drug code is assigned automatically in the background and cannot be edited.">
          <DrugField label="Drug name" required error={errors.name} className="drug-grid__two-thirds">
            <input autoFocus maxLength={255} value={values.name} onChange={(event) => setField("name", event.target.value)} aria-invalid={Boolean(errors.name)} />
          </DrugField>
          <DrugField label="Unit" className="drug-grid__third">
            <select value={values.unitId} onChange={(event) => setField("unitId", event.target.value)} disabled={!options}>
              <option value="">Not set</option>
              {options?.units.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
            </select>
          </DrugField>
          <DrugField label="Package" className="drug-grid__third">
            <input value={values.package} onChange={(event) => setField("package", event.target.value)} />
          </DrugField>
          <DrugField label="Price" error={errors.price} className="drug-grid__third">
            <NumberInput value={values.price} onChange={(value) => setField("price", value)} invalid={Boolean(errors.price)} />
          </DrugField>
          <CheckField className="drug-grid__third" label="Drug record enabled" checked={values.marker} onChange={(value) => setField("marker", value)} />
        </DrugFormSection>

        <DrugFormSection title="Preparation" description="Defaults are selected from local SQLite lookup tables.">
          <DrugField label="Dose per pack" error={errors.dosePerPack} className="drug-grid__third">
            <NumberInput value={values.dosePerPack} onChange={(value) => setField("dosePerPack", value)} invalid={Boolean(errors.dosePerPack)} />
          </DrugField>
          <DrugField label="Volume per pack" hint="mL" error={errors.volumePerPackMl} className="drug-grid__third">
            <NumberInput value={values.volumePerPackMl} onChange={(value) => setField("volumePerPackMl", value)} invalid={Boolean(errors.volumePerPackMl)} />
          </DrugField>
          <DrugField label="Default diluent" className="drug-grid__third">
            <select value={values.defaultDiluentId} onChange={(event) => setField("defaultDiluentId", event.target.value)} disabled={!options}>
              <option value="">Not set</option>
              {options?.diluents.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
            </select>
          </DrugField>
          <DrugField label="Default route" className="drug-grid__third">
            <select value={values.defaultRouteId} onChange={(event) => setField("defaultRouteId", event.target.value)} disabled={!options}>
              <option value="">Not set</option>
              {options?.routes.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
            </select>
          </DrugField>
          <DrugField label="Default rate" error={errors.defaultRate} className="drug-grid__third">
            <DurationInput
              value={values.defaultRate}
              onChange={(value) => setField("defaultRate", value)}
              unitLabel="Default rate unit"
              invalid={Boolean(errors.defaultRate)}
            />
          </DrugField>
          <DrugField label="Expiry time" error={errors.expiryTime} className="drug-grid__third">
            <DurationInput
              value={values.expiryTime}
              onChange={(value) => setField("expiryTime", value)}
              unitLabel="Expiry time unit"
              defaultUnit="hour"
              allowClock
              invalid={Boolean(errors.expiryTime)}
            />
          </DrugField>
          <DrugField label="Preparation detail" className="drug-grid__half">
            <textarea className="drug-textarea--compact" rows={2} value={values.detail} onChange={(event) => setField("detail", event.target.value)} />
          </DrugField>
          <DrugField label="Storage" className="drug-grid__half">
            <textarea className="drug-textarea--compact" rows={2} value={values.storage} onChange={(event) => setField("storage", event.target.value)} />
          </DrugField>
          <DrugField label="Expiry storage" wide>
            <input value={values.expiryStorage} onChange={(event) => setField("expiryStorage", event.target.value)} />
          </DrugField>
        </DrugFormSection>

        <DrugFormSection title="Safety configuration" description="Raw legacy parameters only; no dose or compatibility calculation is performed.">
          <DrugField label="Maximum dose" error={errors.maxDose} className="drug-grid__third">
            <NumberInput value={values.maxDose} onChange={(value) => setField("maxDose", value)} invalid={Boolean(errors.maxDose)} />
          </DrugField>
          <DrugField label="Maximum dilution alert" className="drug-grid__third">
            <TriStateSelect value={values.maxDilutionAlert} onChange={(value) => setField("maxDilutionAlert", value)} />
          </DrugField>
          <DrugField label="Maximum dilution threshold" error={errors.maxDilutionHard} className="drug-grid__third">
            <NumberInput value={values.maxDilutionHard} onChange={(value) => setField("maxDilutionHard", value)} invalid={Boolean(errors.maxDilutionHard)} />
          </DrugField>
          <DrugField label="Cumulative alert" className="drug-grid__half">
            <TriStateSelect value={values.cumulativeAlert} onChange={(value) => setField("cumulativeAlert", value)} />
          </DrugField>
          <DrugField label="Cumulative threshold" error={errors.cumulativeAlertHard} className="drug-grid__half">
            <NumberInput value={values.cumulativeAlertHard} onChange={(value) => setField("cumulativeAlertHard", value)} invalid={Boolean(errors.cumulativeAlertHard)} />
          </DrugField>
          <DrugField label="Warning" className="drug-grid__half">
            <textarea className="drug-textarea--compact" rows={2} value={values.warning} onChange={(event) => setField("warning", event.target.value)} />
          </DrugField>
          <DrugField label="Dilution incompatibility" className="drug-grid__half">
            <textarea className="drug-textarea--compact" rows={2} value={values.dilutionIncompatibility} onChange={(event) => setField("dilutionIncompatibility", event.target.value)} />
          </DrugField>
        </DrugFormSection>

        <DrugFormSection title="Inventory configuration" description="Receiving and requisition workflows are outside this milestone.">
          <CheckField className="drug-grid__half" label="Inventory tracking enabled" checked={values.inventoryEnabled} onChange={(value) => setField("inventoryEnabled", value)} />
          <DrugField label="Cut-off flag" className="drug-grid__half">
            <TriStateSelect value={values.inventoryCut} onChange={(value) => setField("inventoryCut", value)} />
          </DrugField>
          <DrugField label="Minimum inventory" error={errors.inventoryMin} className="drug-grid__half">
            <NumberInput value={values.inventoryMin} onChange={(value) => setField("inventoryMin", value)} invalid={Boolean(errors.inventoryMin)} />
          </DrugField>
          <DrugField label="Maximum inventory" error={errors.inventoryMax} className="drug-grid__half">
            <NumberInput value={values.inventoryMax} onChange={(value) => setField("inventoryMax", value)} invalid={Boolean(errors.inventoryMax)} />
          </DrugField>
          {drug && (
            <div className="read-only-field drug-grid__half">
              <span>Current quantity</span>
              <strong>{drug.inventoryQuantity ?? "Not recorded"}</strong>
              <small>Read-only until the inventory workflow milestone.</small>
            </div>
          )}
        </DrugFormSection>

        {hasErrors && <p className="form-error-summary" role="alert">Review the highlighted fields before saving.</p>}
        <div className="form-actions">
          <button className="button button--secondary" type="button" onClick={onCancel} disabled={saving}>Cancel</button>
          <button className="button button--primary" type="submit" disabled={saving || (!drug && !options)}>{saving ? "Saving…" : drug ? "Save changes" : "Create drug"}</button>
        </div>
      </form>
    </section>
  );
}

function DrugFormSection({ title, description, children }: { title: string; description?: string; children: React.ReactNode }) {
  return <fieldset className="form-section"><legend>{title}</legend>{description && <p className="form-section__description">{description}</p>}<div className="form-grid drug-form-grid">{children}</div></fieldset>;
}

function DrugField({ label, required = false, wide = false, className, hint, error, children }: { label: string; required?: boolean; wide?: boolean; className?: string; hint?: string; error?: string; children: React.ReactNode }) {
  return (
    <label className={`form-field ${wide ? "is-wide" : ""} ${className ?? ""}`.trim()}>
      <span className="field-label">{label} {required && <em>Required</em>} {hint && <small>{hint}</small>}</span>
      {children}
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function NumberInput({ value, onChange, invalid }: { value: string; onChange: (value: string) => void; invalid: boolean }) {
  return <input type="number" min="0" step="any" inputMode="decimal" value={value} onChange={(event) => onChange(event.target.value)} aria-invalid={invalid} />;
}

function DurationInput({
  value,
  onChange,
  unitLabel,
  defaultUnit = "minute",
  allowClock = false,
  invalid,
}: {
  value: string;
  onChange: (value: string) => void;
  unitLabel: string;
  defaultUnit?: DurationUnit;
  allowClock?: boolean;
  invalid: boolean;
}) {
  const parsed = useMemo(
    () => parseDuration(value, { allowClock, defaultUnit }),
    [allowClock, defaultUnit, value],
  );
  const [unit, setUnit] = useState<DurationUnit>(() => parsed.unit);

  useEffect(() => {
    if (parsed.value !== null) setUnit(parsed.unit);
  }, [parsed.unit, parsed.value]);

  function changeUnit(nextUnit: DurationUnit) {
    if (parsed.value !== null) {
      onChange(serializeDuration(convertDurationValue(parsed.value, unit, nextUnit), nextUnit));
    }
    setUnit(nextUnit);
  }

  const unparsedValue = parsed.original && parsed.value === null ? parsed.original : null;
  return (
    <div className="rate-control">
      <div className="input-with-unit">
        <input
          type="number"
          min="0"
          step="any"
          inputMode="decimal"
          value={parsed.value ?? ""}
          onChange={(event) => onChange(serializeDuration(event.target.value, unit))}
          aria-invalid={invalid}
        />
        <span>{unit === "minute" ? "min" : "hr"}</span>
      </div>
      <div className="rate-unit-toggle" role="group" aria-label={unitLabel}>
        <button className={unit === "minute" ? "is-active" : ""} type="button" aria-pressed={unit === "minute"} onClick={() => changeUnit("minute")}>Minutes</button>
        <button className={unit === "hour" ? "is-active" : ""} type="button" aria-pressed={unit === "hour"} onClick={() => changeUnit("hour")}>Hours</button>
      </div>
      {unparsedValue && <small>Existing value: {unparsedValue}. Enter a number to replace it.</small>}
    </div>
  );
}

function TriStateSelect({ value, onChange }: { value: TriState; onChange: (value: TriState) => void }) {
  return (
    <select value={value} onChange={(event) => onChange(event.target.value as TriState)}>
      <option value="">Not recorded</option>
      <option value="true">Enabled</option>
      <option value="false">Disabled</option>
    </select>
  );
}

function CheckField({ label, checked, className, onChange }: { label: string; checked: boolean; className?: string; onChange: (value: boolean) => void }) {
  return <label className={`checkbox-field ${className ?? ""}`.trim()}><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span>{label}</span></label>;
}
