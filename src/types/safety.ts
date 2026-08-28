export type SafetySeverity = "info" | "warning";
export type SafetyFindingStatus = "triggered" | "advisory" | "unsupported";
export type SafetyEvaluationMode = "active" | "historical_not_evaluated";

export interface SafetyEvidence {
  label: string;
  value: string;
}

export interface SafetyFinding {
  id: string;
  fingerprint: string;
  ruleId: string;
  rulesetVersion: string;
  severity: SafetySeverity;
  title: string;
  message: string;
  evidence: SafetyEvidence[];
  source: string;
  status: SafetyFindingStatus;
  orderItemId: number | null;
  acknowledgementRequired: boolean;
}

export interface SafetyEvaluation {
  mode: SafetyEvaluationMode;
  rulesetVersion: string;
  findings: SafetyFinding[];
  evaluatedRuleCount: number;
  unsupportedRuleCount: number;
  notice: string;
}
