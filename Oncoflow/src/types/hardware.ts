import type { PreparationOutput } from "./output";

export type PrinterLanguage = "escpos" | "tspl";

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

export interface LabelPrinterConfig {
  spoolerName: string;
  language: PrinterLanguage;
  widthMm: number;
  heightMm: number;
  dpi: number;
  gapMm: number;
  preprintHeaderSpacingMm: number;
  fontSizes: LabelFontSizes;
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
