export interface PreparationLabelData {
  snapshotId: number;
  templateVersion: string;
  generatedAt: string;
  printTime: string;
  expirationAt: string | null;
  preparationId: number;
  orderId: number;
  orderReference: string;
  patientIdentifier: string;
  patientName: string | null;
  hospitalName: string | null;
  regimenName: string | null;
  treatmentAt: string | null;
  treatmentDay: string | null;
  drugCode: string;
  drugName: string;
  orderedDoseText: string | null;
  doseUnitText: string | null;
  diluentName: string | null;
  diluentVolumeMl: number | null;
  withdrawalVolumeMl: string | null;
  finalVolumeMl: number | null;
  routeName: string | null;
  infusionRateOrDuration: string | null;
  warningText: string | null;
  expiryTimeText: string | null;
  expiryStorageText: string | null;
  preparedBy: string | null;
  preparedAt: string | null;
  verifiedBy: string | null;
  verifiedAt: string;
}

export interface PreparationSummaryData {
  preparationInstructions: string | null;
  preparationNotes: string | null;
  storageReference: string | null;
  safetyReviewStatus: "verified_workflow_complete";
  inventoryPostingStatus: string | null;
  inventoryMovementId: number | null;
  containersRequired: number | null;
  inventoryBalanceBefore: number | null;
  inventoryBalanceAfter: number | null;
  inventoryStockState: "normal" | "low" | "out" | "shortage" | null;
  calculationRulesetVersion: string | null;
  calculationRuleId: string | null;
  presentationNotice: string;
}

export interface PreparationOutput {
  label: PreparationLabelData;
  containers?: PreparationContainerLabelData[];
  summary: PreparationSummaryData;
  printRequestCount: number;
}

export interface PreparationContainerLabelData {
  containerIndex: number;
}
