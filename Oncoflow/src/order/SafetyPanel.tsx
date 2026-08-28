import type { SafetyEvaluation, SafetyFinding } from "../types/safety";
import { groupSafetyFindings, unacknowledgedWarningCount } from "./safety";

export function SafetyPanel({
  evaluation,
  acknowledged,
  acknowledgementDetails = new Map(),
  loading,
  error,
  onAcknowledge,
  collapsible = false,
}: {
  evaluation: SafetyEvaluation | null;
  acknowledged: ReadonlySet<string>;
  acknowledgementDetails?: ReadonlyMap<string, string>;
  loading: boolean;
  error: string | null;
  onAcknowledge?: (findingId: string) => void;
  collapsible?: boolean;
}) {
  if (loading) {
    return <section className="safety-panel surface" aria-busy="true"><p className="eyebrow">Clinical safety</p><h2>Evaluating local legacy rules…</h2></section>;
  }
  if (error) {
    return <section className="safety-panel safety-panel--error surface" role="alert"><p className="eyebrow">Clinical safety unavailable</p><h2>Warnings could not be evaluated</h2><p>{error}</p><p>Order values remain saved and editable; no value was changed by this failure.</p></section>;
  }
  if (!evaluation) return null;
  if (evaluation.mode === "historical_not_evaluated") {
    return <section className="safety-panel safety-panel--historical surface"><p className="eyebrow">Clinical safety</p><h2>Historical order not retrospectively evaluated</h2><p>{evaluation.notice}</p><p>No current finding is presented as evidence of what appeared when this order was created.</p></section>;
  }

  const groups = groupSafetyFindings(evaluation);
  const outstanding = unacknowledgedWarningCount(groups.warnings, acknowledged);
  const details = <>
    {outstanding > 0 && <p className="safety-review-callout" role="status">{outstanding} warning{outstanding === 1 ? " requires" : "s require"} acknowledgement for this review. Editing remains available.</p>}
    {groups.warnings.length > 0 && <FindingGroup title="Warnings" className="safety-group--warning" findings={groups.warnings} acknowledged={acknowledged} acknowledgementDetails={acknowledgementDetails} onAcknowledge={onAcknowledge} />}
    {groups.information.length > 0 && <FindingGroup title="Information" findings={groups.information} acknowledged={acknowledged} acknowledgementDetails={acknowledgementDetails} onAcknowledge={onAcknowledge} />}
    {groups.unsupported.length > 0 && <FindingGroup title="Pending investigation / unsupported" className="safety-group--pending" findings={groups.unsupported} acknowledged={acknowledged} acknowledgementDetails={acknowledgementDetails} onAcknowledge={onAcknowledge} />}
    {evaluation.findings.length === 0 && <div className="safety-empty"><strong>No active findings from supported rules.</strong><span>This is not treatment approval and does not replace pharmacist review.</span></div>}
    <footer className="safety-panel__footer">Ruleset <code>{evaluation.rulesetVersion}</code> · {evaluation.evaluatedRuleCount} confirmed checks evaluated. Persisted acknowledgements apply only to an unchanged finding fingerprint and are not clinical approval.</footer>
  </>;
  const summary = <div className="safety-panel__summary"><span className="safety-count safety-count--warning">{groups.warnings.length} warning{groups.warnings.length === 1 ? "" : "s"}</span><span className="safety-count">{groups.information.length} information</span><span className="safety-count safety-count--pending">{groups.unsupported.length} pending</span></div>;

  if (collapsible) {
    return <details className="safety-panel safety-panel--collapsible surface">
      <summary className="safety-panel__toggle"><strong>Pharmacist review</strong>{summary}<span className="safety-panel__chevron" aria-hidden="true">⌄</span></summary>
      <div className="safety-panel__details"><div className="safety-panel__heading"><div><h2>Clinical safety findings</h2><p>{evaluation.notice}</p></div></div>{details}</div>
    </details>;
  }

  return <section className="safety-panel surface" aria-labelledby="safety-panel-heading">
    <div className="safety-panel__heading"><div><p className="eyebrow">Pharmacist review</p><h2 id="safety-panel-heading">Clinical safety findings</h2><p>{evaluation.notice}</p></div>{summary}</div>
    {details}
  </section>;
}

function FindingGroup({
  title,
  className = "",
  findings,
  acknowledged,
  acknowledgementDetails,
  onAcknowledge,
}: {
  title: string;
  className?: string;
  findings: SafetyFinding[];
  acknowledged: ReadonlySet<string>;
  acknowledgementDetails: ReadonlyMap<string, string>;
  onAcknowledge?: (findingId: string) => void;
}) {
  return <section className={`safety-group ${className}`}><h3>{title}</h3><div className="safety-finding-list">{findings.map((finding) => <FindingCard key={finding.id} finding={finding} acknowledged={acknowledged.has(finding.id)} acknowledgementDetail={acknowledgementDetails.get(finding.id)} onAcknowledge={onAcknowledge} />)}</div></section>;
}

function FindingCard({ finding, acknowledged, acknowledgementDetail, onAcknowledge }: { finding: SafetyFinding; acknowledged: boolean; acknowledgementDetail?: string; onAcknowledge?: (findingId: string) => void }) {
  return <details className={`safety-finding safety-finding--${finding.status}`} open={finding.severity === "warning"}>
    <summary><span>{finding.title}</span><span className="safety-finding__status">{label(finding.status)}</span></summary>
    <div className="safety-finding__body"><p>{finding.message}</p>{finding.evidence.length > 0 && <dl>{finding.evidence.map((item) => <div key={`${item.label}:${item.value}`}><dt>{item.label}</dt><dd>{item.value}</dd></div>)}</dl>}<p className="safety-finding__source"><strong>Rule:</strong> {finding.ruleId} · <strong>Version:</strong> {finding.rulesetVersion}<br/><strong>Fingerprint:</strong> <code>{finding.fingerprint.slice(0, 12)}…</code><br/><strong>Source:</strong> {finding.source}</p>{finding.acknowledgementRequired && (acknowledged ? <span className="safety-acknowledged">✓ Acknowledged{acknowledgementDetail ? ` by ${acknowledgementDetail}` : " for this review"}</span> : onAcknowledge ? <button className="button button--secondary button--compact" type="button" onClick={() => onAcknowledge(finding.id)}>Acknowledge finding</button> : <span className="safety-review-location">Review and persist this acknowledgement in the preparation workspace.</span>)}</div>
  </details>;
}

function label(status: SafetyFinding["status"]): string {
  if (status === "triggered") return "Review";
  if (status === "unsupported") return "Pending";
  return "Advisory";
}
