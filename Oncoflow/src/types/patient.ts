export type PatientSortField = "hn" | "name" | "lastUpdated";
export type SortDirection = "asc" | "desc";

export interface PatientListRequest {
  search?: string | null;
  sortBy: PatientSortField;
  sortDirection: SortDirection;
  limit: number;
  offset: number;
}

export interface PatientSummary {
  id: number;
  hn: string;
  title: string | null;
  firstName: string | null;
  lastName: string | null;
  diagnosis: string | null;
  regimen: string | null;
  lastUpdated: string | null;
}

export interface PatientListResponse {
  items: PatientSummary[];
  total: number;
}

export interface PatientDetail {
  id: number;
  hn: string;
  cancerNo: string | null;
  title: string | null;
  firstName: string | null;
  lastName: string | null;
  sex: string | null;
  telephone: string | null;
  weightKg: number | null;
  heightCm: number | null;
  birthDate: string | null;
  ageYears: number | null;
  occupation: string | null;
  address: string | null;
  diagnosisId: number | null;
  diagnosis: string | null;
  regimenId: number | null;
  regimen: string | null;
  stage: string | null;
  her2: string | null;
  erpr: string | null;
  allergy: string | null;
  patientHistory: string | null;
  counselling: boolean;
  appointmentCard: boolean;
  treatmentEnded: boolean | null;
  treatmentEndDate: string | null;
  recordBy: string | null;
  recordTime: string | null;
}

export interface PatientInput {
  hn: string;
  cancerNo: string | null;
  title: string | null;
  firstName: string | null;
  lastName: string | null;
  sex: string | null;
  telephone: string | null;
  weightKg: number | null;
  heightCm: number | null;
  birthDate: string | null;
  ageYears: number | null;
  occupation: string | null;
  address: string | null;
  diagnosisId: number | null;
  regimenId: number | null;
  stage: string | null;
  her2: string | null;
  erpr: string | null;
  allergy: string | null;
  patientHistory: string | null;
  counselling: boolean;
  appointmentCard: boolean;
  treatmentEnded: boolean | null;
  treatmentEndDate: string | null;
}

export interface LookupOption {
  id: number;
  code: string | null;
  label: string;
}

export interface PatientFormOptions {
  diagnoses: LookupOption[];
  regimens: LookupOption[];
}
