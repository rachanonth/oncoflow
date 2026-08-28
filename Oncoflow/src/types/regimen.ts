export type RegimenSortField = "code" | "name" | "items";
export type SortDirection = "asc" | "desc";

export interface RegimenListRequest {
  search?: string | null;
  sortBy?: RegimenSortField;
  sortDirection?: SortDirection;
  limit?: number;
  offset?: number;
}

export interface RegimenSummary {
  id: number;
  code: string;
  name: string;
  marker: boolean;
  groupCount: number;
  itemCount: number;
}

export interface RegimenListResponse {
  items: RegimenSummary[];
  total: number;
}

export interface RegimenItemDetail {
  id: number;
  regimenGroupId: number;
  drugId: number;
  drugCode: string;
  drugName: string;
  dose: number | null;
  doseText: string | null;
  unitText: string | null;
  routeText: string | null;
  details: string | null;
  itemGroup: string | null;
  duration: string | null;
  startDay: number | null;
  orderingNo: number | null;
  defaultDiluentId: number | null;
  defaultDiluent: string | null;
  defaultRouteId: number | null;
  defaultRoute: string | null;
  defaultRate: string | null;
}

export interface RegimenGroupDetail {
  id: number;
  legacyCode: string | null;
  note: string | null;
  cycleDay: number | null;
  cycleCount: number | null;
  items: RegimenItemDetail[];
}

export interface RegimenDetail {
  id: number;
  code: string;
  name: string;
  marker: boolean;
  flag: boolean;
  cycleCheck: boolean;
  autoMode: boolean;
  drugAlert: boolean;
  appointmentAlert: boolean;
  counselAlert: boolean;
  groups: RegimenGroupDetail[];
}

export interface RegimenInput {
  code: string;
  name: string;
  marker: boolean;
  flag: boolean;
  cycleCheck: boolean;
  autoMode: boolean;
  drugAlert: boolean;
  appointmentAlert: boolean;
  counselAlert: boolean;
}

export interface RegimenGroupInput {
  note: string | null;
  cycleDay: number | null;
  cycleCount: number | null;
}

export interface RegimenItemInput {
  regimenGroupId: number;
  drugId: number;
  doseText: string | null;
  unitText: string | null;
  routeText: string | null;
  details: string | null;
  itemGroup: string | null;
  duration: number | null;
  startDay: number | null;
  orderingNo: number | null;
  defaultDiluentId: number | null;
  defaultRouteId: number | null;
  defaultRate: string | null;
}

export interface RegimenReorderInput {
  regimenGroupId: number;
  itemGroup: string | null;
  itemIds: number[];
}

export interface RegimenLookupOption {
  id: number;
  code: string | null;
  label: string;
}

export interface RegimenLookups {
  drugs: RegimenLookupOption[];
  routes: RegimenLookupOption[];
  diluents: RegimenLookupOption[];
}
