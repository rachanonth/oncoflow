export interface MasterDataListRequest {
  search?: string | null;
}

export interface DoctorRecord {
  id: number;
  legacyCode: string | null;
  name: string;
}

export interface DoctorInput {
  legacyCode?: string | null;
  name: string;
}

export interface WardRecord {
  id: number;
  legacyCode: string | null;
  name: string;
  telephone: string | null;
}

export interface WardInput {
  legacyCode?: string | null;
  name: string;
  telephone?: string | null;
}

export interface RouteRecord {
  id: number;
  legacyCode: string | null;
  name: string;
}

export interface RouteInput {
  legacyCode?: string | null;
  name: string;
}

export interface DiluentRecord {
  id: number;
  legacyCode: string | null;
  name: string;
  volumeMl: number | null;
}

export interface DiluentInput {
  legacyCode?: string | null;
  name: string;
  volumeMl?: number | null;
}

export interface DiagnosisRecord {
  id: number;
  name: string;
}

export interface DiagnosisInput {
  name: string;
}
