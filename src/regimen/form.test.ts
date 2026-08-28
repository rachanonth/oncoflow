import { describe, expect, it } from "vitest";

import {
  emptyItemValues,
  emptyRegimenValues,
  toGroupInput,
  toItemInput,
  toRegimenInput,
  validateGroup,
  validateItem,
  validateRegimen,
} from "./form";

describe("regimen form mapping", () => {
  it("requires regimen code and name", () => {
    expect(validateRegimen(emptyRegimenValues)).toEqual({
      code: "Regimen code is required.",
      name: "Regimen name is required.",
    });
  });

  it("trims and preserves Thai regimen text", () => {
    const input = toRegimenInput({
      ...emptyRegimenValues,
      code: "  TH-01 ",
      name: "  สูตรยาทดสอบ ",
    });
    expect(input.code).toBe("TH-01");
    expect(input.name).toBe("สูตรยาทดสอบ");
  });

  it("converts blank group fields to NULL and validates integers", () => {
    expect(toGroupInput({ note: "  ", cycleDay: "", cycleCount: "" })).toEqual({
      note: null,
      cycleDay: null,
      cycleCount: null,
    });
    expect(validateGroup({ note: "", cycleDay: "-1", cycleCount: "1.5" })).toEqual({
      cycleDay: "Enter a whole number of zero or greater.",
      cycleCount: "Enter a whole number of zero or greater.",
    });
  });

  it("requires a local drug and validates item ordering fields", () => {
    const values = emptyItemValues(10);
    values.startDay = "-1";
    values.orderingNo = "1.5";
    expect(validateItem(values)).toEqual({
      drugId: "Drug is required.",
      startDay: "Enter a whole number of zero or greater.",
      orderingNo: "Enter a whole number of zero or greater.",
    });
  });

  it("preserves raw dose expressions and NULL lookup values", () => {
    const values = emptyItemValues(10);
    values.drugId = "4";
    values.doseText = " AUC 5 ";
    values.unitText = " mg/m² ";
    const input = toItemInput(values);
    expect(input.doseText).toBe("AUC 5");
    expect(input.unitText).toBe("mg/m²");
    expect(input.defaultRouteId).toBeNull();
    expect(input.defaultDiluentId).toBeNull();
  });
});
