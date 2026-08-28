import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { InventoryUsageReportRow } from "../types/report";
import { groupInventoryUsagePeriods, InventoryUsageReport, InventoryUsageTable } from "./InventoryUsageReport";
import { ReportsWorkspace } from "./ReportsWorkspace";

const rows: InventoryUsageReportRow[] = [
  { periodStart: "2026-08-28", drugId: 1, drugCode: "D001", drugName: "Paclitaxel", sourcePackage: "vial", prescriptionCount: 3, preparedBottleCount: 5, issuedSourceContainerCount: 4, awaitingVerificationCount: 1, manualReconciliationCount: 0, trackingDisabledCount: 0, unrecordedInventoryCount: 0, currentStock: 8, minimumStock: 3, stockState: "normal" },
  { periodStart: "2026-08-28", drugId: 2, drugCode: "D002", drugName: "Bleomycin", sourcePackage: "ampoule", prescriptionCount: 2, preparedBottleCount: 2, issuedSourceContainerCount: 3, awaitingVerificationCount: 0, manualReconciliationCount: 1, trackingDisabledCount: 0, unrecordedInventoryCount: 0, currentStock: 2, minimumStock: 3, stockState: "low" },
];

describe("InventoryUsageReport", () => {
  it("summarizes prescriptions, prepared bottles, and actual source issues per period", () => {
    expect(groupInventoryUsagePeriods(rows)[0]).toMatchObject({ prescriptionCount: 5, preparedBottleCount: 7, issuedSourceContainerCount: 7 });
  });

  it("renders actual vial or ampoule issues, reconciliation states, current stock, and totals", () => {
    const html = renderToStaticMarkup(<InventoryUsageTable interval="daily" rows={rows} />);
    expect(html).toContain("5 ตำรับ / 7 ขวด");
    expect(html).toContain("4</strong><span>vial");
    expect(html).toContain("3</strong><span>ampoule");
    expect(html).toContain("รอตรวจ 1");
    expect(html).toContain("กระทบยอดเอง 1");
    expect(html).toContain("รวมทั้งหมด");
    expect(html).toContain("ต่ำ");
  });

  it("offers the same daily, weekly, monthly date controls", () => {
    const html = renderToStaticMarkup(<InventoryUsageReport />);
    expect(html).toContain(">Daily</button>");
    expect(html).toContain(">Weekly</button>");
    expect(html).toContain(">Monthly</button>");
    expect(html).toContain("ตั้งแต่");
    expect(html).toContain("ถึง");
  });

  it("adds the second report to the reports workspace", () => {
    const html = renderToStaticMarkup(<ReportsWorkspace />);
    expect(html).toContain("จำนวนการเตรียมยา");
    expect(html).toContain("การใช้ยาและ Stock");
  });
});
