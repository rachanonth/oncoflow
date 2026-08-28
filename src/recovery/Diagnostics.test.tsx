import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DiagnosticsView } from "./Diagnostics";

const diagnostics = {
  applicationName: "OncoFlow",
  applicationVersion: "0.1.0",
  schemaVersion: 10,
  clinicalRulesetVersion: "legacy-cytotoxic-v8",
  labelTemplateVersion: "oncoflow-preparation-label-v1",
  labelRendererVersion: "oncoflow-raw-label-raster-v2",
  databaseLocation: "C:\\Synthetic\\oncoflow.db",
  databaseSizeBytes: 524288,
  integrityCheck: "ok",
  foreignKeyViolations: 0,
  lastBackupAt: null,
  platform: "windows",
  automaticBackupPolicy: "Manual backups are operator controlled.",
};

describe("Diagnostics", () => {
  it("renders separate release identities and privacy-safe database health", () => {
    const html = renderToStaticMarkup(<DiagnosticsView state={{ kind: "ready", diagnostics, printers: ["Synthetic queue"], printerError: null }} printerName="Synthetic queue" printerLanguage="tspl" printerAvailable onRetry={() => undefined} onOpenDataFolder={() => undefined}/>);
    expect(html).toContain("OncoFlow 0.1.0");
    expect(html).toContain("legacy-cytotoxic-v8");
    expect(html).toContain("oncoflow-preparation-label-v1");
    expect(html).toContain("oncoflow-raw-label-raster-v2");
    expect(html).toContain("Integrity");
    expect(html).toContain("0 violations");
    expect(html).toContain("never display passwords");
  });

  it("marks a missing configured printer without claiming physical output", () => {
    const html = renderToStaticMarkup(<DiagnosticsView state={{ kind: "ready", diagnostics, printers: ["Different queue"], printerError: null }} printerName="Xprinter XP-420B" printerLanguage="tspl" printerAvailable={false} onRetry={() => undefined} onOpenDataFolder={() => undefined}/>);
    expect(html).toContain("Xprinter XP-420B");
    expect(html).toContain("Unavailable / disconnected");
    expect(html).toContain("not proof of paper output");
    expect(html).toContain("operator-controlled test label");
  });
});
