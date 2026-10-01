import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { LabelPrinterConfig } from "../types/hardware";
import { DEFAULT_LABEL_FONT_SIZES } from "./printerSettings";
import { HardwareSettingsView, validatePrinterConfig } from "./HardwareSettings";

const config: LabelPrinterConfig = {
  spoolerName: "Synthetic TSPL queue",
  language: "tspl",
  widthMm: 100,
  heightMm: 70,
  dpi: 203,
  gapMm: 3,
  preprintHeaderSpacingMm: 5,
  fontSizes: DEFAULT_LABEL_FONT_SIZES,
};

const handlers = {
  onConfig: () => undefined,
  onRefresh: () => undefined,
  onSave: () => undefined,
  onTest: () => undefined,
};

describe("HardwareSettings", () => {
  it("shows installed Windows queues and configurable RAW label settings", () => {
    const html = renderToStaticMarkup(<HardwareSettingsView config={config} printers={["Synthetic TSPL queue", "Office printer"]} loading={false} busy={false} error={null} message={null} {...handlers} />);
    expect(html).toContain("Windows RAW spooler");
    expect(html).toContain("Synthetic TSPL queue");
    expect(html).toContain("ZPL (Zebra)");
    expect(html).toContain("TSPL");
    expect(html).toContain("ESC/POS");
    expect(html).toContain("203 dpi");
    expect(html).toContain("Print test label");
    expect(html).toContain("Preparation label font sizes");
    expect(html).not.toContain('max="40"');
    expect(html).toContain("Top margin / ระยะขอบบน");
    expect(html).toContain("value=\"5\"");
    expect(html).toContain("Withdrawal volume");
    expect(html).toContain("Expiration");
    expect(html).toContain("does not probe the printer");
  });

  it("lists local label fonts and retains an unavailable saved selection", () => {
    const selected = { ...config, fontName: "Tahoma" };
    const html = renderToStaticMarkup(<HardwareSettingsView config={selected} printers={[config.spoolerName]} fonts={["Tahoma", "Leelawadee UI"]} loading={false} busy={false} error={null} message={null} {...handlers} />);
    expect(html).toContain("ฟอนต์ฉลากยา");
    expect(html).toContain('value="Tahoma" selected=""');
    expect(html).toContain("Leelawadee UI");
    const missing = renderToStaticMarkup(<HardwareSettingsView config={selected} printers={[config.spoolerName]} fonts={[]} loading={false} busy={false} error={null} message={null} {...handlers} />);
    expect(missing).toContain("ไม่พบในเครื่องนี้");
    expect(missing).toContain("กรุณาติดตั้งฟอนต์เดิมหรือเลือกฟอนต์อื่นก่อนพิมพ์");
  });

  it("flags a saved queue that Windows no longer exposes", () => {
    const html = renderToStaticMarkup(<HardwareSettingsView config={config} printers={["Different queue"]} loading={false} busy={false} error={null} message={null} {...handlers} />);
    expect(html).toContain("saved queue is not currently installed or visible");
    expect(html).toContain("saved, unavailable");
    expect(html).toContain("disabled=\"\"");
  });

  it("validates queue, dimensions, dpi, and gap locally", () => {
    expect(validatePrinterConfig(config)).toBeNull();
    for (const margin of [0, 14.2, 60]) {
      expect(validatePrinterConfig({ ...config, preprintHeaderSpacingMm: margin })).toBeNull();
    }
    for (const size of [41, 72, 120, 500]) {
      expect(validatePrinterConfig({ ...config, fontSizes: { ...config.fontSizes, warning: size } })).toBeNull();
    }
    expect(validatePrinterConfig({ ...config, fontSizes: { ...config.fontSizes, warning: Infinity } })).toContain("Warning");
    expect(validatePrinterConfig({ ...config, spoolerName: "" })).toContain("Select");
    expect(validatePrinterConfig({ ...config, widthMm: 500 })).toContain("width");
    expect(validatePrinterConfig({ ...config, heightMm: 0 })).toContain("height");
    expect(validatePrinterConfig({ ...config, dpi: 72 })).toContain("resolution");
    expect(validatePrinterConfig({ ...config, gapMm: -1 })).toContain("gap");
    expect(validatePrinterConfig({ ...config, preprintHeaderSpacingMm: 66 })).toContain("Top margin");
    expect(validatePrinterConfig({ ...config, fontSizes: { ...config.fontSizes, warning: 9 } })).toContain("Warning");
    expect(validatePrinterConfig({ ...config, spoolerName: "ZDesigner ZD220-203dpi ZPL" })).toContain("ZPL");
  });
});
