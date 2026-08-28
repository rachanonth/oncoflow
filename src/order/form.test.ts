import { describe, expect, it } from "vitest";
import { acquireSubmissionLock, convertRateValue, emptyOrderItemValues, emptyOrderValues, itemToValues, toOrderInput, toOrderItemInput, validateOrder, validateOrderItem } from "./form";

describe("order form mapping", () => {
  it("requires a patient and limits the raw legacy type", () => {
    expect(validateOrder({ ...emptyOrderValues(), orderTime: "", orderType: "ABC" })).toEqual({ patientId: "Patient is required.", assignedPreparerUserId: "Preparation pharmacist is required.", orderType: "Use 2 characters or fewer." });
  });
  it("trims Thai notes and converts blank lookups to NULL", () => {
    const input = toOrderInput({ ...emptyOrderValues(7), orderTime: "", regimenId: "", note: "  บันทึกทดสอบ  " });
    expect(input.note).toBe("บันทึกทดสอบ");
    expect(input.regimenId).toBeNull();
  });
  it("preserves a raw dose expression and optional route/diluent NULLs", () => {
    const input = toOrderItemInput({ ...emptyOrderItemValues(), drugId: "3", doseText: " AUC 5 " });
    expect(input.doseText).toBe("AUC 5");
    expect(input.routeId).toBeNull();
    expect(input.diluentId).toBeNull();
    expect(input.diluentVolumeMl).toBeNull();
  });
  it("maps and validates an optional per-line diluent volume", () => {
    expect(toOrderItemInput({ ...emptyOrderItemValues(), drugId: "3", diluentVolumeMl: " 250.5 " }).diluentVolumeMl).toBe(250.5);
    expect(validateOrderItem({ ...emptyOrderItemValues(), drugId: "3", diluentVolumeMl: "-1" })).toEqual({ diluentVolumeMl: "Enter zero or a positive volume." });
  });
  it("converts equivalent minute and hour rate values", () => {
    expect(convertRateValue("90", "minute", "hour")).toBe("1.5");
    expect(convertRateValue("1.5", "hour", "minute")).toBe("90");
    expect(toOrderItemInput({ ...emptyOrderItemValues(), drugId: "3", rateValue: "2", rateUnit: "hour", rateTouched: true }).rate).toBe("2 hr");
  });
  it("preserves unrecognized existing rate text until it is replaced", () => {
    const values = itemToValues({ id: 1, drugId: 3, drugName: "Synthetic", diluentId: null, diluentName: null, diluentVolumeMl: null, routeId: null, routeName: null, startDate: null, stopDate: null, dose: null, doseText: null, scheduleTime: null, numberOfDrug: null, missing: false, printed: false, rate: "slow infusion per protocol", orderingNo: null, runningNo: null, runningSum: null, inventoryDate: null, sourceRegimenItemId: null, regimenDoseText: null, regimenUnitText: null, regimenRouteText: null, regimenDetails: null, regimenItemGroup: null, regimenDuration: null, regimenStartDay: null, regimenOrderingNo: null });
    expect(toOrderItemInput(values).rate).toBe("slow infusion per protocol");
  });
  it("validates dates, quantity, and numeric dose without rejecting text expressions", () => {
    expect(validateOrderItem({ ...emptyOrderItemValues(), drugId: "1", startDate: "2026-08-20", stopDate: "2026-08-19", numberOfDrug: "-1" })).toEqual({ stopDate: "Stop date cannot be before start date.", numberOfDrug: "Enter zero or a positive number." });
    expect(validateOrderItem({ ...emptyOrderItemValues(), drugId: "1", doseText: "100 mg/m²" })).toEqual({});
  });
  it("defaults new drug start and stop dates to today's Bangkok date", () => {
    const values = emptyOrderItemValues(new Date("2026-08-25T18:30:00Z"));
    expect(values.startDate).toBe("2026-08-26");
    expect(values.stopDate).toBe("2026-08-26");
  });
  it("acquires a synchronous lock before a duplicate order submission", () => {
    const lock = { current: false };
    expect(acquireSubmissionLock(lock)).toBe(true);
    expect(acquireSubmissionLock(lock)).toBe(false);
    lock.current = false;
    expect(acquireSubmissionLock(lock)).toBe(true);
  });
});
