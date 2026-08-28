import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { PreparationWorkspace } from "../types/preparation";
import { preparationTaskSequences, PreparationWorkspaceView } from "./PreparationWorkspace";

const assignedPreparer = { id: 8, displayName: "เภสัชกรผู้เตรียม", role: "pharmacist" as const };

const workspace: PreparationWorkspace = {
  orderId: 10,
  orderCode: "OF-SYN-10",
  patientHn: "SYN-HN",
  patientName: "ผู้ป่วยทดสอบ",
  wardName: "หอผู้ป่วยสังเคราะห์",
  regimenName: "สูตรสังเคราะห์",
  treatmentTime: "2026-08-23T09:00",
  preparationDate: "2026-08-23",
  assignedPreparer,
  editable: true,
  eligibilityRuleId: "legacy-cytotoxic-v8:preparation-marker",
  excludedItemCount: 1,
  pharmacists: [assignedPreparer],
  safety: { mode: "active", rulesetVersion: "legacy-cytotoxic-v8", evaluatedRuleCount: 0, unsupportedRuleCount: 0, notice: "Deferred in this release.", findings: [] },
  items: [{
    orderItemId: 11,
    drugId: 1,
    drugCode: "PREP",
    drugName: "ยาสังเคราะห์สำหรับเตรียม",
    orderedDoseText: "100",
    doseUnitText: "mg",
    diluentName: "สารละลายทดสอบ",
    diluentVolumeMl: 100,
    routeName: "IV",
    rateText: "60 min",
    treatmentDay: "1",
    sequenceNo: 1,
    regimenDetails: "คำแนะนำสังเคราะห์",
    drugDetail: "รายละเอียดสังเคราะห์",
    drugStorage: "เก็บแบบทดสอบ",
    eligibility: { status: "eligible", ruleId: "legacy-cytotoxic-v8:preparation-marker", reason: "Enabled by synthetic marker." },
    referenceQuantity: { status: "calculated", drugSolutionVolumeMl: "20", packageEquivalent: "2", formula: "synthetic confirmed formula", notice: "Reference only." },
    calculation: {
      status: "calculated",
      rulesetVersion: "legacy-cytotoxic-v8",
      ruleId: "legacy-cytotoxic-v8:preparation-container-use",
      orderedDose: { value: "100", unit: "mg" },
      presentation: { amountPerContainer: { value: "50", unit: "mg" }, volumePerContainerMl: "10", containerLabel: "Amp.", rawPackageLabel: "Amp." },
      concentration: "5 mg/mL",
      withdrawalVolumeMl: "20",
      containersRequired: "2",
      unusedAmount: { value: "0", unit: "mg" },
      inventoryProjection: { trackingEnabled: true, currentStock: "1", containersRequired: "2", projectedStock: "-1", minimumStock: "1", state: "shortage", unitNotice: "Read-only preview." },
      legacyReference: { storedQuantity: "2", storedQuantitySemantics: "UNKNOWN", calculatedPackageEquivalent: "2", calculatedSolutionVolumeMl: "20", comparisonStatus: "not_comparable", notice: "Preserved raw." },
      trace: [{ step: "container-count", expression: "FixNumber(100 / 50)", result: "2", confidence: "CONFIRMED" }],
      warnings: [{ code: "projected-inventory-shortage", message: "Projected inventory is negative. This advisory shortage does not block preparation." }],
    },
    defaultPreparationVolumeMl: "120",
    task: {
      id: 1,
      sourceOrderId: 10,
      sourceOrderItemId: 11,
      preparationDate: "2026-08-23",
      drugId: 1,
      state: "pending",
      orderedDoseText: "100",
      doseUnitText: "mg",
      diluentId: 1,
      diluentName: "สารละลายทดสอบ",
      diluentVolumeMl: 100,
      routeId: 1,
      routeName: "IV",
      rateText: "60 min",
      treatmentDay: "1",
      startDate: "2026-08-23",
      stopDate: "2026-08-27",
      sequenceNo: 1,
      regimenDetails: "คำแนะนำสังเคราะห์",
      drugDetail: "รายละเอียดสังเคราะห์",
      drugStorage: "เก็บแบบทดสอบ",
      preparationVolumeMl: 120,
      preparationNotes: "บันทึกการเตรียม",
      finalContainerCount: 1,
      createdAt: "2026-08-23 09:00:00",
      updatedAt: "2026-08-23 09:01:00",
      preparedAt: null,
      verifiedAt: null,
      preparedBy: null,
      verifiedBy: null,
      inventoryPosting: null,
    },
  }],
  safetyAcknowledgements: [],
};

const handlers = {
  onBack: () => undefined,
  onOpenOrder: () => undefined,
  onSave: () => undefined,
  onCheck: () => undefined,
  onOutput: () => undefined,
  onToggleSelected: () => undefined,
  onPrintAll: () => undefined,
  onPrintSelected: () => undefined,
};

function render(value: PreparationWorkspace, selected = new Set<number>()): string {
  return renderToStaticMarkup(<PreparationWorkspaceView workspace={value} busy={false} operationError={null} batchMessage={null} selectedTaskIds={selected} {...handlers} />);
}

describe("PreparationWorkspace", () => {
  it("uses the pharmacist assigned during ordering and exposes one check step", () => {
    const html = render(workspace);
    expect(html).toContain('class="back-button"');
    expect(html).toContain("← Preparation queue");
    expect(html).toContain("HN SYN-HN: ผู้ป่วยทดสอบ");
    expect(html).toContain('class="preparation-hero__order-number">Order OF-SYN-10');
    expect(html).toContain("แก้ไขคำสั่งใช้ยา");
    expect(html).toContain("Preparation pharmacist");
    expect(html).toContain("เภสัชกรผู้เตรียม");
    expect(html).toContain("Check preparation");
    expect(html).not.toContain("Verify preparation");
    expect(html).not.toContain("Select preparation pharmacist");
    expect(html).not.toContain("Initialize daily preparation");
  });

  it("keeps ordered values authoritative and renders deterministic preparation references", () => {
    const html = render(workspace);
    expect(html).not.toContain("Order values are authoritative");
    expect(html).toContain("100 mg");
    expect(html).not.toContain("PREP ·");
    expect(html).not.toContain("Enabled by synthetic marker.");
    expect(html).toContain("20 mL");
    expect(html).toContain("2 × Amp.");
    expect(html).not.toContain("Ordered dose remains authoritative");
    expect(html).not.toContain("Inventory preview");
    expect(html).not.toContain("1 → -1");
    expect(html).not.toContain("Shortage is advisory");
    expect(html).toContain("Warnings and precautions are deferred to a future release");
  });

  it("defaults final volume to diluent plus withdrawal and allows manual entry", () => {
    const html = render(workspace);
    expect(html).toContain("ปริมาตรสารละลาย + ปริมาตรยา");
    expect(html).toContain("(ค่าเริ่มต้น)");
    expect(html).toContain("100 mL + 20 mL = 120 mL");
    expect(html).toMatch(/checked="" value="solution_plus_drug"|value="solution_plus_drug" checked=""/);
    expect(html).toContain('value="120"');
    expect(html).toContain("ระบุปริมาตรสุดท้ายเอง");

    const manual = {
      ...workspace,
      items: workspace.items.map((item) => ({
        ...item,
        task: item.task ? { ...item.task, preparationVolumeMl: 110 } : null,
      })),
    };
    expect(render(manual)).toMatch(/checked="" value="manual"|value="manual" checked=""/);
  });

  it("defaults to one duplicate label and distinguishes it from inventory vials", () => {
    const html = render(workspace);
    expect(html).toContain("จำนวนฉลากยา");
    expect(html).toContain("จำนวนฉลากสำหรับยารายการนี้");
    expect(html).toContain('min="1" max="20" step="1" value="1"');
    expect(html).toContain("ordered dose และ final volume เหมือนกัน");
    expect(html).not.toContain("แบ่ง dose และ volume");
    expect(html).not.toContain("ระบุแต่ละขวด/ถุง");
  });

  it("shows checked output, order-batch printing, and individual correction printing", () => {
    const checked: PreparationWorkspace = {
      ...workspace,
      items: workspace.items.map((item) => ({ ...item, task: item.task && {
        ...item.task,
        state: "verified",
        preparedAt: "2026-08-23 09:02:00",
        preparedBy: assignedPreparer,
        verifiedAt: "2026-08-23 09:03:00",
        verifiedBy: { id: 9, displayName: "ผู้ช่วยเภสัชกรผู้ตรวจ", role: "pharmacist" },
        inventoryPosting: {
          id: 31,
          status: "posted",
          inventoryMovementId: 73,
          containersRequired: "2",
          balanceBefore: "1",
          balanceAfter: "-1",
          resultingStockState: "shortage",
          calculationStatus: "calculated",
          calculationRulesetVersion: "legacy-cytotoxic-v8",
          calculationRuleId: "legacy-cytotoxic-v8:preparation-container-use",
          workflowRuleId: "oncoflow-preparation-inventory-v1",
          reasonCode: "supported_container_requirement",
          issuedAt: "2026-08-23 09:03:00",
          recordedAt: "2026-08-23 09:03:00",
          actor: { id: 9, displayName: "ผู้ช่วยเภสัชกรผู้ตรวจ", role: "pharmacist" },
        },
      } })),
    };
    const html = render(checked, new Set([1]));
    expect(html).toContain("Checked by");
    expect(html).toContain("Print all labels in order");
    expect(html).toContain("Print selected (1)");
    expect(html).toContain("พิมพ์ทั้งใบสั่งยา หรือพิมพ์เฉพาะรายการที่เลือก");
    expect(html).toContain("Preview / print this label");
    expect(html).not.toContain("Assigned preparation pharmacist");
    expect(html).not.toContain("Prepared by");
    expect(html.indexOf("Select for label batch")).toBeLessThan(html.indexOf("Ordered dose"));
    expect(html.indexOf("Preview / print this label")).toBeLessThan(html.indexOf("Ordered dose"));
    expect(html).not.toContain("Inventory consumption");
    expect(html).not.toContain("Automatic issue posted");
    expect(html).not.toContain("Shortage recorded");
  });

  it("shows regimen metadata only when a regimen was selected", () => {
    expect(render(workspace)).toContain("<small>Regimen</small>สูตรสังเคราะห์");
    expect(render({ ...workspace, regimenName: null })).not.toContain("<small>Regimen</small>");
  });

  it("renders an empty preparation workspace after every order item is removed", () => {
    const html = render({ ...workspace, items: [], excludedItemCount: 0 });
    expect(html).toContain("ไม่มีรายการยาสำหรับเตรียม");
    expect(html).toContain("ใบสั่งยานี้ไม่มีรายการยาที่ต้องเตรียมในวันที่เลือก");
    expect(html).toContain("แก้ไขคำสั่งใช้ยา");
    expect(html).not.toContain("Preparation unavailable");
    expect(html).not.toContain("local preparation selector");
  });

  it("numbers drugs by the preparation tasks due that day instead of source order sequence", () => {
    const first = workspace.items[0];
    const second = {
      ...first,
      orderItemId: 12,
      sequenceNo: 9,
      task: first.task ? { ...first.task, id: 2, sourceOrderItemId: 12 } : null,
    };
    const sequences = preparationTaskSequences([first, second]);

    expect(sequences.get(1)).toBe(1);
    expect(sequences.get(2)).toBe(2);
  });

  it("keeps unsupported calculations available for checking without guessed conversion", () => {
    const unsupported: PreparationWorkspace = { ...workspace, items: workspace.items.map((item) => ({ ...item, calculation: { ...item.calculation, status: "unsupported", withdrawalVolumeMl: null, containersRequired: null, unusedAmount: null, warnings: [{ code: "unit-relationship-unknown", message: "No conversion was guessed." }] } })) };
    const html = render(unsupported);
    expect(html).toContain("Unsupported");
    expect(html).toContain("No conversion was guessed");
    expect(html).toContain("Check preparation");
  });
});
