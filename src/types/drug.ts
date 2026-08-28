export type DrugSortField = "code" | "name" | "unit" | "inventory";
export type SortDirection = "asc" | "desc";

export interface DrugListRequest {
  search?: string | null;
  inventoryEnabled?: boolean | null;
  sortBy: DrugSortField;
  sortDirection: SortDirection;
  limit: number;
  offset: number;
}

export interface DrugSummary {
  id: number;
  code: string;
  name: string;
  unit: string | null;
  package: string | null;
  inventoryEnabled: boolean;
  inventoryMin: number | null;
  inventoryMax: number | null;
  inventoryQuantity: number | null;
}

export interface DrugListResponse {
  items: DrugSummary[];
  total: number;
}

export interface DrugDetail {
  id: number;
  code: string;
  name: string;
  unitId: number | null;
  unit: string | null;
  dosePerPack: number | null;
  volumePerPackMl: number | null;
  package: string | null;
  detail: string | null;
  price: number | null;
  theory: string | null;
  marker: boolean;
  defaultDiluentId: number | null;
  defaultDiluent: string | null;
  defaultRouteId: number | null;
  defaultRoute: string | null;
  defaultRate: string | null;
  warning: string | null;
  storage: string | null;
  flag: boolean;
  expiryTime: string | null;
  expiryStorage: string | null;
  maxDose: number | null;
  maxDilutionAlert: boolean | null;
  maxDilutionHard: number | null;
  cumulativeAlert: boolean | null;
  cumulativeAlertHard: number | null;
  dilutionIncompatibility: string | null;
  inventoryCut: boolean | null;
  inventoryMin: number | null;
  inventoryMax: number | null;
  inventoryQuantity: number | null;
  inventoryEnabled: boolean;
  legacyMappingCode: string | null;
  legacyExp: number | null;
  legacyReg: string | null;
}

export interface DrugInput {
  code: string;
  name: string;
  unitId: number | null;
  dosePerPack: number | null;
  volumePerPackMl: number | null;
  package: string | null;
  detail: string | null;
  price: number | null;
  theory: string | null;
  marker: boolean;
  defaultDiluentId: number | null;
  defaultRouteId: number | null;
  defaultRate: string | null;
  warning: string | null;
  storage: string | null;
  flag: boolean;
  expiryTime: string | null;
  expiryStorage: string | null;
  maxDose: number | null;
  maxDilutionAlert: boolean | null;
  maxDilutionHard: number | null;
  cumulativeAlert: boolean | null;
  cumulativeAlertHard: number | null;
  dilutionIncompatibility: string | null;
  inventoryCut: boolean | null;
  inventoryMin: number | null;
  inventoryMax: number | null;
  inventoryEnabled: boolean;
}

export interface DrugLookupOption {
  id: number;
  code: string | null;
  label: string;
  volumeMl: number | null;
}

export interface DrugFormOptions {
  suggestedCode: string;
  units: DrugLookupOption[];
  routes: DrugLookupOption[];
  diluents: DrugLookupOption[];
}
