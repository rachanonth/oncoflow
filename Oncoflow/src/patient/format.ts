import type { PatientDetail, PatientSummary } from "../types/patient";

export { displayDateTime } from "../shared/dateTime";

type PatientName = Pick<PatientSummary, "title" | "firstName" | "lastName">;

export function patientName(patient: PatientName): string {
  const name = [patient.title, patient.firstName, patient.lastName]
    .filter((part): part is string => Boolean(part?.trim()))
    .join(" ");
  return name || "Name unavailable";
}

export function displayValue(value: string | number | null): string {
  if (value === null || value === "") return "—";
  return String(value);
}

export function detailInitials(patient: PatientDetail): string {
  const values = [patient.firstName, patient.lastName]
    .filter((value): value is string => Boolean(value))
    .map((value) => Array.from(value.trim())[0])
    .filter(Boolean)
    .join("");
  return values.slice(0, 2).toUpperCase() || "P";
}
