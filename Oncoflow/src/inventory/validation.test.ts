import { describe, expect, it } from "vitest";

import { emptyInventoryMovementDraft, validateInventoryMovement } from "./validation";

describe("inventory movement validation", () => {
  it("rejects zero, negative, and malformed quantities", () => {
    for (const quantity of ["", "0", "-2", "not-a-number"]) {
      expect(validateInventoryMovement({ ...emptyInventoryMovementDraft(), quantity }).quantity).toBeDefined();
    }
  });

  it("allows a positive receipt without a note", () => {
    expect(validateInventoryMovement({ ...emptyInventoryMovementDraft(), quantity: "2.5" })).toEqual({});
  });

  it("requires a reason for adjustments and manual issues", () => {
    const base = { ...emptyInventoryMovementDraft(), quantity: "3" };
    expect(validateInventoryMovement({ ...base, operation: "adjustment" }).note).toContain("required");
    expect(validateInventoryMovement({ ...base, operation: "manualIssue" }).note).toContain("required");
    expect(validateInventoryMovement({ ...base, operation: "manualIssue", note: "Synthetic reason" })).toEqual({});
  });

  it("requires whole numbers for manual issues while receipts may remain fractional", () => {
    expect(validateInventoryMovement({ ...emptyInventoryMovementDraft(), quantity: "2.5" })).toEqual({});
    expect(validateInventoryMovement({ ...emptyInventoryMovementDraft(), operation: "manualIssue", quantity: "2.5", note: "Synthetic reason" }).quantity).toContain("whole number");
  });
});
