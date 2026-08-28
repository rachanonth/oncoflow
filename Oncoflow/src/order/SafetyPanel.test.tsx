import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { SafetyEvaluation } from "../types/safety";
import { groupSafetyFindings, unacknowledgedWarningCount } from "./safety";
import { SafetyPanel } from "./SafetyPanel";

const evaluation: SafetyEvaluation = {
  mode: "active",
  rulesetVersion: "legacy-cytotoxic-v8",
  evaluatedRuleCount: 3,
  unsupportedRuleCount: 1,
  notice: "Warnings inform review and never alter values.",
  findings: [
    {
      id: "warning-1",
      fingerprint: "fingerprint-warning-1",
      ruleId: "legacy.synthetic_threshold",
      rulesetVersion: "legacy-cytotoxic-v8",
      severity: "warning",
      status: "triggered",
      title: "Review synthetic threshold",
      message: "Observed 11 is above configured 10.",
      evidence: [{ label: "Observed", value: "11" }, { label: "Configured threshold", value: "10" }],
      source: "synthetic fixture",
      orderItemId: 1,
      acknowledgementRequired: true,
    },
    {
      id: "info-1",
      fingerprint: "fingerprint-info-1",
      ruleId: "legacy.synthetic_advisory",
      rulesetVersion: "legacy-cytotoxic-v8",
      severity: "info",
      status: "advisory",
      title: "Synthetic advisory",
      message: "Display-only information.",
      evidence: [],
      source: "synthetic fixture",
      orderItemId: null,
      acknowledgementRequired: false,
    },
    {
      id: "pending-1",
      fingerprint: "fingerprint-pending-1",
      ruleId: "legacy.synthetic_unknown",
      rulesetVersion: "legacy-cytotoxic-v8",
      severity: "info",
      status: "unsupported",
      title: "Synthetic rule pending",
      message: "No behavior was guessed.",
      evidence: [],
      source: "synthetic fixture",
      orderItemId: null,
      acknowledgementRequired: false,
    },
  ],
};

describe("SafetyPanel", () => {
  it("renders warning details, evidence, ruleset, and explicit acknowledgement", () => {
    const html = renderToStaticMarkup(<SafetyPanel evaluation={evaluation} acknowledged={new Set()} loading={false} error={null} onAcknowledge={() => undefined} />);
    expect(html).toContain("Review synthetic threshold");
    expect(html).toContain("Observed 11 is above configured 10");
    expect(html).toContain("Configured threshold");
    expect(html).toContain("legacy-cytotoxic-v8");
    expect(html).toContain("Acknowledge finding");
    expect(html).toContain("Editing remains available");
    expect(html).toContain("fingerprint-");
  });

  it("groups findings and recognizes a persisted acknowledgement", () => {
    const groups = groupSafetyFindings(evaluation);
    expect(groups.warnings).toHaveLength(1);
    expect(groups.information).toHaveLength(1);
    expect(groups.unsupported).toHaveLength(1);
    const acknowledged = new Set(["warning-1"]);
    expect(unacknowledgedWarningCount(groups.warnings, acknowledged)).toBe(0);
    const html = renderToStaticMarkup(<SafetyPanel evaluation={evaluation} acknowledged={acknowledged} acknowledgementDetails={new Map([["warning-1", "Synthetic Pharmacist · 23 Aug 2026"]])} loading={false} error={null} onAcknowledge={() => undefined} />);
    expect(html).toContain("Acknowledged by Synthetic Pharmacist");
    expect(html).toContain("unchanged finding fingerprint");
  });

  it("can collapse order-detail findings behind the three summary counts", () => {
    const html = renderToStaticMarkup(<SafetyPanel evaluation={evaluation} acknowledged={new Set()} loading={false} error={null} collapsible />);

    expect(html).toContain("<details");
    expect(html).toContain("Pharmacist review");
    expect(html).toContain("1 warning");
    expect(html).toContain("1 information");
    expect(html).toContain("1 pending");
    expect(html).not.toContain("<details open");
  });

  it("labels historical orders as not retrospectively evaluated", () => {
    const historical: SafetyEvaluation = { ...evaluation, mode: "historical_not_evaluated", findings: [], evaluatedRuleCount: 0, unsupportedRuleCount: 0, notice: "No retrospective evaluation was run." };
    const html = renderToStaticMarkup(<SafetyPanel evaluation={historical} acknowledged={new Set()} loading={false} error={null} onAcknowledge={() => undefined} />);
    expect(html).toContain("Historical order not retrospectively evaluated");
    expect(html).toContain("No current finding is presented as evidence");
    expect(html).not.toContain("Acknowledge finding");
  });
});
