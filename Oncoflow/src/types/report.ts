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
