import type { SafetyEvaluation, SafetyFinding } from "../types/safety";

export interface SafetyGroups {
  warnings: SafetyFinding[];
  information: SafetyFinding[];
  unsupported: SafetyFinding[];
}

export function groupSafetyFindings(evaluation: SafetyEvaluation): SafetyGroups {
  return evaluation.findings.reduce<SafetyGroups>((groups, finding) => {
    if (finding.status === "unsupported") groups.unsupported.push(finding);
    else if (finding.severity === "warning") groups.warnings.push(finding);
    else groups.information.push(finding);
    return groups;
  }, { warnings: [], information: [], unsupported: [] });
}

export function unacknowledgedWarningCount(
  findings: SafetyFinding[],
  acknowledged: ReadonlySet<string>,
): number {
  return findings.filter(
    (finding) => finding.acknowledgementRequired && !acknowledged.has(finding.id),
  ).length;
}
