import { describe, expect, it } from "vitest";

import {
  emptyDrugForm,
  formToDrugInput,
  type DrugFormValues,
  validateDrugForm,
  withSuggestedDrugCode,
} from "./form";

function form(overrides: Partial<DrugFormValues> = {}): DrugFormValues {
  return {
    ...emptyDrugForm,
    code: "SYN-D01",
    name: "Synthetic medicine",
    ...overrides,
  };
}

describe("drug form validation", () => {
  it("requires code and name and rejects negative numeric values", () => {
    const errors = validateDrugForm(
      form({ code: " ", name: "", price: "-1", maxDose: "-2" }),
    );

    expect(errors.code).toBeDefined();
    expect(errors.name).toBeDefined();
    expect(errors.price).toBeDefined();
    expect(errors.maxDose).toBeDefined();
  });

  it("rejects an inventory maximum below the minimum", () => {
    const errors = validateDrugForm(
      form({ inventoryMin: "20", inventoryMax: "10" }),
    );

    expect(errors.inventoryMax).toBeDefined();
  });

  it("rejects negative default-rate and expiry durations", () => {
    const errors = validateDrugForm(
      form({ defaultRate: "-15 min", expiryTime: "-2 hr" }),
    );

    expect(errors.defaultRate).toBeDefined();
    expect(errors.expiryTime).toBeDefined();
  });

  it("trims Thai text and converts blank optional values to null", () => {
    const input = formToDrugInput(
      form({
        code: "  TH-01  ",
        name: "  ยาทดสอบ  ",
        warning: "  ",
        dosePerPack: "25.5",
      }),
    );

    expect(input.code).toBe("TH-01");
    expect(input.name).toBe("ยาทดสอบ");
    expect(input.warning).toBeNull();
    expect(input.dosePerPack).toBe(25.5);
  });

  it("preserves nullable legacy alert flags", () => {
    expect(formToDrugInput(form()).maxDilutionAlert).toBeNull();
    expect(
      formToDrugInput(
        form({ maxDilutionAlert: "true", inventoryCut: "false" }),
      ),
    ).toMatchObject({ maxDilutionAlert: true, inventoryCut: false });
  });

  it("applies an automatic code only while the new-drug code is blank", () => {
    expect(withSuggestedDrugCode({ ...emptyDrugForm }, "OF-D000001").code).toBe("OF-D000001");
    expect(withSuggestedDrugCode(form({ code: "CUSTOM" }), "OF-D000002").code).toBe("CUSTOM");
  });
});
