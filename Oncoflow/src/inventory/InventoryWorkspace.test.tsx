import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { InventoryDetail, InventoryMovement, InventorySummary } from "../types/inventory";
import { InventoryDetailView, movementLabel } from "./InventoryDetail";
import { buildInventoryListRequest, InventoryTable, stockStateLabel } from "./InventoryList";

const shortage: InventorySummary = {
  drugId: 7,
  drugCode: "SYN-TH",
  drugName: "ยาคลังสังเคราะห์",
  legacyDrugUnit: "หน่วยเดิม",
  package: "Synthetic pack",
  currentStock: -2,
  minimumStock: 2,
  maximumStock: 12,
  trackingEnabled: true,
  stockState: "shortage",
};

const detail: InventoryDetail = {
  ...shortage,
  legacyInventorySnapshot: 1,
  legacyInventoryCutoff: true,
  dosePerPack: 50,
  volumePerPackMl: null,
  legacyInventoryEventCount: 1,
  quantitySemantics: "unresolved_legacy_inventory_unit",
};

const movements: InventoryMovement[] = [
  {
    id: 2,
    movementType: "manual_issue",
    quantityDelta: -3,
    resultingBalance: -2,
    occurredAt: "2026-08-23T09:00:00Z",
    createdAt: "2026-08-23T09:00:00Z",
    actorDisplayName: "เภสัชกรทดสอบ",
    referenceType: "manual_issue_reference",
    referenceId: "SYN-REF",
    note: "Synthetic shortage fixture",
    preparationTaskId: null,
  },
  {
    id: 1,
    movementType: "opening_balance",
    quantityDelta: 1,
    resultingBalance: 1,
    occurredAt: null,
    createdAt: "2026-08-23T08:00:00Z",
    actorDisplayName: null,
    referenceType: "legacy_drug_inventory",
    referenceId: "SYN-TH",
    note: "Synthetic opening",
    preparationTaskId: null,
  },
];

describe("Inventory workspace", () => {
  it("builds database-backed search and low-stock filter requests", () => {
    expect(buildInventoryListRequest("  คลัง  ", false, true, "state", "asc", 2, 100)).toEqual({
      search: "คลัง",
      trackedOnly: true,
      lowStockOnly: true,
      sortBy: "state",
      sortDirection: "asc",
      limit: 100,
      offset: 200,
    });
  });

  it("renders Thai inventory data and clearly flags a negative shortage", () => {
    const html = renderToStaticMarkup(<InventoryTable items={[shortage]} selected={null} sortBy="state" sortDirection="asc" onSelect={() => undefined} onOpen={() => undefined} onSort={() => undefined}/>);
    expect(html).toContain("ยาคลังสังเคราะห์");
    expect(html).toContain("SYN-TH");
    expect(html).toContain("Shortage");
    expect(html).toContain("-2");
    expect(stockStateLabel("shortage")).toBe("Shortage");
  });

  it("renders configuration, actor-attributed history, receipt/adjustment workflow, and scope boundary", () => {
    const html = renderToStaticMarkup(<InventoryDetailView inventory={detail} movements={movements} onBack={() => undefined} onRecord={() => undefined}/>);
    expect(html).toContain("Current stock");
    expect(html).toContain("Legacy Inv snapshot");
    expect(html).toContain("Quantity semantics remain explicit");
    expect(html).toContain("posts stock only from a fully supported container calculation");
    expect(html).toContain("Receipt");
    expect(html).toContain("Adjustment");
    expect(html).toContain("Manual issue");
    expect(html).toContain("เภสัชกรทดสอบ");
    expect(html).toContain("Legacy migration");
    expect(html).toContain("SYN-REF");
    expect(movementLabel("preparation_issue")).toBe("Preparation issue");
  });
});
