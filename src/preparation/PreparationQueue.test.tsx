import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { PreparationQueueItem } from "../types/preparation";
import { PreparationQueue, PreparationQueueTable, labelPrintStatus } from "./PreparationQueue";

const item: PreparationQueueItem = {
  orderId: 10,
  orderCode: "OF-SYN-10",
  patientHn: "SYN-HN",
  patientName: "ผู้ป่วยทดสอบ",
  wardName: "หอผู้ป่วยสังเคราะห์",
  regimenName: "สูตรสังเคราะห์",
  treatmentTime: "2026-08-23T09:00",
  preparationDate: "2026-08-26",
  sourceKind: "continuing",
  eligibleItemCount: 2,
  initializedItemCount: 2,
  pendingItemCount: 1,
  preparedItemCount: 1,
  verifiedItemCount: 0,
  printedLabelCount: 0,
};

describe("PreparationQueue", () => {
  it("offers the same Today and Yesterday quick filters as the orders list", () => {
    const html = renderToStaticMarkup(<PreparationQueue onOpen={() => undefined} />);

    expect(html).toContain('aria-label="Quick preparation date"');
    expect(html).toContain('aria-label="Preparation source"');
    expect(html).toContain(">Today</button>");
    expect(html).toContain(">Yesterday</button>");
    expect(html).toContain(">Today order</button>");
    expect(html).toContain(">Continuing</button>");
    expect(html).toContain("Uses the current preparation view");
    expect(html).not.toContain("Working formula date");
  });

  it("renders the local eligible queue with Thai UTF-8 data and label print status", () => {
    const html = renderToStaticMarkup(<PreparationQueueTable items={[item]} selected={null} onSelect={() => undefined} onOpen={() => undefined} onKey={() => undefined} />);
    expect(html).toContain("OF-SYN-10");
    expect(html).toContain("ผู้ป่วยทดสอบ");
    expect(html).toContain("หอผู้ป่วยสังเคราะห์");
    expect(html).toContain("<th>Ward</th>");
    expect(html.indexOf("HN SYN-HN")).toBeLessThan(html.indexOf("ผู้ป่วยทดสอบ"));
    expect(html).not.toContain("Regimen");
    expect(html).not.toContain("Preparation progress");
    expect(html).toContain("สถานะพิมพ์ฉลาก");
    expect(html).toContain("รอตรวจสอบ");
    expect(html).toContain("Continuing order");
    expect(html).not.toContain("ผู้ป่วยไม่มาตามนัด");
    expect(html).toContain("2 items due");
  });

  it("reports ready, partial, and completed label printing from persisted counts", () => {
    expect(labelPrintStatus({ ...item, verifiedItemCount: 2 }).label).toBe("พร้อมพิมพ์");
    expect(labelPrintStatus({ ...item, verifiedItemCount: 2, printedLabelCount: 1 }).label).toBe("พิมพ์บางส่วน 1/2");
    expect(labelPrintStatus({ ...item, verifiedItemCount: 2, printedLabelCount: 2 }).label).toBe("พิมพ์แล้ว");
  });

});
