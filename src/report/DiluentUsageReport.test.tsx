import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { DiluentUsageDocument, DiluentUsageReport } from "./DiluentUsageReport";
import { ReportsWorkspace } from "./ReportsWorkspace";

describe("daily diluent usage", () => {
  it("shows the example's six diluent/volume groups and 19 bottles", () => {
    const rows = [["D-5-W", 250, 3], ["NSS", 50, 1], ["NSS", 100, 5], ["NSS", 250, 5], ["NSS", 500, 3], ["NSS", 1000, 2]].map(([name, volume, count]) => ({ diluentName: String(name), volumeMl: Number(volume), bottleCount: Number(count), doseCount: Number(count) }));
    const html = renderToStaticMarkup(<DiluentUsageDocument report={{ preparationDate: "2026-10-01", rows, totalBottles: 19, totalDoses: 19 }} />);
    expect(html).toContain("01/10/2569");
    expect(html).toContain("19 ขวด");
    expect(html).toContain("19 Dose");
    expect(html.match(/<tr>/g)).toHaveLength(8);
    expect(html).toContain("1000");
  });

  it("separates doses from bottles and displays unknown values without inventing a volume", () => {
    const html = renderToStaticMarkup(<DiluentUsageDocument report={{ preparationDate: "2026-10-01", rows: [{ diluentName: null, volumeMl: null, bottleCount: 2, doseCount: 1 }, { diluentName: "NSS", volumeMl: 0, bottleCount: 1, doseCount: 1 }], totalBottles: 3, totalDoses: 2 }} />);
    expect(html).toContain("3 ขวด");
    expect(html).toContain("2 Dose");
    expect(html).toContain("ไม่ระบุสารละลาย");
    expect(html).toContain("ไม่ระบุปริมาตร");
    expect(html).toContain("<td>0</td>");
  });

  it("offers navigation and disables printing before data is ready", () => {
    expect(renderToStaticMarkup(<ReportsWorkspace />)).toContain("จำนวนใช้ Diluent รายวัน");
    const html = renderToStaticMarkup(<DiluentUsageReport />);
    expect(html).toContain('disabled="">พิมพ์รายงาน Diluent');
    expect(html).not.toContain('class="diluent-usage-print-root"');
  });
});
