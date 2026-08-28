import { useEffect, useMemo, useState } from "react";

import {
  commandError,
  createPatient,
  getPatientFormOptions,
  updatePatient,
} from "../api/commands";
import { BuddhistDateInput } from "../components/BuddhistDateInput";
import { PageDescription } from "../guidance/PageGuidance";
import type {
  PatientDetail,
  PatientFormOptions,
} from "../types/patient";
import {
  emptyPatientForm,
  formToInput,
  patientToForm,
  type FormErrors,
  type PatientFormValues,
  validatePatientForm,
} from "./form";
import { calculateAgeYears } from "./age";

interface PatientFormProps {
  patient?: PatientDetail;
  initialHn?: string;
  onCancel: () => void;
  onSaved: (patient: PatientDetail) => void;
}

const STANDARD_SEX_VALUES = new Set(["Male", "Female"]);

export function PatientForm({ patient, initialHn, onCancel, onSaved }: PatientFormProps) {
  const [values, setValues] = useState<PatientFormValues>(() =>
    patient
      ? patientToForm(patient)
      : { ...emptyPatientForm, hn: initialHn?.trim() ?? "" },
  );
  const [errors, setErrors] = useState<FormErrors>({});
  const [options, setOptions] = useState<PatientFormOptions | null>(null);
  const [optionsError, setOptionsError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void getPatientFormOptions()
      .then((response) => {
        if (active) setOptions(response);
      })
      .catch((error: unknown) => {
        if (active) {
          setOptionsError(
            commandError(error).message ?? "Unable to load diagnosis and regimen options.",
          );
        }
      });
    return () => {
      active = false;
    };
  }, []);

  const heading = patient ? "Edit patient" : "New patient";
  const hasErrors = useMemo(() => Object.keys(errors).length > 0, [errors]);
  const hasLegacySexValue = values.sex !== "" && !STANDARD_SEX_VALUES.has(values.sex);

  function setField<K extends keyof PatientFormValues>(
    field: K,
    value: PatientFormValues[K],
  ) {
    setValues((current) => ({ ...current, [field]: value }));
    setErrors((current) => {
      if (!current[field]) return current;
      const next = { ...current };
      delete next[field];
      return next;
    });
    setSaveError(null);
  }

  function setBirthDate(value: string) {
    setValues((current) => ({
      ...current,
      birthDate: value,
      ageYears: value ? calculateAgeYears(value)?.toString() ?? "" : current.ageYears,
    }));
    setErrors((current) => {
      const next = { ...current };
      delete next.birthDate;
      delete next.ageYears;
      return next;
    });
    setSaveError(null);
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextErrors = validatePatientForm(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) {
      window.requestAnimationFrame(() => {
        document.querySelector<HTMLElement>("[aria-invalid='true']")?.focus();
      });
      return;
    }

    setSaving(true);
    setSaveError(null);
    try {
      const input = formToInput(values);
      const saved = patient
        ? await updatePatient(patient.id, input)
        : await createPatient(input);
      onSaved(saved);
    } catch (error) {
      const failure = commandError(error);
      if (failure.field && failure.field in values) {
        setErrors((current) => ({
          ...current,
          [failure.field as keyof PatientFormValues]: failure.message ?? "Invalid value.",
        }));
      }
      setSaveError(failure.message ?? "The patient could not be saved.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="workspace form-workspace" aria-labelledby="patient-form-heading">
      <button className="back-button" type="button" onClick={onCancel}>← Cancel</button>
      <div className="page-heading form-heading">
        <div>
          <p className="eyebrow">Patient record</p>
          <h1 id="patient-form-heading">{heading}</h1>
          <PageDescription pageKey="patient_form" />
        </div>
        {patient && <span className="identity-chip"><b>HN</b> {patient.hn}</span>}
      </div>

      {optionsError && (
        <div className="inline-alert" role="alert">
          {optionsError} Diagnosis and regimen fields are temporarily unavailable.
        </div>
      )}
      {saveError && (
        <div className="inline-alert inline-alert--error" role="alert">{saveError}</div>
      )}

      <form className="patient-form patient-record-form" onSubmit={(event) => void submit(event)} noValidate>
        <FormSection
          title="Identity"
          description="HN is the required clinical identifier."
          gridClassName="patient-form-grid identity-grid"
        >
          <Field label="HN" required error={errors.hn} className="identity-grid__third">
            <input
              autoFocus
              value={values.hn}
              onChange={(event) => setField("hn", event.target.value)}
              aria-invalid={Boolean(errors.hn)}
              aria-describedby={errors.hn ? "hn-error" : undefined}
              maxLength={64}
            />
          </Field>
          <Field label="CA number" className="identity-grid__third">
            <input value={values.cancerNo} onChange={(event) => setField("cancerNo", event.target.value)} />
          </Field>
          <Field label="Birth date" error={errors.birthDate} className="identity-grid__half">
            <BuddhistDateInput value={values.birthDate} onChange={setBirthDate} invalid={Boolean(errors.birthDate)} describedBy={errors.birthDate ? "birth-date-error" : "birth-date-hint"} />
            <small id="birth-date-hint" className="form-field__hint">Selecting a birth date calculates age automatically.</small>
          </Field>
          <Field label="Exact age" hint="years" error={errors.ageYears} className="identity-grid__half">
            <input
              inputMode="numeric"
              type="number"
              min="0"
              max="150"
              step="1"
              value={values.ageYears}
              readOnly={Boolean(values.birthDate)}
              aria-readonly={Boolean(values.birthDate)}
              aria-invalid={Boolean(errors.ageYears)}
              aria-describedby={errors.ageYears ? "exact-age-error" : "exact-age-hint"}
              onChange={(event) => setField("ageYears", event.target.value)}
            />
            <small id="exact-age-hint" className="form-field__hint">
              {values.birthDate ? "Calculated from birth date." : "Enter age when birth date is unavailable."}
            </small>
          </Field>
          <Field label="Title" className="identity-grid__third">
            <input value={values.title} onChange={(event) => setField("title", event.target.value)} />
          </Field>
          <Field label="First name" className="identity-grid__third">
            <input value={values.firstName} onChange={(event) => setField("firstName", event.target.value)} />
          </Field>
          <Field label="Last name" className="identity-grid__third">
            <input value={values.lastName} onChange={(event) => setField("lastName", event.target.value)} />
          </Field>
          <Field label="Sex" className="identity-grid__half">
            <select value={values.sex} onChange={(event) => setField("sex", event.target.value)}>
              <option value="">Not specified</option>
              <option value="Male">Male</option>
              <option value="Female">Female</option>
              {hasLegacySexValue && <option value={values.sex}>{values.sex} (legacy value)</option>}
            </select>
          </Field>
          <Field label="Telephone" className="identity-grid__half">
            <input value={values.telephone} onChange={(event) => setField("telephone", event.target.value)} />
          </Field>
        </FormSection>

        <FormSection title="Measurements" gridClassName="patient-form-grid measurements-grid">
          <Field label="Weight" hint="kg" error={errors.weightKg}>
            <input
              inputMode="decimal"
              type="number"
              min="0"
              max="500"
              step="0.01"
              value={values.weightKg}
              onChange={(event) => setField("weightKg", event.target.value)}
              aria-invalid={Boolean(errors.weightKg)}
            />
          </Field>
          <Field label="Height" hint="cm" error={errors.heightCm}>
            <input
              inputMode="decimal"
              type="number"
              min="0"
              max="300"
              step="0.01"
              value={values.heightCm}
              onChange={(event) => setField("heightCm", event.target.value)}
              aria-invalid={Boolean(errors.heightCm)}
            />
          </Field>
        </FormSection>

        <FormSection
          title="Clinical"
          description="Select only values supported by the local master data."
          gridClassName="patient-form-grid clinical-grid"
        >
          <Field label="Diagnosis" className="clinical-grid__primary">
            <select
              value={values.diagnosisId}
              onChange={(event) => setField("diagnosisId", event.target.value)}
              disabled={!options}
            >
              <option value="">Not recorded</option>
              {options?.diagnoses.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Regimen" className="clinical-grid__primary">
            <select
              value={values.regimenId}
              onChange={(event) => setField("regimenId", event.target.value)}
              disabled={!options}
            >
              <option value="">Not recorded</option>
              {options?.regimens.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Stage" className="clinical-grid__tertiary">
            <input value={values.stage} onChange={(event) => setField("stage", event.target.value)} />
          </Field>
          <Field label="HER2" className="clinical-grid__tertiary">
            <input value={values.her2} onChange={(event) => setField("her2", event.target.value)} />
          </Field>
          <Field label="ER/PR" className="clinical-grid__tertiary">
            <input value={values.erpr} onChange={(event) => setField("erpr", event.target.value)} />
          </Field>
          <Field label="Allergy" wide>
            <textarea
              className="patient-textarea--short"
              rows={2}
              value={values.allergy}
              onChange={(event) => setField("allergy", event.target.value)}
            />
          </Field>
          <label className="checkbox-field">
            <input
              type="checkbox"
              checked={values.counselling}
              onChange={(event) => setField("counselling", event.target.checked)}
            />
            <span>Patient counselling recorded</span>
          </label>
          <label className="checkbox-field">
            <input
              type="checkbox"
              checked={values.appointmentCard}
              onChange={(event) => setField("appointmentCard", event.target.checked)}
            />
            <span>Appointment card enabled</span>
          </label>
        </FormSection>

        <FormSection title="Other information" gridClassName="patient-form-grid other-information-grid">
          <Field label="Occupation" className="other-information-grid__half">
            <input value={values.occupation} onChange={(event) => setField("occupation", event.target.value)} />
          </Field>
          <Field label="Address" wide>
            <textarea
              className="patient-textarea--short"
              rows={2}
              value={values.address}
              onChange={(event) => setField("address", event.target.value)}
            />
          </Field>
          <Field label="Patient history" wide>
            <textarea
              className="patient-textarea--medium"
              rows={3}
              value={values.patientHistory}
              onChange={(event) => setField("patientHistory", event.target.value)}
            />
          </Field>
          <Field label="Treatment status" className="other-information-grid__half">
            <select
              value={values.treatmentEnded}
              onChange={(event) =>
                setField(
                  "treatmentEnded",
                  event.target.value as PatientFormValues["treatmentEnded"],
                )
              }
            >
              <option value="">Not recorded</option>
              <option value="false">Active / ongoing</option>
              <option value="true">Treatment ended</option>
            </select>
          </Field>
          <Field label="Treatment end date" error={errors.treatmentEndDate} className="other-information-grid__half">
            <BuddhistDateInput value={values.treatmentEndDate} onChange={(value) => setField("treatmentEndDate", value)} invalid={Boolean(errors.treatmentEndDate)} describedBy={errors.treatmentEndDate ? "treatment-end-date-error" : undefined} />
          </Field>
        </FormSection>

        {hasErrors && (
          <p className="form-error-summary" role="alert">
            Review the highlighted fields before saving.
          </p>
        )}
        <div className="form-actions">
          <button className="button button--secondary" type="button" onClick={onCancel} disabled={saving}>
            Cancel
          </button>
          <button className="button button--primary" type="submit" disabled={saving}>
            {saving ? "Saving…" : patient ? "Save changes" : "Create patient"}
          </button>
        </div>
      </form>
    </section>
  );
}

function FormSection({
  title,
  description,
  gridClassName,
  children,
}: {
  title: string;
  description?: string;
  gridClassName?: string;
  children: React.ReactNode;
}) {
  return (
    <fieldset className="form-section">
      <legend>{title}</legend>
      {description && <p className="form-section__description">{description}</p>}
      <div className={`form-grid ${gridClassName ?? ""}`.trim()}>{children}</div>
    </fieldset>
  );
}

function Field({
  label,
  required = false,
  wide = false,
  className,
  hint,
  error,
  children,
}: {
  label: string;
  required?: boolean;
  wide?: boolean;
  className?: string;
  hint?: string;
  error?: string;
  children: React.ReactNode;
}) {
  const errorId = `${label.toLowerCase().replace(/\W+/g, "-")}-error`;
  return (
    <label className={`form-field ${wide ? "is-wide" : ""} ${className ?? ""}`.trim()}>
      <span className="field-label">
        {label} {required && <em>Required</em>} {hint && <small>{hint}</small>}
      </span>
      {children}
      {error && <span id={errorId} className="field-error">{error}</span>}
    </label>
  );
}
