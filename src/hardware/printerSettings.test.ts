import { afterEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LABEL_PRINTER, DEFAULT_LABEL_ROW_STYLES, LABEL_SPOOLER_KEY, loadLabelPrinterConfig, saveLabelPrinterConfig } from "./printerSettings";

afterEach(() => vi.unstubAllGlobals());

describe("workstation label font settings", () => {
  it("loads old settings with automatic font and persists the selected local font", () => {
    const values = new Map<string, string>([[LABEL_SPOOLER_KEY, "Synthetic TSPL"]]);
    vi.stubGlobal("window", { localStorage: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    } });
    expect(loadLabelPrinterConfig()?.fontName).toBeNull();
    saveLabelPrinterConfig({ ...DEFAULT_LABEL_PRINTER, spoolerName: "Synthetic TSPL", fontName: "Tahoma" });
    expect(loadLabelPrinterConfig()?.fontName).toBe("Tahoma");
    expect(loadLabelPrinterConfig()?.rowStyles).toEqual(DEFAULT_LABEL_ROW_STYLES);
    const rowStyles = { ...DEFAULT_LABEL_ROW_STYLES, drug: { bold: true, underline: false }, diluent: { bold: false, underline: true } };
    saveLabelPrinterConfig({ ...DEFAULT_LABEL_PRINTER, spoolerName: "Synthetic TSPL", rowStyles });
    expect(loadLabelPrinterConfig()?.rowStyles).toEqual(rowStyles);
    saveLabelPrinterConfig({ ...DEFAULT_LABEL_PRINTER, spoolerName: "Synthetic TSPL", fontName: null });
    expect(loadLabelPrinterConfig()?.fontName).toBeNull();
  });
});
