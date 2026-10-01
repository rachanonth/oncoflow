import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { WardDeliveryRow } from "../types/report";
import { formatDeliveryMedicine, groupWardDelivery, WardDeliveryDialog, WardDeliveryDocument, WardDeliveryReport } from "./WardDeliveryReport";

const row: WardDeliveryRow = {
  taskId: 1, wardId: 10, wardName: "หอผู้ป่วยทดสอบ", patientId: 1, patientHn: "SYN-001", patientName: "ผู้ป่วยสังเคราะห์",
  drugName: "ยาทดสอบ", orderedDoseText: "2300", doseUnitText: "mg", diluentName: "D-5-W", diluentVolumeMl: 1000,
};

describe("Ward delivery report", () => {
  it("opens the report with the date selected in preparation and a close action", () => {
    const html = renderToStaticMarkup(<WardDeliveryDialog date="2026-08-26" onClose={() => undefined} />);
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-label="ปิดใบส่งยา"');
    expect(html).toContain('value="26/08/2569"');
  });
  it("keeps duplicate preparations and separates matching names by patient and ward identity", () => {
    const rows = [row, { ...row, taskId: 2 }, { ...row, taskId: 3, patientId: 2, patientHn: "SYN-002" }, { ...row, taskId: 4, wardId: 11 }];
    const groups = groupWardDelivery(rows);
    expect(groups).toHaveLength(2);
    expect(groups[0].count).toBe(3);
    expect(groups[0].patients.size).toBe(2);
    expect(groups[0].patients.get(1)?.items).toHaveLength(2);
    const html = renderToStaticMarkup(<WardDeliveryDocument date="2026-09-29" rows={rows} printedAt={new Date("2026-09-29T07:00:47Z")} />);
    expect(html).toContain("29/09/2569");
    expect(html).toContain("14:00:47");
    expect(html.match(/ยาทดสอบ 2300 mg in D-5-W 1000 mL/g)).toHaveLength(4);
    expect(html).toContain("รวม 3 รายการ");
    expect(html).toContain("รวมทั้งหมด 4 รายการ");
    expect(html).toContain("ผู้รับยา");
    expect(html).toContain("HN SYN-002");
  });

  it("counts the selected ward only and handles an unrecorded ward", () => {
    const html = renderToStaticMarkup(<WardDeliveryDocument date="2026-09-29" rows={[{ ...row, wardId: null, wardName: "ไม่ระบุหอผู้ป่วย" }]} printedAt={new Date("2026-09-29T07:00:47Z")} />);
    expect(html).toContain("หอผู้ป่วย: ไม่ระบุหอผู้ป่วย");
    expect(html).toContain("รวมทั้งหมด 1 รายการ");
  });

  it("preserves source dose text and zero volume without inventing missing values", () => {
    expect(formatDeliveryMedicine({ ...row, orderedDoseText: "2,300.50", diluentVolumeMl: 0 })).toBe("ยาทดสอบ 2,300.50 mg in D-5-W 0 mL");
    expect(formatDeliveryMedicine({ ...row, orderedDoseText: null, doseUnitText: null, diluentName: null, diluentVolumeMl: null })).toBe("ยาทดสอบ ไม่ระบุขนาดยา in ไม่ระบุสารละลาย ไม่ระบุปริมาตร");
  });

  it("disables printing until checked preparations are loaded", () => {
    const html = renderToStaticMarkup(<WardDeliveryReport />);
    expect(html).toContain("Checked");
    expect(html).toContain('disabled="">พิมพ์ใบส่งยา');
    expect(html).not.toContain('class="ward-delivery-print-root"');
  });
});
