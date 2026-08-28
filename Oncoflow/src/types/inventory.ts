export type InventorySortField =
  | "code"
  | "name"
  | "currentStock"
  | "minimum"
  | "maximum"
  | "state";
export type InventorySortDirection = "asc" | "desc";
export type StockState =
  | "untracked"
  | "unknown"
  | "shortage"
  | "out"
  | "low"
  | "normal";

export interface InventoryListRequest {
  search?: string | null;
  trackedOnly: boolean;
  lowStockOnly: boolean;
  sortBy: InventorySortField;
  sortDirection: InventorySortDirection;
  limit: number;
  offset: number;
}

export interface InventorySummary {
  drugId: number;
  drugCode: string;
  drugName: string;
  legacyDrugUnit: string | null;
  package: string | null;
  currentStock: number | null;
  minimumStock: number | null;
  maximumStock: number | null;
  trackingEnabled: boolean;
  stockState: StockState;
}

export interface InventoryListResponse {
  items: InventorySummary[];
  total: number;
}

export interface InventoryDetail extends InventorySummary {
  legacyInventorySnapshot: number | null;
  legacyInventoryCutoff: boolean | null;
  dosePerPack: number | null;
  volumePerPackMl: number | null;
  legacyInventoryEventCount: number;
  quantitySemantics: "unresolved_legacy_inventory_unit";
}

export type InventoryMovementType =
  | "opening_balance"
  | "receipt"
  | "manual_issue"
  | "adjustment_increase"
  | "adjustment_decrease"
  | "preparation_issue";

export interface InventoryMovement {
  id: number;
  movementType: InventoryMovementType;
  quantityDelta: number;
  resultingBalance: number;
  occurredAt: string | null;
  createdAt: string;
  actorDisplayName: string | null;
  referenceType: string | null;
  referenceId: string | null;
  note: string | null;
  preparationTaskId: number | null;
}

export interface InventoryMovementListResponse {
  items: InventoryMovement[];
  total: number;
}

export interface InventoryReceiptInput {
  drugId: number;
  quantity: number;
  occurredAt: string | null;
  reference: string | null;
  note: string | null;
}

export interface InventoryAdjustmentInput {
  drugId: number;
  direction: "increase" | "decrease";
  quantity: number;
  occurredAt: string | null;
  note: string;
  reference: string | null;
}

export interface InventoryManualIssueInput {
  drugId: number;
  quantity: number;
  occurredAt: string | null;
  note: string;
  reference: string | null;
}

export interface InventoryMovementResult {
  inventory: InventoryDetail;
  movement: InventoryMovement;
}
