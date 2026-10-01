import type { LabelFontSizes, LabelRowStyles, LabelPrinterConfig, PrinterLanguage } from "../types/hardware";

export const LABEL_ROW_STYLES_KEY = "hardware_label_row_styles";
export const DEFAULT_LABEL_ROW_STYLES: LabelRowStyles = {
  header: { bold: false, underline: false },
  patient: { bold: false, underline: false },
  withdrawal: { bold: false, underline: false },
  drug: { bold: false, underline: false },
  diluent: { bold: false, underline: false },
  routeRate: { bold: false, underline: false },
  storage: { bold: false, underline: false },
  warning: { bold: false, underline: false },
  preparedBy: { bold: false, underline: false },
  expiration: { bold: false, underline: false },
};

export const LABEL_SPOOLER_KEY = "hardware_label_spooler";
export const LABEL_LANGUAGE_KEY = "hardware_label_type";
export const LABEL_WIDTH_KEY = "hardware_label_width_mm";
export const LABEL_HEIGHT_KEY = "hardware_label_height_mm";
export const LABEL_DPI_KEY = "hardware_label_dpi";
export const LABEL_GAP_KEY = "hardware_label_gap_mm";
export const LABEL_PREPRINT_HEADER_SPACING_KEY = "hardware_label_preprint_header_spacing_mm";
export const LABEL_FONT_NAME_KEY = "hardware_label_font_name";
export const LABEL_FONT_SIZES_KEY = "hardware_label_font_sizes";

export const DEFAULT_LABEL_FONT_SIZES: LabelFontSizes = {
  header: 22,
  patient: 20,
  withdrawal: 16,
  drug: 21,
  routeRate: 18,
  storage: 16,
  warning: 16,
  preparedBy: 15,
  expiration: 18,
};

export const DEFAULT_LABEL_PRINTER: Omit<LabelPrinterConfig, "spoolerName"> = {
  language: "tspl",
  widthMm: 80,
  heightMm: 70,
  dpi: 203,
  gapMm: 3,
  preprintHeaderSpacingMm: 5,
  fontSizes: DEFAULT_LABEL_FONT_SIZES,
  fontName: null,
  rowStyles: DEFAULT_LABEL_ROW_STYLES,
};

export function loadLabelPrinterConfig(): LabelPrinterConfig | null {
  if (typeof window === "undefined") return null;
  try {
    const spoolerName = window.localStorage.getItem(LABEL_SPOOLER_KEY)?.trim();
    if (!spoolerName) return null;
    const rawLanguage = window.localStorage.getItem(LABEL_LANGUAGE_KEY);
    const storedLanguage: PrinterLanguage =
      rawLanguage === "zpl" ? "zpl" : rawLanguage === "escpos" ? "escpos" : "tspl";
    const language = inferPrinterLanguageFromQueue(spoolerName) ?? storedLanguage;
    return {
      spoolerName,
      language,
      widthMm: readNumber(LABEL_WIDTH_KEY, DEFAULT_LABEL_PRINTER.widthMm),
      heightMm: readNumber(LABEL_HEIGHT_KEY, DEFAULT_LABEL_PRINTER.heightMm),
      dpi: readNumber(LABEL_DPI_KEY, DEFAULT_LABEL_PRINTER.dpi),
      gapMm: readNumber(LABEL_GAP_KEY, DEFAULT_LABEL_PRINTER.gapMm, true),
      preprintHeaderSpacingMm: readNumber(LABEL_PREPRINT_HEADER_SPACING_KEY, DEFAULT_LABEL_PRINTER.preprintHeaderSpacingMm, true),
      fontSizes: readFontSizes(),
      rowStyles: readRowStyles(),
      fontName: window.localStorage.getItem(LABEL_FONT_NAME_KEY)?.trim() || null,
    };
  } catch {
    return null;
  }
}

export function inferPrinterLanguageFromQueue(queueName: string): PrinterLanguage | null {
  const normalized = queueName.trim().toLowerCase();
  if (!normalized) return null;
  if (/(^|[^a-z])zpl([^a-z]|$)/.test(normalized)) return "zpl";
  if (/(^|[^a-z])tspl([^a-z]|$)/.test(normalized)) return "tspl";
  if (normalized.includes("esc/pos") || normalized.includes("escpos")) return "escpos";
  return null;
}
export function saveLabelPrinterConfig(config: LabelPrinterConfig): void {
  window.localStorage.setItem(LABEL_SPOOLER_KEY, config.spoolerName);
  window.localStorage.setItem(LABEL_LANGUAGE_KEY, config.language);
  window.localStorage.setItem(LABEL_WIDTH_KEY, `${config.widthMm}`);
  window.localStorage.setItem(LABEL_HEIGHT_KEY, `${config.heightMm}`);
  window.localStorage.setItem(LABEL_DPI_KEY, `${config.dpi}`);
  window.localStorage.setItem(LABEL_GAP_KEY, `${config.gapMm}`);
  window.localStorage.setItem(LABEL_PREPRINT_HEADER_SPACING_KEY, `${config.preprintHeaderSpacingMm}`);
  window.localStorage.setItem(LABEL_FONT_SIZES_KEY, JSON.stringify(config.fontSizes));
  window.localStorage.setItem(LABEL_FONT_NAME_KEY, config.fontName ?? "");
  window.localStorage.setItem(LABEL_ROW_STYLES_KEY, JSON.stringify(config.rowStyles ?? DEFAULT_LABEL_ROW_STYLES));
}

function readRowStyles(): LabelRowStyles {
  try {
    const saved = JSON.parse(window.localStorage.getItem(LABEL_ROW_STYLES_KEY) ?? "{}");
    return Object.fromEntries(Object.keys(DEFAULT_LABEL_ROW_STYLES).map(key => [key, {
      bold: saved?.[key]?.bold === true, underline: saved?.[key]?.underline === true,
    }])) as LabelRowStyles;
  } catch { return DEFAULT_LABEL_ROW_STYLES; }
}

function readFontSizes(): LabelFontSizes {
  const raw = window.localStorage.getItem(LABEL_FONT_SIZES_KEY);
  if (!raw) return { ...DEFAULT_LABEL_FONT_SIZES };
  try {
    const saved = JSON.parse(raw) as Partial<Record<keyof LabelFontSizes, unknown>>;
    return Object.fromEntries(Object.entries(DEFAULT_LABEL_FONT_SIZES).map(([key, fallback]) => {
      const value = Number(saved[key as keyof LabelFontSizes]);
      return [key, Number.isFinite(value) && value > 0 ? value : fallback];
    })) as unknown as LabelFontSizes;
  } catch {
    return { ...DEFAULT_LABEL_FONT_SIZES };
  }
}

function readNumber(key: string, fallback: number, allowZero = false): number {
  const raw = window.localStorage.getItem(key);
  if (raw === null || raw.trim() === "") return fallback;
  const value = Number(raw);
  return Number.isFinite(value) && (allowZero ? value >= 0 : value > 0) ? value : fallback;
}
