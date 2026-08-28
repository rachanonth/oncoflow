import type { LabelFontSizes, LabelPrinterConfig, PrinterLanguage } from "../types/hardware";

export const LABEL_SPOOLER_KEY = "hardware_label_spooler";
export const LABEL_LANGUAGE_KEY = "hardware_label_type";
export const LABEL_WIDTH_KEY = "hardware_label_width_mm";
export const LABEL_HEIGHT_KEY = "hardware_label_height_mm";
export const LABEL_DPI_KEY = "hardware_label_dpi";
export const LABEL_GAP_KEY = "hardware_label_gap_mm";
export const LABEL_PREPRINT_HEADER_SPACING_KEY = "hardware_label_preprint_header_spacing_mm";
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
  widthMm: 100,
  heightMm: 70,
  dpi: 203,
  gapMm: 3,
  preprintHeaderSpacingMm: 5,
  fontSizes: DEFAULT_LABEL_FONT_SIZES,
};

export function loadLabelPrinterConfig(): LabelPrinterConfig | null {
  if (typeof window === "undefined") return null;
  try {
    const spoolerName = window.localStorage.getItem(LABEL_SPOOLER_KEY)?.trim();
    if (!spoolerName) return null;
    const rawLanguage = window.localStorage.getItem(LABEL_LANGUAGE_KEY);
    const language: PrinterLanguage = rawLanguage === "escpos" ? "escpos" : "tspl";
    return {
      spoolerName,
      language,
      widthMm: readNumber(LABEL_WIDTH_KEY, DEFAULT_LABEL_PRINTER.widthMm),
      heightMm: readNumber(LABEL_HEIGHT_KEY, DEFAULT_LABEL_PRINTER.heightMm),
      dpi: readNumber(LABEL_DPI_KEY, DEFAULT_LABEL_PRINTER.dpi),
      gapMm: readNumber(LABEL_GAP_KEY, DEFAULT_LABEL_PRINTER.gapMm, true),
      preprintHeaderSpacingMm: readNumber(LABEL_PREPRINT_HEADER_SPACING_KEY, DEFAULT_LABEL_PRINTER.preprintHeaderSpacingMm, true),
      fontSizes: readFontSizes(),
    };
  } catch {
    return null;
  }
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
