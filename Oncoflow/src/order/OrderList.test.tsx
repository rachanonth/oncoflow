import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { OrderSummary } from "../types/order";
import { getBangkokQuickDateRange, OrderTable } from "./OrderList";

const order: OrderSummary = {
  id: 1,
  orderId: "OF-001",
  patientId: 2,
  patientHn: "HN-123",
  patientName: "Synthetic Patient",
  orderTime: "2026-08-25T09:30",
  regimenName: "Hidden regimen",
  doctorName: "Synthetic Doctor",
  wardName: "Synthetic Ward",
  orderType: null,
  itemCount: 4,
  drugs: [{ drugName: "Synthetic drug", doseText: "100", unitText: "mg" }],
  editable: true,
  workflowStatus: "active",
};

describe("order list table", () => {
  it("shows the compact requested columns without regimen or the order identifier", () => {
    const html = renderToStaticMarkup(<OrderTable items={[order]} selected={null} sortBy="date" sortDirection="desc" onSort={() => undefined} onSelect={() => undefined} onOpen={() => undefined} onKey={() => undefined} />);

    expect(html).toContain("Date / time");
    expect(html).toContain("HN-123");
    expect(html).toContain("Synthetic Patient");
    expect(html).toContain("Synthetic Doctor");
    expect(html).toContain("Synthetic Ward");
    expect(html).toContain("No. of drugs");
    expect(html).toContain(">4<");
    expect(html).not.toContain("Hidden regimen");
    expect(html).not.toContain(">OF-001<");
  });

  it("builds Today and Yesterday ranges from the Bangkok calendar date", () => {
    const now = new Date("2026-01-01T00:30:00Z");

    expect(getBangkokQuickDateRange("today", now)).toEqual({ dateFrom: "2026-01-01", dateTo: "2026-01-01" });
    expect(getBangkokQuickDateRange("yesterday", now)).toEqual({ dateFrom: "2025-12-31", dateTo: "2025-12-31" });
  });
});
