import type { UserRole } from "./auth";
import type { SafetyEvaluation } from "./safety";

export interface PreparationQueueRequest {
  search?: string | null;
  dateFrom?: string | null;
  dateTo?: string | null;
  preparationDate?: string | null;
  sourceFilter?: PreparationQueueSourceFilter;
  limit?: number;
  offset?: number;
}

export type PreparationQueueSourceFilter = "all" | "same_day" | "continuing" | "rescheduled";

export interface PreparationQueueItem {
  orderId: number;
  orderCode: string;
  patientHn: string;
  patientName: string;
  wardName: string | null;
  regimenName: string | null;
  treatmentTime: string | null;
  preparationDate: string;
  sourceKind: Exclude<PreparationQueueSourceFilter, "all">;
  eligibleItemCount: number;
  initializedItemCount: number;
  pendingItemCount: number;
  preparedItemCount: number;
  verifiedItemCount: number;
  printedLabelCount: number;
}

export interface PreparationQueueResponse {
  items: PreparationQueueItem[];
  total: number;
}

export type PreparationState = "pending" | "prepared" | "verified";

export interface PreparationActor {
  id: number;
  displayName: string;
  role: UserRole;
}

export type PreparationInventoryPostingStatus =
  | "posted"
  | "manual_reconciliation_required"
  | "not_required"
  | "tracking_disabled";

export type PreparationIssueStockState = "normal" | "low" | "out" | "shortage";

export interface PreparationInventoryPosting {
  id: number;
  status: PreparationInventoryPostingStatus;
  inventoryMovementId: number | null;
  containersRequired: string | null;
  balanceBefore: string | null;
  balanceAfter: string | null;
  resultingStockState: PreparationIssueStockState | null;
  calculationStatus: string;
  calculationRulesetVersion: string;
  calculationRuleId: string;
  workflowRuleId: string;
  reasonCode: string;
  issuedAt: string | null;
  recordedAt: string;
  actor: PreparationActor;
}

export interface PreparationTask {
  id: number;
  sourceOrderId: number;
  sourceOrderItemId: number;
  preparationDate: string;
  drugId: number;
  state: PreparationState;
  orderedDoseText: string | null;
  doseUnitText: string | null;
  diluentId: number | null;
  diluentName: string | null;
  diluentVolumeMl: number | null;
  routeId: number | null;
  routeName: string | null;
  rateText: string | null;
  treatmentDay: string | null;
  startDate: string | null;
  stopDate: string | null;
  sequenceNo: number | null;
  regimenDetails: string | null;
  drugDetail: string | null;
  drugStorage: string | null;
  preparationVolumeMl: number | null;
  preparationNotes: string | null;
  finalContainerCount: number;
  createdAt: string;
  updatedAt: string;
  preparedAt: string | null;
  verifiedAt: string | null;
  preparedBy: PreparationActor | null;
  verifiedBy: PreparationActor | null;
  inventoryPosting: PreparationInventoryPosting | null;
}

export type EligibilityStatus = "eligible" | "excluded";

export interface EligibilityDecision {
  status: EligibilityStatus;
  ruleId: string;
  reason: string;
}

export type ReferenceQuantityStatus = "calculated" | "unavailable" | "unsupported";

export interface PreparationReferenceQuantity {
  status: ReferenceQuantityStatus;
  drugSolutionVolumeMl: string | null;
  packageEquivalent: string | null;
  formula: string;
  notice: string;
}

export type PreparationCalculationStatus = "calculated" | "partially_calculated" | "unavailable" | "unsupported";
export type InventoryProjectionState = "normal" | "low" | "out" | "shortage" | "unknown" | "untracked";

export interface CalculationQuantity {
  value: string;
  unit: string;
}

export interface PreparationCalculation {
  status: PreparationCalculationStatus;
  rulesetVersion: string;
  ruleId: string;
  orderedDose: CalculationQuantity | null;
  presentation: {
    amountPerContainer: CalculationQuantity | null;
    volumePerContainerMl: string | null;
    containerLabel: string | null;
    rawPackageLabel: string | null;
  };
  concentration: string | null;
  withdrawalVolumeMl: string | null;
  containersRequired: string | null;
  unusedAmount: CalculationQuantity | null;
  inventoryProjection: {
    trackingEnabled: boolean;
    currentStock: string | null;
    containersRequired: string | null;
    projectedStock: string | null;
    minimumStock: string | null;
    state: InventoryProjectionState;
    unitNotice: string;
  };
  legacyReference: {
    storedQuantity: string | null;
    storedQuantitySemantics: string;
    calculatedPackageEquivalent: string | null;
    calculatedSolutionVolumeMl: string | null;
    comparisonStatus: "formula_confirmed" | "not_comparable" | "unavailable";
    notice: string;
  };
  trace: Array<{ step: string; expression: string; result: string | null; confidence: string }>;
  warnings: Array<{ code: string; message: string }>;
}

export interface PreparationWorkspaceItem {
  orderItemId: number;
  drugId: number;
  drugCode: string;
  drugName: string;
  orderedDoseText: string | null;
  doseUnitText: string | null;
  diluentName: string | null;
  diluentVolumeMl: number | null;
  routeName: string | null;
  rateText: string | null;
  treatmentDay: string | null;
  sequenceNo: number | null;
  regimenDetails: string | null;
  drugDetail: string | null;
  drugStorage: string | null;
  eligibility: EligibilityDecision;
  referenceQuantity: PreparationReferenceQuantity;
  calculation: PreparationCalculation;
  defaultPreparationVolumeMl: string | null;
  task: PreparationTask | null;
}

export interface PreparationWorkspace {
  orderId: number;
  orderCode: string;
  patientHn: string;
  patientName: string;
  wardName: string | null;
  regimenName: string | null;
  treatmentTime: string | null;
  preparationDate: string;
  assignedPreparer: PreparationActor | null;
  editable: boolean;
  eligibilityRuleId: string;
  excludedItemCount: number;
  pharmacists: PreparationActor[];
  items: PreparationWorkspaceItem[];
  safety: SafetyEvaluation;
  safetyAcknowledgements: SafetyAcknowledgement[];
}

export interface SafetyAcknowledgement {
  preparationTaskId: number | null;
  orderItemId: number | null;
  findingId: string;
  findingFingerprint: string;
  ruleId: string;
  rulesetVersion: string;
  user: PreparationActor;
  acknowledgedAt: string;
  sourceSnapshotStale: boolean;
}

export interface PreparationTaskInput {
  preparationVolumeMl: number | null;
  preparationNotes: string | null;
  finalContainerCount?: number;
}
