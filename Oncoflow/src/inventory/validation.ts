export type InventoryOperation = "receipt" | "adjustment" | "manualIssue";

export interface InventoryMovementDraft {
  operation: InventoryOperation;
  quantity: string;
  direction: "increase" | "decrease";
  occurredAt: string;
  reference: string;
  note: string;
}

export type InventoryFormErrors = Partial<Record<"quantity" | "occurredAt" | "reference" | "note", string>>;

export function validateInventoryMovement(draft: InventoryMovementDraft): InventoryFormErrors {
  const errors: InventoryFormErrors = {};
  const quantity = Number(draft.quantity);
  if (!draft.quantity.trim() || !Number.isFinite(quantity) || quantity <= 0) {
    errors.quantity = "Enter a quantity greater than zero.";
  } else if (draft.operation === "manualIssue" && !Number.isInteger(quantity)) {
    errors.quantity = "Issue quantity must be a whole number.";
  }
  if (draft.occurredAt && Number.isNaN(Date.parse(draft.occurredAt))) {
    errors.occurredAt = "Enter a valid date and time.";
  }
  if (draft.reference.trim().length > 120) {
    errors.reference = "Reference must be 120 characters or fewer.";
  }
  if (draft.note.trim().length > 1_000) {
    errors.note = "Note must be 1,000 characters or fewer.";
  } else if (draft.operation !== "receipt" && !draft.note.trim()) {
    errors.note = "A reason is required for this movement.";
  }
  return errors;
}

export function emptyInventoryMovementDraft(): InventoryMovementDraft {
  return {
    operation: "receipt",
    quantity: "",
    direction: "increase",
    occurredAt: "",
    reference: "",
    note: "",
  };
}
