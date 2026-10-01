import type { PreparationOutput } from "./output";

export type PrinterLanguage = "escpos" | "tspl" | "zpl";

export interface LabelFontSizes {
  header: number;
  patient: number;
  withdrawal: number;
  drug: number;
  routeRate: number;
  storage: number;
  warning: number;
  preparedBy: number;
  expiration: number;
}

export type LabelRowKey = "header" | "patient" | "withdrawal" | "drug" | "diluent" | "routeRate" | "storage" | "warning" | "preparedBy" | "expiration";
export interface LabelRowStyle { bold: boolean; underline: boolean; }
export type LabelRowStyles = Record<LabelRowKey, LabelRowStyle>;

export interface LabelPrinterConfig {
  spoolerName: string;
  language: PrinterLanguage;
  widthMm: number;
  heightMm: number;
  dpi: number;
  gapMm: number;
  preprintHeaderSpacingMm: number;
  fontSizes: LabelFontSizes;
  fontName?: string | null;
  rowStyles?: LabelRowStyles;
}

export interface PrintJobReceipt {
  windowsJobId: number;
  bytesSubmitted: number;
  rendererVersion: string;
}

export interface PreparationPrintResult {
  output: PreparationOutput;
  job: PrintJobReceipt;
}

export interface PreparationBatchPrintResult {
  outputs: PreparationOutput[];
  job: PrintJobReceipt;
}
