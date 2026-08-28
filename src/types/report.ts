export type ReportInterval = "daily" | "weekly" | "monthly";

export interface PreparationCountReportRequest {
  interval: ReportInterval;
  dateFrom: string;
  dateTo: string;
}

export interface PreparationCountReportRow {
  periodStart: string;
  drugId: number;
  drugName: string;
  preparerUserId: number | null;
  preparerName: string;
  prescriptionCount: number;
  bottleCount: number;
}

export interface PreparationCountReport {
  interval: ReportInterval;
  dateFrom: string;
  dateTo: string;
  totalPrescriptions: number;
  totalBottles: number;
  rows: PreparationCountReportRow[];
}

export interface InventoryUsageReportRequest {
  interval: ReportInterval;
  dateFrom: string;
  dateTo: string;
}

export interface InventoryUsageReportRow {
  periodStart: string;
  drugId: number;
  drugCode: string;
  drugName: string;
  sourcePackage: string;
  prescriptionCount: number;
  preparedBottleCount: number;
  issuedSourceContainerCount: number;
  awaitingVerificationCount: number;
  manualReconciliationCount: number;
  trackingDisabledCount: number;
  unrecordedInventoryCount: number;
  currentStock: number | null;
  minimumStock: number | null;
  stockState: "untracked" | "unknown" | "shortage" | "out" | "low" | "normal";
}

export interface InventoryUsageReport {
  interval: ReportInterval;
  dateFrom: string;
  dateTo: string;
  totalPrescriptions: number;
  totalPreparedBottles: number;
  totalIssuedSourceContainers: number;
  drugCount: number;
  rows: InventoryUsageReportRow[];
}
