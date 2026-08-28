import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { PreparationWorkspace } from "../types/preparation";
import { buildWorkingFormulaDrugGroups, WorkingFormulaDialog, WorkingFormulaDrugGroups, WorkingFormulaOrder, WorkingFormulaPrintHeader } from "./WorkingFormula";

const workspace: PreparationWorkspace = {
  orderId: 7,
  orderCode: "OF-SYN-7",
  patientHn: "SYN-TH-7",
  patientName: "ผู้ป่วยสังเคราะห์",
  wardName: "หอผู้ป่วยสังเคราะห์",
  regimenName: "สูตรยาสังเคราะห์",
  treatmentTime: "2026-08-26T09:00",
  preparationDate: "2026-08-26",
  assignedPreparer: { id: 2, displayName: "เภสัชกรผู้เตรียม", role: "pharmacist" },
  editable: true,
  eligibilityRuleId: "synthetic",
  excludedItemCount: 0,
  pharmacists: [],
  safety: { mode: "active", rulesetVersion: "legacy-cytotoxic-v8", findings: [], evaluatedRuleCount: 0, unsupportedRuleCount: 0, notice: "Deferred" },
  safetyAcknowledgements: [],
  items: [{
    orderItemId: 8, drugId: 9, drugCode: "SYN", drugName: "ยาเคมีบำบัดสังเคราะห์", orderedDoseText: "100", doseUnitText: "mg", diluentName: "สารละลายสังเคราะห์", diluentVolumeMl: 100, routeName: "IV", rateText: "60 min", treatmentDay: "1", sequenceNo: 1, regimenDetails: "เตรียมแบบสังเคราะห์", drugDetail: null, drugStorage: null,
    eligibility: { status: "eligible", ruleId: "synthetic", reason: "Synthetic" },
    referenceQuantity: { status: "calculated", drugSolutionVolumeMl: "20", packageEquivalent: "2", formula: "Synthetic", notice: "Synthetic" },
    calculation: { status: "calculated", rulesetVersion: "legacy-cytotoxic-v8", ruleId: "synthetic", orderedDose: { value: "100", unit: "mg" }, presentation: { amountPerContainer: { value: "50", unit: "mg" }, volumePerContainerMl: "10", containerLabel: "ampoule", rawPackageLabel: "ampoule" }, concentration: "5 mg/mL", withdrawalVolumeMl: "20", containersRequired: "2", unusedAmount: null, inventoryProjection: { trackingEnabled: true, currentStock: "5", containersRequired: "2", projectedStock: "3", minimumStock: "1", state: "normal", unitNotice: "Synthetic" }, legacyReference: { storedQuantity: null, storedQuantitySemantics: "UNKNOWN", calculatedPackageEquivalent: "2", calculatedSolutionVolumeMl: "20", comparisonStatus: "unavailable", notice: "Synthetic" }, trace: [], warnings: [] },
    defaultPreparationVolumeMl: "120",
    task: {
      id: 18, sourceOrderId: 7, sourceOrderItemId: 8, preparationDate: "2026-08-26", drugId: 9, state: "pending", orderedDoseText: "100", doseUnitText: "mg", diluentId: 1, diluentName: "สารละลายสังเคราะห์", diluentVolumeMl: 100, routeId: 1, routeName: "IV", rateText: "60 min", treatmentDay: "1", startDate: "2026-08-26", stopDate: "2026-08-26", sequenceNo: 1, regimenDetails: "เตรียมแบบสังเคราะห์", drugDetail: null, drugStorage: null, preparationVolumeMl: null, preparationNotes: null, finalContainerCount: 1, createdAt: "2026-08-26 09:00:00", updatedAt: "2026-08-26 09:00:00", preparedAt: null, verifiedAt: null, preparedBy: null, verifiedBy: null, inventoryPosting: null,
    },
  }],
};

describe("WorkingFormula", () => {
  it("shows the configured hospital name after the application name", () => {
    const html = renderToStaticMarkup(<WorkingFormulaPrintHeader date="2026-08-26" hospitalName="โรงพยาบาลทดสอบ" />);
    expect(html).toContain("OncoFlow · โรงพยาบาลทดสอบ");
  });

  it("renders Thai order and preparation formula content", () => {
    const html = renderToStaticMarkup(<WorkingFormulaOrder workspace={workspace} sourceKind="continuing" />);
    expect(html).toContain("ผู้ป่วยสังเคราะห์");
    expect(html).toContain("HN SYN-TH-7");
    expect(html).toContain("HN SYN-TH-7 - ผู้ป่วยสังเคราะห์");
    expect(html).toContain("Order OF-SYN-7");
    expect(html).toContain("Ward: หอผู้ป่วยสังเคราะห์");
    expect(html).toContain("Order time:");
    expect(html).toContain("Prepared by: เภสัชกรผู้เตรียม");
    expect(html).not.toContain("Preparation pharmacist:");
    expect(html).toContain("ยาเคมีบำบัดสังเคราะห์");
    expect(html).toContain("Withdrawal: 20 mL");
    expect(html).toContain("Containers: 2");
    expect(html).toContain("เภสัชกรผู้เตรียม");
    expect(html).toContain("Continuing order");
    expect(html).not.toContain("<th>Check</th>");
  });

  it("omits the regimen line when the order has no recorded regimen", () => {
    const html = renderToStaticMarkup(<WorkingFormulaOrder workspace={{ ...workspace, regimenName: null }} sourceKind="same_day" />);
    expect(html).not.toContain("Regimen not recorded");
    expect(html).not.toContain("สูตรยาสังเคราะห์");
  });

  it("keeps the check column hidden for already checked tasks", () => {
    const checked: PreparationWorkspace = {
      ...workspace,
      items: workspace.items.map((item) => ({ ...item, task: item.task ? { ...item.task, state: "verified" } : null })),
    };
    const html = renderToStaticMarkup(<WorkingFormulaOrder workspace={checked} sourceKind="same_day" />);
    expect(html).toContain("Today order");
    expect(html).not.toContain("<th>Check</th>");
    expect(html).not.toContain("✓ Checked");
  });

  it("offers an operator-controlled print action for the selected date", () => {
    const html = renderToStaticMarkup(<WorkingFormulaDialog date="2026-08-26" items={[]} onClose={() => undefined} />);
    expect(html).toContain("Working formula · 2026-08-26");
    expect(html).toContain("Print working formula");
    expect(html).toContain("Print all labels (0)");
    expect(html).toContain("Batch check all (0)");
    expect(html).toContain("Sort by");
    expect(html).toContain(">Order</button>");
    expect(html).toContain(">Drug</button>");
    expect(html).toContain('aria-label="Close working formula"');
    expect(html).not.toContain(">Close</button>");
    expect(html).toContain("Preparing daily working formula");
  });

  it("groups matching drugs and sorts groups by drug name", () => {
    const alphaWorkspace: PreparationWorkspace = {
      ...workspace,
      orderId: 8,
      orderCode: "OF-SYN-8",
      items: workspace.items.map((item) => ({ ...item, orderItemId: 9, drugId: 10, drugName: "Alpha medicine" })),
    };
    const secondAlphaWorkspace: PreparationWorkspace = {
      ...workspace,
      orderId: 9,
      orderCode: "OF-SYN-9",
      items: workspace.items.map((item) => ({ ...item, orderItemId: 10, drugId: 10, drugName: "Alpha medicine" })),
    };
    const zuluWorkspace: PreparationWorkspace = {
      ...workspace,
      items: workspace.items.map((item) => ({ ...item, drugName: "Zulu medicine" })),
    };
    const groups = buildWorkingFormulaDrugGroups([zuluWorkspace, secondAlphaWorkspace, alphaWorkspace], []);
    expect(groups.map((group) => group.drugName)).toEqual(["Alpha medicine", "Zulu medicine"]);
    expect(groups[0].entries).toHaveLength(2);

    const html = renderToStaticMarkup(<WorkingFormulaDrugGroups workspaces={[zuluWorkspace, secondAlphaWorkspace, alphaWorkspace]} queueItems={[]} />);
    expect(html.indexOf("Alpha medicine")).toBeLessThan(html.indexOf("Zulu medicine"));
    expect(html).toContain("2 preparation task(s)");
    expect(html).toContain("HN SYN-TH-7 - ผู้ป่วยสังเคราะห์");
  });
});
