import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { PreparationOutput } from "../types/output";
import { LABEL_DIMENSIONS, PreparationOutputView } from "./PreparationOutput";

const output: PreparationOutput = {
  label: {
    snapshotId: 91,
    templateVersion: "oncoflow-preparation-label-v1",
    generatedAt: "2026-08-23T09:21:00",
    printTime: "2026-08-23T09:21:00",
    expirationAt: "2026-08-23T17:21:00",
    preparationId: 10,
    orderId: 20,
    orderReference: "OF-SYN-20",
    patientIdentifier: "SYN-HN",
    patientName: "ผู้ป่วยทดสอบ",
    hospitalName: "โรงพยาบาลทดสอบ",
    regimenName: "สูตรสังเคราะห์",
    treatmentAt: "2026-08-23T09:00:00",
    treatmentDay: "Day 1",
    drugCode: "SYN-D",
    drugName: "ยาเคมีบำบัดทดสอบ",
    orderedDoseText: "100.5",
    doseUnitText: "mg",
    diluentName: "สารละลายทดสอบ",
    diluentVolumeMl: 100,
    withdrawalVolumeMl: "20",
    finalVolumeMl: 120,
    routeName: "IV",
    infusionRateOrDuration: "60 min",
    warningText: "คำเตือนทดสอบ",
    expiryTimeText: "8 hr",
    expiryStorageText: "ป้องกันแสง",
    preparedBy: "เภสัชกรหนึ่ง",
    preparedAt: "2026-08-23T09:15:00",
    verifiedBy: "เภสัชกรสอง",
    verifiedAt: "2026-08-23T09:20:00",
  },
  containers: [{ containerIndex: 1 }],
  summary: {
    preparationInstructions: "คำแนะนำสังเคราะห์",
    preparationNotes: "บันทึกสังเคราะห์",
    storageReference: "ข้อความเก็บรักษาเดิม",
    safetyReviewStatus: "verified_workflow_complete",
    inventoryPostingStatus: "posted",
    inventoryMovementId: 73,
    containersRequired: 3,
    inventoryBalanceBefore: 1,
    inventoryBalanceAfter: -2,
    inventoryStockState: "shortage",
    calculationRulesetVersion: "legacy-cytotoxic-v8",
    calculationRuleId: "legacy-cytotoxic-v8:preparation-container-use",
    presentationNotice: "Only persisted verification values are shown; no preparation calculation runs during output rendering.",
  },
  printRequestCount: 0,
};

const handlers = {
  printerName: "Synthetic Windows queue",
  message: null,
  onClose: () => undefined,
  onPrint: () => undefined,
  onDimensions: () => undefined,
};

describe("PreparationOutput", () => {
  it("renders a verified Thai label and pharmacist preparation summary", () => {
    const html = renderToStaticMarkup(<PreparationOutputView output={output} dimensions={LABEL_DIMENSIONS[0]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("Final checked preparation label");
    expect(html).toContain("OncoFlow - โรงพยาบาลทดสอบ");
    expect(html).toContain("ผู้ป่วยทดสอบ");
    expect(html).toContain("ยาเคมีบำบัดทดสอบ");
    expect(html).toContain("ยาเคมีบำบัดทดสอบ 100.5 mg in สารละลายทดสอบ 100 mL");
    expect(html).toContain("Withdrawal: 20 mL");
    expect(html).toContain("IV in 60 min");
    expect(html).toContain("ข้อความเก็บรักษาเดิม");
    expect(html).not.toContain("ข้อความเก็บรักษาเดิม | ป้องกันแสง");
    expect(html).toContain("คำเตือนทดสอบ");
    expect(html).toContain("| HN SYN-HN");
    expect(html).toContain("(1/1)");
    expect(html).toContain("Preparation summary");
    expect(html).toContain("คำแนะนำสังเคราะห์");
    expect(html).toContain("Prepared by เภสัชกรหนึ่ง | 23/08/2569 16:15");
    expect(html).toContain("หมดอายุ 23/08/2569 17:21");
    expect(html).toContain("oncoflow-preparation-label-v1");
    expect(html).toContain("Print label");
    expect(html).not.toContain("DRAFT");
  });

  it("shows issued stock provenance and keeps shortage printing available", () => {
    const html = renderToStaticMarkup(<PreparationOutputView output={output} dimensions={LABEL_DIMENSIONS[0]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("Containers issued");
    expect(html).toContain("#73");
    expect(html).toContain("1 → -2");
    expect(html).toContain("Shortage");
    expect(html).toContain("did not block preparation checking or label output");
    expect(html).toContain(">Print label<");
    expect(html).not.toContain("disabled=\"\"");
  });

  it("renders identical dose and volume on every numbered duplicate label", () => {
    const multi: PreparationOutput = {
      ...output,
      containers: [
        { containerIndex: 1 },
        { containerIndex: 2 },
      ],
    };
    const html = renderToStaticMarkup(<PreparationOutputView output={multi} dimensions={LABEL_DIMENSIONS[0]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("1/2");
    expect(html).toContain("2/2");
    expect(html.match(/ยาเคมีบำบัดทดสอบ 100.5 mg in สารละลายทดสอบ 100 mL/g)).toHaveLength(2);
    expect(html.match(/23\/08\/2569 17:21/g)).toHaveLength(2);
    expect(html).toContain("Print 2 labels");
    expect(html.match(/preparation-label-print-root/g)).toHaveLength(2);
  });

  it("renders missing optional and pre-integration values safely", () => {
    const sparse: PreparationOutput = {
      ...output,
      label: {
        ...output.label,
        patientName: null,
        hospitalName: null,
        regimenName: null,
        treatmentAt: null,
        treatmentDay: null,
        diluentName: null,
        diluentVolumeMl: null,
        withdrawalVolumeMl: null,
        finalVolumeMl: null,
        routeName: null,
        infusionRateOrDuration: null,
        warningText: null,
        expiryTimeText: null,
        expiryStorageText: null,
        expirationAt: null,
        preparedBy: null,
      },
      summary: {
        ...output.summary,
        preparationInstructions: null,
        preparationNotes: null,
        storageReference: null,
        inventoryPostingStatus: null,
        inventoryMovementId: null,
        containersRequired: null,
        inventoryBalanceBefore: null,
        inventoryBalanceAfter: null,
        inventoryStockState: null,
      },
    };
    const html = renderToStaticMarkup(<PreparationOutputView output={sparse} dimensions={LABEL_DIMENSIONS[1]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("Not recorded");
    expect(html).toContain("Pre-integration preparation");
    expect(html).toContain("No inventory posting was backfilled");
    expect(html).not.toContain("undefined");
  });

  it("omits the in prefix when rate is missing or zero", () => {
    const zeroRate = { ...output, label: { ...output.label, infusionRateOrDuration: "0 min" } };
    const html = renderToStaticMarkup(<PreparationOutputView output={zeroRate} dimensions={LABEL_DIMENSIONS[0]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("preparation-label__route-rate\">IV<");
    expect(html).not.toContain("in 0 min");
  });

  it("labels subsequent local print requests as reprints and separates dimensions", () => {
    const reprint = { ...output, printRequestCount: 2 };
    const html = renderToStaticMarkup(<PreparationOutputView output={reprint} dimensions={LABEL_DIMENSIONS[2]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("Reprint label");
    expect(html).toContain("Compact · 100 × 70 mm");
    expect(html).toContain("Narrow · 100 × 50 mm");
    expect(html).toContain("Large · 148 × 105 mm");
    expect(html).toContain("--preparation-label-width:148mm");
    expect(html).toContain("--preparation-label-font-header:22px");
    expect(html).toContain("--preparation-label-font-patient:20px");
    expect(html).toContain("--preparation-label-font-withdrawal:16px");
    expect(html).toContain("--preparation-label-font-drug:21px");
    expect(html).toContain("--preparation-label-font-route-rate:18px");
    expect(html).toContain("--preparation-label-font-storage:16px");
    expect(html).toContain("--preparation-label-font-warning:16px");
    expect(html).toContain("--preparation-label-font-prepared-by:15px");
    expect(html).toContain("--preparation-label-font-expiration:18px");
    expect(html).toContain("Content is fixed; dimensions affect raster layout only");
  });

  it("uses an exact landscape frame for a configured eighty by fifty label", () => {
    const configured = { id: "configured", label: "Configured · 80 × 50 mm", widthMm: 80, heightMm: 50 };
    const html = renderToStaticMarkup(<PreparationOutputView output={output} dimensions={configured} busy={false} error={null} {...handlers} />);
    expect(html).toContain("--preparation-label-width:80mm");
    expect(html).toContain("--preparation-label-height:50mm");
    expect(html).toContain("--preparation-label-margin:2.2857142857142856mm");
    expect(html).toContain("preparation-label__fit");
  });

  it("states that expiration is only derived from configured Drug master duration", () => {
    const html = renderToStaticMarkup(<PreparationOutputView output={output} dimensions={LABEL_DIMENSIONS[0]} busy={false} error={null} {...handlers} />);
    expect(html).toContain("Legacy storage reference");
    expect(html).toContain("Expiration is shown only when an expiry duration is configured in Drug master");
    expect(html).not.toContain("Beyond-use date");
  });

  it("requires a configured Windows queue before enabling final print", () => {
    const html = renderToStaticMarkup(<PreparationOutputView output={output} dimensions={LABEL_DIMENSIONS[0]} printerName={null} message={null} busy={false} error={null} onClose={() => undefined} onPrint={() => undefined} onDimensions={() => undefined} />);
    expect(html).toContain("Settings → Hardware");
    expect(html).toContain("disabled=\"\"");
  });
});
