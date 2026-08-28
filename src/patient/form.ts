import type { PatientDetail, PatientInput } from "../types/patient";
import { calculateAgeYears } from "./age";

export interface PatientFormValues {
  hn: string;
  cancerNo: string;
  title: string;
  firstName: string;
  lastName: string;
  sex: string;
  telephone: string;
  weightKg: string;
  heightCm: string;
  birthDate: string;
  ageYears: string;
  occupation: string;
  address: string;
  diagnosisId: string;
  regimenId: string;
  stage: string;
  her2: string;
  erpr: string;
  allergy: string;
  patientHistory: string;
  counselling: boolean;
  appointmentCard: boolean;
  treatmentEnded: "" | "true" | "false";
  treatmentEndDate: string;
}

export type FormErrors = Partial<Record<keyof PatientFormValues, string>>;

export const emptyPatientForm: PatientFormValues = {
  hn: "",
  cancerNo: "",
  title: "",
  firstName: "",
  lastName: "",
  sex: "",
  telephone: "",
  weightKg: "",
  heightCm: "",
  birthDate: "",
  ageYears: "",
  occupation: "",
  address: "",
  diagnosisId: "",
  regimenId: "",
  stage: "",
  her2: "",
  erpr: "",
  allergy: "",
  patientHistory: "",
  counselling: false,
  appointmentCard: false,
  treatmentEnded: "",
  treatmentEndDate: "",
};

export function patientToForm(patient: PatientDetail): PatientFormValues {
  return {
    hn: patient.hn,
    cancerNo: patient.cancerNo ?? "",
    title: patient.title ?? "",
    firstName: patient.firstName ?? "",
    lastName: patient.lastName ?? "",
    sex: patient.sex ?? "",
    telephone: patient.telephone ?? "",
    weightKg: patient.weightKg?.toString() ?? "",
    heightCm: patient.heightCm?.toString() ?? "",
    birthDate: patient.birthDate ?? "",
    ageYears: (calculateAgeYears(patient.birthDate) ?? patient.ageYears)?.toString() ?? "",
    occupation: patient.occupation ?? "",
    address: patient.address ?? "",
    diagnosisId: patient.diagnosisId?.toString() ?? "",
    regimenId: patient.regimenId?.toString() ?? "",
    stage: patient.stage ?? "",
    her2: patient.her2 ?? "",
    erpr: patient.erpr ?? "",
    allergy: patient.allergy ?? "",
    patientHistory: patient.patientHistory ?? "",
    counselling: patient.counselling,
    appointmentCard: patient.appointmentCard,
    treatmentEnded:
      patient.treatmentEnded === null
        ? ""
        : patient.treatmentEnded
          ? "true"
          : "false",
    treatmentEndDate: patient.treatmentEndDate ?? "",
  };
}

export function validatePatientForm(values: PatientFormValues): FormErrors {
  const errors: FormErrors = {};
  if (!values.hn.trim()) errors.hn = "HN is required.";
  if (Array.from(values.hn.trim()).length > 64) {
    errors.hn = "HN must be 64 characters or fewer.";
  }
  validateNumber(values.weightKg, "weightKg", "Weight", 500, errors);
  validateNumber(values.heightCm, "heightCm", "Height", 300, errors);
  if (values.birthDate && !isValidDate(values.birthDate)) {
    errors.birthDate = "Enter a valid birth date.";
  }
  validateAge(values.ageYears, errors);
  if (values.treatmentEndDate && !isValidDate(values.treatmentEndDate)) {
    errors.treatmentEndDate = "Enter a valid treatment end date.";
  }
  return errors;
}

export function formToInput(values: PatientFormValues): PatientInput {
  const optional = (value: string) => value.trim() || null;
  const number = (value: string) => (value.trim() ? Number(value) : null);
  const id = (value: string) => (value ? Number(value) : null);
  return {
    hn: values.hn.trim(),
    cancerNo: optional(values.cancerNo),
    title: optional(values.title),
    firstName: optional(values.firstName),
    lastName: optional(values.lastName),
    sex: optional(values.sex),
    telephone: optional(values.telephone),
    weightKg: number(values.weightKg),
    heightCm: number(values.heightCm),
    birthDate: optional(values.birthDate),
    ageYears: number(values.ageYears),
    occupation: optional(values.occupation),
    address: optional(values.address),
    diagnosisId: id(values.diagnosisId),
    regimenId: id(values.regimenId),
    stage: optional(values.stage),
    her2: optional(values.her2),
    erpr: optional(values.erpr),
    allergy: optional(values.allergy),
    patientHistory: optional(values.patientHistory),
    counselling: values.counselling,
    appointmentCard: values.appointmentCard,
    treatmentEnded:
      values.treatmentEnded === "" ? null : values.treatmentEnded === "true",
    treatmentEndDate: optional(values.treatmentEndDate),
  };
}

function validateAge(value: string, errors: FormErrors) {
  if (!value.trim()) return;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0 || parsed > 150) {
    errors.ageYears = "Age must be a whole number from 0 to 150.";
  }
}

function validateNumber(
  value: string,
  field: "weightKg" | "heightCm",
  label: string,
  maximum: number,
  errors: FormErrors,
) {
  if (!value.trim()) return;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0 || parsed > maximum) {
    errors[field] = `${label} must be greater than 0 and no more than ${maximum}.`;
  }
}

function isValidDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}
