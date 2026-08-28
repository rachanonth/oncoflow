import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { PreparationCountReportRow } from "../types/report";
import { aggregateReportRows, defaultReportRange, formatReportPeriod, PreparationCountReport, PreparationCountTable } from "./PreparationCountReport";

const rows: PreparationCountReportRow[] = [
  { periodStart: "2026-08-28", drugId: 4, drugName: "Paclitaxel", preparerUserId: 7, preparerName: "เภสัชกร หนึ่ง", prescriptionCount: 3, bottleCount: 5 },
  { periodStart: "2026-08-28", drugId: 5, drugName: "Bleomycin", preparerUserId: 7, preparerName: "เภสัชกร หนึ่ง", prescriptionCount: 2, bottleCount: 2 },
  { periodStart: "2026-08-28", drugId: 4, drugName: "Paclitaxel", preparerUserId: 8, preparerName: "เภสัชกร สอง", prescriptionCount: 1, bottleCount: 2 },
];

describe("PreparationCountReport", () => {
  it("uses the requested default ranges", () => {
    expect(defaultReportRange("daily", "2026-08-28")).toEqual({ dateFrom: "2026-08-01", dateTo: "2026-08-31" });
    expect(defaultReportRange("weekly", "2026-08-28")).toEqual({ dateFrom: "2026-06-08", dateTo: "2026-08-30" });
    expect(defaultReportRange("monthly", "2026-08-28")).toEqual({ dateFrom: "2026-01-01", dateTo: "2026-12-31" });
  });

  it("formats weekly and monthly periods in the Buddhist calendar", () => {
    expect(formatReportPeriod("2026-08-24", "weekly")).toBe("24/08/2569 – 30/08/2569");
    expect(formatReportPeriod("2026-08-01", "monthly")).toBe("สิงหาคม 2569");
  });

  it("aggregates the same source rows by pharmacist or by drug", () => {
    expect(aggregateReportRows(rows, "pharmacist")[0].items).toEqual([
      { key: "pharmacist:7", label: "เภสัชกร หนึ่ง", prescriptionCount: 5, bottleCount: 7 },
      { key: "pharmacist:8", label: "เภสัชกร สอง", prescriptionCount: 1, bottleCount: 2 },
    ]);
    expect(aggregateReportRows(rows, "drug")[0].items).toEqual([
      { key: "drug:4", label: "Paclitaxel", prescriptionCount: 4, bottleCount: 7 },
      { key: "drug:5", label: "Bleomycin", prescriptionCount: 2, bottleCount: 2 },
    ]);
  });

  it("renders date, grouped item and task count like the requested report", () => {
    const html = renderToStaticMarkup(<PreparationCountTable interval="daily" rows={rows} groupBy="drug" />);
    expect(html).toContain("Paclitaxel");
    expect(html).toContain("Bleomycin");
    expect(html).toContain("จำนวนที่เตรียม");
    expect(html).toContain("6 ตำรับ / 9 ขวด");
    expect(html).toContain("รวมทั้งหมด");
    expect(html).toContain('<div class="report-quantity report-quantity--total"><strong>6</strong><span>ตำรับ</span><b aria-hidden="true">/</b><strong>9</strong><span>ขวด</span></div>');
    expect(html).toContain('<strong>4</strong><span>ตำรับ</span>');
  });

  it("offers daily, weekly, monthly and selectable dates", () => {
    const html = renderToStaticMarkup(<PreparationCountReport />);
    expect(html).toContain(">Daily</button>");
    expect(html).toContain(">Weekly</button>");
    expect(html).toContain(">Monthly</button>");
    expect(html).toContain("Group by");
    expect(html).toContain(">Pharmacist</button>");
    expect(html).toContain(">Drug</button>");
    expect(html).toContain("ตั้งแต่");
    expect(html).toContain("ถึง");
  });
});
